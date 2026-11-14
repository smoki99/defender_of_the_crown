//! mg-forge — RHYTHM (SPEC 6 #9): the smithy. A red-hot blank lies on the
//! anvil and the hammer must fall on the beat, three tempos deep: SLOW,
//! FAST, FURY. A marker sweeps the bar and the ideal blow lands exactly as
//! it touches the glowing hit-line; a hair early or late still sets the
//! steel (GOOD), anything else wastes the swing and cools the temper
//! (MISS). Eighteen beats, every perfect blow and a clean chain of them
//! forge a king-worthy blade — the 95 gold line. The quality of the sword
//! *is* the reward tier: bronze work, silver work, or a gold laurel blade.
//!
//! Determinism: the pattern carries no seed variance at all — tempos and
//! windows are fixed data, so a run is a pure function of the space taps.

use super::framework::{resolve, MgState, Minigame, SPORTS};
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BLOOD, BROWN, CYAN, DARKGRAY, EMBER, GOLD, GRAY, GREEN, NAVY, SILVER, TAN,
    WHITE, YELLOW,
};
use crate::engine::input::{Input, KEY_SPACE};

const PRESENT: u32 = 90; // "THE SMITHY" banner
const PERFECT_W: i32 = 3; // |t - interval| <= 3 -> perfect
const GOOD_W: i32 = 6; // <= 6 -> good; beyond, the beat is lost
const AUTO_MISS_AT: u32 = 7; // judged this far past the line if no swing

/// (interval ticks, beats) per tempo group — SLOW, FAST, FURY.
const TEMPOS: [(u32, u32); 3] = [(40, 6), (28, 6), (18, 6)];
const TOTAL_BEATS: u32 = 18;
const W_PERFECT: u32 = 5;
const W_GOOD: u32 = 2;
const W_CLEAN: u32 = 5; // a chain without a single wasted swing

/// Which tempo group a global beat index falls in, and its interval.
fn tempo_of(beat: u32) -> (usize, u32) {
    let mut b = beat;
    for (i, (iv, n)) in TEMPOS.iter().enumerate() {
        if b < *n {
            return (i, *iv);
        }
        b -= *n;
    }
    (2, TEMPOS[2].0)
}

pub struct Forge {
    tick_n: u32,
    phase: Phase,
    score: u32,
    combo: u32,
    misses: u32,
    beat: u32,    // global beat being armed (0..TOTAL_BEATS)
    t_local: u32, // ticks since this beat was armed
    judge: Judge, // last call, for the flash text
    judge_t: u32, // ticks left of the flash
    heat: u32,    // visual: blank colour decays, perfect blows reforge it
    state: MgState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Present,
    Run,
    Done,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Judge {
    None,
    Perfect,
    Good,
    Miss,
}

impl Forge {
    pub fn new(_seed: u64) -> Self {
        Forge {
            tick_n: 0,
            phase: Phase::Present,
            score: 0,
            combo: 0,
            misses: 0,
            beat: 0,
            t_local: 0,
            judge: Judge::None,
            judge_t: 0,
            heat: 120,
            state: MgState::Running,
        }
    }

    /// Current interval of the armed beat (test seam: tempo ramp).
    #[doc(hidden)]
    pub fn current_interval(&self) -> u32 {
        if self.beat >= TOTAL_BEATS {
            return TEMPOS[2].0;
        }
        tempo_of(self.beat).1
    }

    #[doc(hidden)]
    pub fn combo(&self) -> u32 {
        self.combo
    }

    #[doc(hidden)]
    pub fn beat_no(&self) -> u32 {
        self.beat.min(TOTAL_BEATS)
    }

    fn judge_beat(&mut self, j: Judge) {
        self.judge = j;
        self.judge_t = 45;
        match j {
            Judge::Perfect => {
                self.score += W_PERFECT;
                self.combo += 1;
                self.heat = (self.heat + 14).min(160);
            }
            Judge::Good => {
                self.score += W_GOOD;
                self.combo += 1;
                self.heat = self.heat.saturating_sub(6);
            }
            Judge::Miss | Judge::None => {
                self.combo = 0;
                self.misses += 1;
                self.heat = self.heat.saturating_sub(18);
            }
        }
        self.beat += 1;
        self.t_local = 0;
        if self.beat >= TOTAL_BEATS {
            self.phase = Phase::Done;
        }
    }

    fn finish(&mut self) {
        if self.misses == 0 {
            self.score += W_CLEAN;
        }
        self.state = resolve(self.score.min(100), &SPORTS[8]);
        self.judge = Judge::None;
    }

    fn hammer(&mut self, input: &Input) {
        if !input.key_pressed(KEY_SPACE) {
            return;
        }
        let iv = self.current_interval() as i32;
        let dt = self.t_local as i32 - iv;
        let j = if dt.abs() <= PERFECT_W {
            Judge::Perfect
        } else if dt.abs() <= GOOD_W {
            Judge::Good
        } else {
            Judge::Miss // swung into air, this beat's steel is spent
        };
        self.judge_beat(j);
    }

    fn draw_run(&self, fb: &mut Framebuffer) {
        // forge room ----------------------------------------------------------
        fb.fill_rect(0, 12, 320, 244, NAVY);
        fb.fill_rect(0, 176, 320, 80, BLACK); // floor shadow
                                              // anvil on its stump
        fb.fill_rect(132, 120, 56, 10, GRAY); // face
        fb.fill_rect(144, 130, 32, 8, DARKGRAY); // waist
        fb.fill_rect(136, 138, 48, 10, BROWN); // stump
                                               // the blank: heat-coloured bar on the anvil face
        let hot = self.heat / 40; // 0..=4
        let hot_col = [DARKGRAY, BROWN, EMBER, YELLOW, WHITE][hot as usize];
        fb.fill_rect(136, 112, 48, 6, hot_col);
        // hammer head tracks the marker, arm swings on a judged blow
        let iv = self.current_interval() as i32;
        let t = (self.t_local as i32).min(iv + AUTO_MISS_AT as i32);
        let x = (60 + (t * 200) / iv.max(1)).min(261);
        let lifted = (self.t_local as i32) < iv && self.t_local.is_multiple_of(2);
        let hy = if lifted { 70 } else { 92 };
        fb.fill_rect(x - 2, 66, 4, hy - 66, TAN); // haft
        fb.fill_rect(x - 8, (hy - 8) as i32, 16, 10, GRAY); // head
        if self.judge_t > 30 && self.judge == Judge::Perfect {
            for k in 0..4 {
                fb.fill_rect(140 + k * 12, 104 - (k % 2) * 6, 3, 3, YELLOW); // sparks
            }
        }
        // tempo banner + beat pips ------------------------------------------------
        let (gi, _) = tempo_of(self.beat_no());
        let names = ["TEMPO I - SLOW", "TEMPO II - FAST", "TEMPO III - FURY"];
        let tcol = [CYAN, YELLOW, EMBER][gi];
        draw_text_shadow(
            fb,
            160 - names[gi].len() as i32 * 4,
            26,
            names[gi],
            tcol,
            BLACK,
        );
        let in_g = self.beat_no() - TEMPOS[..gi].iter().map(|t| t.1).sum::<u32>();
        for k in 0..TEMPOS[gi].1 {
            let col = if k < in_g {
                tcol
            } else if k == in_g {
                WHITE
            } else {
                DARKGRAY
            };
            fb.fill_rect(128 + k as i32 * 8, 40, 6, 6, col);
        }
        // combo ------------------------------------------------------------------
        if self.combo > 1 {
            let s = format!("x{}", self.combo);
            draw_text_shadow(fb, 268, 56, &s, GOLD, BLACK);
        }
        // rhythm bar ---------------------------------------------------------------
        fb.rect(58, 200, 208, 18, GRAY);
        fb.fill_rect(60, 202, 204, 14, BLACK);
        // hit line at the sweep end, glowing on the beat
        let glow = if (self.t_local as i32 - iv).abs() <= PERFECT_W && self.phase == Phase::Run {
            GOLD
        } else {
            GREEN
        };
        fb.fill_rect(258, 198, 4, 22, glow);
        fb.fill_rect(58, 200, 2, 18, DARKGRAY);
        // marker: the hammer column also draws onto the bar
        fb.fill_rect(x, 200, 3, 18, tcol);
        // judgment flash -----------------------------------------------------------
        if self.judge_t > 0 && self.judge != Judge::None {
            let (txt, col) = match self.judge {
                Judge::Perfect => ("PERFECT!", GOLD),
                Judge::Good => ("GOOD", SILVER),
                Judge::Miss => ("MISSED", BLOOD),
                Judge::None => ("", WHITE),
            };
            draw_text_shadow(fb, 160 - txt.len() as i32 * 4, 160, txt, col, BLACK);
        }
        if self.phase == Phase::Run && self.beat == 0 && self.t_local < 120 {
            draw_text_shadow(fb, 96, 236, "SPACE ON THE BEAT", WHITE, BLACK);
        }
    }
}

impl Minigame for Forge {
    fn id(&self) -> &'static str {
        SPORTS[8].id
    }

    fn tick(&mut self, _dt: f32, input: &Input, fb: &mut Framebuffer) {
        self.tick_n += 1;
        match self.phase {
            Phase::Present => {
                fb.fill_rect(0, 0, 320, 256, BLACK);
                draw_text_shadow(fb, 112, 116, "THE SMITHY", GOLD, BLACK);
                draw_text_shadow(fb, 88, 136, "HAMMER THE BEAT", WHITE, BLACK);
                if self.tick_n >= PRESENT {
                    self.phase = Phase::Run;
                }
                return;
            }
            Phase::Run => {
                self.hammer(input);
                if self.t_local >= self.current_interval() + AUTO_MISS_AT {
                    self.judge_beat(Judge::Miss); // the beat walked away
                }
            }
            Phase::Done => {
                if self.judge_t == 0 && self.state == MgState::Running {
                    self.finish();
                }
            }
        }
        self.t_local += 1;
        self.judge_t = self.judge_t.saturating_sub(1);
        self.heat = self.heat.saturating_sub(1);
        self.draw_run(fb);
    }

    fn score(&self) -> u32 {
        self.score
    }

    fn state(&self) -> MgState {
        self.state
    }
}
