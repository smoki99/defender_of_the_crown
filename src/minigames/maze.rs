//! mg-maze — RESCUE (SPEC 6 #6): the dungeon. The Saxon champion slips in
//! through the postern (bottom-left), must reach the baron's cell high in
//! the right-hand wall, cut him loose, and escort him back to the door —
//! all while four Norman gaolers jog their beats. Two lit torches wait in
//! the corridors: grab one and the guards cower frozen for a spell, and a
//! cowering guard can be chased off the beat for good.
//!
//! Geometry: the dungeon is the fixed 20x16 [`MAZE`] (`#` = stone); every
//! actor lives in pixel space and slides axis-separately through the 16 px
//! corridors with a 9 px collision box. Gaolers [`GUARDS`] ping-pong along
//! straight beats — their speed and phase come from the seed, the beats
//! themselves never move.

use super::framework::{resolve, MgState, Minigame, SPORTS};
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BROWN, CREAM, DARKGRAY, GOLD, GRAY, GRAY3, NAVY, NORMAN_RED, RED,
    SAXON_BLUE, SILVER, STONE, WHITE,
};
use crate::engine::input::{Input, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_UP};

const PRESENT: u32 = 90; // "TO THE DUNGEON" banner
const TABLEAU: u32 = 100; // results beat before Done
const CELL: f32 = 16.0;
const HALF: f32 = 4.0; // half of the 9 px collision box
const P_SPD: f32 = 1.6; // champion px/tick
const B_SPD: f32 = 1.6; // baron keeps up exactly
const TRAIL_LAG: usize = 90; // baron walks this many frames behind
const TRAIL_MAX: usize = 400;
const HIT_R: f32 = 7.0; // guard catch radius
const GRAB_R: f32 = 9.0; // torch / baron reach
const EXIT_R: f32 = 10.0; // door reach for the champion
const ESCORT_R: f32 = 34.0; // door reach for the baron
const FRIGHT_TICKS: u32 = 260; // a torch's fright spell
const INVULN: u32 = 100; // grace after a catch
const TIME_LIMIT: u32 = 3300; // the watch changes shifts (55 s)

// score weights — torches looted, every guard routed, no catch: ~100.
const W_FREED: i32 = 20;
const W_ESCAPE: i32 = 60;
const W_TORCH: i32 = 5;
const W_ROUT: i32 = 5;
const W_CATCH: i32 = -10;

/// The dungeon. Corridors are 1-cell wide with loops; spawn/exit sit in the
/// bottom-left, the baron's cell is the nook top-right.
const MAZE: [&str; 16] = [
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

const SPAWN: (f32, f32) = (24.0, 216.0); // cell (1,13) — postern door
const BARON_CELL: (f32, f32) = (280.0, 24.0); // cell (17,1)
const TORCHES: [(f32, f32); 2] = [(280.0, 120.0), (216.0, 184.0)]; // (17,7) (13,11)

/// Straight ping-pong beats (cell coords, every segment corridor-true).
const GUARDS: [[[f32; 2]; 2]; 4] = [
    [[7.0, 7.0], [15.0, 7.0]],   // mid street
    [[13.0, 3.0], [15.0, 3.0]],   // upper middle street — watches the column mouth
    [[5.0, 13.0], [16.0, 13.0]], // bottom street — the escort route
    [[3.0, 1.0], [15.0, 1.0]],   // top street — the baron's lane
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Present,
    Run,
    Board,
    Done,
}

#[derive(Clone, Copy)]
struct Guard {
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    spd: f32,
    t: f32, // px along the beat, ping-pongs in 0..=len
    dir: f32,
    routed: bool,
}

impl Guard {
    fn pos(&self) -> (f32, f32) {
        let len = (self.bx - self.ax).hypot(self.by - self.ay);
        let k = if len == 0.0 { 0.0 } else { self.t / len };
        (self.ax + (self.bx - self.ax) * k, self.ay + (self.by - self.ay) * k)
    }
    fn step(&mut self) {
        if self.routed {
            return;
        }
        let len = (self.bx - self.ax).hypot(self.by - self.ay);
        self.t += self.spd * self.dir;
        if self.t > len {
            self.t = len;
            self.dir = -1.0;
        } else if self.t < 0.0 {
            self.t = 0.0;
            self.dir = 1.0;
        }
    }
}

/// One RESCUE run. Pure logic; the host owns pause and the cheer timer.
pub struct Maze {
    tick_n: u32, // ticks since Run opened
    phase: Phase,
    phase_t: u32,
    player: (f32, f32),
    invuln: u32,
    caught: u32,
    guards: [Guard; GUARDS.len()],
    torch_taken: [bool; 2],
    fright: u32,
    freed: bool,
    baron: (f32, f32),
    trail: std::collections::VecDeque<(f32, f32)>,
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

fn wall(px: f32, py: f32) -> bool {
    let cx = (px / CELL) as i32;
    let cy = (py / CELL) as i32;
    if cx < 0 || cy < 0 || cx >= 20 || cy >= 16 {
        return true;
    }
    MAZE[cy as usize].as_bytes()[cx as usize] == b'#'
}

/// Would a collision box centred here overlap stone?
fn blocked(px: f32, py: f32) -> bool {
    wall(px - HALF, py - HALF)
        || wall(px + HALF, py - HALF)
        || wall(px - HALF, py + HALF)
        || wall(px + HALF, py + HALF)
}

impl Maze {
    pub fn new(seed: u64) -> Self {
        // the beats are fixed; the gaolers' tempo and phase are the seed's
        let guards = std::array::from_fn(|i| {
            let beat = GUARDS[i];
            let len = (beat[1][0] - beat[0][0])
                .hypot(beat[1][1] - beat[0][1])
                * CELL;
            let m = mix(seed ^ (i as u64) << 17);
            Guard {
                ax: beat[0][0] * CELL + 8.0,
                ay: beat[0][1] * CELL + 8.0,
                bx: beat[1][0] * CELL + 8.0,
                by: beat[1][1] * CELL + 8.0,
                spd: 0.75 + (m % 450) as f32 / 1000.0, // 0.75..1.2
                t: ((m >> 32) % len as u64) as f32,
                dir: if (m >> 24) & 1 == 1 { 1.0 } else { -1.0 },
                routed: false,
            }
        });
        Self {
            tick_n: 0,
            phase: Phase::Present,
            phase_t: 0,
            player: SPAWN,
            invuln: 0,
            caught: 0,
            guards,
            torch_taken: [false; 2],
            fright: 0,
            freed: false,
            baron: BARON_CELL,
            trail: std::collections::VecDeque::new(),
            score: 0,
            plate: "",
            plate_t: 0,
            state: MgState::Running,
        }
    }

    // test/host introspection ------------------------------------------------
    pub fn player(&self) -> (f32, f32) {
        self.player
    }
    pub fn baron(&self) -> (f32, f32) {
        self.baron
    }
    pub fn freed(&self) -> bool {
        self.freed
    }
    pub fn caught(&self) -> u32 {
        self.caught
    }
    pub fn fright(&self) -> u32 {
        self.fright
    }
    /// `(x, y, routed)` of beat `i` — the beats never leave their line.
    pub fn guard(&self, i: usize) -> (f32, f32, bool) {
        let (x, y) = self.guards[i].pos();
        (x, y, self.guards[i].routed)
    }

    #[doc(hidden)] // test seam: put the champion mid-dungeon
    pub fn debug_place(&mut self, x: f32, y: f32) {
        self.player = (x, y);
        self.trail.clear();
    }

    fn say(&mut self, p: &'static str) {
        self.plate = p;
        self.plate_t = 60;
    }

    fn move_player(&mut self, input: &Input) {
        let mut dx = 0.0;
        let mut dy = 0.0;
        if input.key_down(KEY_LEFT) {
            dx -= 1.0;
        }
        if input.key_down(KEY_RIGHT) {
            dx += 1.0;
        }
        if input.key_down(KEY_UP) {
            dy -= 1.0;
        }
        if input.key_down(KEY_DOWN) {
            dy += 1.0;
        }
        let (px, py) = self.player;
        if dx != 0.0 && !blocked(px + dx * P_SPD, py) {
            self.player.0 = px + dx * P_SPD;
        }
        if dy != 0.0 && !blocked(px, py + dy * P_SPD) {
            self.player.1 = py + dy * P_SPD;
        }
    }

    fn run_torches(&mut self) {
        for (i, t) in TORCHES.iter().enumerate() {
            if !self.torch_taken[i]
                && (self.player.0 - t.0).hypot(self.player.1 - t.1) < GRAB_R
            {
                self.torch_taken[i] = true;
                self.fright = FRIGHT_TICKS;
                self.score += W_TORCH;
                self.say("TORCH! THEY COWER");
            }
        }
    }

    fn run_guards(&mut self) {
        if self.fright > 0 {
            self.fright -= 1;
            if self.fright == 0 {
                // the spell burns out — any routed gaoler quit for good
            }
        }
        for g in self.guards.iter_mut() {
            if self.fright == 0 {
                g.step();
            }
        }
        for i in 0..self.guards.len() {
            if self.guards[i].routed {
                continue;
            }
            let (gx, gy) = self.guards[i].pos();
            let d = (self.player.0 - gx).hypot(self.player.1 - gy);
            if d >= HIT_R {
                continue;
            }
            if self.fright > 0 {
                self.guards[i].routed = true;
                self.score += W_ROUT;
                self.say("GUARD ROUTED!");
            } else if self.invuln == 0 {
                self.caught += 1;
                self.score += W_CATCH;
                self.player = SPAWN;
                self.invuln = INVULN;
                self.trail.clear();
                if self.freed {
                    self.freed = false;
                    self.baron = BARON_CELL;
                    self.say("BOTH BACK IN CHAINS");
                } else {
                    self.say("SEIZED AT THE WALL");
                }
            }
        }
    }

    fn run_baron(&mut self) {
        let (bx, by) = self.baron;
        if !self.freed {
            if (self.player.0 - bx).hypot(self.player.1 - by) < GRAB_R {
                self.freed = true;
                self.score += W_FREED;
                self.say("THE BARON IS FREE");
                self.trail.clear();
            }
            return;
        }
        // he follows the champion's footprints, TRAIL_LAG frames behind
        self.trail.push_front(self.player);
        while self.trail.len() > TRAIL_MAX {
            self.trail.pop_back();
        }
        let lag = TRAIL_LAG.min(self.trail.len() - 1);
        let (tx, ty) = self.trail[lag];
        let d = ((tx - bx).powi(2) + (ty - by).powi(2)).sqrt();
        if d > 1.0 {
            let k = (d.min(B_SPD)) / d;
            self.baron = (bx + (tx - bx) * k, by + (ty - by) * k);
        }
        let e = |p: (f32, f32)| (p.0 - SPAWN.0).hypot(p.1 - SPAWN.1);
        if e(self.player) < EXIT_R && e(self.baron) < ESCORT_R {
            self.score += W_ESCAPE;
            self.score = self.score.clamp(0, 100);
            self.state = resolve(self.score as u32, &SPORTS[5]);
            self.phase = Phase::Board;
            self.phase_t = 0;
        }
    }
}

impl Minigame for Maze {
    fn id(&self) -> &'static str {
        "maze"
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
        if self.invuln > 0 {
            self.invuln -= 1;
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
                self.move_player(input);
                self.run_torches();
                self.run_guards();
                self.run_baron();
                if self.phase != Phase::Run {
                    self.paint(fb);
                    return; // the door opened under us
                }
                if self.tick_n >= TIME_LIMIT {
                    self.score = self.score.clamp(0, 100);
                    self.state = resolve(self.score as u32, &SPORTS[5]);
                    self.phase = Phase::Board;
                    self.phase_t = 0;
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

impl Maze {
    fn paint(&self, fb: &mut Framebuffer) {
        // floor and stone ------------------------------------------------------
        fb.fill_rect(0, 0, 320, 256, BLACK);
        for (cy, row) in MAZE.iter().enumerate() {
            for (cx, c) in row.bytes().enumerate() {
                let (x, y) = (cx as i32 * 16, cy as i32 * 16);
                if c == b'#' {
                    fb.fill_rect(x, y, 16, 16, STONE);
                    fb.fill_rect(x, y, 16, 1, GRAY);
                    fb.fill_rect(x + 15, y, 1, 16, BLACK);
                    fb.fill_rect(x + 7, y + 3, 4, 3, GRAY3);
                } else {
                    fb.fill_rect(x, y, 16, 16, DARKGRAY);
                    if (cx * 7 + cy * 13) % 11 == 0 {
                        fb.fill_rect(x + 3, y + 9, 1, 1, BLACK);
                    }
                }
            }
        }
        // the postern -----------------------------------------------------------
        let flick = self.phase == Phase::Run && self.freed && self.tick_n % 30 < 18;
        fb.fill_rect(SPAWN.0 as i32 - 6, SPAWN.1 as i32 - 7, 12, 14, BROWN);
        fb.rect(SPAWN.0 as i32 - 6, SPAWN.1 as i32 - 7, 12, 14, if flick { GOLD } else { BLACK });
        fb.fill_rect(SPAWN.0 as i32 - 4, SPAWN.1 as i32 - 5, 8, 10, if flick { GOLD } else { NAVY });
        // the baron's cell: iron bars across the nook ---------------------------
        for k in 0..3 {
            fb.fill_rect(265, 25 + k * 5, 1, 4, GRAY3);
        }
        // torches ----------------------------------------------------------------
        for (i, t) in TORCHES.iter().enumerate() {
            if self.torch_taken[i] {
                continue;
            }
            let (tx, ty) = (t.0 as i32, t.1 as i32);
            fb.fill_rect(tx - 1, ty - 2, 2, 7, BROWN);
            let f = self.tick_n % 8 < 4;
            fb.fill_rect(tx - 2, ty - 6 - if f { 1 } else { 0 }, 4, 4, GOLD);
            fb.fill_rect(tx - 1, ty - 7 - if f { 1 } else { 0 }, 2, 2, RED);
        }
        // the baron -----------------------------------------------------------------
        let (bx, by) = (self.baron.0 as i32, self.baron.1 as i32);
        let hop = if self.freed && self.tick_n % 24 < 12 { -1 } else { 0 };
        fb.fill_rect(bx - 3, by - 4 + hop, 6, 9, CREAM);
        fb.fill_rect(bx - 3, by - 7 + hop, 6, 3, GOLD);
        if !self.freed {
            fb.fill_rect(bx - 5, by - 2, 2, 4, GRAY3);
            fb.fill_rect(bx + 3, by - 2, 2, 4, GRAY3);
        }
        // gaolers --------------------------------------------------------------------
        for i in 0..self.guards.len() {
            let (gx, gy, routed) = self.guard(i);
            if routed {
                continue;
            }
            let (gx, gy) = (gx as i32, gy as i32);
            let quake = if self.fright > 0 { (self.tick_n % 4 / 2) as i32 } else { 0 };
            if self.fright > 0 {
                fb.fill_rect(gx - 3 + quake, gy - 4, 6, 9, SILVER);
                fb.fill_rect(gx - 3 + quake, gy - 7, 6, 3, SAXON_BLUE);
            } else {
                fb.fill_rect(gx - 3, gy - 4, 6, 9, NORMAN_RED);
                fb.fill_rect(gx - 3, gy - 7, 6, 3, SILVER);
            }
        }
        // the champion ------------------------------------------------------------------
        let blink = self.invuln > 0 && self.tick_n % 6 < 3;
        if !blink && self.phase != Phase::Board && self.phase != Phase::Done {
            let (px, py) = (self.player.0 as i32, self.player.1 as i32);
            fb.fill_rect(px - 3, py - 4, 6, 9, SAXON_BLUE);
            fb.fill_rect(px - 3, py - 7, 6, 3, SILVER);
            fb.fill_rect(px - 1, py - 9, 2, 2, RED);
        }
        // HUD ------------------------------------------------------------------------------
        draw_text_shadow(fb, 4, 2, "RESCUE", GOLD, BLACK);
        let goal = if self.freed { "ESCORT HIM OUT" } else { "FIND THE BARON" };
        draw_text_shadow(
            fb,
            104,
            2,
            goal,
            if self.freed { GOLD } else { CREAM },
            BLACK,
        );
        draw_text_shadow(
            fb,
            246,
            2,
            &format!("CAUGHT {}", self.caught),
            if self.caught > 0 { RED } else { CREAM },
            BLACK,
        );
        if self.fright > 0 {
            let w = (self.fright as i32 * 300) / FRIGHT_TICKS as i32;
            fb.fill_rect(10, 13, w as u32, 1, GOLD);
        }
        if self.plate_t > 0 {
            let c = if self.plate == "SEIZED AT THE WALL" || self.plate == "BOTH BACK IN CHAINS" {
                RED
            } else {
                GOLD
            };
            draw_text_shadow(fb, 100, 120, self.plate, c, BLACK);
        }
        match self.phase {
            Phase::Present => {
                if self.phase_t % 24 < 16 {
                    crate::ui::kit::centered(fb, 128, "TO THE DUNGEON", WHITE, BLACK);
                }
            }
            Phase::Board | Phase::Done => {
                let s = self.score.max(0) as u32;
                draw_text_shadow(fb, 110, 60, &format!("SCORE {}", s), GOLD, BLACK);
                let msg = match self.state {
                    MgState::Lost => "STILL IN CHAINS",
                    _ => "THE BARON IS FREE",
                };
                let col = if self.state == MgState::Lost { RED } else { GOLD };
                draw_text_shadow(fb, 96, 84, msg, col, BLACK);
            }
            _ => {}
        }
    }
}
