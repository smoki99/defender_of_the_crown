//! mg-archery — GALLERY (SPEC 6 #4): the butts at the tournament field.
//! Steer the drawn arrow with the arrows keys and loose it with SPACE.
//! Targets rise from the booths; the fool runs along the front and an apple
//! swings on a silk from the gantry. Ten targets, one apple, fourteen
//! shafts — a clean sweep plus the apple is the only Gold.

use super::framework::{resolve, MgState, Minigame, SportSpec, SPORTS};
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BLOOD, BROWN, CREAM, GOLD, GREEN, LINEN, NAVY, ORANGE,
    RED, SAXON_BLUE, TAN, WHITE, YELLOW,
};
use crate::engine::input::{Input, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_SPACE, KEY_UP};

const PRESENT: u32 = 90; // "NOCK ARROWS" banner
const TABLEAU: u32 = 100; // results beat before Done
const VOLLEY: u32 = 48; // ticks per volley
const VOLLEYS: u32 = 10;
const POP_AT: u32 = 6; // within a volley: target starts rising
const UP_AT: u32 = 12; // fully up (hittable) ...
const DOWN_AT: u32 = 34; // ... holds, then begins to duck
const GONE_AT: u32 = 42;
const HOLES: [(i32, i32); 8] = [
    (72, 120),
    (128, 120),
    (184, 120),
    (240, 120),
    (72, 168),
    (128, 168),
    (184, 168),
    (240, 168),
];
const FOOL_Y: f32 = 196.0;
const FOOL_SPEED: f32 = 2.2; // px per tick along the front
const ARROWS: u32 = 14;
const HIT_R: f32 = 16.0; // target boss, px
const FOOL_R: f32 = 11.0;
const APPLE_R: f32 = 9.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Present,
    Run,
    Board,
    Done,
}

/// One GALLERY run. Pure logic; the host owns pause and the cheer timer.
pub struct Archery {
    seed: u64,
    tick_n: u32, // ticks since Run opened
    phase: Phase,
    phase_t: u32,
    cross: (f32, f32), // drawn arrow tip
    arrows: u32,
    hits: u32,
    hit: u64, // volley*4+slot bits for targets already collected
    fool_x: f32,
    fool_dir: f32,
    apple_up: bool, // still swinging (unshot)
    plate: &'static str,
    plate_t: u32,
    score: i32,
    state: MgState,
}

fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x.wrapping_mul(0x94D0_49BB_1331_11EB)
}

impl Archery {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            tick_n: 0,
            phase: Phase::Present,
            phase_t: 0,
            cross: (160.0, 140.0),
            arrows: ARROWS,
            hits: 0,
            hit: 0,
            fool_x: 20.0,
            fool_dir: 1.0,
            apple_up: true,
            plate: "",
            plate_t: 0,
            score: 0,
            state: MgState::Running,
        }
    }

    pub fn cross(&self) -> (f32, f32) {
        self.cross
    }
    pub fn arrows(&self) -> u32 {
        self.arrows
    }
    pub fn hits(&self) -> u32 {
        self.hits
    }
    /// Raw running total (may dip below zero after fool penalties).
    pub fn tally(&self) -> i32 {
        self.score
    }
    pub fn fool_x(&self) -> f32 {
        self.fool_x
    }
    pub fn volley(&self) -> u32 {
        (self.tick_n / VOLLEY).min(VOLLEYS)
    }
    /// Ticks since the run opened (drives volley-local math).
    pub fn tick(&self) -> u32 {
        self.tick_n
    }
    /// World position of hole `h`.
    pub fn hole_xy(h: usize) -> (f32, f32) {
        let (x, y) = HOLES[h];
        (x as f32, y as f32)
    }
    /// Which holes volley `v` will pop, in slot order (None = idle slot).
    pub fn scheduled(&self, v: u32) -> [Option<usize>; 3] {
        let n = if v >= 6 { 3 } else if v >= 3 { 2 } else { 1 };
        let mut out = [None; 3];
        for s in 0..n {
            out[s] = Some(self.hole(v, s as u32));
        }
        out
    }

    /// The hole popped for volley `v`, slot `s` (0..3, at most 3 distinct).
    fn hole(&self, v: u32, s: u32) -> usize {
        (mix(self.seed ^ (v as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (s as u64).wrapping_mul(0x85EB_CA6B))
            % 8) as usize
    }

    /// How high each hole's target shows right now (0.0 = gone, 1.0 = up).
    pub fn targets(&self) -> [(usize, f32); 3] {
        if self.phase != Phase::Run || self.tick_n >= VOLLEYS * VOLLEY {
            return [(0, 0.0); 3];
        }
        let v = self.volley();
        let local = self.tick_n % VOLLEY;
        let lift = |start: u32| -> f32 {
            if local < start || local >= start + GONE_AT - POP_AT {
                0.0
            } else if local < start + UP_AT - POP_AT {
                (local - start) as f32 / (UP_AT - POP_AT) as f32
            } else if local < start + DOWN_AT - POP_AT {
                1.0
            } else {
                1.0 - (local - (start + DOWN_AT - POP_AT)) as f32 / (GONE_AT - DOWN_AT) as f32
            }
        };
        let mut out = [(0usize, 0f32); 3];
        let raw = [
            (self.hole(v, 0), lift(POP_AT)),
            (
                self.hole(v, 1),
                if v >= 3 { lift(POP_AT + 12) } else { 0.0 },
            ),
            (
                self.hole(v, 2),
                if v >= 6 { lift(POP_AT + 24) } else { 0.0 },
            ),
        ];
        for s in 0..3 {
            out[s] = if self.struck(v, s) { (raw[s].0, 0.0) } else { raw[s] };
        }
        out
    }

    /// The apple's live position on the gantry, or `None` once loosed.
    pub fn apple(&self) -> Option<(f32, f32)> {
        if !self.apple_up || self.phase != Phase::Run || self.volley() < 3 {
            return None;
        }
        let th = (self.tick_n as f32 * std::f32::consts::TAU / 90.0).sin() * 0.9;
        Some((160.0 + 34.0 * th.sin(), 74.0 - 8.0 * th.cos()))
    }
    /// A struck target is remembered for the rest of its volley.
    fn struck(&self, v: u32, slot: usize) -> bool {
        (self.hit >> (v * 4 + slot as u32)) & 1 == 1
    }
    fn mark_hit(&mut self, v: u32, slot: usize) {
        self.hit |= 1 << (v * 4 + slot as u32);
    }

    fn loose(&mut self) {
        if self.arrows == 0 {
            self.plate = "OUT OF SHAFTS";
            self.plate_t = 30;
            return;
        }
        self.arrows -= 1;
        let (cx, cy) = self.cross;
        let d = |x: f32, y: f32| ((cx - x).powi(2) + (cy - y).powi(2)).sqrt();
        if let Some((ax, ay)) = self.apple() {
            if d(ax, ay) <= APPLE_R {
                self.apple_up = false;
                self.score += 25;
                self.plate = "THE APPLE!";
                self.plate_t = 30;
                return;
            }
        }
        let v = self.volley();
        for (slot, (h, vis)) in self.targets().into_iter().enumerate() {
            if vis >= 0.6 {
                let (hx, hy) = HOLES[h];
                if d(hx as f32, hy as f32) <= HIT_R {
                    self.mark_hit(v, slot);
                    self.hits += 1;
                    self.score += 8;
                    self.plate = "BULLSEYE";
                    self.plate_t = 30;
                    return;
                }
            }
        }
        if d(self.fool_x, FOOL_Y) <= FOOL_R {
            self.score -= 15;
            self.plate = "THE CROWD BOOS";
            self.plate_t = 30;
        } else {
            self.plate = "A WASTED SHAFT";
            self.plate_t = 30;
        }
    }

    fn finish_run(&mut self, spec: &SportSpec) {
        self.score = self.score.clamp(0, 100);
        self.state = resolve(self.score as u32, spec);
    }
}

impl Minigame for Archery {
    fn id(&self) -> &'static str {
        "archery"
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
            Phase::Run => {
                self.tick_n += 1;
                // steer the drawn shaft
                let sp = 9.0;
                if input.key_down(KEY_LEFT) {
                    self.cross.0 -= sp;
                }
                if input.key_down(KEY_RIGHT) {
                    self.cross.0 += sp;
                }
                if input.key_down(KEY_UP) {
                    self.cross.1 -= sp;
                }
                if input.key_down(KEY_DOWN) {
                    self.cross.1 += sp;
                }
                self.cross.0 = self.cross.0.clamp(8.0, 312.0);
                self.cross.1 = self.cross.1.clamp(56.0, 210.0);
                // the fool trots the front rail, bouncing
                self.fool_x += FOOL_SPEED * self.fool_dir;
                if self.fool_x < 18.0 {
                    self.fool_dir = 1.0;
                } else if self.fool_x > 302.0 {
                    self.fool_dir = -1.0;
                }
                if input.key_pressed(KEY_SPACE) {
                    self.loose();
                }
                if self.tick_n >= VOLLEYS * VOLLEY {
                    self.phase = Phase::Board;
                    self.phase_t = 0;
                    self.finish_run(&SPORTS[3]);
                }
            }
            Phase::Board => {
                if self.phase_t >= TABLEAU {
                    self.phase = Phase::Done;
                }
            }
        }
        self.paint(fb);
    }
}

impl Archery {
    fn paint(&self, fb: &mut Framebuffer) {
        fb.fill_rect(0, 0, 320, 256, NAVY);
        fb.fill_rect(0, 200, 320, 56, GREEN); // turf the fool trots on
        for x in (6..316).step_by(12) {
            fb.fill_rect(x, 44, 6, 6, [BLOOD, GOLD, SAXON_BLUE, LINEN][((x / 12) % 4) as usize]);
        }
        fb.fill_rect(14, 92, 292, 108, LINEN); // booth wall
        fb.fill_rect(14, 92, 292, 3, BROWN);
        fb.fill_rect(14, 197, 292, 3, BROWN);
        // holes + targets ----------------------------------------------------
        for (hx, hy) in HOLES {
            fb.fill_rect(hx - 9, hy - 11, 19, 22, BLACK);
        }
        for (h, vis) in self.targets() {
            if vis <= 0.0 {
                continue;
            }
            let (hx, hy) = HOLES[h];
            let rise = ((1.0 - vis) * 14.0) as i32;
            let ty = hy + rise;
            for y in 0..11 {
                for x in 0..11 {
                    let dx = x - 5;
                    let dy = y - 5;
                    if dx * dx + dy * dy <= 25 {
                        fb.fill_rect(hx + dx, ty + dy, 1, 1, BLOOD);
                    }
                }
            }
            fb.fill_rect(hx - 2, ty - 2, 4, 4, GOLD);
        }
        // the apple on its silk ----------------------------------------------
        if let Some((ax, ay)) = self.apple() {
            let (ax, ay) = (ax as i32, ay as i32);
            let mut t = 0.0;
            while t < 1.0 {
                fb.fill_rect((160.0 + (ax as f32 - 160.0) * t) as i32, (66.0 + (ay as f32 - 66.0) * t) as i32, 1, 1, WHITE);
                t += 0.1;
            }
            fb.fill_rect(ax - 3, ay - 2, 6, 6, RED);
            fb.fill_rect(ax + 1, ay - 4, 3, 2, GREEN);
        }
        // the fool trotting the front ----------------------------------------
        if self.phase == Phase::Run || self.phase == Phase::Board {
            let fx = self.fool_x as i32;
            fb.fill_rect(fx - 3, FOOL_Y as i32 - 2, 6, 7, SAXON_BLUE);
            fb.fill_rect(fx - 2, FOOL_Y as i32 - 6, 4, 4, ORANGE);
            fb.fill_rect(fx - 4, FOOL_Y as i32 - 9, 3, 3, YELLOW);
            fb.fill_rect(fx + 1, FOOL_Y as i32 - 9, 3, 3, RED);
        }
        // the drawn shaft (crosshair) ----------------------------------------
        if self.phase != Phase::Board && self.phase != Phase::Done {
            let (cx, cy) = (self.cross.0 as i32, self.cross.1 as i32);
            fb.fill_rect(cx - 5, cy - 1, 11, 1, BLACK);
            fb.fill_rect(cx - 1, cy - 5, 1, 11, BLACK);
            fb.fill_rect(cx - 4, cy, 9, 1, WHITE);
            fb.fill_rect(cx, cy - 4, 1, 9, WHITE);
            fb.fill_rect(cx, cy, 1, 1, RED);
        }
        // HUD -----------------------------------------------------------------
        draw_text_shadow(fb, 4, 2, "GALLERY", GOLD, BLACK);
        draw_text_shadow(fb, 72, 2, &format!("SHAFTS {}", self.arrows), CREAM, BLACK);
        draw_text_shadow(fb, 240, 2, &format!("HITS {}/10", self.hits), CREAM, BLACK);
        if self.plate_t > 0 {
            let c = if self.plate == "BULLSEYE" || self.plate == "THE APPLE!" {
                GOLD
            } else if self.plate == "THE CROWD BOOS" {
                RED
            } else {
                TAN
            };
            draw_text_shadow(fb, 96, 96, self.plate, c, BLACK);
        }
        match self.phase {
            Phase::Present => {
                if self.phase_t % 24 < 16 {
                    draw_text_shadow(fb, 96, 128, "NOCK ARROWS", WHITE, BLACK);
                }
            }
            Phase::Board => {
                let s = self.score.max(0) as u32;
                draw_text_shadow(fb, 110, 60, &format!("SCORE {}", s), GOLD, BLACK);
                draw_text_shadow(fb, 60, 84, &format!("{} TARGETS + APPLE", self.hits), CREAM, BLACK);
            }
            Phase::Done => {
                let s = self.score.max(0) as u32;
                draw_text_shadow(fb, 110, 60, &format!("SCORE {}", s), GOLD, BLACK);
            }
            _ => {}
        }
    }
}
