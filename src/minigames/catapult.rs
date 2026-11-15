//! mg-catapult — SIEGE (SPEC 6 #2): six stones against the wind. Dial the
//! angle with ←/→ (or drag back from the arm like a slingshot), hold SPACE
//! to swell the power meter (or drag length), release to loose. The first
//! four stones breach castle towers; the last two must drop on the sailing
//! supply ship.
//!
//! Scoring is normalized to 0..=100: tower 15, direct hit 22, ship 20,
//! broadside 30 — three clean towers and one ship hit knock on Silver's
//! door, and Gold (92) demands direct hits plus a broadside on the deck.

use super::framework::{resolve, MgState, Minigame, SportSpec, SPORTS};
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BROWN, CREAM, DARKGRAY, DEEPWATER, GOLD, GRASS, GRAY, LINEN, NAVY, ORANGE,
    RED, SILVER, STONE, TAN, WHITE, YELLOW,
};
use crate::engine::input::{Input, KEY_LEFT, KEY_RIGHT, KEY_SPACE};

pub const G: f32 = 140.0; // stone gravity, px/s^2
pub const WIND_A: f32 = 18.0; // px/s^2 per wind pip
pub const LAUNCH: (f32, f32) = (40.0, 196.0); // where the stone leaves the arm
pub const GROUND_Y: f32 = 206.0;
pub const WALL: (i32, i32) = (190, 312); // castle wall face, x span
pub const WALL_TOP: f32 = 150.0;
pub const TOWERS: [i32; 3] = [206, 250, 294];
pub const TOWER_TOP: f32 = 118.0;
pub const SEA_X: f32 = 120.0; // ship phase: open water begins here
pub const WATER_Y: f32 = 192.0;
pub const SHIP_LO: f32 = 150.0;
pub const SHIP_HI: f32 = 290.0;
pub const SHIP_V: f32 = 40.0; // px/s

pub const TOWER_BAND: f32 = 7.0; // |dx| for any tower hit
pub const DIRECT_BAND: f32 = 3.0; // |dx| for the direct-hit bonus
pub const SHIP_BAND: f32 = 10.0; // |dx| for a ship hit
pub const BROADSIDE_BAND: f32 = 4.0; // |dx| for the broadside bonus

const PRESENT: u32 = 80; // banner + instructions
const IMPACT: u32 = 34; // smoke / rubble beat before the next stone

/// Power gained per frame while SPACE is held (meter ping-pongs 0..=100).
pub const CHARGE_STEP: f32 = 1.6;

/// Outcome of a resolved stone; points per SPEC 6 #2 scaling.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Landing {
    Ground,
    Wall,
    Tower,
    Direct,
    Ship,
    Broadside,
}

impl Landing {
    fn points(self) -> u32 {
        match self {
            Landing::Tower => 15,
            Landing::Direct => 22,
            Landing::Ship => 20,
            Landing::Broadside => 30,
            _ => 0,
        }
    }
    fn word(self) -> &'static str {
        match self {
            Landing::Ground => "A MILE SHORT",
            Landing::Wall => "THUD ON THE WALL",
            Landing::Tower => "TOWER DOWN",
            Landing::Direct => "DIRECT HIT",
            Landing::Ship => "SHIP HOLED",
            Landing::Broadside => "BROADSIDE",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Present,
    Aim,
    Flight,
    Impact,
    Done,
}

fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x.wrapping_mul(0x94D0_49BB_1331_11EB)
}

/// Bresenham segment: the engine framebuffer keeps only rect/hline/vline.
fn seg(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, c: u8) {
    let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);
    for _ in 0..128 {
        fb.set(x, y, c);
        if x == x1 && y == y1 {
            return;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Launch speed from a 0..=100 power reading.
pub fn speed(power: f32) -> f32 {
    60.0 + power * 1.35
}

/// Score bands from a stone's resting x — the single judge both the live
/// game and every caller (tests, browser harness) share.
pub fn judge_x(x: f32, down: [bool; 3], ship_x: Option<f32>) -> Landing {
    if let Some(sx) = ship_x {
        let d = (x - sx).abs();
        return if d <= BROADSIDE_BAND {
            Landing::Broadside
        } else if d <= SHIP_BAND {
            Landing::Ship
        } else {
            Landing::Ground
        };
    }
    for (i, tx) in TOWERS.iter().enumerate() {
        if !down[i] && (x - *tx as f32).abs() <= TOWER_BAND {
            return if (x - *tx as f32).abs() <= DIRECT_BAND { Landing::Direct } else { Landing::Tower };
        }
    }
    if (WALL.0 as f32..=WALL.1 as f32).contains(&x) {
        Landing::Wall
    } else {
        Landing::Ground
    }
}

/// Power that carries a 45-degree stone to `target_x` in still air —
/// the closed form the aim solver starts from; `predict` verifies it.
pub fn reach_power(target_x: f32) -> f32 {
    let dx = (target_x - LAUNCH.0).max(0.0);
    (((dx * G).sqrt() - 60.0) / 1.35).clamp(0.0, 100.0)
}

/// The whole flight stepped exactly like the live integrator: where the
/// stone stops, after how many 60 Hz ticks, and what it hits. Tests and the
/// browser harness solve their aim with this, so a "perfect" scripted throw
/// is a property of the game itself, not of a re-implementation.
pub fn predict(
    angle_deg: f32,
    power: f32,
    wind: i32,
    ship_phase: bool,
    down: [bool; 3],
) -> (f32, u32, Landing) {
    let th = angle_deg.to_radians();
    let v = speed(power);
    let (mut x, mut y) = LAUNCH;
    let (mut vx, mut vy) = (v * th.cos(), -v * th.sin());
    let ax = wind as f32 * WIND_A;
    let dt = 1.0 / 60.0;
    for i in 1..=900u32 {
        let py = y;
        vx += ax * dt;
        vy += G * dt;
        x += vx * dt;
        y += vy * dt;
        if x <= 0.0 || x >= 320.0 {
            return (x, i, Landing::Ground);
        }
        if ship_phase {
            // Castle is scenery now; the sea band starts at SEA_X.
            if (x >= SEA_X && y >= WATER_Y) || (x < SEA_X && y >= GROUND_Y) {
                return (x, i, Landing::Ground); // x only; ship judged live
            }
        } else if (vy > 0.0
            && (WALL.0 as f32..=WALL.1 as f32).contains(&x)
            && y >= WALL_TOP
            && py < WALL_TOP)
            || y >= GROUND_Y
        {
            return (x, i, judge_x(x, down, None));
        }
    }
    (x, 900, Landing::Ground)
}

/// One siege. Pure logic; the host owns pause and the cheer timer.
pub struct Catapult {
    seed: u64,
    phase: Phase,
    phase_t: u32,
    shot: usize, // 0..=5
    wind: i32,   // -2..=2 for the current stone
    angle: f32,  // 10..=80 degrees
    power: f32,  // charge 0..=100 while held/aimed
    charging: bool,
    charge_dir: f32,
    drag: Option<(i32, i32)>, // pointer origin while the slingshot is held
    // stone in flight
    sx: f32,
    sy: f32,
    svx: f32,
    svy: f32,
    last: Landing,
    smoke: u32,
    smoke_at: (i32, i32),
    down: [bool; 3], // towers felled
    ship_t: u32,     // ticks since the ship phase opened (drives ship_x)
    ship_up: bool,   // seeded initial traverse direction
    score: u32,
    state: MgState,
}

impl Catapult {
    pub fn new(seed: u64) -> Self {
        let mut g = Self {
            seed,
            phase: Phase::Present,
            phase_t: 0,
            shot: 0,
            wind: 0,
            angle: 45.0,
            power: 55.0,
            charging: false,
            charge_dir: 1.0,
            drag: None,
            sx: 0.0,
            sy: 0.0,
            svx: 0.0,
            svy: 0.0,
            last: Landing::Ground,
            smoke: 0,
            smoke_at: (0, 0),
            down: [false; 3],
            ship_t: 0,
            ship_up: mix(seed ^ 0xBEEF).is_multiple_of(2),
            score: 0,
            state: MgState::Running,
        };
        g.roll_wind();
        g
    }

    fn spec(&self) -> &'static SportSpec {
        SPORTS.iter().find(|s| s.id == self.id()).unwrap()
    }

    fn roll_wind(&mut self) {
        let w = mix(self.seed ^ (self.shot as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)) % 5;
        self.wind = w as i32 - 2;
    }

    pub fn ship_phase(&self) -> bool {
        self.shot >= 4
    }

    /// Waterline x of the supply ship `t` ticks into the ship phase: a
    /// triangle wave, seeded direction. Public so harness/tests can lead
    /// the target without cheating.
    pub fn ship_x(t: u32, up: bool) -> f32 {
        let span = (SHIP_HI - SHIP_LO) / SHIP_V * 60.0; // ticks per traverse
        let mut p = t as f32 % (2.0 * span);
        if up {
            p = 2.0 * span - p;
        }
        if p < span {
            SHIP_LO + (p / span) * (SHIP_HI - SHIP_LO)
        } else {
            SHIP_HI - ((p - span) / span) * (SHIP_HI - SHIP_LO)
        }
    }

    // ---- public readouts for HUD, tests and the browser harness ----------

    pub fn wind(&self) -> i32 {
        self.wind
    }
    pub fn shot_no(&self) -> usize {
        self.shot
    }
    pub fn angle(&self) -> f32 {
        self.angle
    }
    pub fn power(&self) -> f32 {
        self.power
    }
    pub fn landed(&self) -> Landing {
        self.last
    }
    pub fn down(&self) -> [bool; 3] {
        self.down
    }
    pub fn ship_now(&self) -> f32 {
        Self::ship_x(self.ship_t, self.ship_up)
    }
    /// Where the ship will be `ahead` ticks from now — the lead solver's
    /// oracle; the harness reads it (via pixels) only to confirm.
    pub fn ship_x_in(&self, ahead: u32) -> f32 {
        Self::ship_x(self.ship_t + ahead, self.ship_up)
    }
    /// Resting x of the last resolved stone.
    pub fn last_x(&self) -> f32 {
        self.sx
    }
    /// True while the battery waits for the thrower's input.
    pub fn aiming(&self) -> bool {
        self.phase == Phase::Aim
    }

    fn loose(&mut self) {
        let th = self.angle.to_radians();
        let v = speed(self.power);
        self.sx = LAUNCH.0;
        self.sy = LAUNCH.1;
        self.svx = v * th.cos();
        self.svy = -v * th.sin();
        self.charging = false;
        self.drag = None;
        self.phase = Phase::Flight;
        self.phase_t = 0;
    }

    fn resolve_stone(&mut self) {
        let ship = if self.ship_phase() { Some(self.ship_now()) } else { None };
        self.last = judge_x(self.sx, self.down, ship);
        if matches!(self.last, Landing::Tower | Landing::Direct) {
            for (i, tx) in TOWERS.iter().enumerate() {
                if !self.down[i] && (self.sx - *tx as f32).abs() <= TOWER_BAND {
                    self.down[i] = true;
                }
            }
        }
        self.score = (self.score + self.last.points()).min(100);
        self.smoke = 26;
        self.smoke_at = (self.sx.clamp(6.0, 314.0) as i32, (self.sy - 4.0).max(14.0) as i32);
        self.phase = Phase::Impact;
        self.phase_t = 0;
    }

    fn next_stone(&mut self) {
        self.shot += 1;
        if self.shot >= 6 {
            self.state = resolve(self.score, self.spec());
            self.phase = Phase::Done;
            return;
        }
        if self.shot == 4 {
            self.ship_t = 0;
        }
        self.roll_wind();
        self.phase = Phase::Aim;
        self.phase_t = 0;
    }
}

impl Minigame for Catapult {
    fn id(&self) -> &'static str {
        "catapult"
    }

    fn score(&self) -> u32 {
        self.score
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
        if self.ship_phase() {
            self.ship_t += 1;
        }
        match self.phase {
            Phase::Present => {
                if self.phase_t >= PRESENT {
                    self.phase = Phase::Aim;
                    self.phase_t = 0;
                }
            }
            Phase::Aim => {
                if input.key_down(KEY_LEFT) {
                    self.angle = (self.angle - 0.75).max(10.0);
                }
                if input.key_down(KEY_RIGHT) {
                    self.angle = (self.angle + 0.75).min(80.0);
                }
                // SPACE hold: ping-ponging power meter
                if input.key_pressed(KEY_SPACE) {
                    self.charging = true;
                    self.charge_dir = 1.0;
                    self.power = 0.0; // every charge starts dead cold
                }
                if self.charging {
                    self.power = (self.power + self.charge_dir * CHARGE_STEP).clamp(0.0, 100.0);
                    if self.power >= 100.0 {
                        self.charge_dir = -1.0;
                    } else if self.power <= 0.0 {
                        self.charge_dir = 1.0;
                    }
                }
                if self.charging && input.key_released(KEY_SPACE) {
                    self.loose();
                }
                // slingshot: press near the arm, pull back-down-left, release
                if input.pressed() {
                    let (ox0, oy0, _) = input.mouse();
                    self.drag = Some((ox0, oy0));
                    self.charging = false;
                } else if let Some((ox, oy)) = self.drag {
                    let (mx, my, _) = input.mouse();
                    let dx = ox as f32 - mx as f32; // pull vector flipped = throw
                    let dy = oy as f32 - my as f32;
                    if dx > 2.0 {
                        self.angle = (-dy).atan2(dx).to_degrees().clamp(10.0, 80.0);
                        self.power = (dx * dx + dy * dy).sqrt().clamp(0.0, 100.0);
                    }
                    if input.released() {
                        if dx > 2.0 && input.drag_travel() >= 12 {
                            self.loose();
                        } else {
                            self.drag = None;
                        }
                    }
                }
            }
            Phase::Flight => {
                let steps = ((dt * 240.0) as u32).clamp(1, 8);
                let h = dt / steps as f32;
                for _ in 0..steps {
                    self.svx += self.wind as f32 * WIND_A * h;
                    self.svy += G * h;
                    let py = self.sy;
                    self.sx += self.svx * h;
                    self.sy += self.svy * h;
                    let mut stop = self.sx <= 0.0 || self.sx >= 320.0;
                    if self.ship_phase() {
                        // Castle is scenery now: sea band or left bank only.
                        stop |= (self.sx >= SEA_X && self.sy >= WATER_Y)
                            || (self.sx < SEA_X && self.sy >= GROUND_Y);
                    } else {
                        let wall_cross = self.svy > 0.0
                            && (WALL.0 as f32..=WALL.1 as f32).contains(&self.sx)
                            && self.sy >= WALL_TOP
                            && py < WALL_TOP;
                        stop |= self.sy >= GROUND_Y || wall_cross;
                    }
                    if stop {
                        self.resolve_stone();
                        break;
                    }
                }
            }
            Phase::Impact => {
                if self.phase_t >= IMPACT {
                    self.next_stone();
                }
            }
            Phase::Done => {}
        }
        if self.smoke > 0 {
            self.smoke -= 1;
        }
        self.paint(fb);
    }
}

impl Catapult {
    fn paint(&self, fb: &mut Framebuffer) {
        // sky / sea / ground -------------------------------------------------
        fb.fill_rect(0, 0, 320, 12, BLACK);
        fb.fill_rect(0, 12, 320, 194, NAVY);
        if self.ship_phase() {
            fb.fill_rect(SEA_X as i32, 176, (320.0 - SEA_X) as u32, 30, DEEPWATER);
            fb.fill_rect(0, GROUND_Y as i32, SEA_X as u32, 50, GRASS);
            fb.fill_rect(SEA_X as i32, GROUND_Y as i32, (320.0 - SEA_X) as u32, 50, BROWN);
        } else {
            fb.fill_rect(0, GROUND_Y as i32, 320, 50, GRASS);
        }
        // keep + wall + towers -----------------------------------------------
        fb.fill_rect(150, 132, 26, 20, STONE);
        fb.fill_rect(WALL.0, WALL_TOP as i32, (WALL.1 - WALL.0) as u32, 56, STONE);
        for x in (WALL.0..WALL.1).step_by(8) {
            fb.fill_rect(x, (WALL_TOP - 4.0) as i32, 4, 4, STONE);
        }
        for (i, tx) in TOWERS.iter().enumerate() {
            if self.down[i] {
                fb.fill_rect(*tx - 7, (WALL_TOP - 6.0) as i32, 14, 6, DARKGRAY);
            } else {
                fb.fill_rect(*tx - 6, TOWER_TOP as i32, 12, 32, STONE);
                fb.fill_rect(*tx - 7, (TOWER_TOP - 4.0) as i32, 4, 4, STONE);
                fb.fill_rect(*tx + 3, (TOWER_TOP - 4.0) as i32, 4, 4, STONE);
                fb.fill_rect(*tx - 2, (TOWER_TOP + 8.0) as i32, 4, 6, BLACK);
            }
        }
        // supply ship during the last two stones ------------------------------
        if self.ship_phase() {
            let cx = self.ship_now();
            fb.fill_rect((cx - 13.0) as i32, 184, 26, 7, BROWN);
            fb.fill_rect((cx - 13.0) as i32, 184, 26, 2, TAN);
            fb.fill_rect((cx - 1.0) as i32, 166, 2, 18, DARKGRAY);
            fb.fill_rect((cx + 1.0) as i32, 168, 8, 10, WHITE);
        }
        // catapult + aim guide -------------------------------------------------
        fb.fill_rect(28, 200, 20, 6, BROWN);
        fb.fill_rect(30, 194, 3, 8, TAN);
        let th = self.angle.to_radians();
        let (ax, ay) = (46.0 + 20.0 * th.cos(), 200.0 - 20.0 * th.sin());
        seg(fb, 46, 200, ax as i32, ay as i32, TAN);
        fb.fill_rect((ax - 2.0) as i32, (ay - 2.0) as i32, 4, 4, GRAY);
        if self.phase == Phase::Aim {
            seg(
                fb,
                ax as i32,
                ay as i32,
                (ax + 18.0 * th.cos()) as i32,
                (ay - 18.0 * th.sin()) as i32,
                YELLOW,
            );
        }
        // stone in flight + impact smoke ---------------------------------------
        if self.phase == Phase::Flight {
            fb.fill_rect(self.sx as i32, self.sy as i32, 3, 3, WHITE);
        }
        if self.smoke > 0 {
            let (px, py) = self.smoke_at;
            let r = ((26 - self.smoke) / 5) as i32;
            for (ox, oy) in [(-4, 0), (4, 0), (0, -4), (0, 4), (0, 0)] {
                fb.fill_rect((px + ox - r).max(0), (py + oy - r).max(13), 3, 3, GRAY);
            }
            let c = if self.last.points() > 0 { GOLD } else { RED };
            draw_text_shadow(fb, 96, 96, self.last.word(), c, BLACK);
        }
        // HUD -------------------------------------------------------------------
        draw_text_shadow(fb, 4, 2, "SIEGE", GOLD, BLACK);
        draw_text_shadow(fb, 64, 2, &format!("STONES {}", 6 - self.shot.min(6)), CREAM, BLACK);
        // Wind gauge (T410): pennant on the pole points downwind, a flag
        // per pip below it - readable at a glance, not a dash soup.
        fb.fill_rect(296, 2, 1, 16, WHITE);
        if self.wind > 0 {
            fb.fill_rect(297, 3, 7, 4, YELLOW);
        } else if self.wind < 0 {
            fb.fill_rect(288, 3, 7, 4, YELLOW);
        } else {
            fb.fill_rect(297, 3, 4, 2, YELLOW); // limp in a dead calm
        }
        for i in 0..self.wind.abs() {
            let y = 9 + i * 3;
            if self.wind > 0 {
                fb.fill_rect(299, y, 3, 2, SILVER);
            } else if self.wind < 0 {
                fb.fill_rect(291, y, 3, 2, SILVER);
            }
        }
        draw_text_shadow(fb, 258, 18, "WIND", SILVER, BLACK);
        if self.charging || self.drag.is_some() {
            fb.fill_rect(8, 246, 64, 6, DARKGRAY);
            let w = (self.power.clamp(0.0, 100.0) as u32 * 62 / 100).max(1);
            fb.fill_rect(9, 247, w, 4, ORANGE);
        }
        match self.phase {
            Phase::Present => {
                fb.fill_rect(36, 100, 248, 52, BLACK);
                fb.rect(36, 100, 248, 52, GOLD);
                crate::ui::kit::centered(fb, 108, "THE SIEGE", GOLD, BLACK);
                crate::ui::kit::centered(fb, 122, "DRAG BACK FROM THE ARM, OR", LINEN, BLACK);
                crate::ui::kit::centered(fb, 132, "ARROWS SET ANGLE - SPACE PUMP", LINEN, BLACK);
                crate::ui::kit::centered(fb, 143, "RELEASE TO LOOSE", CREAM, BLACK);
            }
            Phase::Aim => {
                let tip =
                    if self.ship_phase() { "SINK THE SUPPLY SHIP" } else { "BREACH THE TOWERS" };
                draw_text_shadow(fb, 104, 246, tip, GRAY, BLACK);
            }
            _ => {}
        }
    }
}
