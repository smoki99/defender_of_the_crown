//! mg-falcon — FALCONER (SPEC 6 #11): inertia flight over the corn. Ten
//! crows rise over the field and each picks one ear of corn; the hawk
//! answers the mouse through mass - acceleration, top speed and a glide
//! that overshoots - and any touch makes a crow give up. A crow that
//! reaches its ear grabs it and drags it off the top of the field at a
//! crawl; a rescue during the drag still counts and the ear stays. The
//! score is the intercept rate: every crow stopped is 100, nine of ten
//! (90) is still the 88 gold line, and the corn taken is the corn lost.
//! The spawn order, ear choice and wobble are the seed's only dice.

use super::framework::{resolve, MgState, Minigame, SPORTS};
use crate::campaign::rng::Xor;
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BROWN, DARKGREEN, GOLD, GREEN, NAVY, ORANGE, SILVER, TAN, WHITE, YELLOW,
};
use crate::engine::input::Input;

const PRESENT: u32 = 90; // "THE FALCON'S CAST" banner
const CROWS: usize = 10; // one wave, one ear each
const SAWN: u32 = 150; // ticks between launches (2.5 s)

// crow stages: 0 approaching, 1 hovering on the ear, 2 hauling it off,
// 3 gone (turned back or escaped), 4 still in the nest (not launched -
// it must NOT count as resolved or the wave dies at the first catch).
const GRAB_STAGE: u8 = 1;
const GONE: u8 = 3;
const PENDING: u8 = 4;
const NO_EAR: usize = usize::MAX;

// field geometry: grass strip along the bottom, ten ears on it
const FIELD_Y: i32 = 216; // grass top
const EAR_Y: i32 = 206;
fn ear_x(i: usize) -> i32 {
    16 + i as i32 * 29
}

// crow flight: descent speed, grab hover, drag-off crawl
const C_SPEED: f32 = 0.9; // px/tick toward the ear
const GRAB: u32 = 40; // ticks hovering at the ear before dragging
const DRAG: f32 = 0.55; // px/tick hauling the corn skyward

// hawk flight model: the toy is the sport
const H_ACC: f32 = 0.10; // blend toward the wanted velocity (mass)
const H_MAX: f32 = 3.2; // top speed px/tick
const H_GAIN: f32 = 0.14; // how hard the mouse pulls per tick
                          // A hawk tracking a 0.9 px/tick crow (plus 0.45 lateral) settles ~7 px
                          // behind it at this gain - the radius must clear that or pure chase can
                          // never close.
const CATCH: f32 = 14.0;

#[derive(Clone, Copy)]
struct Crow {
    x: f32,
    y: f32,
    ear: usize, // which ear it wants (NO_EAR = flying for nothing)
    stage: u8,
    timer: u32, // grab hover countdown
    dx: f32,    // lateral drift while carrying
    wob: u32,   // per-crow wobble phase
}

pub struct Falcon {
    tick_n: u32,
    phase: Phase,
    elapsed: u32, // ticks of active flight
    crows: [Crow; CROWS],
    spawned: usize, // crows launched so far
    hx: f32,        // hawk position and velocity: inertia, always
    hy: f32,
    vx: f32,
    vy: f32,
    inter: u32, // crows turned back (rescues included)
    ears_lost: [bool; CROWS],
    state: MgState,
    rng: Xor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Present,
    Run,
    Done,
}

impl Falcon {
    pub fn new(seed: u64) -> Self {
        Falcon {
            tick_n: 0,
            phase: Phase::Present,
            elapsed: 0,
            crows: [Crow {
                x: 0.0,
                y: 0.0,
                ear: 0,
                stage: PENDING, // not yet launched
                timer: 0,
                dx: 0.0,
                wob: 0,
            }; CROWS],
            spawned: 0,
            hx: 160.0,
            hy: 120.0,
            vx: 0.0,
            vy: 0.0,
            inter: 0,
            ears_lost: [false; CROWS],
            state: MgState::Running,
            rng: Xor::new(seed ^ 0x46_41_4C_43),
        }
    }

    /// Test seam: the most advanced crow still in play, if any.
    #[doc(hidden)]
    pub fn live_crow(&self) -> Option<usize> {
        (0..CROWS).find(|&i| self.crows[i].stage < GONE)
    }

    #[doc(hidden)]
    pub fn crow_pos(&self, i: usize) -> (i32, i32) {
        (self.crows[i].x as i32, self.crows[i].y as i32)
    }

    #[doc(hidden)]
    pub fn crow_stage(&self, i: usize) -> u8 {
        self.crows[i].stage
    }

    #[doc(hidden)]
    pub fn crow_ear(&self, i: usize) -> usize {
        self.crows[i].ear
    }

    #[doc(hidden)]
    pub fn inter(&self) -> u32 {
        self.inter
    }

    #[doc(hidden)]
    pub fn ears_lost(&self) -> u32 {
        self.ears_lost.iter().filter(|&&l| l).count() as u32
    }

    #[doc(hidden)]
    pub fn resolved(&self) -> u32 {
        self.crows.iter().filter(|c| c.stage == GONE).count() as u32
    }

    #[doc(hidden)]
    pub fn ear_lost(&self, e: usize) -> bool {
        self.ears_lost[e]
    }

    #[doc(hidden)]
    pub fn hawk_pos(&self) -> (f32, f32) {
        (self.hx, self.hy)
    }

    fn launch(&mut self) {
        // A crow takes the earliest ear still standing and unclaimed; a
        // stripped field sends it hovering on nothing, then home.
        let taken =
            |e: usize| self.ears_lost[e] || self.crows.iter().any(|c| c.stage < GONE && c.ear == e);
        let ear = (0..CROWS).find(|&e| !taken(e));
        let i = self.spawned;
        self.spawned += 1;
        let x = 20.0 + (self.rng.next_u64() % 281) as f32;
        self.crows[i] = Crow {
            x,
            y: -8.0,
            ear: ear.unwrap_or(NO_EAR),
            stage: ear.map_or(GRAB_STAGE, |_| 0),
            timer: if ear.is_none() { GRAB } else { 0 },
            dx: if self.rng.next_u64() & 1 == 0 {
                -0.3
            } else {
                0.3
            },
            wob: (self.rng.next_u64() % 64) as u32,
        };
    }

    fn fly_crows(&mut self) {
        for i in 0..CROWS {
            let mut c = self.crows[i];
            if c.stage >= GONE {
                continue;
            }
            match c.stage {
                0 => {
                    let pull = (ear_x(c.ear) as f32 - c.x).signum() * C_SPEED * 0.5;
                    let wob = ((self.elapsed + c.wob) % 64) as f32;
                    c.x += pull + (wob - 32.0) * 0.004;
                    c.y += C_SPEED;
                    if c.y >= EAR_Y as f32 - 4.0 {
                        c.y = EAR_Y as f32 - 4.0;
                        c.stage = GRAB_STAGE;
                        c.timer = GRAB;
                    }
                }
                GRAB_STAGE => {
                    c.timer -= 1;
                    if c.timer == 0 {
                        if c.ear < CROWS {
                            c.stage = 2;
                        } else {
                            c.stage = GONE; // nothing to take
                        }
                    }
                }
                _ => {
                    c.y -= DRAG;
                    c.x += c.dx;
                    if c.y < -16.0 {
                        // away with the corn: the ear only counts lost once
                        // the crow clears the field - a rescue during the
                        // drag gives it back
                        if c.ear < CROWS {
                            self.ears_lost[c.ear] = true;
                        }
                        c.stage = GONE;
                    }
                }
            }
            // the hawk: any touch in any stage turns the crow back
            let dx = c.x - self.hx;
            let dy = c.y - self.hy;
            if dx * dx + dy * dy < CATCH * CATCH {
                c.stage = GONE;
                self.inter += 1;
            }
            self.crows[i] = c;
        }
    }

    fn fly_hawk(&mut self, input: &Input) {
        let (mx, my, _) = input.mouse();
        let want_x = ((mx as f32 - self.hx) * H_GAIN).clamp(-H_MAX, H_MAX);
        let want_y = ((my as f32 - self.hy) * H_GAIN).clamp(-H_MAX, H_MAX);
        self.vx = self.vx * (1.0 - H_ACC) + want_x * H_ACC;
        self.vy = self.vy * (1.0 - H_ACC) + want_y * H_ACC;
        self.hx = (self.hx + self.vx).clamp(8.0, 312.0);
        self.hy = (self.hy + self.vy).clamp(16.0, 248.0);
    }

    fn finish(&mut self) {
        self.phase = Phase::Done;
        let sc = self.inter * 100 / CROWS as u32;
        self.state = resolve(sc, &SPORTS[10]);
    }

    fn draw_run(&self, fb: &mut Framebuffer) {
        // dusk field: sky, grass, standing corn
        fb.fill_rect(0, 12, 320, 244, NAVY);
        fb.fill_rect(0, FIELD_Y, 320, (256 - FIELD_Y) as u32, DARKGREEN);
        for x in (4..320).step_by(13) {
            fb.fill_rect(x, FIELD_Y + 2, 2, 4, GREEN);
        }
        for (e, &lost) in self.ears_lost.iter().enumerate() {
            let x = ear_x(e);
            let held = self.crows.iter().any(|k| k.stage == 2 && k.ear == e);
            if lost {
                fb.fill_rect(x + 2, EAR_Y + 8, 2, 6, BROWN); // stubble
            } else {
                fb.fill_rect(x + 2, EAR_Y + 2, 2, 12, GREEN); // stalk
                if !held {
                    fb.fill_rect(x, EAR_Y + 4, 6, 7, YELLOW); // ear
                    fb.fill_rect(x, EAR_Y + 3, 6, 2, TAN);
                }
            }
        }
        // crows: black bodies with a beating wing; the haul glows corn
        let flap = (self.elapsed / 8).is_multiple_of(2);
        for c in self.crows.iter() {
            if c.stage >= GONE {
                continue;
            }
            let (x, y) = (c.x as i32, c.y as i32);
            fb.fill_rect(x - 3, y - 1, 7, 3, BLACK);
            let wy = if flap { y - 3 } else { y + 1 };
            fb.fill_rect(x - 6, wy, 3, 2, BLACK);
            fb.fill_rect(x + 4, wy, 3, 2, BLACK);
            fb.fill_rect(x + 4, y - 2, 2, 2, ORANGE); // beak
            if c.stage == 2 {
                fb.fill_rect(x - 2, y + 3, 4, 5, YELLOW); // the stolen ear
            }
        }
        // the hawk: a brown kite with tan wingtips, gliding where thrown
        let (hx, hy) = (self.hx as i32, self.hy as i32);
        fb.fill_rect(hx - 4, hy - 1, 9, 3, BROWN);
        let wy = if (self.elapsed / 10).is_multiple_of(2) {
            hy - 4
        } else {
            hy + 2
        };
        fb.fill_rect(hx - 8, wy, 4, 2, TAN);
        fb.fill_rect(hx + 5, wy, 4, 2, TAN);
        fb.fill_rect(hx + 3, hy - 2, 2, 2, SILVER); // hood glint
                                                    // drop shadow keeps the flight on the field plane
        fb.fill_rect(hx - 2, FIELD_Y + 1, 5, 2, BLACK);
        draw_text_shadow(fb, 96, 240, "CATCH THE CROWS", WHITE, BLACK);
    }
}

impl Minigame for Falcon {
    fn id(&self) -> &'static str {
        SPORTS[10].id
    }

    fn tick(&mut self, _dt: f32, input: &Input, fb: &mut Framebuffer) {
        self.tick_n += 1;
        match self.phase {
            Phase::Present => {
                fb.fill_rect(0, 0, 320, 256, BLACK);
                draw_text_shadow(fb, 100, 116, "THE FALCON'S CAST", GOLD, BLACK);
                draw_text_shadow(fb, 100, 136, "CATCH THE CROWS", WHITE, BLACK);
                if self.tick_n >= PRESENT {
                    self.phase = Phase::Run;
                }
            }
            Phase::Run => {
                self.elapsed += 1;
                if self.spawned < CROWS && self.elapsed == 1 + self.spawned as u32 * SAWN {
                    self.launch();
                }
                self.fly_hawk(input);
                self.fly_crows();
                if self.resolved() == CROWS as u32 {
                    self.finish();
                }
            }
            Phase::Done => {}
        }
        if self.phase != Phase::Present {
            self.draw_run(fb);
        }
    }

    fn score(&self) -> u32 {
        self.inter * 100 / CROWS as u32
    }

    fn state(&self) -> MgState {
        self.state
    }
}
