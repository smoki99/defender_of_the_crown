//! mg-rampart — DEFEND (SPEC 6 #5): the wall at dawn. Scaling ladders drop
//! in six waves along the curtain; loose arrows in the wall-walk quiver
//! (they re-gather off the field), and two carts of rocks wait on the
//! battlements for the fat of a climbing party. Aim with the mouse, loose
//! with the button, tip the rocks with SPACE. Three men over the wall and
//! the castle is lost.

use super::framework::{resolve, MgState, Minigame, SportSpec, SPORTS};
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BLOOD, BROWN, CREAM, DARKGRAY, GOLD, GREEN, GRAY, LINEN, NAVY,
    NORMAN_RED, RED, SAXON_BLUE, SILVER, STONE, TAN, WHITE,
};
use crate::engine::input::{Input, KEY_SPACE};

const PRESENT: u32 = 90; // "TO THE WALL" banner
const TABLEAU: u32 = 100; // results beat before Done
const WAVES: usize = 6;
const WAVE_GAP: u32 = 130; // ticks between ladder deployments
const MARCH_TICKS: u32 = 96; // right edge to the curtain
const MARCH_SPD: f32 = 2.4; // px per tick
const MARCH_X0: f32 = 316.0;
const LADDER_X: f32 = 78.0; // centre of the ladder run
const SET_TICKS: u32 = 40; // planting the ladder
const CLIMB_SPD: f32 = 1.1; // px per tick up the face
const STAGGER: u32 = 30; // ticks between the three climbers
const TOP_Y: f32 = 44.0; // battlements — over this and they are in
const WALL_X0: i32 = 44;
const WALL_X1: i32 = 68;
const GROUND_Y: i32 = 232;
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
    seed: u64,
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
        let ladders = [Ladder {
            state: LState::Unborn,
            t: 0,
            lane: 0.0,
            runner: [false; 2],
            climber: [false; 3],
            fell: false,
        }; WAVES];
        Self {
            seed,
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
                y: 0.0;
                ROCKS as usize
            ]..try_into().unwrap(),
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
