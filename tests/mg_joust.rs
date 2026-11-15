//! T502: mg-joust core — lane matching, lance timing, tiers, honesty.
//! The tests drive the pure core through the same Input/Framebuffer the
//! web host feeds it.

use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::{Input, Raw, KEY_LEFT, KEY_RIGHT, KEY_SPACE};
use defender_of_the_crown::minigames::framework::{MgState, Minigame};
use defender_of_the_crown::minigames::joust::Joust;

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
    fn frame(&mut self, g: &mut Joust, events: &[Raw]) {
        for e in events {
            self.inp.push(*e);
        }
        self.frame += 1;
        self.inp.begin_frame(self.frame);
        g.tick(1.0 / 60.0, &self.inp, &mut self.fb);
    }
    fn tap_key(&mut self, g: &mut Joust, code: u32) {
        self.frame(g, &[Raw::Key { code, pressed: true }, Raw::Key { code, pressed: false }]);
    }
    /// Steer toward a rail (one press per frame), then keep cantering.
    fn steer(&mut self, g: &mut Joust, lane: usize, settle: u32) {
        while g.target() != lane {
            let k = if g.target() < lane { KEY_RIGHT } else { KEY_LEFT };
            self.tap_key(g, k);
        }
        for _ in 0..settle {
            self.frame(g, &[]);
        }
    }
}

/// Gallop until the gap enters `band` (px around IDEAL), then throw the
/// lance. Returns false if the pass rode by unused.
fn lance_when(g: &mut Joust, s: &mut Sim, band: i32) -> bool {
    for _ in 0..300 {
        s.frame(g, &[]);
        let off = (g.gap() - 26.0).abs() as i32;
        if off <= band {
            s.tap_key(g, KEY_SPACE);
            return true;
        }
        if g.gap() <= 3.0 {
            return false;
        }
    }
    false
}

fn full_run(seed: u64, mut plan: impl FnMut(usize) -> (usize, i32)) -> (u32, MgState) {
    let mut g = Joust::new(seed);
    let mut s = Sim::new();
    for pass in 0..2 {
        // read the rival's rail the moment the pass opens, match it, lance
        let (want, band) = plan(g.rival_lane(pass));
        s.steer(&mut g, want, 4);
        lance_when(&mut g, &mut s, band);
        // let the clash + return (or crown) play out
        for _ in 0..(30 + if pass == 0 { 45 } else { 120 }) {
            s.frame(&mut g, &[]);
        }
    }
    (g.score(), g.state())
}

#[test]
fn crowned_double_pass_takes_the_wreath() {
    // both passes: same rail as the rival, lance inside the perfect band
    let (score, state) = full_run(7, |r| (r, 3));
    assert_eq!(score, 97, "two perfect passes + crown bonus");
    assert_eq!(state, MgState::Won(defender_of_the_crown::minigames::framework::Tier::Gold));
}

#[test]
fn one_solid_one_perfect_lands_silver() {
    let mut seen = 0;
    let (score, state) = full_run(11, |r| {
        seen += 1;
        if seen == 1 {
            (r, 10) // solid band (8..=12 around ideal)
        } else {
            (r, 3)
        }
    });
    assert!((70..95).contains(&score), "score {score} must land silver");
    assert_eq!(state, MgState::Won(defender_of_the_crown::minigames::framework::Tier::Silver));
}

#[test]
fn wrong_rail_both_times_sends_thee_home_empty() {
    let (score, state) = full_run(3, |r| ((r + 1) % 3, 3));
    assert_eq!(score, 0, "sheared twice scores nothing");
    assert_eq!(state, MgState::Lost);
}

#[test]
fn lancing_early_misses_nothing_and_costs_nothing() {
    // throw the lance while still far from the tilt: clean air, no penalty
    let mut g = Joust::new(5);
    let mut s = Sim::new();
    let rl = g.rival_lane(0);
    s.steer(&mut g, rl, 4);
    for _ in 0..120 {
        s.frame(&mut g, &[]);
    }
    assert!(g.gap() > 60.0, "still far from the tilt");
    s.tap_key(&mut g, KEY_SPACE);
    for _ in 0..300 {
        s.frame(&mut g, &[]);
        if g.pass() == 1 && g.gap() > 170.0 {
            break;
        }
    }
    assert_eq!(g.pass(), 1, "the early whiff ends pass one");
    assert_eq!(g.score(), 0, "early lance is a whiff, not a foul");
    let rl = g.rival_lane(1);
    s.steer(&mut g, rl, 4);
    assert!(lance_when(&mut g, &mut s, 3));
    for _ in 0..150 {
        s.frame(&mut g, &[]);
    }
    assert!(g.score() >= 45, "second pass still scores: {}", g.score());
}

#[test]
fn factory_arms_joust_and_friends() {
    use defender_of_the_crown::minigames::framework::SPORTS;
    use defender_of_the_crown::ui::host::{game_exists, make_game};
    assert!(make_game("joust", 1).is_some(), "joust shipped");
    assert!(make_game("dragon", 1).is_some(), "dragon shipped");
    assert!(game_exists());
    let other = SPORTS.iter().filter(|s| s.id != "joust").count();
    assert_eq!(other, 12);
}
