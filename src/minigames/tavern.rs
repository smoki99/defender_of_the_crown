//! mg-tavern - BRAWL (SPEC 6 #12): three waves of ruffians through the
//! common room, and only your fists, your grip and three stout barrels
//! between them and the barmaid. A punch staggers, a punch on a staggered
//! rogue grabs him, a punch while gripping hurls him - and a hurled rogue
//! bowls over everyone he flies into. Roll a barrel and it flattens the
//! line until it dies at the wall. One rule above all: the barmaid
//! crossing the floor with the ales is OUT OF BOUNDS - foul her and the
//! whole room howls at you. Six a rogue; fifteen rogues, and only a
//! clean floor (never fouled) adds the +2 that tops a sweep to 92 gold.
//! Ruffians time and place themselves; the seed's only dice is where
//! the barmaid returns after a foul.

use super::framework::{resolve, MgState, Minigame, SPORTS};
use crate::campaign::rng::Xor;
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BROWN, GOLD, NAVY, RED, SAXON_BLUE, SILVER, TAN, WHITE, YELLOW,
};
use crate::engine::input::{Input, KEY_LEFT, KEY_RIGHT, KEY_SPACE};

const PRESENT: u32 = 90; // "THE TAVERN BRAWL" banner
const WAVES: [usize; 3] = [4, 5, 6]; // rogues per wave, 15 in all
const W_START: [usize; 3] = [0, 4, 9]; // pool slice per wave
const ROGUES: usize = 15;
const W_GAP: u32 = 60; // calm between waves
const IN_GAP: u32 = 45; // ticks between two rogues of a wave
const W_SPEED: [f32; 3] = [0.6, 0.75, 0.9];

// rogue stages
const MARCH: u8 = 0;
const STAG: u8 = 1;
const GRAB: u8 = 2;
const THROW: u8 = 3;
const DOWN: u8 = 4; // KO on the floor, out of play
const NO_R: usize = usize::MAX;

const FLOOR_Y: i32 = 170; // sprite baseline row
const P_MIN: f32 = 12.0;
const P_MAX: f32 = 308.0;
const P_SPEED: f32 = 2.2;
const P_HIT: f32 = 16.0; // punch lands within 4..16 px of the facing
const P_CD: u32 = 14; // punch cooldown
const STAG_T: u32 = 72; // a stagger wears off
const SWING: u32 = 70; // rogue swing cadence once in melee
const STUN: u32 = 30; // player stun after eating one
const THROW_V: f32 = 5.5;
const BAR_V: f32 = 4.0;

// barmaid patrol: counter beats, then floor crossings (the foul window)
const BM_COUNTER: u32 = 90;
const BM_FLOOR: u32 = 120;

#[derive(Clone, Copy)]
struct Rogue {
    x: f32,
    state: u8,
    timer: u32, // stagger countdown
    vx: f32,    // throw velocity
    swing: u32, // swing cooldown
}

#[derive(Clone, Copy)]
struct Barrel {
    x: f32,
    roll: f32, // 0 = standing, != 0 = rolling this way
}

#[derive(Clone, Copy)]
struct Barmaid {
    x: f32,
    dir: f32,
    floor: bool, // on the brawl row right now
    timer: u32,
    gone: bool, // fled after a foul
    gone_t: u32,
}

pub struct Tavern {
    tick_n: u32,
    phase: Phase,
    wave: usize, // 1..=3
    wave_ko: usize, // rogues down in the current wave
    gap: u32, // inter-wave / spawn countdown
    to_spawn: usize, // rogues of this wave still offstage
    rf: [Rogue; ROGUES],
    ko: u32,
    px: f32,
    face: i32,
    stun: u32,
    p_cd: u32,
    p_fx: u8, // fist-out frames
    held: usize, // NO_R, or the rogue overhead
    barrels: [Barrel; 3],
    maid: Barmaid,
    fouls: u32,
    flash: u32, // "BARMAID!" howl
    state: MgState,
    rng: Xor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Present,
    Run,
    Done,
}

impl Tavern {
    pub fn new(seed: u64) -> Self {
        Tavern {
            tick_n: 0,
            phase: Phase::Present,
            wave: 1,
            wave_ko: 0,
            gap: 1,
            to_spawn: WAVES[0],
            rf: [Rogue {
                x: -40.0, // waiting offstage until the wave calls him
                state: MARCH,
                timer: 0,
                vx: 0.0,
                swing: 0,
            }; ROGUES],
            ko: 0,
            px: 160.0,
            face: 1,
            stun: 0,
            p_cd: 0,
            p_fx: 0,
            held: NO_R,
            barrels: [
                Barrel { x: 90.0, roll: 0.0 },
                Barrel { x: 200.0, roll: 0.0 },
                Barrel { x: 240.0, roll: 0.0 },
            ],
            maid: Barmaid {
                x: 120.0,
                dir: 0.6,
                floor: false,
                timer: BM_COUNTER,
                gone: false,
                gone_t: 0,
            },
            fouls: 0,
            flash: 0,
            state: MgState::Running,
            rng: Xor::new(seed ^ 0x54_41_56_33),
        }
    }

    // ---- test seams ----

    /// Test seam: player position and facing.
    #[doc(hidden)]
    pub fn px(&self) -> (f32, i32) {
        (self.px, self.face)
    }
    #[doc(hidden)]
    pub fn wave(&self) -> (usize, usize) {
        (self.wave, self.wave_ko)
    }
    #[doc(hidden)]
    pub fn ko(&self) -> u32 {
        self.ko
    }
    #[doc(hidden)]
    pub fn held(&self) -> Option<usize> {
        if self.held == NO_R {
            None
        } else {
            Some(self.held)
        }
    }
    #[doc(hidden)]
    pub fn rogue(&self, i: usize) -> (f32, u8) {
        (self.rf[i].x, self.rf[i].state)
    }
    #[doc(hidden)]
    pub fn barrels_left(&self) -> usize {
        self.barrels
            .iter()
            .filter(|b| b.roll == 0.0 && b.x > 0.0)
            .count()
    }
    /// Test seam: barmaid x and whether she is on the brawl floor now.
    #[doc(hidden)]
    pub fn maid(&self) -> (f32, bool) {
        (self.maid.x, self.maid.floor && !self.maid.gone)
    }
    #[doc(hidden)]
    pub fn fouls(&self) -> u32 {
        self.fouls
    }
    #[doc(hidden)]
    pub fn stunned(&self) -> bool {
        self.stun > 0
    }
    /// Test seam: next rogue of the live wave still in play on the floor.
    #[doc(hidden)]
    pub fn live_rogue(&self) -> Option<usize> {
        let hi = W_START[self.wave - 1] + WAVES[self.wave - 1];
        (W_START[self.wave - 1]..hi).find(|&i| self.rf[i].state < DOWN && self.rf[i].x > 0.0)
    }

    // ---- internals ----

    fn zone(&self) -> (f32, f32) {
        let a = self.px + 4.0 * self.face as f32;
        let b = self.px + P_HIT * self.face as f32;
        (a.min(b), a.max(b))
    }

    fn foul(&mut self) {
        // the room howls: the maid flees and returns on the seed's side
        self.fouls += 1;
        self.flash = 70;
        self.maid.gone = true;
        self.maid.gone_t = 100;
    }

    fn down(&mut self, i: usize) {
        if self.rf[i].state < DOWN {
            if self.held == i {
                self.held = NO_R;
            }
            self.rf[i].state = DOWN;
            self.ko += 1;
            self.wave_ko += 1;
        }
    }

    fn punch(&mut self) {
        self.p_cd = P_CD;
        self.p_fx = 8;
        if self.held != NO_R {
            // hurl the grip: he flies and flattens his friends
            let i = self.held;
            self.rf[i].state = THROW;
            self.rf[i].vx = self.face as f32 * THROW_V;
            self.held = NO_R;
            return;
        }
        let (z0, z1) = self.zone();
        // the barmaid rule first: a fist in her lane is a foul, period
        if self.maid.floor && !self.maid.gone && self.maid.x >= z0 && self.maid.x <= z1 {
            self.foul();
            return;
        }
        // barrels: a stout kick sends one rolling down the row - but
        // never into her: with a maid in the roll lane the kick holds
        // and the fist carries on to the rogues instead
        for bi in 0..self.barrels.len() {
            let b = self.barrels[bi];
            if b.roll == 0.0 && b.x > 0.0 && b.x >= z0 - 4.0 && b.x <= z1 + 4.0 {
                let lane_clear = !self.maid.floor
                    || self.maid.gone
                    || (self.face == 1 && self.maid.x < b.x - 8.0)
                    || (self.face == -1 && self.maid.x > b.x + 8.0);
                if lane_clear {
                    self.barrels[bi].roll = self.face as f32 * BAR_V;
                    return;
                }
            }
        }
        // rogues: march staggers, stagger is grabbed
        let hit = (0..ROGUES).find(|&i| {
            let r = self.rf[i];
            r.state == MARCH && r.x > 0.0 && r.x >= z0 && r.x <= z1
        });
        if let Some(i) = hit {
            self.rf[i].state = STAG;
            self.rf[i].timer = STAG_T;
            return;
        }
        let grip = (0..ROGUES).find(|&i| {
            let r = self.rf[i];
            r.state == STAG && r.x >= z0 && r.x <= z1
        });
        if let Some(i) = grip {
            self.rf[i].state = GRAB;
            self.held = i;
        }
    }

    fn move_player(&mut self, input: &Input) {
        if self.stun > 0 {
            self.stun -= 1;
            if self.held != NO_R {
                // stunned arms let go
                let i = self.held;
                self.rf[i].state = STAG;
                self.rf[i].timer = STAG_T;
                self.held = NO_R;
            }
            return;
        }
        if input.key_down(KEY_LEFT) {
            self.px -= P_SPEED;
            self.face = -1;
        } else if input.key_down(KEY_RIGHT) {
            self.px += P_SPEED;
            self.face = 1;
        }
        self.px = self.px.clamp(P_MIN, P_MAX);
        if self.p_fx > 0 {
            self.p_fx -= 1;
        }
        if self.p_cd > 0 {
            self.p_cd -= 1;
        } else if input.key_pressed(KEY_SPACE) {
            self.punch();
        }
    }

    fn waves(&mut self) {
        if self.to_spawn > 0 {
            if self.gap > 0 {
                self.gap -= 1;
                return;
            }
            // the next rogue of the wave enters through alternating doors
            let i = W_START[self.wave - 1] + (WAVES[self.wave - 1] - self.to_spawn);
            let side = (WAVES[self.wave - 1] - self.to_spawn) % 2 == 1;
            self.rf[i].x = if side { 328.0 } else { -8.0 };
            self.rf[i].state = MARCH;
            self.rf[i].swing = SWING / 2;
            self.to_spawn -= 1;
            self.gap = IN_GAP;
            return;
        }
        if self.wave_ko == WAVES[self.wave - 1] {
            if self.wave == 3 {
                self.finish();
            } else {
                self.wave += 1;
                self.wave_ko = 0;
                self.to_spawn = WAVES[self.wave - 1];
                self.gap = W_GAP;
            }
        }
    }

    fn brawl(&mut self) {
        let sp = W_SPEED[self.wave - 1];
        // belt-rule: the closest marcher is the one who swings, so a
        // pile of ruffians can never chain-stun the brawler forever
        let near = (0..ROGUES)
            .filter(|&j| self.rf[j].state == MARCH && self.rf[j].x > 0.0)
            .min_by(|&a, &b| {
                let da = (self.rf[a].x - self.px).abs();
                let db = (self.rf[b].x - self.px).abs();
                da.total_cmp(&db)
            })
            .unwrap_or(NO_R);
        for i in 0..ROGUES {
            let mut r = self.rf[i];
            match r.state {
                MARCH => {
                    if r.x <= 0.0 {
                        if r.x > -20.0 {
                            r.x += 1.6; // walking in through the left door
                        }
                    } else {
                        let ring = 24.0 + 6.0 * (i % 4) as f32;
                        let want = (self.px + (r.x - self.px).signum() * ring).clamp(8.0, 312.0);
                        let d = want - r.x;
                        if d.abs() > sp {
                            r.x += d.signum() * sp;
                        }
                        if r.swing > 0 {
                            r.swing -= 1;
                        } else if i == near && (r.x - self.px).abs() < 26.0 && self.stun == 0 {
                            self.stun = STUN;
                            r.swing = SWING;
                        }
                    }
                }
                STAG => {
                    r.timer -= 1;
                    if r.timer == 0 {
                        r.state = MARCH;
                    }
                }
                GRAB => {
                    r.x = self.px + 12.0 * self.face as f32;
                }
                THROW => {
                    r.x += r.vx;
                    // a flying rogue flattens marchers and staggers alike
                    for j in 0..ROGUES {
                        let o = self.rf[j];
                        if j != i
                            && o.x > 0.0
                            && (o.state == MARCH || o.state == STAG)
                            && (o.x - r.x).abs() < 10.0
                        {
                            self.down(j);
                        }
                    }
                    if self.maid.floor && !self.maid.gone && (self.maid.x - r.x).abs() < 8.0 {
                        self.foul();
                    }
                    if r.x < 10.0 || r.x > 310.0 {
                        r.state = DOWN;
                        self.down(i);
                    }
                }
                _ => {}
            }
            self.rf[i] = r;
        }
        // rolling barrels: one flat line, until the wall stops them
        for bi in 0..self.barrels.len() {
            let roll = self.barrels[bi].roll;
            if roll == 0.0 || self.barrels[bi].x <= 0.0 {
                continue;
            }
            self.barrels[bi].x += roll;
            let bx = self.barrels[bi].x;
            for j in 0..ROGUES {
                let o = self.rf[j];
                if o.x > 0.0 && (o.state == MARCH || o.state == STAG) && (o.x - bx).abs() < 9.0 {
                    self.down(j);
                }
            }
            if self.maid.floor && !self.maid.gone && (self.maid.x - bx).abs() < 8.0 {
                self.foul();
            }
            if bx < 8.0 || bx > 312.0 {
                self.barrels[bi].x = -1.0; // spent
            }
        }
        // the barmaid: counter beats, then a floor crossing to mind
        if self.maid.gone {
            self.maid.gone_t -= 1;
            if self.maid.gone_t == 0 {
                self.maid.gone = false;
                self.maid.floor = false;
                self.maid.timer = BM_COUNTER;
                self.maid.x = if self.rng.next_u64() & 1 == 0 { 60.0 } else { 220.0 };
                self.maid.dir = if self.maid.x < 160.0 { 0.6 } else { -0.6 };
            }
        } else {
            self.maid.timer -= 1;
            let lim = if self.maid.floor { 295.0 } else { 275.0 };
            self.maid.x += self.maid.dir * if self.maid.floor { 1.4 } else { 0.6 };
            if self.maid.x > lim || self.maid.x < 25.0 {
                self.maid.dir = -self.maid.dir.signum();
            }
            if self.maid.timer == 0 {
                self.maid.floor = !self.maid.floor;
                self.maid.timer = if self.maid.floor { BM_FLOOR } else { BM_COUNTER };
            }
        }
        if self.flash > 0 {
            self.flash -= 1;
        }
    }

    fn finish(&mut self) {
        self.phase = Phase::Done;
        self.state = resolve(self.score(), &SPORTS[11]);
    }

    // ---- rendering ----

    fn draw_room(&self, fb: &mut Framebuffer) {
        // common room: wall, floor, bar counter, tables
        fb.fill_rect(0, 12, 320, 244, NAVY);
        fb.fill_rect(0, 180, 320, 76, BROWN);
        for x in (8..320).step_by(24) {
            fb.fill_rect(x, 182, 2, 74, BLACK); // floorboard seams
        }
        fb.fill_rect(28, 104, 264, 10, BROWN); // bar
        fb.fill_rect(28, 104, 264, 2, TAN);
        fb.fill_rect(60, 140, 34, 4, TAN); // tables
        fb.fill_rect(64, 144, 3, 10, BROWN);
        fb.fill_rect(87, 144, 3, 10, BROWN);
        fb.fill_rect(220, 140, 34, 4, TAN);
        fb.fill_rect(224, 144, 3, 10, BROWN);
        fb.fill_rect(247, 144, 3, 10, BROWN);
        // barrels: upright until kicked, then rolling on their side
        for b in self.barrels.iter() {
            if b.x <= 0.0 {
                continue;
            }
            let bx = b.x as i32;
            if b.roll == 0.0 {
                fb.fill_rect(bx - 3, 162, 7, 9, BROWN);
                fb.fill_rect(bx - 3, 163, 7, 1, TAN);
                fb.fill_rect(bx - 3, 168, 7, 1, TAN);
            } else {
                fb.fill_rect(bx - 4, 166, 9, 6, BROWN);
                fb.fill_rect(bx - 1, 166, 2, 6, TAN);
            }
        }
        // the barmaid: red dress above the bar, crossing with the ales
        if !self.maid.gone {
            let mx = self.maid.x as i32;
            let my = if self.maid.floor { 160 } else { 94 };
            fb.fill_rect(mx - 2, my, 5, 9, RED);
            fb.fill_rect(mx - 1, my - 4, 3, 3, TAN);
            fb.fill_rect(mx - 1, my - 5, 3, 1, YELLOW);
            if self.maid.floor {
                fb.fill_rect(mx + self.maid.dir.signum() as i32 * 4, my + 2, 2, 2, TAN);
            }
        }
        // rogues: black bruisers with red bandanas
        for i in 0..ROGUES {
            let r = self.rf[i];
            if r.x <= 0.0 {
                continue;
            }
            let rx = r.x as i32;
            match r.state {
                DOWN => {
                    fb.fill_rect(rx - 5, FLOOR_Y + 4, 11, 3, BLACK);
                    fb.fill_rect(rx + 5, FLOOR_Y + 4, 2, 2, TAN);
                }
                THROW => {
                    fb.fill_rect(rx - 5, FLOOR_Y - 2, 11, 4, BLACK);
                    fb.fill_rect(rx - 6, FLOOR_Y - 2, 2, 2, RED);
                }
                GRAB => {
                    fb.fill_rect(rx - 3, FLOOR_Y - 16, 7, 9, BLACK);
                    fb.fill_rect(rx - 3, FLOOR_Y - 18, 7, 2, RED);
                    fb.fill_rect(rx + 4, FLOOR_Y - 14, 2, 2, TAN);
                }
                STAG => {
                    fb.fill_rect(rx - 3, FLOOR_Y - 8, 7, 12, BLACK);
                    fb.fill_rect(rx - 3, FLOOR_Y - 10, 7, 2, RED);
                    fb.fill_rect(rx + 4, FLOOR_Y - 6, 2, 3, TAN);
                    if self.tick_n % 8 < 4 {
                        fb.fill_rect(rx - 1, FLOOR_Y - 14, 1, 1, SILVER);
                        fb.fill_rect(rx + 3, FLOOR_Y - 16, 1, 1, SILVER);
                    }
                }
                _ => {
                    if r.x < 6.0 || r.x > 314.0 {
                        continue; // still in the doorway
                    }
                    fb.fill_rect(rx - 3, FLOOR_Y - 9, 7, 13, BLACK);
                    fb.fill_rect(rx - 3, FLOOR_Y - 11, 7, 2, RED);
                    fb.fill_rect(
                        rx + if r.x < self.px { 4 } else { -5 },
                        FLOOR_Y - 7,
                        2,
                        3,
                        TAN,
                    );
                    if self.tick_n / 10 % 2 == (i % 2) as u32 && r.swing == 0 {
                        fb.fill_rect(rx - 5, FLOOR_Y + 4, 3, 2, BLACK); // mid stride
                    }
                }
            }
        }
        // the brawler: blue tunic, white fists when swinging
        let px = self.px as i32;
        fb.fill_rect(px - 3, FLOOR_Y - 10, 7, 14, SAXON_BLUE);
        fb.fill_rect(px - 2, FLOOR_Y - 14, 5, 4, TAN);
        fb.fill_rect(px - 2, FLOOR_Y - 15, 5, 1, SILVER);
        if self.p_fx > 0 {
            let fx = px + self.face * 10;
            fb.fill_rect(fx - 1, FLOOR_Y - 7, 3, 3, WHITE);
        } else {
            fb.fill_rect(px - 5, FLOOR_Y - 6, 2, 3, WHITE);
            fb.fill_rect(px + 4, FLOOR_Y - 6, 2, 3, WHITE);
        }
        if self.stun > 0 && self.tick_n % 8 < 4 {
            fb.fill_rect(px - 4, FLOOR_Y - 19, 1, 1, YELLOW);
            fb.fill_rect(px + 4, FLOOR_Y - 20, 1, 1, YELLOW);
        }
        // room notes
        draw_text_shadow(fb, 4, 240, &format!("WAVE {}/3", self.wave), WHITE, BLACK);
        draw_text_shadow(fb, 110, 240, "PUNCH: SPACE", WHITE, BLACK);
        draw_text_shadow(
            fb,
            250,
            240,
            &format!("BARRELS {}", self.barrels_left()),
            TAN,
            BLACK,
        );
        if self.flash > 0 {
            draw_text_shadow(fb, 112, 120, "BARMAID!", YELLOW, RED);
        }
    }
}

impl Minigame for Tavern {
    fn id(&self) -> &'static str {
        SPORTS[11].id
    }

    fn tick(&mut self, _dt: f32, input: &Input, fb: &mut Framebuffer) {
        self.tick_n += 1;
        match self.phase {
            Phase::Present => {
                fb.fill_rect(0, 0, 320, 256, BLACK);
                draw_text_shadow(fb, 94, 116, "THE TAVERN BRAWL", GOLD, BLACK);
                draw_text_shadow(fb, 80, 136, "DON'T HIT THE BARMAID", WHITE, BLACK);
                if self.tick_n >= PRESENT {
                    self.phase = Phase::Run;
                }
            }
            Phase::Run => {
                self.move_player(input);
                self.waves();
                self.brawl();
                self.draw_room(fb);
            }
            Phase::Done => self.draw_room(fb),
        }
    }

    fn score(&self) -> u32 {
        let bonus = if self.phase == Phase::Done && self.fouls == 0 {
            2
        } else {
            0
        };
        (self.ko * 6 + bonus).saturating_sub(self.fouls * 10)
    }

    fn state(&self) -> MgState {
        self.state
    }
}
