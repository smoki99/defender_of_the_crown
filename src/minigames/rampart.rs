//! mg-rampart — DEFEND (SPEC 6 #5): the wall at dawn. Scaling ladders drop
//! in six waves along the curtain; loose arrows in the wall-walk quiver
//! (they re-gather off the field), and two carts of rocks wait on the
//! battlements for the fat of a climbing party. Aim with the mouse, loose
//! with the button, tip the rocks with SPACE. Three men over the wall and
//! the castle is lost.
//!
//! Geometry: the curtain face is the strip `WALL_X0..=WALL_X1`; a ladder
//! marches in along its lane row (`LANES`, far row first) from the right
//! edge to the face, plants, and three climbers run up the run toward
//! `TOP_Y` — past that they are over the battlements. The archer post
//! [`BOW`] sits on the wall-walk and shots fly at the sight line.

use super::framework::{resolve, MgState, Minigame, SportSpec, SPORTS};
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BROWN, CREAM, DARKGRAY, GOLD, GRAY, GRAY3, GREEN, LINEN, NAVY,
    NORMAN_RED, RED, SAXON_BLUE, SILVER, STONE, TAN, WHITE,
};
use crate::engine::input::{Input, KEY_SPACE};

const PRESENT: u32 = 90; // "TO THE WALL" banner
const TABLEAU: u32 = 100; // results beat before Done
const WAVES: usize = 6;
const WAVE_GAP: u32 = 130; // ticks between ladder deployments
const MARCH_SPD: f32 = 2.4; // px per tick
const MARCH_X0: f32 = 316.0; // right edge, where the park starts
const LADDER_X: f32 = 78.0; // centre of the ladder run at the curtain
const RUNNER_X: f32 = 94.0; // ladder-team base once planted
const SET_TICKS: u32 = 40; // planting the ladder
const CLIMB_TICKS: u32 = 150; // uniform face-climb time, road to battlement
const STAGGER: u32 = 30; // ticks between the three climbers
const TOP_Y: f32 = 44.0; // battlements — over this and they are in
const GROUND_Y: f32 = 232.0; // base of the face
const WALL_X0: i32 = 44;
const WALL_X1: i32 = 68;
const BOW: (f32, f32) = (56.0, 50.0); // archer post on the wall-walk
const ARROW_SPD: f32 = 9.0;
const HIT_R: f32 = 7.0;
const QUIVER_MAX: u32 = 12;
const REGEN_EVERY: u32 = 26; // loose arrows re-gathered
const ROCKS: u32 = 2;
const ROCK_FALL_SPD: f32 = 8.0;
const ROCK_Y0: f32 = 46.0;
const ROCK_LOCK: f32 = 46.0; // how near the sight a ladder must be
const END_TICKS: u32 = 1400; // the siege gives up
const MAX_BREACH: u32 = 3;

// score weights — a bow-perfect siege (every man loosed at, every ladder
// held) lands just above Gold, SPEC-normalised 0..=100.
const W_CLIMBER: i32 = 4;
const W_RUNNER: i32 = 2;
const W_REPEL: i32 = 1;
const W_BREACH: i32 = -12;

const LANES: [f32; WAVES] = [70.0, 96.0, 122.0, 148.0, 174.0, 200.0];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Present,
    Run,
    Board,
    Done,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LState {
    Unborn,
    March,
    Set,
    Climb,
    Held, // repelled or shattered — no more men on this ladder
    Breached,
}

#[derive(Clone, Copy)]
struct Ladder {
    state: LState,
    t: u32,
    lane: f32,
    x: f32, // current march position of the team
    runner: [bool; 2], // still marching/alive
    climber: [bool; 3],
    fell: bool, // rocks broke the ladder
}

#[derive(Clone, Copy)]
struct Arrow {
    live: bool,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

#[derive(Clone, Copy)]
struct Rock {
    live: bool,
    lane: usize,
    y: f32,
}

/// One DEFEND run. Pure logic; the host owns pause and the cheer timer.
pub struct Rampart {
    tick_n: u32, // ticks since Run opened
    phase: Phase,
    phase_t: u32,
    ladders: [Ladder; WAVES],
    arrows: [Arrow; 3],
    rocks: [Rock; ROCKS as usize],
    rocks_left: u32,
    quiver: u32,
    breaches: u32,
    score: i32,
    sight: (f32, f32),
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

impl Rampart {
    pub fn new(seed: u64) -> Self {
        // the siege shifts its deployment order with the seed, far row first
        let mut lanes = LANES;
        for i in (1..WAVES).rev() {
            let j = (mix(seed ^ i as u64) % (i as u64 + 1)) as usize;
            lanes.swap(i, j);
        }
        let mut ladders = [Ladder {
            state: LState::Unborn,
            t: 0,
            lane: 0.0,
            x: MARCH_X0,
            runner: [true; 2],
            climber: [true; 3],
            fell: false,
        }; WAVES];
        for (i, l) in ladders.iter_mut().enumerate() {
            l.lane = lanes[i];
        }
        Self {
            tick_n: 0,
            phase: Phase::Present,
            phase_t: 0,
            ladders,
            arrows: [Arrow {
                live: false,
                x: 0.0,
                y: 0.0,
                vx: 0.0,
                vy: 0.0,
            }; 3],
            rocks: [Rock {
                live: false,
                lane: 0,
                y: 0.0,
            }; ROCKS as usize],
            rocks_left: ROCKS,
            quiver: QUIVER_MAX,
            breaches: 0,
            score: 0,
            sight: (160.0, 128.0),
            plate: "",
            plate_t: 0,
            state: MgState::Running,
        }
    }

    // ---- test/debug views -------------------------------------------------

    pub fn breaches(&self) -> u32 {
        self.breaches
    }
    pub fn quiver(&self) -> u32 {
        self.quiver
    }
    pub fn rocks_left(&self) -> u32 {
        self.rocks_left
    }
    pub fn tick(&self) -> u32 {
        self.tick_n
    }
    /// Ladder `i`: `(state as u8, climbers alive, runners alive)` — 0
    /// Unborn, 1 March, 2 Set, 3 Climb, 4 Held, 5 Breached.
    pub fn ladder(&self, i: usize) -> (u8, u8, u8) {
        let l = &self.ladders[i];
        (
            l.state as u8,
            l.climber.iter().filter(|c| **c).count() as u8,
            l.runner.iter().filter(|r| **r).count() as u8,
        )
    }
    /// World position of climber `j` on ladder `i`, if alive and climbing.
    pub fn climber_xy(&self, i: usize, j: usize) -> Option<(f32, f32)> {
        Self::climber_pos(&self.ladders[i], j)
    }
    /// Ladder-team base once planted (runners are shootable from `Set`).
    pub fn runner_xy(&self, i: usize, j: usize) -> Option<(f32, f32)> {
        let l = &self.ladders[i];
        if !l.runner[j] || matches!(l.state, LState::Unborn | LState::March) {
            return None;
        }
        Some((RUNNER_X + 8.0 * j as f32, l.lane))
    }

    // ---- internals ---------------------------------------------------------

    fn plate(&mut self, msg: &'static str) {
        self.plate = msg;
        self.plate_t = 30;
    }

    fn loose(&mut self) {
        if self.quiver == 0 {
            self.plate("QUIVER EMPTY");
            return;
        }
        let (sx, sy) = self.sight;
        let dx = sx - BOW.0;
        let dy = sy - BOW.1;
        let d = (dx * dx + dy * dy).sqrt();
        if d < 1.0 {
            return;
        }
        let Some(slot) = self.arrows.iter().position(|a| !a.live) else {
            return; // three shafts is the wall-walk's draw rate
        };
        self.arrows[slot] = Arrow {
            live: true,
            x: BOW.0,
            y: BOW.1,
            vx: dx / d * ARROW_SPD,
            vy: dy / d * ARROW_SPD,
        };
        self.quiver -= 1;
    }

    /// One sub-stepped shaft advance; returns on the first man it finds.
    fn fly(&mut self) {
        for k in 0..self.arrows.len() {
            if !self.arrows[k].live {
                continue;
            }
            let (vx, vy) = (self.arrows[k].vx, self.arrows[k].vy);
            let (mut x, mut y) = (self.arrows[k].x, self.arrows[k].y);
            let mut dead = false;
            for _ in 0..3 {
                x += vx / 3.0;
                y += vy / 3.0;
                if !(-4.0..=324.0).contains(&x) || !(-4.0..=260.0).contains(&y) {
                    dead = true;
                    break;
                }
                if self.slay(x, y) {
                    dead = true;
                    break;
                }
            }
            self.arrows[k].x = x;
            self.arrows[k].y = y;
            if dead {
                self.arrows[k].live = false;
            }
        }
    }

    /// Scan for the first man under the shaft, then apply — the ladder
    /// list and the tally never borrow at the same time.
    fn slay(&mut self, x: f32, y: f32) -> bool {
        let hit = |px: f32, py: f32| (x - px).abs() <= HIT_R && (y - py).abs() <= HIT_R;
        let mut found: Option<(usize, usize, bool)> = None; // (ladder, slot, runner?)
        'scan: for (i, l) in self.ladders.iter().enumerate() {
            if matches!(l.state, LState::Unborn | LState::Held | LState::Breached) {
                continue;
            }
            if l.state == LState::March {
                // the team marching in the open, bunched on the run
                for (j, r) in l.runner.iter().enumerate() {
                    if *r && hit(l.x + 4.0 + 5.0 * j as f32, l.lane - 3.0) {
                        found = Some((i, j, true));
                        break 'scan;
                    }
                }
                continue;
            }
            for (j, _) in l.climber.iter().enumerate() {
                if let Some((cx, cy)) = Self::climber_pos(l, j) {
                    if hit(cx, cy) {
                        found = Some((i, j, false));
                        break 'scan;
                    }
                }
            }
            for (j, r) in l.runner.iter().enumerate() {
                let (rx, ry) = (RUNNER_X + 8.0 * j as f32, l.lane);
                if *r && hit(rx, ry) {
                    found = Some((i, j, true));
                    break 'scan;
                }
            }
        }
        match found {
            Some((i, j, false)) => {
                self.ladders[i].climber[j] = false;
                self.score += W_CLIMBER;
                self.plate = "CLIMBER DOWN";
                self.plate_t = 30;
                let l = &mut self.ladders[i];
                Self::settle(l);
                true
            }
            Some((i, j, true)) => {
                self.ladders[i].runner[j] = false;
                self.score += W_RUNNER;
                self.plate = "MARKSMAN";
                self.plate_t = 30;
                let l = &mut self.ladders[i];
                Self::settle(l);
                true
            }
            None => false,
        }
    }

    /// Climber `j`'s live position — `None` unless it is alive and the
    /// ladder is actually being climbed (a planted one has no men on it).
    /// Every road row is scaled so the face takes the same CLIMB_TICKS
    /// whether the ladder stands at the battlements or the river road.
    fn climber_pos(l: &Ladder, j: usize) -> Option<(f32, f32)> {
        if !l.climber[j] || l.state != LState::Climb || l.t < (j as u32) * STAGGER {
            return None;
        }
        let climbed = (l.t - (j as u32) * STAGGER) as f32;
        let frac = (climbed / CLIMB_TICKS as f32).min(1.0);
        Some((LADDER_X + 4.0, l.lane - frac * (l.lane - TOP_Y)))
    }

    /// A planted ladder with no living climbers is repelled; a breached one
    /// stays breached.
    fn settle(l: &mut Ladder) {
        if l.state == LState::Climb && !l.climber.iter().any(|c| *c) {
            l.state = LState::Held;
        }
    }

    fn drop_rock(&mut self) {
        if self.rocks_left == 0 {
            self.plate("NO ROCKS LEFT");
            return;
        }
        // nearest planted ladder to the sight
        let mut best: Option<(usize, f32)> = None;
        for (i, l) in self.ladders.iter().enumerate() {
            if matches!(l.state, LState::Unborn | LState::March | LState::Held | LState::Breached) {
                continue;
            }
            let dy = self.sight.1.clamp(TOP_Y, l.lane) - self.sight.1;
            let dx = (self.sight.0 - LADDER_X).max(0.0);
            let d = (dx * dx + dy * dy).sqrt();
            if d <= ROCK_LOCK && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        let Some((i, _)) = best else {
            self.plate("NO LADDER IN REACH");
            return;
        };
        if let Some(slot) = self.rocks.iter().position(|r| !r.live && r.lane == i).or_else(|| {
            self.rocks.iter().position(|r| !r.live)
        }) {
            self.rocks[slot] = Rock {
                live: true,
                lane: i,
                y: ROCK_Y0,
            };
            self.rocks_left -= 1;
        }
    }

    fn land_rocks(&mut self) {
        // the landing plan first: rocks and ladders never borrow at once,
        // and settlement happens only after the plan exists
        let mut land = [None; ROCKS as usize];
        for (land_slot, rock) in land.iter_mut().zip(self.rocks.iter()) {
            if !rock.live {
                continue;
            }
            if rock.y + ROCK_FALL_SPD >= self.ladders[rock.lane].lane {
                *land_slot = Some(rock.lane);
            }
        }
        for (rock, &lane_i) in self.rocks.iter_mut().zip(land.iter()) {
            let Some(lane_i) = lane_i else {
                if rock.live {
                    rock.y += ROCK_FALL_SPD; // still in the air
                }
                continue;
            };
            let (had, repelled) = {
                let l = &mut self.ladders[lane_i];
                let had = l.climber.iter().any(|c| *c) || l.runner.iter().any(|r| *r);
                let repelled = !matches!(l.state, LState::Held | LState::Breached);
                l.climber = [false; 3];
                l.runner = [false; 2];
                l.fell = true;
                if repelled {
                    l.state = LState::Held;
                }
                (had, repelled)
            };
            if repelled {
                self.score += W_REPEL;
            }
            self.plate = if had { "LADDER SHATTERED" } else { "A WASTED ROCK" };
            self.plate_t = 30;
            rock.live = false;
        }
    }

    fn step_ladders(&mut self) {
        let tick = self.tick_n;
        let mut breach = false;
        let mut repels = 0usize;
        for (i, l) in self.ladders.iter_mut().enumerate() {
            let born = tick >= i as u32 * WAVE_GAP;
            match l.state {
                LState::Unborn if born => {
                    l.state = LState::March;
                    l.t = 0;
                }
                LState::March => {
                    l.x -= MARCH_SPD;
                    if l.x <= LADDER_X + 2.0 {
                        l.x = LADDER_X + 2.0;
                        l.state = LState::Set;
                        l.t = 0;
                    }
                }
                LState::Set => {
                    l.t += 1;
                    if l.t >= SET_TICKS {
                        l.state = LState::Climb;
                        l.t = 0;
                    }
                }
                LState::Climb => {
                    l.t += 1;
                    let mut got_in = false;
                    for j in 0..3 {
                        if let Some((_, y)) = Self::climber_pos(l, j) {
                            if y <= TOP_Y {
                                // over the battlements — the ladder wins
                                l.state = LState::Breached;
                                l.climber = [false; 3];
                                l.runner = [false; 2];
                                got_in = true;
                                break;
                            }
                        }
                    }
                    if got_in {
                        breach = true;
                    } else if !l.climber.iter().any(|c| *c) {
                        l.state = LState::Held;
                        repels += 1;
                    }
                }
                _ => {}
            }
        }
        self.score += repels as i32 * W_REPEL;
        if breach {
            self.breaches += 1;
            self.score += W_BREACH;
            self.plate = "THEY ARE IN!";
            self.plate_t = 30;
            if self.breaches >= MAX_BREACH {
                self.state = MgState::Lost;
                self.phase = Phase::Board;
                self.phase_t = 0;
            }
        }
    }

    fn siege_over(&self) -> bool {
        self.ladders
            .iter()
            .all(|l| matches!(l.state, LState::Held | LState::Breached))
    }

    fn finish_hold(&mut self, spec: &SportSpec) {
        self.score = self.score.clamp(0, 100);
        self.state = resolve(self.score as u32, spec);
    }
}

impl Minigame for Rampart {
    fn id(&self) -> &'static str {
        "rampart"
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
                let (mx, my, _) = input.mouse();
                self.sight = (
                    (mx as f32).clamp(4.0, 316.0),
                    (my as f32).clamp(4.0, 252.0),
                );
                if input.pressed() {
                    self.loose();
                }
                if input.key_pressed(KEY_SPACE) {
                    self.drop_rock();
                }
                self.fly();
                self.land_rocks();
                self.step_ladders();
                if self.phase != Phase::Run {
                    // a third breach ended it mid-step
                    self.paint(fb);
                    return;
                }
                if self.tick_n.is_multiple_of(REGEN_EVERY) && self.quiver < QUIVER_MAX {
                    self.quiver += 1; // boys in the field re-gather the shafts
                }
                if self.siege_over() || self.tick_n >= END_TICKS {
                    self.phase = Phase::Board;
                    self.phase_t = 0;
                    self.finish_hold(&SPORTS[4]);
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

impl Rampart {
    fn paint(&self, fb: &mut Framebuffer) {
        // field and sky ------------------------------------------------------
        fb.fill_rect(0, 0, 320, 256, NAVY);
        for (i, lane) in LANES.iter().enumerate() {
            let y = *lane as i32;
            fb.fill_rect(72, y - 2, 248, 5, if i % 2 == 0 { GREEN } else { DARKGRAY });
        }
        // the curtain ---------------------------------------------------------
        fb.fill_rect(WALL_X0, 24, (WALL_X1 - WALL_X0) as u32, GROUND_Y as u32, STONE);
        for x in (WALL_X0..WALL_X1).step_by(6) {
            fb.fill_rect(x, 24, 4, 8, STONE);
            fb.fill_rect(x + 2, 28, 2, 4, BLACK);
        }
        fb.fill_rect(WALL_X0, TOP_Y as i32, (WALL_X1 - WALL_X0) as u32, 2, GRAY);
        // wall-walk archer ----------------------------------------------------
        fb.fill_rect(BOW.0 as i32 - 2, BOW.1 as i32 - 4, 5, 8, SAXON_BLUE);
        fb.fill_rect(BOW.0 as i32 - 2, BOW.1 as i32 - 7, 4, 3, SILVER);
        // rock carts on the battlements ---------------------------------------
        for k in 0..self.rocks_left as usize {
            fb.fill_rect(20 + 10 * k as i32, 58, 7, 6, GRAY);
            fb.fill_rect(22 + 10 * k as i32, 55, 3, 3, GRAY3);
        }
        // ladders, teams, climbers --------------------------------------------
        for l in self.ladders.iter() {
            match l.state {
                LState::Unborn => {}
                LState::March => {
                    // the team joggs in with the ladder on its shoulder
                    let x = l.x as i32;
                    let y = l.lane as i32;
                    fb.fill_rect(x - 12, y - 4, 22, 2, BROWN);
                    for (j, r) in l.runner.iter().enumerate() {
                        if *r {
                            fb.fill_rect(x + 6 * j as i32 - 6, y - 8, 4, 9, NORMAN_RED);
                            fb.fill_rect(x + 6 * j as i32 - 6, y - 11, 3, 3, SILVER);
                        }
                    }
                }
                _ => {
                    let x = LADDER_X as i32;
                    let base = l.lane as i32;
                    let top = TOP_Y as i32;
                    if !l.fell {
                        fb.fill_rect(x, top, 2, (base - top) as u32, BROWN);
                        fb.fill_rect(x + 6, top, 2, (base - top) as u32, BROWN);
                        let mut y = base - 4;
                        while y > top {
                            fb.fill_rect(x, y, 8, 1, TAN);
                            y -= 8;
                        }
                        // the team bracing the foot
                        for (j, r) in l.runner.iter().enumerate() {
                            if *r {
                                let rx = (RUNNER_X + 8.0 * j as f32) as i32;
                                fb.fill_rect(rx - 2, base - 9, 4, 9, NORMAN_RED);
                                fb.fill_rect(rx - 2, base - 12, 3, 3, SILVER);
                            }
                        }
                    } else {
                        // a shattered ladder lies askew
                        fb.fill_rect(x - 2, base - 2, 16, 2, BROWN);
                    }
                    if !l.fell {
                        for j in 0..3 {
                            if let Some((cx, cy)) = Self::climber_pos(l, j) {
                                let (cx, cy) = (cx as i32, cy as i32);
                                fb.fill_rect(cx - 2, cy - 7, 4, 8, NORMAN_RED);
                                fb.fill_rect(cx - 2, cy - 10, 3, 3, SILVER);
                            }
                        }
                    }
                }
            }
        }
        // rocks falling --------------------------------------------------------
        for r in self.rocks.iter().filter(|r| r.live) {
            fb.fill_rect(LADDER_X as i32 + 1, r.y as i32, 6, 6, GRAY);
            fb.fill_rect(LADDER_X as i32 + 2, r.y as i32 + 1, 2, 2, GRAY3);
        }
        // shafts in flight ------------------------------------------------------
        for a in self.arrows.iter().filter(|a| a.live) {
            let (ax, ay) = (a.x as i32, a.y as i32);
            fb.fill_rect(ax - 1, ay - 1, 2, 2, WHITE);
            fb.fill_rect(
                (a.x - a.vx) as i32,
                (a.y - a.vy) as i32,
                1,
                1,
                LINEN,
            );
        }
        // the sight -------------------------------------------------------------
        if self.phase != Phase::Board && self.phase != Phase::Done {
            let (sx, sy) = (self.sight.0 as i32, self.sight.1 as i32);
            fb.fill_rect(sx - 5, sy - 1, 11, 1, BLACK);
            fb.fill_rect(sx - 1, sy - 5, 1, 11, BLACK);
            fb.fill_rect(sx - 4, sy, 9, 1, WHITE);
            fb.fill_rect(sx, sy - 4, 1, 9, WHITE);
            fb.fill_rect(sx, sy, 1, 1, RED);
        }
        // HUD ---------------------------------------------------------------------
        draw_text_shadow(fb, 4, 2, "DEFEND", GOLD, BLACK);
        draw_text_shadow(fb, 72, 2, &format!("ARROWS {}", self.quiver), CREAM, BLACK);
        draw_text_shadow(
            fb,
            180,
            2,
            &format!("ROCKS {}", self.rocks_left),
            CREAM,
            BLACK,
        );
        draw_text_shadow(
            fb,
            258,
            2,
            &format!("LOST {}/{}", self.breaches, MAX_BREACH),
            if self.breaches > 0 { RED } else { CREAM },
            BLACK,
        );
        if self.plate_t > 0 {
            let c = if self.plate == "CLIMBER DOWN"
                || self.plate == "LADDER SHATTERED"
                || self.plate == "MARKSMAN"
            {
                GOLD
            } else if self.plate == "THEY ARE IN!" || self.plate == "QUIVER EMPTY" {
                RED
            } else {
                TAN
            };
            draw_text_shadow(fb, 100, 96, self.plate, c, BLACK);
        }
        match self.phase {
            Phase::Present => {
                if self.phase_t % 24 < 16 {
                    draw_text_shadow(fb, 104, 128, "TO THE WALL", WHITE, BLACK);
                }
            }
            Phase::Board => {
                let s = self.score.max(0) as u32;
                draw_text_shadow(fb, 110, 60, &format!("SCORE {}", s), GOLD, BLACK);
                let msg = if self.state == MgState::Lost {
                    "THE CASTLE IS LOST"
                } else {
                    "THE CASTLE HOLDS"
                };
                draw_text_shadow(fb, 84, 84, msg, if self.state == MgState::Lost { RED } else { GOLD }, BLACK);
            }
            Phase::Done => {
                let s = self.score.max(0) as u32;
                draw_text_shadow(fb, 110, 60, &format!("SCORE {}", s), GOLD, BLACK);
            }
            _ => {}
        }
    }
}
