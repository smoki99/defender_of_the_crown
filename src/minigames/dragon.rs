//! mg-dragon - HOARD (SPEC 6 #13, rare-event-only): one thief, one sleeping
//! dragon, one way out. Three loot sacks and the cursed crown sit on the
//! gold mound; walking over loot pockets it, and every sack slows you. The
//! dragon cycles sleep -> doze -> a sweeping eye: his gaze beam catches any
//! thief the cover does not hide, and a noisy sack (the near ones, under
//! his head) grabbed during his doze wakes him outright. The crown always
//! wakes him - that is the curse and the gamble, for only a crown-carrying
//! escape reaches the 100 the gold line demands. Woken, he roars twenty-
//! four, then hammers the floor with fire columns: he marks the thief's
//! spot, and the pillar of flame lands where you STOOD - keep moving and
//! every volley misses. Walk out the west door with a pocketful; be caught
//! by the fire and the hoard keeps you.

use super::framework::{resolve, MgState, Minigame, SPORTS};
use crate::campaign::rng::Xor;
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BROWN, DARKGRAY, DARKGREEN, DARKRED, EMBER, GOLD, GREEN, LIGHTBLUE, LINEN,
    NAVY, ORANGE, PEACH, RED, SAXON_BLUE, SILVER, TAN, WHITE, YELLOW,
};
use crate::engine::input::{Dir, Gesture, Input, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_UP};

const PRESENT: u32 = 90; // "THE DRAGON'S HOARD" banner
const CYCLE: u32 = 150; // sleep -> doze -> gaze -> settle
const DOZE_AT: u32 = 95; // eye cracks open: warning
const GAZE_AT: u32 = 115; // beam sweeps right to left
const GAZE_END: u32 = 135;
const GAZE_SPEED: f32 = 14.0; // px/tick of the sweep
const ROAR: u32 = 24; // wake-up roar before the first mark
const MARK_GAP: u32 = 45; // ticks between fire volleys
const MARK_T: u32 = 16; // ember marker telegraph
const FLAME_T: u32 = 10; // flame column duration
const FLAME_W: f32 = 9.0; // half-width of the kill zone
const STROKE_T: u32 = 12; // ticks a swipe/tap flee stroke runs

const SP0: f32 = 1.6; // unencumbered speed
const SLOW: f32 = 0.15; // speed lost per weight point - at the full hoard
                        // (weight 6) the stride must still outwalk flame:
                        // 16 * (1.6 - 6 * SLOW) > FLAME_W
const X_MIN: f32 = 8.0;
const X_MAX: f32 = 308.0;
const Y_MIN: f32 = 140.0;
const Y_MAX: f32 = 222.0;
const DOOR: f32 = 12.0; // escaping at the west door
const BAG: f32 = 10.0; // pockets loot when this close
const SACK_V: u32 = 18; // points per sack
const CROWN_V: u32 = 36; // the cursed crown
const ESCAPE_B: u32 = 10; // walking out alive

// loot: three sacks and the crown on top of the mound; sacks 1/2 sit under
// the dragon's head (NEAR) and jangle loudly if taken during his doze.
const LOOT_X: [f32; 4] = [150.0, 208.0, 266.0, 232.0];
const LOOT_Y: [f32; 4] = [200.0, 212.0, 168.0, 150.0];
const NEAR: [bool; 4] = [false, true, true, true];
// cover the gaze cannot read through: a fallen pillar and a fat butt
const COVER: [(f32, f32); 2] = [(120.0, 176.0), (86.0, 208.0)];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Present,
    Run,
    Done,
}

pub struct Dragon {
    tick_n: u32,
    phase: Phase,
    px: f32,
    py: f32,
    face: i32,
    carried: u32,
    weight: u32,
    taken: [bool; 4], // 0..2 sacks, 3 = crown
    crown: bool,
    awake: bool,
    roar: u32,
    stage: u8, // 0 stalk, 1 marked, 2 flame
    vt: u32,   // current stage countdown
    sx: f32,   // latched flee-stroke vector (T-UX swipe/tap)
    sy: f32,
    stroke: u32, // ticks left in the current stroke
    tx: f32,     // marked column
    ph0: u32,    // seeded offset into the sleep cycle
    escaped: bool,
    final_s: u32,
    state: MgState,
}

impl Dragon {
    pub fn new(seed: u64) -> Self {
        let mut rng = Xor::new(seed ^ 0x44_52_41_4E);
        Dragon {
            tick_n: 0,
            phase: Phase::Present,
            px: 22.0,
            py: 190.0,
            face: -1,
            carried: 0,
            weight: 0,
            taken: [false; 4],
            crown: false,
            awake: false,
            roar: 0,
            stage: 0,
            vt: 0,
            sx: 0.0,
            sy: 0.0,
            stroke: 0,
            tx: 0.0,
            ph0: rng.below(40),
            escaped: false,
            final_s: 0,
            state: MgState::Running,
        }
    }

    // ---- test seams ----

    /// Test seam: thief position and facing.
    #[doc(hidden)]
    pub fn pos(&self) -> (f32, f32, i32) {
        (self.px, self.py, self.face)
    }
    /// Test seam: carried value, weight, crown flag.
    #[doc(hidden)]
    pub fn loot(&self) -> (u32, u32, bool) {
        (self.carried, self.weight, self.crown)
    }
    /// Test seam: sleep phase (0 sleep, 1 doze, 2 gaze, 3 settle), gaze
    /// beam x (only meaningful during gaze) and whether he is awake.
    #[doc(hidden)]
    pub fn dragon(&self) -> (u8, f32, bool) {
        let (p, gx) = self.cycle();
        (p, gx, self.awake)
    }
    /// Test seam: volley stage (0 stalk, 1 marked, 2 flame) and mark.
    #[doc(hidden)]
    pub fn volley(&self) -> (u8, f32) {
        (self.stage, self.tx)
    }
    /// Test seam: whether cover hides the thief right now.
    #[doc(hidden)]
    pub fn hidden(&self) -> bool {
        self.in_cover()
    }
    /// Test seam: did the run end through the west door.
    #[doc(hidden)]
    pub fn escaped(&self) -> bool {
        self.escaped
    }

    fn cycle(&self) -> (u8, f32) {
        if self.tick_n < PRESENT {
            return (0, 0.0);
        }
        let c = (self.tick_n - PRESENT + self.ph0) % CYCLE;
        if c < DOZE_AT {
            (0, 0.0)
        } else if c < GAZE_AT {
            (1, 0.0)
        } else if c < GAZE_END {
            (2, 310.0 - (c - GAZE_AT) as f32 * GAZE_SPEED)
        } else {
            (3, 0.0)
        }
    }

    fn in_cover(&self) -> bool {
        COVER
            .iter()
            .any(|(cx, cy)| (self.px - cx).abs() <= 14.0 && (self.py - cy).abs() <= 16.0)
    }

    fn wake(&mut self) {
        if !self.awake {
            self.awake = true;
            self.roar = ROAR;
            self.stage = 0;
            self.vt = MARK_GAP; // roar ticks burn first; first mark lands 45 after the roar
        }
    }

    fn move_player(&mut self, input: &Input) {
        let sp = (SP0 - SLOW * self.weight as f32).max(0.7);
        // T-UX (audit: flee was keyboard-only): a swipe strides along its
        // Dir and any tap is a flee-LEFT stroke; touch has no hold, so a
        // stroke runs STROKE_T ticks at the thief's encumbered pace.
        for g in input.gestures() {
            match *g {
                Gesture::Swipe { dir: d } => {
                    let (ux, uy) = match d {
                        Dir::N => (0.0, -1.0),
                        Dir::NE => (1.0, -1.0),
                        Dir::E => (1.0, 0.0),
                        Dir::SE => (1.0, 1.0),
                        Dir::S => (0.0, 1.0),
                        Dir::SW => (-1.0, 1.0),
                        Dir::W => (-1.0, 0.0),
                        Dir::NW => (-1.0, -1.0),
                    };
                    self.sx = ux;
                    self.sy = uy;
                    self.stroke = STROKE_T;
                }
                Gesture::Tap { .. } => {
                    self.sx = -1.0;
                    self.sy = 0.0;
                    self.stroke = STROKE_T;
                }
                _ => {}
            }
        }
        let mut vx = 0.0;
        if input.key_down(KEY_LEFT) {
            vx = -sp;
            self.face = -1;
        } else if input.key_down(KEY_RIGHT) {
            vx = sp;
            self.face = 1;
        }
        let mut vy = 0.0;
        if input.key_down(KEY_UP) {
            vy = -sp * 0.7;
        } else if input.key_down(KEY_DOWN) {
            vy = sp * 0.7;
        }
        if vx != 0.0 || vy != 0.0 {
            self.stroke = 0; // a keyboard hold rules over a stroke
        } else if self.stroke > 0 {
            self.stroke -= 1;
            vx = self.sx * sp;
            vy = self.sy * sp * 0.7;
            if vx > 0.0 {
                self.face = 1;
            } else if vx < 0.0 {
                self.face = -1;
            }
        }
        self.px = (self.px + vx).clamp(X_MIN, X_MAX);
        self.py = (self.py + vy).clamp(Y_MIN, Y_MAX);
    }

    fn grabs(&mut self) {
        for i in 0..4 {
            if self.taken[i] {
                continue;
            }
            if (self.px - LOOT_X[i]).abs() > BAG || (self.py - LOOT_Y[i]).abs() > BAG + 4.0 {
                continue;
            }
            self.taken[i] = true;
            if i == 3 {
                self.crown = true;
                self.carried += CROWN_V;
                self.weight += 3; // the crown is heavy as three sacks
                self.wake(); // the crown is cursed: it always wakes him
                return;
            }
            self.carried += SACK_V;
            self.weight += 1;
            let (cyc, _) = self.cycle();
            if (cyc == 2 && !self.in_cover()) || (NEAR[i] && cyc == 1) {
                self.wake(); // jangling coins in the doze, or grabbed under his open eye
            }
        }
    }

    fn volley_tick(&mut self) {
        if !self.awake {
            let (cyc, gx) = self.cycle();
            if cyc == 2 && !self.in_cover() && (self.px - gx).abs() <= 20.0 {
                self.wake(); // caught dead to rights by the sweeping eye
            }
            return;
        }
        if self.roar > 0 {
            self.roar -= 1;
            return;
        }
        match self.stage {
            0 => {
                self.vt -= 1;
                if self.vt == 0 {
                    self.tx = self.px;
                    self.stage = 1;
                    self.vt = MARK_T;
                }
            }
            1 => {
                self.vt -= 1;
                if self.vt == 0 {
                    self.stage = 2;
                    self.vt = FLAME_T;
                }
            }
            _ => {
                self.vt -= 1;
                if (self.px - self.tx).abs() < FLAME_W {
                    self.finish(false); // the fire keeps what it marks
                    return;
                }
                if self.vt == 0 {
                    self.stage = 0;
                    self.vt = MARK_GAP;
                }
            }
        }
    }

    fn finish(&mut self, escaped: bool) {
        self.phase = Phase::Done;
        self.escaped = escaped;
        self.final_s = if escaped { self.carried + ESCAPE_B } else { 0 };
        self.state = resolve(self.final_s, &SPORTS[12]);
    }

    // ---- drawing ----

    fn draw_room(&self, fb: &mut Framebuffer) {
        // black stone hall with vault ribs
        fb.fill_rect(0, 12, 320, 244, BLACK);
        for y in (16..232).step_by(16) {
            fb.hline(0, 320, y, NAVY);
        }
        for x in (8..320).step_by(48) {
            fb.fill_rect(x, 16, 2, 216, DARKGRAY);
        }
        // west door: the way out
        fb.fill_rect(2, 150, 8, 60, BROWN);
        fb.fill_rect(2, 150, 8, 2, TAN);
        // the gold mound
        for i in 0..40i32 {
            let mx = 170 + i * 3;
            let mh = (40 - (20 - i).abs()) * 3;
            fb.fill_rect(
                mx,
                200 - mh,
                3,
                mh as u32,
                if (mx / 3 + 200 - mh) % 7 == 0 {
                    YELLOW
                } else {
                    GOLD
                },
            );
        }
        self.draw_dragon(fb);
        // loot sacks (linen bodies, gold cinch) and the crown
        for i in 0..3 {
            if self.taken[i] {
                continue;
            }
            let (lx, ly) = (LOOT_X[i] as i32, LOOT_Y[i] as i32);
            fb.fill_rect(lx - 5, ly - 10, 10, 10, LINEN);
            fb.fill_rect(lx - 5, ly - 10, 10, 2, SILVER);
            fb.fill_rect(lx - 3, ly - 12, 6, 2, GOLD);
        }
        if !self.taken[3] {
            let (cx, cy) = (LOOT_X[3] as i32, LOOT_Y[3] as i32);
            fb.fill_rect(cx - 6, cy - 2, 12, 4, GOLD);
            fb.fill_rect(cx - 6, cy - 6, 2, 4, GOLD);
            fb.fill_rect(cx - 1, cy - 7, 2, 5, GOLD);
            fb.fill_rect(cx + 4, cy - 6, 2, 4, GOLD);
            fb.fill_rect(cx - 1, cy - 1, 2, 2, RED);
        }
        // cover: the fallen pillar and a fat butt
        fb.fill_rect(116, 140, 8, 52, DARKGRAY);
        fb.fill_rect(113, 138, 14, 4, SILVER);
        fb.fill_rect(78, 196, 17, 24, BROWN);
        fb.fill_rect(78, 200, 17, 2, DARKGRAY);
        fb.fill_rect(78, 212, 17, 2, DARKGRAY);
        // torches on the walls
        for tx in [40, 296] {
            fb.fill_rect(tx, 60, 3, 10, BROWN);
            fb.fill_rect(
                tx,
                54,
                3,
                6,
                if self.tick_n % 20 < 10 { EMBER } else { ORANGE },
            );
        }
        self.draw_fire(fb);
        self.draw_player(fb);
        // HUD row
        draw_text_shadow(fb, 4, 240, &format!("LOOT {}", self.carried), WHITE, BLACK);
        let (cyc, _) = self.cycle();
        let (hint, hcol) = if self.awake {
            ("HE WAKES!", RED)
        } else if cyc == 2 {
            ("EYES OPEN", EMBER)
        } else if cyc == 1 {
            ("HE STIRS", GOLD)
        } else {
            ("SLEEPING", SILVER)
        };
        draw_text_shadow(fb, 130, 240, hint, hcol, BLACK);
        if self.tick_n.saturating_sub(PRESENT) < 180 {
            // T-UX: the text strip fades ~3 s into the heist (audit); the
            // painted west door stays as the escape's non-text mark.
            draw_text_shadow(fb, 236, 240, "FLEE LEFT", SAXON_BLUE, BLACK);
        }
    }

    fn draw_dragon(&self, fb: &mut Framebuffer) {
        // body curled over the mound, head low at the left lip facing the door
        fb.fill_rect(196, 120, 92, 34, DARKGREEN);
        for x in (200..288).step_by(6) {
            fb.fill_rect(x, 124 + (x % 12) / 3, 3, 2, GREEN);
        }
        fb.fill_rect(188, 132, 22, 16, DARKGREEN); // head
        fb.fill_rect(186, 140, 8, 6, DARKGREEN); // snout
        fb.fill_rect(196, 126, 4, 6, GREEN); // brow horn
        let (cyc, _) = self.cycle();
        if self.awake {
            fb.fill_rect(190, 136, 5, 4, EMBER); // roaring eye
                                                 // open maw with a flame gutter
            fb.fill_rect(182, 142, 6, 5, RED);
            fb.fill_rect(182, 144, 4, 3, ORANGE);
        } else if cyc == 2 {
            fb.fill_rect(190, 136, 5, 3, EMBER); // eye wide, pupil
            fb.fill_rect(192, 136, 1, 3, BLACK);
        } else if cyc == 1 {
            fb.fill_rect(190, 137, 5, 1, EMBER); // just a slit
        } else {
            fb.fill_rect(190, 138, 5, 1, BLACK); // fast asleep
                                                 // drifting Zzz
            let z = ((self.tick_n / 16) % 3) as i32;
            draw_text_shadow(fb, 210 + z * 8, 104 - z * 7, "Z", YELLOW, BLACK);
        }
        // the gaze beam sweeps right to left across the floor
        if !self.awake && cyc == 2 {
            let (_, gx) = self.cycle();
            let gxi = gx as i32;
            for y in (140..232).step_by(3) {
                fb.fill_rect(gxi, y, 1, 2, LIGHTBLUE);
            }
            fb.fill_rect(gxi - 2, 132, 5, 2, SAXON_BLUE);
        }
    }

    fn draw_fire(&self, fb: &mut Framebuffer) {
        if !self.awake || self.roar > 0 {
            return;
        }
        let txi = self.tx as i32;
        if self.stage == 1 {
            // ember marker: the floor where he AIMED
            for k in 0..2 {
                fb.fill_rect(txi - 10, 168 + k * 52, 20, 2, EMBER);
            }
            fb.fill_rect(
                txi - 1,
                148,
                2,
                84,
                if self.tick_n % 4 < 2 { EMBER } else { DARKRED },
            );
        } else if self.stage == 2 {
            // the column of flame, flickering
            for x in -9..10 {
                let h = (96 - ((x * x) / 9)) as u32;
                let c = if (x + self.tick_n as i32) % 3 == 0 {
                    YELLOW
                } else {
                    ORANGE
                };
                fb.fill_rect(txi + x, 232 - h as i32, 1, h, c);
            }
            fb.fill_rect(txi - 9, 232 - 40, 18, 40, RED);
        }
    }

    fn draw_player(&self, fb: &mut Framebuffer) {
        let (px, py) = (self.px as i32, self.py as i32);
        // legs, body, hooded head - a thief in a brown cloak
        fb.fill_rect(px - 2, py - 6, 2, 6, DARKGRAY);
        fb.fill_rect(px + 1, py - 6, 2, 6, DARKGRAY);
        fb.fill_rect(px - 3, py - 14, 7, 8, BROWN);
        fb.fill_rect(px - 2, py - 19, 5, 5, PEACH);
        fb.fill_rect(px - 3, py - 20, 7, 3, TAN); // hood
        if self.face > 0 {
            fb.fill_rect(px + 2, py - 18, 1, 2, BLACK);
        } else {
            fb.fill_rect(px - 2, py - 18, 1, 2, BLACK);
        }
        // carried loot bulges the cloak; the crown rides the hood
        for k in 0..3 {
            if self.taken[k as usize] {
                fb.fill_rect(px - 4 + k * 3, py - 12 + k, 3, 3, GOLD);
            }
        }
        if self.crown {
            fb.fill_rect(px - 3, py - 24, 7, 3, GOLD);
            fb.fill_rect(px - 1, py - 26, 2, 2, RED);
        }
    }

    fn draw_done(&self, fb: &mut Framebuffer) {
        self.draw_room(fb);
        if self.escaped {
            draw_text_shadow(fb, 84, 40, "THE HOARD IS ROBBED!", GOLD, BLACK);
        } else if self.tick_n % 30 < 18 {
            draw_text_shadow(fb, 70, 40, "THE DRAGON KEEPS HIS OWN", RED, BLACK);
        }
    }
}

impl Minigame for Dragon {
    fn id(&self) -> &'static str {
        SPORTS[12].id
    }

    fn tick(&mut self, _dt: f32, input: &Input, fb: &mut Framebuffer) {
        self.tick_n += 1;
        match self.phase {
            Phase::Present => {
                fb.fill_rect(0, 0, 320, 256, BLACK);
                crate::ui::kit::centered(fb, 116, "THE DRAGON'S HOARD", GOLD, BLACK);
                crate::ui::kit::centered(fb, 136, "SNEAK IN. GRAB. GET OUT.", WHITE, BLACK);
                if self.tick_n >= PRESENT {
                    self.phase = Phase::Run;
                }
            }
            Phase::Run => {
                self.move_player(input);
                self.grabs();
                self.volley_tick();
                if self.phase == Phase::Run && self.px <= DOOR && (150.0..216.0).contains(&self.py)
                {
                    self.finish(true);
                }
                if self.phase == Phase::Run {
                    self.draw_room(fb);
                }
            }
            Phase::Done => self.draw_done(fb),
        }
    }

    fn score(&self) -> u32 {
        if self.phase == Phase::Done {
            self.final_s
        } else {
            0 // inside the hall the score IS the loot; caught inside pays nothing
        }
    }

    fn state(&self) -> MgState {
        self.state
    }
}
