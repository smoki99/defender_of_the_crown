//! mg-herbs — MEMORY (SPEC 6 #10): the herb salve. Sixteen face-down
//! tiles hide eight kinds of herb; click two to turn them, match the kind
//! and the pair drops into the cauldron. The pot is on the boil from the
//! first turn and the brew boils over at the end of `BOWL` ticks - every
//! pair matched in time scores, and a *fast* flawless brew rides the
//! remaining boil up into the 90+ gold band. A kind seen stays seen: the
//! test of the sport is memory, not luck, and the shuffle is the only
//! seed variance.
//!
//! Geometry: the 4x4 herb grid of 36 px tiles is fixed; the mouse picks a
//! tile, clicks flip. A single revealed tile stays up until its partner
//! (or any other) is chosen; a mismatch re-hides both after a short show.

use super::framework::{resolve, MgState, Minigame, SPORTS};
use crate::campaign::rng::Xor;
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BLOOD, BROWN, CYAN, DARKGRAY, DARKGREEN, EMBER, GOLD, GREEN, NAVY, OLIVE,
    ORANGE, PINK, SAXON_BLUE, SILVER, TAN, TEAL, WHITE, YELLOW,
};
use crate::engine::input::Input;

const PRESENT: u32 = 90; // "THE HERB SALVE" banner
const BOWL: u32 = 2400; // ticks until the boil boils over (40 s)
const SHOW: u32 = 30; // mismatch stays face-up this long
const NONE: usize = usize::MAX;

// score weights: eight pairs, and up to 20 for the speed of the brew.
const W_PAIR: u32 = 10;
const W_SPEED: u32 = 20;

// 4x4 grid of 36 px tiles, 6 px gutters - fits above the hint line.
const TILE: i32 = 36;
const GAP: i32 = 6;
const X0: i32 = (320 - (4 * TILE + 3 * GAP)) / 2;
const Y0: i32 = 76;

/// Herb kinds: (letter, tile colour).
const HERBS: [(char, u8); 8] = [
    ('S', GREEN),
    ('R', TEAL),
    ('M', CYAN),
    ('Y', YELLOW),
    ('T', OLIVE),
    ('N', DARKGREEN),
    ('H', PINK),
    ('F', ORANGE),
];
const LETTERS: [&str; 8] = ["S", "R", "M", "Y", "T", "N", "H", "F"];

pub struct Herbs {
    tick_n: u32,
    phase: Phase,
    kinds: [u8; 16], // which herb each tile hides
    up: [bool; 16],  // face-up right now (seen or matched)
    matched: [bool; 16],
    first: usize,  // the lonely revealed tile
    second: usize, // pending comparison
    show: u32,     // ticks left of the mismatch display
    pairs: u32,
    elapsed: u32, // ticks of active brewing
    state: MgState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Present,
    Run,
    Done,
}

impl Herbs {
    pub fn new(seed: u64) -> Self {
        // Fisher-Yates over the doubled kind bag - same seed, same board.
        let mut rng = Xor::new(seed ^ 0x48_45_52_42);
        let mut kinds = [0u8; 16];
        for (i, k) in kinds.iter_mut().enumerate() {
            *k = (i / 2) as u8;
        }
        for i in (1..16).rev() {
            let j = (rng.next_u64() % (i as u64 + 1)) as usize;
            kinds.swap(i, j);
        }
        Herbs {
            tick_n: 0,
            phase: Phase::Present,
            kinds,
            up: [false; 16],
            matched: [false; 16],
            first: NONE,
            second: NONE,
            show: 0,
            pairs: 0,
            elapsed: 0,
            state: MgState::Running,
        }
    }

    /// The hidden layout - a test seam; the shipped player earns it the
    /// hard way, by remembering.
    #[doc(hidden)]
    pub fn layout(&self) -> [u8; 16] {
        self.kinds
    }

    #[doc(hidden)]
    pub fn is_up(&self, i: usize) -> bool {
        self.up[i]
    }

    #[doc(hidden)]
    pub fn pairs(&self) -> u32 {
        self.pairs
    }

    #[doc(hidden)]
    pub fn elapsed(&self) -> u32 {
        self.elapsed
    }

    fn tile_at(mx: i32, my: i32) -> Option<usize> {
        let gx = mx - X0;
        let gy = my - Y0;
        if gx < 0 || gy < 0 || gx >= 4 * TILE + 3 * GAP || gy >= 4 * TILE + 3 * GAP {
            return None;
        }
        let cx = gx / (TILE + GAP);
        let cy = gy / (TILE + GAP);
        if gx % (TILE + GAP) >= TILE || gy % (TILE + GAP) >= TILE {
            return None; // clicked the gutter
        }
        Some((cy * 4 + cx) as usize)
    }

    fn finish(&mut self, timed_out: bool) {
        self.phase = Phase::Done;
        // A finished brew only banks the speed bonus if the pot is still
        // off the rim; a timed-out brew gets pairs alone.
        let bonus = if !timed_out && self.pairs == 8 {
            (W_SPEED * (BOWL - self.elapsed.min(BOWL - 1)) / BOWL).clamp(1, W_SPEED)
        } else {
            0
        };
        let sc = (self.pairs * W_PAIR + bonus).min(100);
        self.state = resolve(sc, &SPORTS[9]);
    }

    fn click(&mut self, input: &Input) {
        if !input.pressed() {
            return;
        }
        let (mx, my, _) = input.mouse();
        let Some(i) = Self::tile_at(mx, my) else {
            return;
        };
        if self.matched[i] || self.up[i] || self.second != NONE {
            return; // locked tiles, and no picking during a mismatch show
        }
        self.up[i] = true;
        if self.first == NONE {
            self.first = i;
        } else {
            self.second = i;
            self.show = SHOW;
        }
    }

    fn settle(&mut self) {
        let (a, b) = (self.first, self.second);
        if self.kinds[a] == self.kinds[b] {
            self.matched[a] = true;
            self.matched[b] = true;
            self.pairs += 1;
            if self.pairs == 8 {
                self.finish(false);
                return;
            }
        } else {
            self.up[a] = false;
            self.up[b] = false;
        }
        self.first = NONE;
        self.second = NONE;
    }

    fn draw_run(&self, input: &Input, fb: &mut Framebuffer) {
        fb.fill_rect(0, 12, 320, 244, NAVY);
        // the cauldron: liquid climbs and darkens as the boil runs
        let f = self.elapsed.min(BOWL) as i32 * 60 / BOWL as i32; // 0..60
        fb.fill_rect(120, 26, 80, 44, BLACK); // pot
        fb.rect(118, 24, 84, 48, DARKGRAY);
        let lvl = 4 + f * 34 / 60; // liquid height (a rest even at t=0)
        let col = if f < 30 {
            OLIVE
        } else if f < 50 {
            GOLD
        } else {
            EMBER
        };
        fb.fill_rect(121, 69 - lvl, 78, lvl as u32, col);
        // bubbles: fixed wobble, honest to the tick
        let b = (self.elapsed as i32) / 6;
        for k in 0..4 {
            let bx = 126 + ((b * (k + 3)) % 68);
            let by = 66 - ((b * (k + 1)) % (lvl.max(1)));
            if lvl > 4 {
                fb.fill_rect(bx, by, 3, 3, YELLOW);
            }
        }
        if self.elapsed >= BOWL.saturating_sub(120) {
            draw_text_shadow(fb, 118, 78, "IT BOILS OVER", BLOOD, BLACK);
        }
        // the tile grid -------------------------------------------------------
        let (mx, my, _) = input.mouse();
        for i in 0..16 {
            let cx = X0 + (i as i32 % 4) * (TILE + GAP);
            let mut cy = Y0 + (i as i32 / 4) * (TILE + GAP);
            if self.second != NONE && (i == self.first || i == self.second) {
                cy += if self.elapsed % 4 < 2 { -1 } else { 1 }; // shake
            }
            if self.matched[i] {
                fb.rect(cx, cy, TILE as u32, TILE as u32, DARKGRAY); // spent slot
                fb.fill_rect(cx + 1, cy + 1, (TILE - 2) as u32, (TILE - 2) as u32, BLACK);
                continue;
            }
            if self.up[i] {
                let (_, hc) = HERBS[self.kinds[i] as usize];
                fb.rect(cx, cy, TILE as u32, TILE as u32, GOLD);
                fb.fill_rect(cx + 1, cy + 1, (TILE - 2) as u32, (TILE - 2) as u32, TAN);
                fb.fill_rect(cx + 12, cy + 8, 16, 16, hc);
                draw_text_shadow(
                    fb,
                    cx + 14,
                    cy + 27,
                    LETTERS[self.kinds[i] as usize],
                    BLACK,
                    TAN,
                );
            } else {
                let hov = Self::tile_at(mx, my) == Some(i);
                fb.rect(
                    cx,
                    cy,
                    TILE as u32,
                    TILE as u32,
                    if hov { WHITE } else { SAXON_BLUE },
                );
                fb.fill_rect(
                    cx + 1,
                    cy + 1,
                    (TILE - 2) as u32,
                    (TILE - 2) as u32,
                    DARKGRAY,
                );
                // pouch motif
                fb.fill_rect(cx + 12, cy + 12, 12, 16, BROWN);
                fb.fill_rect(cx + 15, cy + 9, 6, 4, SILVER);
            }
        }
        draw_text_shadow(fb, 118, 240, "MATCH THE HERBS", WHITE, BLACK);
    }
}

impl Minigame for Herbs {
    fn id(&self) -> &'static str {
        SPORTS[9].id
    }

    fn tick(&mut self, _dt: f32, input: &Input, fb: &mut Framebuffer) {
        self.tick_n += 1;
        match self.phase {
            Phase::Present => {
                fb.fill_rect(0, 0, 320, 256, BLACK);
                draw_text_shadow(fb, 104, 116, "THE HERB SALVE", GOLD, BLACK);
                draw_text_shadow(fb, 96, 136, "CLICK THE PAIRS", WHITE, BLACK);
                if self.tick_n >= PRESENT {
                    self.phase = Phase::Run;
                }
            }
            Phase::Run => {
                self.elapsed += 1;
                self.click(input);
                if self.second != NONE {
                    self.show -= 1;
                    if self.show == 0 {
                        self.settle();
                    }
                }
                if self.phase == Phase::Run && self.elapsed >= BOWL {
                    self.finish(true);
                }
            }
            Phase::Done => {}
        }
        if self.phase != Phase::Present {
            self.draw_run(input, fb);
        }
    }

    fn score(&self) -> u32 {
        self.pairs * W_PAIR
    }

    fn state(&self) -> MgState {
        self.state
    }
}
