//! mg-joust — TOURNAMENT (SPEC 6 #1): two passes at full gallop. Read the
//! rival's lane with ←/→, match it, and snap the lance home with SPACE in
//! the clash window. Two clean passes and the crown is thine.
//!
//! Scoring is normalized to 0..=100: graze 15, solid lance 28, perfect
//! 45, plus a crown bonus of 7 for a flawless pair.

use super::framework::{resolve, MgState, Minigame, SportSpec, SPORTS};
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BLOOD, CREAM, DARKGRAY, DARKRED, GOLD, GRASS, GRAY, GREEN, LINEN, NAVY,
    SAXON_BLUE, STONE, WHITE, YELLOW,
};
use crate::engine::input::{Input, KEY_LEFT, KEY_RIGHT, KEY_SPACE};

const PRESENT: u32 = 90; // "PRESENT ARMS" banner
const CROWN: u32 = 110; // victory / defeat tableau
const TURN: u32 = 45; // ride-back between passes
const SPEED: f32 = 56.0; // rival's approach, px/s
const START_GAP: f32 = 190.0; // rival's x lead at the list opening
const IDEAL: i32 = 26; // where the steel should meet the tilt
const PERFECT: i32 = 5; // lance snaps clean
const SOLID: i32 = 12; // a sound strike
const GRAZE: i32 = 19; // tip kisses the shield
const LANE_Y: [i32; 3] = [118, 150, 182];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Present,
    Ride,
    Clash, // brief freeze showing this pass' sparks
    Return,
    Crown,
    Done,
}

/// One tournament ride. Pure logic; the host owns pause and the cheer timer.
pub struct Joust {
    seed: u64,
    tick_n: u32,
    phase: Phase,
    phase_t: u32,
    pass: usize, // 0, then 1
    lane: usize, // player's rail, slides toward want
    want: usize,
    gap: f32, // rival distance, px; clash near IDEAL
    slide: f32, // rendered lane offset in lanes (fractional)
    thrust: Option<u32>, // tick the lance left the couched rest
    hit: [i32; 2], // per pass result: 45 perfect, 28 solid, 15 graze, -10 shorn
    sparks: u32,
    sparks_at: (i32, i32),
    score: u32,
    state: MgState,
}

fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x.wrapping_mul(0x94D0_49BB_1331_11EB)
}

impl Joust {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            tick_n: 0,
            phase: Phase::Present,
            phase_t: 0,
            pass: 0,
            lane: 1,
            want: 1,
            gap: START_GAP,
            slide: 1.0,
            thrust: None,
            hit: [0; 2],
            sparks: 0,
            sparks_at: (0, 0),
            score: 0,
            state: MgState::Running,
        }
    }

    /// The rail the rival rides this pass — readable the instant the pass
    /// opens (the game shows it too, per SPEC: aim the lance lane).
    pub fn rival_lane(&self, pass: usize) -> usize {
        (mix(self.seed ^ (pass as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)) % 3) as usize
    }

    /// Current rival distance; tests and the rail HUD both read it.
    pub fn gap(&self) -> f32 {
        self.gap
    }

    pub fn score_raw(&self) -> u32 {
        self.score
    }

    /// The rail the horse is being steered toward (HUD/test readout).
    pub fn target(&self) -> usize {
        self.want
    }

    /// Which of the two passes is running (0/1).
    pub fn pass(&self) -> usize {
        self.pass
    }

    fn lane_hit(&self) -> bool {
        (self.slide - self.rival_lane(self.pass) as f32).abs() < 0.45
    }

    /// One pass outcome from the lance commit tick: perfect by how close
    /// the gap sat to IDEAL; a lane miss takes the rival's counter (-10).
    fn judge(&mut self) {
        let g = self.gap as i32;
        let off = (g - IDEAL).abs();
        let pts = if !self.lane_hit() {
            -10 // rode the wrong list: the rival shears past
        } else if off <= PERFECT {
            45
        } else if off <= SOLID {
            28
        } else if off <= GRAZE {
            15
        } else {
            0 // away with the lance into open air
        };
        self.hit[self.pass] = pts;
        if pts > 0 {
            self.sparks = 14;
            self.sparks_at = (150, LANE_Y[self.lane] + 3);
        }
    }

    fn finish_run(&mut self, spec: &SportSpec) {
        let crowned = self.hit[0] == 45 && self.hit[1] == 45;
        let base: i32 = self.hit[0] + self.hit[1];
        let total = (base.max(0) as u32).min(100);
        self.score = if crowned { (total + 7).min(100) } else { total };
        self.state = resolve(self.score, spec);
    }
}

impl Minigame for Joust {
    fn id(&self) -> &'static str {
        "joust"
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
        self.tick_n += 1;
        self.phase_t += 1;
        if self.sparks > 0 {
            self.sparks -= 1;
        }
        match self.phase {
            Phase::Done => {}
            Phase::Present => {
                if input.key_pressed(KEY_LEFT) {
                    self.want = self.want.saturating_sub(1);
                }
                if input.key_pressed(KEY_RIGHT) {
                    self.want = (self.want + 1).min(2);
                }
                if self.phase_t >= PRESENT {
                    self.phase = Phase::Ride;
                    self.phase_t = 0;
                    self.gap = START_GAP;
                    self.thrust = None;
                }
            }
            Phase::Ride => {
                if input.key_pressed(KEY_LEFT) {
                    self.want = self.want.saturating_sub(1);
                }
                if input.key_pressed(KEY_RIGHT) {
                    self.want = (self.want + 1).min(2);
                }
                self.gap -= SPEED * dt;
                // the lance: one commit per pass; judgement waits for it
                if input.key_pressed(KEY_SPACE) && self.thrust.is_none() {
                    self.thrust = Some(self.tick_n);
                    self.judge();
                    self.phase = Phase::Clash;
                    self.phase_t = 0;
                } else if self.gap <= 2.0 {
                    // rode past steel-unthrown: if the lanes matched the
                    // rival's counter takes the plume off
                    self.hit[self.pass] = if self.lane_hit() { -10 } else { 0 };
                    self.phase = Phase::Clash;
                    self.phase_t = 0;
                }
            }
            Phase::Clash => {
                if self.phase_t >= 30 {
                    if self.pass == 0 {
                        self.pass = 1;
                        self.phase = Phase::Return;
                        self.phase_t = 0;
                    } else {
                        self.phase = Phase::Crown;
                        self.phase_t = 0;
                        self.finish_run(&SPORTS[0]);
                    }
                }
            }
            Phase::Return => {
                // the lists are chosen on the way back; slide is live
                if input.key_pressed(KEY_LEFT) {
                    self.want = self.want.saturating_sub(1);
                }
                if input.key_pressed(KEY_RIGHT) {
                    self.want = (self.want + 1).min(2);
                }
                if self.phase_t >= TURN {
                    self.phase = Phase::Ride;
                    self.phase_t = 0;
                    self.gap = START_GAP;
                    self.thrust = None;
                }
            }
            Phase::Crown => {
                if self.phase_t >= CROWN {
                    self.phase = Phase::Done;
                }
            }
        }
        // inertial slide toward the wanted rail (the horse needs a beat)
        let d = self.want as f32 - self.slide;
        self.slide += d.signum() * d.abs().min(0.06);
        if d.abs() < 0.01 {
            self.lane = self.want;
        } else {
            self.lane = self.slide.round() as usize;
        }
        self.paint(fb);
    }
}

impl Joust {
    fn paint(&self, fb: &mut Framebuffer) {
        // ---- lists: sky, crowd stand, tiltyard --------------------------
        fb.fill_rect(0, 0, 320, 34, NAVY);
        fb.fill_rect(0, 34, 320, 26, STONE); // stands
        for x in (4..316).step_by(6) {
            let c = [BLOOD, GOLD, SAXON_BLUE, LINEN][(((x / 6) as u32 + self.tick_n / 24) % 4) as usize];
            fb.fill_rect(x, 40 + (x / 6) % 3, 2, 3, c);
        }
        fb.fill_rect(0, 60, 320, 22, DARKRED); // banner rail
        for x in (0..320).step_by(16) {
            fb.fill_rect(x, 62, 8, 18, if (x / 16 + self.pass as i32) % 2 == 0 { BLOOD } else { SAXON_BLUE });
        }
        fb.fill_rect(0, 82, 320, 174, GRASS);
        // three lists (rails)
        for &y in LANE_Y.iter() {
            fb.hline(24, 300, y + 12, DARKGRAY);
            for x in (28..300).step_by(10) {
                fb.fill_rect(x, y + 8, 2, 4, STONE);
            }
        }

        // ---- rivals -----------------------------------------------------
        let ry = LANE_Y[self.rival_lane(self.pass)];
        let rx = ((86.0 + self.gap.max(0.0)) as i32).min(306);
        let py = LANE_Y[self.lane];
        if self.phase != Phase::Present && self.phase != Phase::Done {
            self.rider(fb, rx, ry, true);
            self.rider(fb, 78, py, false);
            // lances
            if self.thrust.is_some() {
                fb.hline(96, 150, py + 5, CREAM);
            } else {
                fb.hline(92, 108, py + 5, CREAM);
            }
            if self.phase == Phase::Ride {
                fb.hline(rx, rx + 18, ry + 5, CREAM);
            }
        }
        if self.sparks > 0 {
            let (sx, sy) = self.sparks_at;
            for k in 0..5 {
                fb.fill_rect(sx + k * 2 - 4 + self.sparks as i32 / 3, sy + (k - 2) * 2, 2, 2, YELLOW);
            }
        }

        // ---- HUD --------------------------------------------------------
        fb.fill_rect(0, 0, 320, 12, BLACK);
        draw_text_shadow(fb, 4, 2, "TOURNAMENT", GOLD, BLACK);
        draw_text_shadow(
            fb,
            118,
            2,
            if self.pass == 0 { "PASS 1/2" } else { "PASS 2/2" },
            WHITE,
            BLACK,
        );
        draw_text_shadow(fb, 210, 2, &format!("LANCE {}", self.score), WHITE, BLACK);
        // the lane the rival rides, telegraphed plainly (SPEC: aim the lane)
        draw_text_shadow(fb, 4, 246, "RIVAL RIDES -", WHITE, BLACK);
        for i in 0..3usize {
            fb.fill_rect(104 + i as i32 * 12, 246, 8, 8, if i == self.rival_lane(self.pass) && self.phase != Phase::Present { BLOOD } else { GRAY });
        }
        draw_text_shadow(fb, 150, 246, "SPACE AT THE TILT", WHITE, BLACK);

        // ---- phase banners ---------------------------------------------
        match self.phase {
            Phase::Present => {
                banner(fb, "PRESENT ARMS");
                banner2(fb, "ARROWS TO MATCH, SPACE TO LANCE");
            }
            Phase::Clash => {
                let h = self.hit[self.pass];
                banner(
                    fb,
                    match h {
                        45 => "THE LANCE SNAPS CLEAN",
                        28 => "A SOUND STRIKE",
                        15 => "A GRAZE",
                        -10 => "THOU ART SHORN",
                        _ => "INTO THE AIR",
                    },
                );
            }
            Phase::Return => banner(fb, "ABOUT FOR THE SECOND RIDE"),
            Phase::Crown => {
                if matches!(self.state, MgState::Won(_)) {
                    banner(fb, "THEE IS CROWNED");
                    banner2(fb, "THE LIST CRIES THY NAME");
                } else {
                    banner(fb, "OUT OF THE TILT");
                    banner2(fb, "THE RIVAL WEARS THE WREATH");
                }
            }
            _ => {}
        }
    }

    /// Blocky knight on a cantering horse; `rival` flips the facing colors.
    fn rider(&self, fb: &mut Framebuffer, x: i32, y: i32, rival: bool) {
        let tack = if rival { BLOOD } else { SAXON_BLUE };
        let bob = ((self.tick_n / 6) % 2) as i32;
        // horse
        fb.fill_rect(x - 14, y + 2 + bob, 26, 8, if rival { DARKRED } else { LINEN });
        fb.fill_rect(x + (if rival { -20 } else { 10 }), y - 2 + bob, 8, 8, if rival { DARKRED } else { LINEN });
        fb.fill_rect(x - 10, y + 10, 3, 8, if rival { DARKRED } else { LINEN });
        fb.fill_rect(x + 6, y + 10, 3, 8, if rival { DARKRED } else { LINEN });
        // knight + plume
        fb.fill_rect(x - 4, y - 10 + bob, 8, 10, GREEN);
        fb.fill_rect(x - 3, y - 15 + bob, 6, 6, CREAM);
        fb.fill_rect(x - 2, y - 17 + bob, 3, 2, tack);
        // caparison stripe
        fb.fill_rect(x - 14, y + 5 + bob, 26, 2, tack);
    }
}

fn banner(fb: &mut Framebuffer, s: &str) {
    let w = s.len() as i32 * 7 + 12;
    let x = (320 - w) / 2;
    fb.fill_rect(x, 96, w as u32, 16, BLACK);
    fb.rect(x, 96, w as u32, 16, GOLD);
    draw_text_shadow(fb, x + 6, 100, s, GOLD, BLACK);
}

fn banner2(fb: &mut Framebuffer, s: &str) {
    let w = s.len() as i32 * 7 + 12;
    let x = (320 - w) / 2;
    fb.fill_rect(x, 116, w as u32, 14, BLACK);
    draw_text_shadow(fb, x + 6, 119, s, WHITE, BLACK);
}
