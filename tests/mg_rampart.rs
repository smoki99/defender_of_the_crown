//! T602: the DEFEND siege — ladders mature on a seeded schedule, shafts
//! pick off climbers, a stone shatters a whole ladder, the quiver refills,
//! and an idle watch is overrun. Pure logic driven through a synthetic
//! Input; the web host stays a thin adapter.

use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::{Input, Raw, KEY_SPACE};
use defender_of_the_crown::minigames::framework::{MgState, Minigame};
use defender_of_the_crown::minigames::rampart::Rampart;

// mirrored geometry from the module (the module keeps them private)
const PRESENT: u32 = 90;
const LADDER_X: f32 = 78.0;
const BOW: (f32, f32) = (56.0, 50.0);
const ARROW_SPD: f32 = 9.0;
const CLIMB_TICKS: f32 = 150.0;
const TOP_Y: f32 = 44.0;
const QUIVER_MAX: u32 = 12;
const REGEN_EVERY: u32 = 26;
const ROCKS: u32 = 2;
const END_TICKS: u32 = 1400;

struct Sim {
    inp: Input,
    fb: Framebuffer,
    frame: u64,
}

impl Sim {
    fn new() -> Self {
        Self {
            inp: Input::new(),
            fb: Framebuffer::new(),
            frame: 0,
        }
    }
    /// One 60 Hz frame; queued events land as this frame's edges.
    fn frame(&mut self, g: &mut Rampart, events: &[Raw]) {
        for e in events {
            self.inp.push(*e);
        }
        self.frame += 1;
        self.inp.begin_frame(self.frame);
        g.tick(1.0 / 60.0, &self.inp, &mut self.fb);
    }
    fn idle(&mut self, g: &mut Rampart, ticks: u32) {
        for _ in 0..ticks {
            self.frame(g, &[]);
        }
    }
    /// Sight to (x, y) without clicking.
    fn aim(&mut self, g: &mut Rampart, x: i32, y: i32) {
        self.frame(g, &[Raw::Move { x, y }]);
    }
    /// Sight to (x, y) and loose one shaft this frame.
    fn loose(&mut self, g: &mut Rampart, x: i32, y: i32) {
        self.frame(
            g,
            &[Raw::Move { x, y }, Raw::Down { x, y }],
        );
        self.frame(g, &[Raw::Up { x, y }]);
    }
    fn tap_space(&mut self, g: &mut Rampart) {
        self.frame(g, &[Raw::Key { code: KEY_SPACE, pressed: true }, Raw::Key { code: KEY_SPACE, pressed: false }]);
    }
}

/// Wait out the banner, then run until `stop` or the siege ends.
fn run_until(g: &mut Rampart, sim: &mut Sim, stop: impl Fn(&Rampart) -> bool) {
    sim.idle(g, PRESENT + 1);
    for _ in 0..END_TICKS {
        if stop(g) || !matches!(g.state(), MgState::Running) {
            return;
        }
        sim.idle(g, 1);
    }
}

#[test]
fn an_idle_watch_is_overrun() {
    let mut g = Rampart::new(7);
    let mut sim = Sim::new();
    run_until(&mut g, &mut sim, |_| false);
    assert!(g.breaches() >= 3, "a passive wall must be breached thrice");
    assert!(matches!(g.state(), MgState::Lost), "three breaches lose the castle");
}

#[test]
fn a_levelled_bow_felleth_a_climber() {
    let mut g = Rampart::new(7);
    let mut sim = Sim::new();
    // wait for the first ladder to start its climb with a live climber
    run_until(&mut g, &mut sim, |g| g.climber_xy(0, 0).is_some());
    assert!(
        g.climber_xy(0, 0).is_some(),
        "wave one must put a man on the ladder"
    );
    let row = g.climber_xy(0, 0).unwrap().1;
    assert!(row > TOP_Y, "the climb starts at the road row, not the top");
    let lane = g.runner_xy(0, 0).unwrap().1;
    let rate = (lane - TOP_Y) / CLIMB_TICKS; // that row's px per tick

    // lead each shaft: flight time along the straight bow->sight line
    // times that row's climb rate; re-aim every couple of frames
    let mut killed = false;
    for _ in 0..400 {
        if !matches!(g.state(), MgState::Running) {
            break;
        }
        if let Some((cx, cy)) = g.climber_xy(0, 0) {
            let flight = ((cx - BOW.0).powi(2) + (cy - BOW.1).powi(2)).sqrt() / ARROW_SPD;
            let lead = flight * rate;
            let (x, y) = (cx as i32, (cy - lead) as i32);
            sim.loose(&mut g, x, y);
            sim.idle(&mut g, 2);
        } else {
            sim.idle(&mut g, 1);
        }
        if g.ladder(0).1 < 3 {
            killed = true;
            break;
        }
    }
    assert!(killed, "a leading shot must bring down a climber");
    assert!(g.quiver() < QUIVER_MAX, "loosing spends a shaft");

    // the field boys re-gather after REGEN_EVERY ticks
    let q = g.quiver();
    sim.idle(&mut g, REGEN_EVERY + 1);
    assert!(g.quiver() > q, "the quiver refills from the field");
}

#[test]
fn a_stone_shattereth_the_ladder() {
    let mut g = Rampart::new(7);
    let mut sim = Sim::new();
    // wait for wave one to plant its ladder (state Set = 2)
    run_until(&mut g, &mut sim, |g| g.ladder(0).0 >= 2);
    assert_eq!(g.ladder(0).0, 2, "wave one plants before any climb");
    // find its row: the base runner y gives the lane
    let lane = g.runner_xy(0, 0).unwrap().1;
    // sight near the ladder foot and let the stone go
    sim.aim(&mut g, (LADDER_X + 34.0) as i32, lane as i32);
    sim.tap_space(&mut g);
    assert_eq!(g.rocks_left(), ROCKS - 1, "one stone leaves the cart");
    // wait for it to land and shatter the ladder
    for _ in 0..120 {
        sim.idle(&mut g, 1);
        if g.ladder(0).0 == 4 {
            break;
        }
    }
    let (state, climbers, runners) = g.ladder(0);
    assert_eq!(state, 4, "the shattered ladder is done (Held)");
    assert_eq!(climbers, 0, "nobody rides a broken ladder");
    assert_eq!(runners, 0, "the whole team goes with it");
}

#[test]
fn the_same_seed_layeth_the_same_siege() {
    let snapshot = |seed: u64| {
        let mut g = Rampart::new(seed);
        let mut sim = Sim::new();
        sim.idle(&mut g, PRESENT + 1 + 700);
        (
            [
                g.ladder(0),
                g.ladder(1),
                g.ladder(2),
                g.ladder(3),
                g.ladder(4),
                g.ladder(5),
            ],
            g.breaches(),
            g.quiver(),
            g.rocks_left(),
        )
    };
    assert_eq!(snapshot(999), snapshot(999), "the siege must replay identically");
}
