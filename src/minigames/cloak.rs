//! mg-cloak — STEALTH (SPEC 6 #8): the keep at night. A Saxon thief slips
//! in through the postern (bottom-left) and must douse all five lit torches
//! and slip back out — while five gaolers sweep their beats with sight
//! cones. Brass bells lie on the floors: step one and the alarm escalates,
//! every guard quickening its step. A lever deep in the keep draws the port-
//! cullis hiding the fifth torch in the top-right nook. Seen three times and
//! the thief is in irons. The clean sweep — no bell, no eye — is the only
//! route to a 95+.
//!
//! Geometry: the keep is the fixed 20x16 [`KEEP`] (`#` = stone); the thief
//! steps one 16 px cell per beat of the movement cooldown, guards step one
//! cell every `period` ticks along fixed ping-pong beats. The only seed
//! variance is each guard's tempo and starting phase, so a run is fully
//! predictable once observed — and fully deterministic in tests.

use super::framework::{resolve, MgState, Minigame, SPORTS};
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BROWN, CREAM, DARKGRAY, GOLD, GRAY, GRAY3, NAVY, NORMAN_RED, RED,
    SAXON_BLUE, SILVER, STONE, WHITE,
};
use crate::engine::input::{Input, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_UP};

const PRESENT: u32 = 90; // "THE KEEP AT NIGHT" banner
const TABLEAU: u32 = 100; // results beat before Done
const CELL: i32 = 16;
const MOVE_CD: u32 = 9; // ticks between grid steps while a key is held
const TIME_LIMIT: u32 = 3600; // the watch changes shifts (60 s)
const CATCH_FREEZE: u32 = 90; // guards blink while the thief is re-sent
const CLOAKS: u32 = 3; // three sightings and irons
const CONE_LEN: i32 = 4; // sight cone length in cells

// score weights — five torches, the out-door, and a clean sweep: 100.
const W_TORCH: i32 = 15;
const W_ESCAPE: i32 = 10;
const W_CLEAN: i32 = 15; // never rung a bell, never seen
const SPEEDUP: u32 = 4; // ticks off every guard period per rung bell
const MIN_PERIOD: u32 = 12;

/// The keep. Corridors are 1-cell wide with loops; the postern sits bottom-
/// left, the hidden nook (fifth torch) is the dead cell at top right behind
/// the gated mouth at (17,2).
const KEEP: [&str; 16] = [
    "####################",
    "#.#...............##",
    "#.#######...#####..#",
    "#.......#.#.#...#.##",
    "#######.#.##..#...##",
    "#.......#.....#...##",
    "#..###.##.######...#",
    "#.....#.........#..#",
    "####..####...#..#.##",
    "#...#.....#.....#.##",
    "#.##.####..#####...#",
    "#..................#",
    "#.###.###########..#",
    "#...#.............##",
    "#######...##..#.####",
    "####################",
];

const GATE: (i32, i32) = (1, 13); // postern — spawn and out-door
const TORCHES: [(i32, i32); 5] = [(5, 7), (13, 3), (1, 1), (17, 7), (18, 2)];
const BELLS: [(i32, i32); 3] = [(6, 13), (11, 11), (16, 11)];
const LEVER: (i32, i32) = (14, 9); // draws the portcullis
const PORTCULLIS: (i32, i32) = (17, 2); // the nook's only mouth

/// Fixed ping-pong beats (cell coords, every segment corridor-true).
const BEATS: [[(i32, i32); 2]; 5] = [
    [(3, 11), (16, 11)], // the lower street — bells and crossings
    [(4, 8), (4, 11)],   // the left gatepost — watches T1's back door
    [(2, 3), (6, 3)],    // the barracks row — T3's only approach
    [(3, 1), (15, 1)],   // the top gallery — the long dash
    [(17, 6), (17, 11)], // the east column — T4, the lever road, the nook
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Present,
    Run,
    Board,
    Done,
}

/// A gaoler: beat endpoints in cells, `s` steps travelled (ping-pongs over
/// `2 * len`), facing the direction of the last step.
#[derive(Clone, Copy)]
struct Guard {
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
    period: u32,
    s: i32,
    fx: i32,
    fy: i32,
}


impl Guard {
    fn beat_len(&self) -> i32 {
        (self.bx - self.ax).abs() + (self.by - self.ay).abs()
    }
    /// Cell after `steps` total steps — the beat is a fixed line, so this is
    /// pure arithmetic (tests predict future sight from it).
    fn cell_at(&self, steps: i32) -> (i32, i32) {
        let len = self.beat_len();
        if len == 0 {
            return (self.ax, self.ay);
        }
        let cyc = 2 * len;
        let mut k = steps % cyc;
        if k < 0 {
            k += cyc;
        }
        if k > len {
            k = cyc - k;
        }
        let dx = (self.bx - self.ax).signum();
        let dy = (self.by - self.ay).signum();
        (self.ax + dx * k, self.ay + dy * k)
    }
    fn cell(&self) -> (i32, i32) {
        self.cell_at(self.s)
    }
    /// Facing while standing at total-step `k` — the mirror of [`Guard::step`]:
    /// the direction of the step that ARRIVED at `k`; after the pivot step at
    /// the far beat end, the way back out.
    fn face_at(&self, k: i32) -> (i32, i32) {
        let here = self.cell_at(k);
        let prev = self.cell_at(k - 1);
        if here != prev {
            return ((here.0 - prev.0).signum(), (here.1 - prev.1).signum());
        }
        let nxt = self.cell_at(k + 1);
        ((nxt.0 - here.0).signum(), (nxt.1 - here.1).signum())
    }
    fn step(&mut self) {
        let next = self.cell_at(self.s + 1);
        let here = self.cell();
        if next == here {
            if here == (self.ax, self.ay) {
                self.fx = (self.bx - self.ax).signum();
                self.fy = (self.by - self.ay).signum();
            } else {
                self.fx = (self.ax - self.bx).signum();
                self.fy = (self.ay - self.by).signum();
            }
        } else {
            self.fx = next.0 - here.0;
            self.fy = next.1 - here.1;
        }
        self.s += 1;
    }
}

/// Cells seen from `(gx, gy)` facing `(fx, fy)`: `CONE_LEN` straight ahead
/// (stop at stone) plus the two flanks of the first cell.
fn cone_from(gx: i32, gy: i32, fx: i32, fy: i32, keep: &[&str; 16]) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for i in 1..=CONE_LEN {
        let (cx, cy) = (gx + fx * i, gy + fy * i);
        if !is_floor(keep, cx, cy) {
            break;
        }
        out.push((cx, cy));
    }
    if fx != 0 {
        for dy in [-1, 1] {
            let c = (gx + fx, gy + dy);
            if is_floor(keep, c.0, c.1) {
                out.push(c);
            }
        }
    } else {
        for dx in [-1, 1] {
            let c = (gx + dx, gy + fy);
            if is_floor(keep, c.0, c.1) {
                out.push(c);
            }
        }
    }
    out
}
fn cone_cells(g: &Guard, keep: &[&str; 16]) -> Vec<(i32, i32)> {
    let (gx, gy) = g.cell();
    cone_from(gx, gy, g.fx, g.fy, keep)
}

fn is_floor(keep: &[&str; 16], x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < 20 && y < 16 && keep[y as usize].as_bytes()[x as usize] != b'#'
}

/// One STEALTH run. Pure logic; the host owns pause and the cheer timer.
pub struct Cloak {
    tick_n: u32, // ticks since Run opened
    phase: Phase,
    phase_t: u32,
    guard_t: u32, // free-running clock the beats answer to
    px: i32,
    py: i32,
    last_move: u32,
    guards: Vec<Guard>,
    frozen: u32, // guards blink right after a sighting
    torch_out: [bool; 5],
    bell_rang: [bool; 3],
    bells: u32, // alarm level = bells rung
    seen: u32,
    cloaks: u32,
    gate_open: bool,
    score: i32,
    plate: &'static str,
    plate_t: u32,
    state: MgState,
}

fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x.wrapping_mul(0x94D0_49BB_1331_11EB)
}

impl Cloak {
    pub fn new(seed: u64) -> Self {
        let guards = BEATS
            .iter()
            .enumerate()
            .map(|(i, beat)| {
                let m = mix(seed ^ (i as u64) << 13);
                let period = 20 + (m % 8) as u32; // 20..27
                let len = (beat[0].0 - beat[1].0).abs() + (beat[0].1 - beat[1].1).abs();
                let s = ((m >> 32) % ((2 * len).max(1) as u64)) as i32;
                let g = Guard {
                    ax: beat[0].0,
                    ay: beat[0].1,
                    bx: beat[1].0,
                    by: beat[1].1,
                    period,
                    s,
                    fx: 0,
                    fy: 0,
                };
                let (fx, fy) = g.face_at(s);
                Guard { fx, fy, ..g }
            })
            .collect();
        Self {
            tick_n: 0,
            phase: Phase::Present,
            phase_t: 0,
            guard_t: 0,
            px: GATE.0,
            py: GATE.1,
            last_move: u32::MAX / 2,
            guards,
            frozen: 0,
            torch_out: [false; 5],
            bell_rang: [false; 3],
            bells: 0,
            seen: 0,
            cloaks: CLOAKS,
            gate_open: false,
            score: 0,
            plate: "",
            plate_t: 0,
            state: MgState::Running,
        }
    }

    // test/host introspection ------------------------------------------------
    pub fn torches_out(&self) -> u32 {
        self.torch_out.iter().filter(|t| **t).count() as u32
    }
    pub fn bells(&self) -> u32 {
        self.bells
    }
    pub fn seen(&self) -> u32 {
        self.seen
    }
    pub fn cloaks(&self) -> u32 {
        self.cloaks
    }
    pub fn gate_open(&self) -> bool {
        self.gate_open
    }
    pub fn thief_cell(&self) -> (i32, i32) {
        (self.px, self.py)
    }
    /// `(cell_x, cell_y, steps)` of beat `i`.
    pub fn guard(&self, i: usize) -> (i32, i32, i32) {
        let (x, y) = self.guards[i].cell();
        (x, y, self.guards[i].s)
    }
    /// Effective guard period after alarm escalation.
    pub fn period(&self, i: usize) -> u32 {
        self.guards[i]
            .period
            .saturating_sub(self.bells * SPEEDUP)
            .max(MIN_PERIOD)
    }
    /// Union of every guard sight cone `ahead` ticks from now, assuming no
    /// new bell rings — cones plus the gaolers' own cells, since standing
    /// on a beat is also seen. Bells quicken the watch and a freeze stalls
    /// it, so an escape step is only safe when its landing cell is absent.
    #[doc(hidden)] // test seam: predicted vision for safety checks
    pub fn predicted_sight(&self, ahead: u32) -> Vec<(i32, i32)> {
        let mut frozen = self.frozen;
        let mut gt = self.guard_t;
        let mut steps: Vec<i32> = self.guards.iter().map(|g| g.s).collect();
        for _ in 0..ahead {
            if frozen > 0 {
                frozen -= 1;
                continue;
            }
            gt += 1;
            for (i, st) in steps.iter_mut().enumerate() {
                if gt.is_multiple_of(self.period(i)) {
                    *st += 1;
                }
            }
        }
        let mut out: Vec<(i32, i32)> = Vec::new();
        for (i, g) in self.guards.iter().enumerate() {
            let (gx, gy) = g.cell_at(steps[i]);
            let (fx, fy) = g.face_at(steps[i]);
            let mut seen_here = cone_from(gx, gy, fx, fy, &KEEP);
            seen_here.push((gx, gy)); // standing on a beat is also "seen"
            for c in seen_here {
                if !out.contains(&c) {
                    out.push(c);
                }
            }
        }
        out
    }

    #[doc(hidden)] // test seam: put the thief mid-keep
    pub fn debug_place(&mut self, x: i32, y: i32) {
        self.px = x;
        self.py = y;
    }

    #[doc(hidden)] // test seam: open the portcullis
    pub fn debug_open_gate(&mut self) {
        self.gate_open = true;
    }

    fn say(&mut self, p: &'static str) {
        self.plate = p;
        self.plate_t = 70;
    }

    fn finish(&mut self) {
        self.score = self.score.clamp(0, 100);
        self.state = resolve(self.score as u32, &SPORTS[7]);
        self.phase = Phase::Board;
        self.phase_t = 0;
    }

    /// Cell a thief-step would land on, honouring stone and a shut gate.
    fn passable(&self, x: i32, y: i32) -> bool {
        is_floor(&KEEP, x, y) && (self.gate_open || (x, y) != PORTCULLIS)
    }

    /// Direction for the given key-test over arrows AND WASD (both cases —
    /// the web adapter passes raw codepoints for printable keys).
    fn held_dir(input: &Input, test: fn(&Input, u32) -> bool) -> Option<(i32, i32)> {
        let is = |arrow: u32, w: u8| {
            test(input, arrow) || test(input, w as u32) || test(input, w.to_ascii_uppercase() as u32)
        };
        if is(KEY_UP, b'w') {
            Some((0, -1))
        } else if is(KEY_DOWN, b's') {
            Some((0, 1))
        } else if is(KEY_LEFT, b'a') {
            Some((-1, 0))
        } else if is(KEY_RIGHT, b'd') {
            Some((1, 0))
        } else {
            None
        }
    }

    fn move_thief(&mut self, input: &Input) {
        // a fresh press wins over a held key; held steps repeat on cooldown
        let mut dir = Self::held_dir(input, Input::key_pressed);
        if dir.is_none() && self.tick_n.saturating_sub(self.last_move) >= MOVE_CD {
            dir = Self::held_dir(input, Input::key_down);
        }
        if let Some((dx, dy)) = dir {
            let (nx, ny) = (self.px + dx, self.py + dy);
            if self.passable(nx, ny) {
                self.px = nx;
                self.py = ny;
                self.last_move = self.tick_n;
            }
        }
    }

    fn run_cell(&mut self) {
        // torches -------------------------------------------------------------
        for (i, t) in TORCHES.iter().enumerate() {
            if !self.torch_out[i] && (self.px, self.py) == *t {
                self.torch_out[i] = true;
                self.score += W_TORCH;
                let left = 5 - self.torches_out();
                self.say(if left == 0 {
                    "ALL DARK - RUN!"
                } else {
                    "A TORCH GUTS OUT"
                });
            }
        }
        // bells ---------------------------------------------------------------
        for (i, b) in BELLS.iter().enumerate() {
            if !self.bell_rang[i] && (self.px, self.py) == *b {
                self.bell_rang[i] = true;
                self.bells += 1;
                self.say("THE BELL TOLLS!");
            }
        }
        // the lever -----------------------------------------------------------
        if (self.px, self.py) == LEVER && !self.gate_open {
            self.gate_open = true;
            self.say("THE PORTCULLIS DRAWS");
        }
        // the out-door ----------------------------------------------------------
        if (self.px, self.py) == GATE && self.torch_out.iter().all(|t| *t) {
            self.score += W_ESCAPE;
            if self.bells == 0 && self.seen == 0 {
                self.score += W_CLEAN;
            }
            self.finish();
        }
    }

    fn run_guards(&mut self) {
        if self.frozen > 0 {
            self.frozen -= 1;
            return;
        }
        self.guard_t += 1;
        for g in self.guards.iter_mut() {
            let p = g.period.saturating_sub(self.bells * SPEEDUP).max(MIN_PERIOD);
            if self.guard_t.is_multiple_of(p) {
                g.step();
            }
        }
    }

    fn check_eyes(&mut self) {
        if self.frozen > 0 {
            return; // eyes blink while the thief is re-sent
        }
        for g in self.guards.iter() {
            let (gx, gy) = g.cell();
            let touched = (gx, gy) == (self.px, self.py);
            let seen = touched || cone_cells(g, &KEEP).contains(&(self.px, self.py));
            if !seen {
                continue;
            }
            self.seen += 1;
            self.cloaks -= 1;
            self.px = GATE.0;
            self.py = GATE.1;
            self.frozen = CATCH_FREEZE;
            if self.cloaks == 0 {
                self.say("IN IRONS!");
                self.finish();
            } else {
                self.say("HALT! AN EYE FOUND YOU");
            }
            return;
        }
    }
}

impl Minigame for Cloak {
    fn id(&self) -> &'static str {
        "cloak"
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
                self.move_thief(input);
                self.run_cell();
                if self.phase != Phase::Run {
                    self.paint(fb);
                    return; // out the door, or in irons
                }
                self.run_guards();
                self.check_eyes();
                if self.phase != Phase::Run {
                    self.paint(fb);
                    return;
                }
                if self.tick_n >= TIME_LIMIT {
                    self.say("THE WATCH RELIEVES");
                    self.finish();
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

impl Cloak {
    fn paint(&self, fb: &mut Framebuffer) {
        // floor and stone -------------------------------------------------------
        fb.fill_rect(0, 0, 320, 256, BLACK);
        for (cy, row) in KEEP.iter().enumerate() {
            for (cx, c) in row.bytes().enumerate() {
                let (x, y) = (cx as i32 * CELL, cy as i32 * CELL);
                if c == b'#' {
                    fb.fill_rect(x, y, CELL as u32, CELL as u32, STONE);
                    fb.fill_rect(x, y, CELL as u32, 1, GRAY);
                    fb.fill_rect(x + 15, y, 1, CELL as u32, BLACK);
                } else {
                    fb.fill_rect(x, y, CELL as u32, CELL as u32, NAVY);
                    if (cx * 7 + cy * 13) % 11 == 0 {
                        fb.fill_rect(x + 3, y + 9, 1, 1, DARKGRAY);
                    }
                }
            }
        }
        // sight cones (dithered NORMAN_RED) -------------------------------------
        if self.phase == Phase::Run || self.phase == Phase::Board {
            for g in self.guards.iter() {
                for (cx, cy) in cone_cells(g, &KEEP) {
                    for dy in 0..CELL {
                        for dx in 0..CELL {
                            if (dx + dy) % 2 == 0 {
                                fb.set(cx * CELL + dx, cy * CELL + dy, NORMAN_RED);
                            }
                        }
                    }
                }
            }
        }
        // the postern ------------------------------------------------------------
        let lit = self.phase == Phase::Run && self.torch_out.iter().all(|t| *t);
        fb.fill_rect(
            GATE.0 * CELL + 2,
            GATE.1 * CELL + 1,
            12,
            14,
            BROWN,
        );
        fb.rect(
            GATE.0 * CELL + 2,
            GATE.1 * CELL + 1,
            12,
            14,
            if lit { GOLD } else { BLACK },
        );
        fb.fill_rect(
            GATE.0 * CELL + 4,
            GATE.1 * CELL + 3,
            8,
            10,
            if lit { GOLD } else { BLACK },
        );
        // the portcullis -----------------------------------------------------------
        if !self.gate_open {
            for k in 0..4 {
                fb.fill_rect(PORTCULLIS.0 * CELL + 2 + k * 4, PORTCULLIS.1 * CELL + 1, 2, 14, GRAY3);
            }
            fb.fill_rect(PORTCULLIS.0 * CELL + 1, PORTCULLIS.1 * CELL + 1, 14, 2, GRAY);
        } else {
            fb.fill_rect(PORTCULLIS.0 * CELL + 1, PORTCULLIS.1 * CELL + 1, 14, 3, GRAY3);
        }
        // the lever -------------------------------------------------------------------
        let (lx, ly) = (LEVER.0 * CELL, LEVER.1 * CELL);
        fb.fill_rect(lx + 6, ly + 6, 3, 8, if self.gate_open { GRAY3 } else { BROWN });
        fb.fill_rect(lx + 5, ly + (if self.gate_open { 3 } else { 4 }), 6, 3, if self.gate_open { SILVER } else { GOLD });
        // bells ------------------------------------------------------------------------
        for (i, b) in BELLS.iter().enumerate() {
            let (bx, by) = (b.0 * CELL, b.1 * CELL);
            let col = if self.bell_rang[i] { GRAY3 } else { GOLD };
            fb.fill_rect(bx + 4, by + 7, 8, 5, col);
            fb.fill_rect(bx + 5, by + 5, 6, 2, col);
            fb.fill_rect(bx + 7, by + 12, 2, 2, col);
        }
        // torches ------------------------------------------------------------------------
        for (i, t) in TORCHES.iter().enumerate() {
            let (tx, ty) = (t.0 * CELL + 8, t.1 * CELL + 8);
            fb.fill_rect(tx - 1, ty - 2, 2, 8, BROWN);
            if !self.torch_out[i] {
                let f = self.phase_t % 8 < 4;
                fb.fill_rect(tx - 2, ty - 6 - if f { 1 } else { 0 }, 4, 4, GOLD);
                fb.fill_rect(tx - 1, ty - 7 - if f { 1 } else { 0 }, 2, 2, RED);
            } else {
                fb.fill_rect(tx - 2, ty - 5, 4, 3, GRAY3);
            }
        }
        // gaolers -------------------------------------------------------------------------
        for i in 0..self.guards.len() {
            let (gx, gy, _) = self.guard(i);
            let (gx, gy) = (gx * CELL + 8, gy * CELL + 8);
            let quake = if self.frozen > 0 { (self.phase_t % 4 / 2) as i32 } else { 0 };
            fb.fill_rect(gx - 3 + quake, gy - 4, 6, 9, NORMAN_RED);
            fb.fill_rect(gx - 3 + quake, gy - 7, 6, 3, SILVER);
        }
        // the thief ------------------------------------------------------------------------
        if self.phase != Phase::Board && self.phase != Phase::Done {
            let (px, py) = (self.px * CELL + 8, self.py * CELL + 8);
            fb.fill_rect(px - 3, py - 4, 6, 9, SAXON_BLUE);
            fb.fill_rect(px - 3, py - 7, 6, 3, CREAM);
            fb.fill_rect(px - 3, py - 7, 6, 1, SAXON_BLUE);
        }
        // HUD --------------------------------------------------------------------------------
        draw_text_shadow(fb, 4, 2, "STEALTH", GOLD, BLACK);
        draw_text_shadow(fb, 108, 2, &format!("TORCHES {}/5", self.torches_out()), CREAM, BLACK);
        for k in 0..self.cloaks {
            fb.fill_rect(224 + k as i32 * 8, 4, 6, 7, if self.cloaks == 1 { RED } else { SILVER });
        }
        for k in 0..3 {
            fb.fill_rect(262 + k * 8, 3, 6, 7, if (k as u32) < self.bells { GOLD } else { DARKGRAY });
        }
        if self.plate_t > 0 {
            let c = if self.plate == "HALT! AN EYE FOUND YOU" || self.plate == "IN IRONS!" {
                RED
            } else {
                GOLD
            };
            draw_text_shadow(fb, 96, 120, self.plate, c, BLACK);
        }
        match self.phase {
            Phase::Present => {
                if self.phase_t % 24 < 16 {
                    draw_text_shadow(fb, 88, 128, "THE KEEP AT NIGHT", WHITE, BLACK);
                }
            }
            Phase::Board | Phase::Done => {
                let s = self.score.max(0) as u32;
                draw_text_shadow(fb, 110, 60, &format!("SCORE {}", s), GOLD, BLACK);
                let msg = match self.state {
                    MgState::Lost => "IN THE TOWER",
                    _ => "INTO THE NIGHT",
                };
                let col = if self.state == MgState::Lost { RED } else { GOLD };
                draw_text_shadow(fb, 96, 84, msg, col, BLACK);
            }
            _ => {}
        }
    }
}
