//! mg-river — CANOE (SPEC 6 #7): pole the champion upstream to the falls.
//! Hold SPACE to bend the pole, release to vault: the leap carries a burst
//! of speed AND lifts the canoe clear of whatever the river owns — a boulder
//! gets vaulted, a pike boat gets ridden over (the crew swims). Touch either
//! from the water and the river wins that argument: wipeout, swept back.
//! Landing is its own safety: the moment the hull touches back down the
//! canoe skims — a mark met in that skim is HOPPED (credited like a vault),
//! never a capsize, and a landing in a rock's wash only shakes the canoe.
//!
//! Geometry: the world is one number — `s`, the px poled upstream. Rocks sit
//! at fixed marks on seed-chosen lanes; a pike boat sweeps its hull the full
//! width of the stream, except when a rock looms right behind it — then the
//! hull huddles in that rock's wake row, or the pair would be unclearnable.
//! All marks are constant; lanes, tempo and phase come from the seed.

use super::framework::{resolve, MgState, Minigame, SPORTS};
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BROWN, CREAM, DARKGRAY, DEEPWATER, GOLD, GRAY, GRAY3, LIGHTBLUE, NAVY,
    NORMAN_RED, SAXON_BLUE, SILVER, STONE, TAN, TEAL, WHITE,
};
use crate::engine::input::{Input, KEY_LEFT, KEY_RIGHT, KEY_SPACE};

const PRESENT: u32 = 90; // "UP THE RIVER" banner
const TABLEAU: u32 = 100; // results beat before Done
const CANOE_Y: i32 = 200; // the canoe never leaves this row
const X0: f32 = 46.0; // left bank foot
const X1: f32 = 274.0; // right bank foot
const STEER: f32 = 2.2; // px/tick
const CURRENT: f32 = 0.9; // px/tick the river gives for free
const MAX_CHARGE: u32 = 30;
const MIN_VAULT: u32 = 8;
const STUN: u32 = 45; // paddle-drift after a wipeout
const GRACE: u32 = 135; // hazard immunity from wipe to calm water
const WASH_BACK: f32 = 25.0; // px the river spits you down
const TIME_LIMIT: u32 = 3400; // dusk on the water (≈57 s)
const P_TOTAL: f32 = 2600.0; // px to the falls

// score weights — falls + every vault + every sunk boat, clean: 104 → 100.
const W_FALLS: i32 = 40;
const W_VAULT: i32 = 3;
const W_SUNK: i32 = 5;
const W_CLEAN: i32 = 20;
const W_WIPE: i32 = -8;

/// Rock marks (px upstream); the lanes are the seed's.
const ROCK_P: [f32; 8] = [400.0, 700.0, 1000.0, 1250.0, 1500.0, 1750.0, 2000.0, 2200.0];
/// Pike-boat marks (px upstream); their sweeps are the seed's.
const BOAT_P: [f32; 4] = [550.0, 1150.0, 1650.0, 1900.0];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Present,
    Run,
    Board,
    Done,
}

#[derive(Clone, Copy)]
struct Rock {
    p: f32,
    x: f32,
    settled: bool, // vaulted, drowned, or left behind
    hit: bool,    // cleared it in the air
}

#[derive(Clone, Copy)]
struct Boat {
    p: f32,
    spd: f32,
    ph: f32,
    mid: f32,  // sweep centre
    half: f32, // sweep half-width
    settled: bool,
    sunk: bool,
}

fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x.wrapping_mul(0x94D0_49BB_1331_11EB)
}

/// One CANOE run. Pure logic; the host owns pause and the cheer timer.
pub struct River {
    tick_n: u32, // ticks since Run opened
    phase: Phase,
    phase_t: u32,
    x: f32,
    s: f32,   // px poled upstream
    vel: f32, // leftover pole thrust
    charge: u32,
    prev_space: bool,
    air: u32,     // vault ticks left
    air_len: u32, // for the arc shape
    stun: u32,
    invuln: u32,
    landed: u32, // touch-down skim: marks met now are hopped, not hit
    wipes: u32,
    rocks: [Rock; 8],
    boats: [Boat; 4],
    score: i32,
    plate: &'static str,
    plate_t: u32,
    state: MgState,
}

impl River {
    pub fn new(seed: u64) -> Self {
        let rocks = std::array::from_fn(|i| Rock {
            p: ROCK_P[i],
            x: 110.0 + (mix(seed ^ (i as u64) << 21) % 101) as f32, // 110..=210
            settled: false,
            hit: false,
        });
        let boats = std::array::from_fn(|i| {
            let m = mix(seed ^ (i as u64) << 21 ^ 0xDEAD);
            // a rock lying directly behind a boat drags the hull into its
            // wake row — a full-width sweep there would make both marks at
            // once unhittable for any canoe
            let wake = ROCK_P
                .iter()
                .find(|p| **p - BOAT_P[i] > 0.0 && **p - BOAT_P[i] <= 130.0)
                .map(|p| rocks[ROCK_P.iter().position(|q| q == p).unwrap()].x);
            let (mid, half) = match wake {
                Some(x) => (x, 20.0),
                None => ((X0 + X1) / 2.0, (X1 - X0) / 2.0),
            };
            Boat {
                p: BOAT_P[i],
                spd: 0.7 + (m % 50) as f32 / 100.0, // 0.7..1.2 px/t
                ph: ((m >> 32) % 456) as f32,
                mid,
                half,
                settled: false,
                sunk: false,
            }
        });
        Self {
            tick_n: 0,
            phase: Phase::Present,
            phase_t: 0,
            x: 160.0,
            s: 0.0,
            vel: 0.0,
            charge: 0,
            prev_space: false,
            air: 0,
            air_len: 1,
            stun: 0,
            invuln: 0,
            landed: 0,
            wipes: 0,
            rocks,
            boats,
            score: 0,
            plate: "",
            plate_t: 0,
            state: MgState::Running,
        }
    }

    // test/host introspection ------------------------------------------------
    pub fn scroll(&self) -> f32 {
        self.s
    }
    pub fn canoe_x(&self) -> f32 {
        self.x
    }
    pub fn ticks(&self) -> u32 {
        self.tick_n
    }
    pub fn charge(&self) -> u32 {
        self.charge
    }
    pub fn air(&self) -> u32 {
        self.air
    }
    pub fn stun(&self) -> u32 {
        self.stun
    }
    pub fn wipes(&self) -> u32 {
        self.wipes
    }
    pub fn vel(&self) -> f32 {
        self.vel
    }
    pub fn rock(&self, i: usize) -> (f32, f32, bool, bool) {
        (self.rocks[i].p, self.rocks[i].x, self.rocks[i].settled, self.rocks[i].hit)
    }
    pub fn boat(&self, i: usize) -> (f32, bool, bool) {
        (self.boats[i].p, self.boats[i].settled, self.boats[i].sunk)
    }
    /// Where boat `i`'s hull sits at absolute tick `t` — fully predictable.
    pub fn boat_x(&self, i: usize, t: u32) -> f32 {
        let b = &self.boats[i];
        let m = (b.spd * t as f32 + b.ph) % (4.0 * b.half);
        if m < 2.0 * b.half {
            b.mid - b.half + m
        } else {
            b.mid + b.half - (m - 2.0 * b.half)
        }
    }
    /// `(centre, half-width)` of boat `i`'s sweep.
    pub fn boat_sweep(&self, i: usize) -> (f32, f32) {
        (self.boats[i].mid, self.boats[i].half)
    }

    #[doc(hidden)] // test seam: drop the canoe mid-river
    pub fn debug_set_scroll(&mut self, s: f32) {
        self.s = s;
    }

    fn say(&mut self, p: &'static str) {
        self.plate = p;
        self.plate_t = 60;
    }

    fn finish(&mut self) {
        self.score = self.score.clamp(0, 100);
        self.state = resolve(self.score as u32, &SPORTS[6]);
        self.phase = Phase::Board;
        self.phase_t = 0;
    }

    fn wipeout(&mut self) {
        self.wipes += 1;
        self.score += W_WIPE;
        self.stun = STUN;
        self.invuln = GRACE;
        self.vel = 0.0;
        self.charge = 0;
        self.air = 0;
        self.s = (self.s - WASH_BACK).max(0.0);
        self.say("WIPEOUT!");
    }

    fn steer(&mut self, input: &Input) {
        if self.stun > 0 {
            return;
        }
        let mut dx = 0.0;
        if input.key_down(KEY_LEFT) {
            dx -= STEER;
        }
        if input.key_down(KEY_RIGHT) {
            dx += STEER;
        }
        self.x = (self.x + dx).clamp(X0, X1);
    }

    fn pole(&mut self, input: &Input) {
        let held = input.key_down(KEY_SPACE);
        if self.stun > 0 || self.air > 0 {
            self.charge = 0;
            self.prev_space = held;
            return;
        }
        if held {
            self.charge = (self.charge + 1).min(MAX_CHARGE);
        } else {
            if self.prev_space && self.charge >= MIN_VAULT {
                let c = self.charge;
                self.vel = (self.vel + 0.09 * c as f32).min(3.2);
                self.air = 26 + c;
                self.air_len = self.air;
            }
            self.charge = 0;
        }
        self.prev_space = held;
    }

    fn hazards(&mut self) {
        // A mark bites only from the water: airborne (or still skimming from
        // the last touch-down) it is hopped for points; otherwise the river
        // wins the argument — unless the swim-wash still shields the pole.
        for i in 0..self.rocks.len() {
            let r = &mut self.rocks[i];
            if r.settled {
                continue;
            }
            if self.s > r.p + 9.0 {
                r.settled = true;
                continue;
            }
            if (self.s - r.p).abs() <= 9.0 && (self.x - r.x).abs() < 13.0 {
                if self.air > 0 || self.landed > 0 {
                    r.settled = true;
                    r.hit = true;
                    self.score += W_VAULT;
                    self.say("CLEAN VAULT!");
                } else if self.invuln == 0 {
                    r.settled = true;
                    self.wipeout();
                    return;
                }
            }
        }
        for i in 0..self.boats.len() {
            if self.boats[i].settled {
                continue;
            }
            if self.s > self.boats[i].p + 9.0 {
                self.boats[i].settled = true;
                continue;
            }
            let hull = self.boat_x(i, self.tick_n);
            let b = &mut self.boats[i];
            if (self.s - b.p).abs() <= 9.0 && (self.x - hull).abs() < 15.0 {
                if self.air > 0 || self.landed > 0 {
                    b.settled = true;
                    b.sunk = true;
                    self.score += W_SUNK;
                    self.say("PIKE SUNK!");
                } else if self.invuln == 0 {
                    b.settled = true;
                    self.wipeout();
                    return;
                }
            }
        }
    }

    fn run(&mut self, input: &Input) {
        self.tick_n += 1;
        if self.invuln > 0 {
            self.invuln -= 1;
        }
        if self.landed > 0 {
            self.landed -= 1;
        }
        if self.stun > 0 {
            self.stun -= 1;
        }
        if self.air > 0 {
            self.air -= 1;
            if self.air == 0 {
                // carve recovery: landing never flips the canoe
                self.invuln = self.invuln.max(14);
                self.landed = 14;
            }
        }
        self.steer(input);
        self.pole(input);
        self.s += CURRENT + self.vel;
        self.vel = (self.vel - 0.018).max(0.0);
        self.hazards();

        if self.s >= P_TOTAL {
            self.score += W_FALLS;
            if self.wipes == 0 {
                self.score += W_CLEAN;
            }
            self.say("THE FALLS!");
            self.finish();
            return;
        }
        if self.tick_n >= TIME_LIMIT {
            self.say("DUSK ON THE WATER");
            self.finish();
        }
    }
}

impl Minigame for River {
    fn id(&self) -> &'static str {
        "river"
    }

    fn score(&self) -> u32 {
        (self.score.max(0) as u32).min(100)
    }

    fn state(&self) -> MgState {
        self.state
    }

    fn tick(&mut self, dt: f32, input: &Input, fb: &mut Framebuffer) {
        if dt <= 0.0 {
            self.paint(fb);
            return;
        }
        self.phase_t += 1;
        if self.plate_t > 0 {
            self.plate_t -= 1;
        }
        match self.phase {
            Phase::Done => {}
            Phase::Present => {
                if self.phase_t >= PRESENT {
                    self.phase = Phase::Run;
                    self.phase_t = 0;
                }
            }
            Phase::Run => self.run(input),
            Phase::Board => {
                if self.phase_t >= TABLEAU {
                    self.phase = Phase::Done;
                }
            }
        }
        self.paint(fb);
    }
}

impl River {
    fn paint(&self, fb: &mut Framebuffer) {
        let so = self.s as i32;
        // water ----------------------------------------------------------------
        fb.fill_rect(0, 0, 320, 256, DEEPWATER);
        for y in (0..256).step_by(3) {
            if (y + so) % 24 < 2 {
                fb.hline(28, 292, y, NAVY);
            }
        }
        for y in (4..256).step_by(7) {
            let seed = (y as u32).wrapping_mul(2654435761).wrapping_add(so as u32 * 97);
            let xx = 36 + ((seed >> 13) % 244) as i32;
            if (seed >> 25) & 3 == 0 {
                fb.fill_rect(xx, y, 2, 1, TEAL);
            } else if (seed >> 25) & 7 == 0 {
                fb.fill_rect(xx, y, 1, 1, LIGHTBLUE);
            }
        }
        // banks ------------------------------------------------------------------
        for y in 0..256 {
            let j = ((((y + so) as u32).wrapping_mul(2246822519)) >> 22) % 5;
            let el = 30 + j as i32; // left bank edge
            let er = 290 - j as i32;
            fb.fill_rect(0, y, el as u32, 1, STONE);
            fb.fill_rect(er, y, (320 - er) as u32, 1, STONE);
            if (y + so) % 16 == 0 {
                fb.fill_rect(2, y, (el - 4) as u32, 1, GRAY);
                fb.fill_rect(er + 2, y, (316 - er) as u32, 1, GRAY);
            }
            if (y + so) % 8 < 2 {
                fb.fill_rect(el, y, 1, 1, WHITE);
                fb.fill_rect(er - 1, y, 1, 1, WHITE);
            }
            if (y + so) % 64 == 5 {
                fb.fill_rect(el - 4, y, 3, 1, TEAL); // bank reeds
                fb.fill_rect(er + 2, y, 3, 1, TEAL);
            }
        }
        // the falls approach --------------------------------------------------------
        let to_falls = P_TOTAL - self.s;
        if to_falls < 240.0 {
            let fy = CANOE_Y - to_falls as i32;
            fb.fill_rect(28, fy - 8, 264, 9, WHITE);
            fb.fill_rect(28, fy - 8, 264, 2, SILVER);
            for k in 0..264 {
                if (k * 13 + so) % 17 < 4 {
                    fb.fill_rect(28 + k, fy + 1 + (k % 3), 1, 1, LIGHTBLUE);
                }
            }
        }
        // rocks ----------------------------------------------------------------------
        for r in self.rocks.iter() {
            let dy = r.p - self.s;
            if !(-30.0..=230.0).contains(&dy) {
                continue;
            }
            let y = CANOE_Y - dy as i32;
            let x = r.x as i32;
            fb.fill_rect(x - 10, y - 6, 20, 12, GRAY3);
            fb.fill_rect(x - 8, y - 6, 10, 3, SILVER);
            fb.fill_rect(x - 10, y + 6, 20, 2, WHITE); // foam
        }
        // pike boats --------------------------------------------------------------------
        for i in 0..self.boats.len() {
            let b = &self.boats[i];
            let dy = b.p - self.s;
            if !(-30.0..=230.0).contains(&dy) {
                continue;
            }
            let y = CANOE_Y - dy as i32;
            let x = self.boat_x(i, self.tick_n) as i32;
            if b.sunk {
                fb.fill_rect(x - 8, y + 1, 16, 3, DARKGRAY); // drowned silhouette
                continue;
            }
            fb.fill_rect(x - 9, y, 18, 4, BROWN);
            fb.fill_rect(x - 11, y + 1, 2, 2, BROWN);
            fb.fill_rect(x + 9, y + 1, 2, 2, BROWN);
            fb.fill_rect(x - 1, y - 7, 1, 7, BLACK); // mast
            fb.fill_rect(x, y - 7, 4, 3, NORMAN_RED); // pennant
            fb.fill_rect(x + 10, y - 2, 6, 1, SILVER); // pike
            fb.fill_rect(x + 4, y - 3, 3, 3, NORMAN_RED); // crew
        }
        // the canoe ------------------------------------------------------------------------
        let k = self.air as f32 / self.air_len as f32;
        let lift = if self.air > 0 { (4.0 * k * (1.0 - k) * 8.0) as i32 } else { 0 };
        let cx = self.x as i32;
        let cy = CANOE_Y;
        let blink = self.invuln > 0 && self.stun > 0 && self.tick_n % 6 < 3;
        if !blink {
            if lift > 0 {
                fb.fill_rect(cx - 6, cy + 3, 12, 2, DARKGRAY); // shadow
            }
            let hy = cy - lift;
            fb.fill_rect(cx - 8, hy, 16, 4, TAN);
            fb.fill_rect(cx - 8, hy + 3, 16, 1, BROWN);
            fb.fill_rect(cx - 10, hy + 1, 2, 2, TAN);
            fb.fill_rect(cx + 8, hy + 1, 2, 2, TAN);
            fb.fill_rect(cx - 2, hy - 6, 4, 6, SAXON_BLUE);
            fb.fill_rect(cx - 1, hy - 8, 2, 2, CREAM);
            if self.charge > 0 {
                // the bent pole reaching downstream
                for t in 0..10 {
                    fb.fill_rect(cx + 2 + t, hy - 4 + t + (self.charge as i32 / 6), 1, 1, SILVER);
                }
            }
        }
        // HUD ---------------------------------------------------------------------------------
        draw_text_shadow(fb, 4, 2, "CANOE", GOLD, BLACK);
        let rem = (P_TOTAL - self.s).max(0.0) as i32;
        draw_text_shadow(fb, 116, 2, &format!("FALLS {rem}"), CREAM, BLACK);
        draw_text_shadow(
            fb,
            250,
            2,
            &format!("WIPE {}", self.wipes),
            if self.wipes > 0 { NORMAN_RED } else { CREAM },
            BLACK,
        );
        // pole power + progress gauge
        fb.fill_rect(6, 60, 4, 62, DARKGRAY);
        let h = (self.charge * 2) as i32;
        fb.fill_rect(6, 122 - h, 4, h as u32, GOLD);
        fb.fill_rect(306, 20, 3, 202, DARKGRAY);
        let ph = ((self.s / P_TOTAL) * 200.0) as i32;
        fb.fill_rect(306, 220 - ph, 3, ph.max(1) as u32, GOLD);
        fb.fill_rect(304, 18, 7, 3, WHITE); // the falls mark
        if self.plate_t > 0 {
            let c = if self.plate == "WIPEOUT!" { NORMAN_RED } else { GOLD };
            draw_text_shadow(fb, 104, 120, self.plate, c, BLACK);
        }
        match self.phase {
            Phase::Present => {
                if self.phase_t % 24 < 16 {
                    draw_text_shadow(fb, 100, 128, "UP THE RIVER", WHITE, BLACK);
                }
            }
            Phase::Board | Phase::Done => {
                let s = self.score.max(0) as u32;
                draw_text_shadow(fb, 110, 60, &format!("SCORE {s}"), GOLD, BLACK);
                let msg = match self.state {
                    MgState::Lost => "THE RIVER WINS",
                    _ => "THE FALLS CLEARED",
                };
                let col = if self.state == MgState::Lost { NORMAN_RED } else { GOLD };
                draw_text_shadow(fb, 96, 84, msg, col, BLACK);
            }
            _ => {}
        }
    }
}
