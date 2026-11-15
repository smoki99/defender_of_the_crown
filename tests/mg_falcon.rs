//! T702: the FALCONER's corn field. Ten crows launch on the seed's clock
//! and each claims a standing ear; the hawk answers the mouse through
//! mass - no teleporting, and a glide that overshoots. Any touch turns a
//! crow back; only a crow that clears the field with the corn makes the
//! ear count lost. Score is the intercept rate: ten stopped is the 100
//! gold line, and the seed alone decides where the crows rise.

use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::{Input, Raw};
use defender_of_the_crown::minigames::falcon::Falcon;
use defender_of_the_crown::minigames::framework::{MgState, Minigame, Tier};

const PRESENT: u32 = 90;
const GONE: u8 = 3;

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
    fn frame(&mut self, g: &mut Falcon, events: &[Raw]) {
        for e in events {
            self.inp.push(*e);
        }
        self.frame += 1;
        self.inp.begin_frame(self.frame);
        g.tick(1.0 / 60.0, &self.inp, &mut self.fb);
    }
    fn idle(&mut self, g: &mut Falcon, ticks: u32) {
        for _ in 0..ticks {
            self.frame(g, &[]);
        }
    }
    /// Steer the hawk: the mouse is the whole control surface.
    fn steer(&mut self, g: &mut Falcon, x: i32, y: i32) {
        self.frame(g, &[Raw::Move { x, y }]);
    }
}

fn open_run(seed: u64) -> (Sim, Falcon) {
    let mut sim = Sim::new();
    let mut g = Falcon::new(seed);
    sim.idle(&mut g, PRESENT + 1); // burn the banner; first crow away
    (sim, g)
}

/// Chase crow `i` until it resolves; the caller asserts the outcome.
fn chase(sim: &mut Sim, g: &mut Falcon, i: usize, budget: u32) {
    for _ in 0..budget {
        let (x, y) = g.crow_pos(i);
        sim.steer(g, x, y);
        if g.crow_stage(i) == GONE {
            return;
        }
    }
    panic!("crow {i} never resolved");
}

#[test]
fn banner_holds_the_field() {
    let mut sim = Sim::new();
    let mut g = Falcon::new(7);
    sim.idle(&mut g, PRESENT); // banner still up on the last banner tick
    assert_eq!(g.live_crow(), None, "no crow while the banner shows");
    assert_eq!(g.state(), MgState::Running);
    sim.idle(&mut g, 1); // first Run tick launches crow 0
    assert_eq!(g.live_crow(), Some(0));
}

#[test]
fn crows_claim_distinct_ears() {
    let (mut sim, mut g) = open_run(3);
    let e0 = g.crow_ear(0);
    assert!(e0 < 10, "crow 0 must want a real ear");
    sim.idle(&mut g, 150); // crow 1 launches on the spawn clock
    assert_eq!(g.live_crow(), Some(0), "crow 0 is still in play");
    assert_ne!(g.crow_ear(1), e0, "two crows never share an ear");
}

#[test]
fn hawk_touch_turns_the_crow() {
    let (mut sim, mut g) = open_run(5);
    chase(&mut sim, &mut g, 0, 300);
    assert_eq!(g.inter(), 1);
    assert_eq!(g.ears_lost(), 0, "turned back means the ear stays");
}

#[test]
fn unhawked_crow_carries_the_ear_away() {
    let (mut sim, mut g) = open_run(5);
    // park the hawk in the corner: nothing comes near it
    for _ in 0..1200 {
        sim.steer(&mut g, 0, 0);
        if g.crow_stage(0) == GONE {
            break;
        }
    }
    assert_eq!(g.inter(), 0);
    assert_eq!(g.crow_stage(0), GONE);
    assert_eq!(g.ears_lost(), 1, "the crow left with the corn");
}

#[test]
fn rescue_during_the_drag_gives_the_ear_back() {
    let (mut sim, mut g) = open_run(11);
    // park the hawk away and wait for any crow to lift an ear (the
    // flight home may graze one on the way - that is allowed)
    let mut victim = None;
    for _ in 0..1500 {
        sim.steer(&mut g, 0, 0);
        if let Some(i) = (0..10).find(|&i| g.crow_stage(i) == 2) {
            victim = Some(i);
            break;
        }
    }
    let t = victim.expect("a crow should have an ear by now");
    let ear = g.crow_ear(t);
    let before = g.inter();
    assert!(!g.ear_lost(ear), "still on the field, still defended");
    chase(&mut sim, &mut g, t, 600);
    assert_eq!(g.inter(), before + 1, "the rescue counts once");
    assert!(!g.ear_lost(ear), "the rescue returns the ear");
}

#[test]
fn perfect_wave_is_gold() {
    let (mut sim, mut g) = open_run(21);
    for _ in 0..4000 {
        if g.state() != MgState::Running {
            break;
        }
        if let Some(i) = g.live_crow() {
            let (x, y) = g.crow_pos(i);
            sim.steer(&mut g, x, y);
        } else {
            sim.idle(&mut g, 1);
        }
    }
    assert_eq!(g.inter(), 10, "every crow turned back");
    assert_eq!(g.ears_lost(), 0);
    assert_eq!(g.score(), 100);
    assert!(matches!(g.state(), MgState::Won(Tier::Gold)));
}

#[test]
fn all_escape_is_a_loss() {
    let (mut sim, mut g) = open_run(21);
    for _ in 0..4000 {
        sim.steer(&mut g, 0, 0);
        if g.resolved() == 10 {
            break;
        }
    }
    assert_eq!(g.resolved(), 10);
    // a hawk flying home to its corner may graze a rising crow on the
    // way - but it never hunts, so the wave must end in defeat
    assert!(
        g.inter() <= 3,
        "a parked hawk at most grazes: {}",
        g.inter()
    );
    assert_eq!(g.score(), g.inter() * 10);
    assert_eq!(g.state(), MgState::Lost);
}

#[test]
fn seed_drives_the_field_alone() {
    let (mut a, mut ga) = open_run(9);
    let (mut b, mut gb) = open_run(9);
    let (mut c, mut gc) = open_run(10);
    for _ in 0..1400 {
        a.idle(&mut ga, 1);
        b.idle(&mut gb, 1);
        c.idle(&mut gc, 1);
    }
    for i in 0..10 {
        assert_eq!(ga.crow_pos(i), gb.crow_pos(i), "same seed, same field");
    }
    let differs = (0..10).any(|i| ga.crow_pos(i) != gc.crow_pos(i));
    assert!(differs, "a different seed must fly differently");
}

#[test]
fn hawk_has_mass_and_an_overshoot() {
    let (mut sim, mut g) = open_run(1);
    let (x0, y0) = g.hawk_pos();
    sim.steer(&mut g, 312, 248);
    let (x1, y1) = g.hawk_pos();
    let step = ((x1 - x0) * (x1 - x0) + (y1 - y0) * (y1 - y0)).sqrt();
    assert!(
        step < 4.0,
        "the hawk accelerates, it never teleports: {step}"
    );
    for _ in 0..30 {
        sim.steer(&mut g, 312, 248);
    }
    // flip mid-flight, while the hawk still has way on: the glide must
    // carry it past its furthest point before the turn completes
    let (xb, _) = g.hawk_pos();
    assert!(xb > x0 + 10.0, "the hawk should have way on: {xb}");
    let mut farthest = xb;
    for _ in 0..40 {
        sim.steer(&mut g, 8, 16);
        let (x, _) = g.hawk_pos();
        farthest = farthest.max(x);
    }
    assert!(
        farthest > xb + 1.0,
        "inertia must overshoot the flip: {xb} -> {farthest}"
    );
    // and a long hold still arrives: the toy is controllable, not wild
    for _ in 0..200 {
        sim.steer(&mut g, 8, 16);
    }
    let (xe, ye) = g.hawk_pos();
    assert!(xe < 20.0 && ye < 40.0, "home at last: {xe},{ye}");
}

#[test]
fn a_dead_crow_stays_dead() {
    let (mut sim, mut g) = open_run(5);
    chase(&mut sim, &mut g, 0, 300);
    let (x, y) = g.crow_pos(0);
    for _ in 0..120 {
        sim.steer(&mut g, x, y); // hover on the corpse
    }
    assert_eq!(g.inter(), 1, "no double count on one crow");
}
