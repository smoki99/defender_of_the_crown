//! T604: the CANOE run — the pole leaps over rocks and rides pike boats
//! into the falls. Pure logic driven through a synthetic Input. The
//! scripted canoe reads the public state, predicts the boat hull at the
//! arrival tick, steers into the lane and times the pole so the vault
//! window always covers the hazard band — a clean run must land GOLD.

use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::{Input, Raw, KEY_LEFT, KEY_RIGHT, KEY_SPACE};
use defender_of_the_crown::minigames::framework::{MgState, Minigame, Tier};
use defender_of_the_crown::minigames::river::River;

const PRESENT: u32 = 90;
const CURRENT: f32 = 0.9;
const TIME_LIMIT: u32 = 3400;

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
    fn frame(&mut self, g: &mut River, events: &[Raw]) {
        for e in events {
            self.inp.push(*e);
        }
        self.frame += 1;
        self.inp.begin_frame(self.frame);
        g.tick(1.0 / 60.0, &self.inp, &mut self.fb);
    }
    fn idle(&mut self, g: &mut River, ticks: u32) {
        for _ in 0..ticks {
            self.frame(g, &[]);
        }
    }
}

fn key(code: u32, down: bool) -> Raw {
    Raw::Key {
        code,
        pressed: down,
    }
}

#[derive(Default)]
struct Held {
    left: bool,
    right: bool,
    space: bool,
}

impl Held {
    /// Edge events to walk the held set to `want`; returns the events.
    fn diff(&mut self, want: (bool, bool, bool)) -> Vec<Raw> {
        let mut ev = Vec::new();
        if self.left && !want.0 {
            ev.push(key(KEY_LEFT, false));
        }
        if !self.left && want.0 {
            ev.push(key(KEY_LEFT, true));
        }
        if self.right && !want.1 {
            ev.push(key(KEY_RIGHT, false));
        }
        if !self.right && want.1 {
            ev.push(key(KEY_RIGHT, true));
        }
        if self.space && !want.2 {
            ev.push(key(KEY_SPACE, false));
        }
        if !self.space && want.2 {
            ev.push(key(KEY_SPACE, true));
        }
        self.left = want.0;
        self.right = want.1;
        self.space = want.2;
        ev
    }
}

/// Unsettled hazards in river order: `(mark, is_boat, index)`.
fn hazards_ahead(g: &River) -> Vec<(f32, bool, usize)> {
    let mut v = Vec::new();
    for i in 0..8 {
        let (p, _, settled, _) = g.rock(i);
        if !settled {
            v.push((p, false, i));
        }
    }
    for i in 0..4 {
        let (p, settled, _) = g.boat(i);
        if !settled {
            v.push((p, true, i));
        }
    }
    v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    v
}

fn next_hazard(g: &River) -> Option<(f32, bool, usize)> {
    hazards_ahead(g).first().copied()
}

/// One scripted frame: steer to the next lane, sprint far from hazards,
/// pole so the vault arc always covers the band.
fn script_frame(sim: &mut Sim, g: &mut River, h: &mut Held) {
    let s = g.scroll();
    let grounded = g.air() == 0 && g.stun() == 0;
    let ahead = hazards_ahead(g);
    let target = match ahead.first() {
        Some((p, true, i)) => {
            // ride the hull at the arrival tick; when the very next mark is
            // a rock right behind the boat, hug the hull on the rock's side
            let v = (CURRENT + g.vel()).max(1.0);
            let arr = g.ticks() + ((p - s).max(0.0) / v) as u32;
            let mut tx = g.boat_x(*i, arr);
            if let Some((rp, false, ri)) = ahead.get(1).copied() {
                if rp - p <= 130.0 {
                    let rx = g.rock(ri).1;
                    tx += (rx - tx).clamp(-14.0, 14.0);
                }
            }
            tx
        }
        Some((_, false, i)) => g.rock(*i).1,
        None => 160.0,
    };
    let want_l = g.canoe_x() - target > 1.0;
    let want_r = target - g.canoe_x() > 1.0;
    let want_space = if !grounded {
        false
    } else if let Some((p, _, _)) = next_hazard(g) {
        if h.space {
            s > p - 200.0 && s < p - 12.0 && g.charge() < 30
        } else {
            s > p - 45.0 || s < p - 200.0
        }
    } else if h.space {
        g.charge() < 30
    } else {
        true
    };
    let ev = h.diff((want_l, want_r, want_space));
    sim.frame(g, &ev);
}

fn run_scripted(seed: u64) -> (River, u32) {
    let mut g = River::new(seed);
    let mut sim = Sim::new();
    let mut h = Held::default();
    sim.idle(&mut g, PRESENT + 1);
    for _ in 0..TIME_LIMIT {
        if g.state() != MgState::Running {
            break;
        }
        script_frame(&mut sim, &mut g, &mut h);
    }
    for i in 0..8 {
        let (p, x, _, hit) = g.rock(i);
        eprintln!("rock{i} p={p} x={x} hit={hit}");
    }
    for i in 0..4 {
        let (p, _, sunk) = g.boat(i);
        eprintln!("boat{i} p={p} sunk={sunk}");
    }
    let ticks = g.ticks();
    (g, ticks)
}

#[test]
fn scripted_canoe_takes_gold() {
    let (g, ticks) = run_scripted(7);
    assert_eq!(g.wipes(), 0, "the clean script must not swim");
    assert!(
        ticks < TIME_LIMIT - 200,
        "reached the falls at tick {ticks}"
    );
    let vaults = (0..8).filter(|i| g.rock(*i).3).count();
    let sunk = (0..4).filter(|i| g.boat(*i).2).count();
    assert_eq!(vaults, 8, "every rock vaulted");
    assert_eq!(sunk, 4, "every pike boat ridden over");
    assert!(g.score() >= 92, "score {} is gold", g.score());
    assert_eq!(g.state(), MgState::Won(Tier::Gold));
}

#[test]
fn the_river_is_deterministic() {
    let (a, ta) = run_scripted(7);
    let (b, tb) = run_scripted(7);
    assert_eq!(a.score(), b.score());
    assert_eq!(ta, tb);
    assert_eq!(a.state(), b.state());
}

#[test]
fn touching_a_rock_from_the_water_is_a_wipeout() {
    let mut g = River::new(7);
    let mut sim = Sim::new();
    let mut h = Held::default();
    sim.idle(&mut g, PRESENT + 1);
    let lane = g.rock(0).1;
    // steer into the rock lane and never pole
    for _ in 0..TIME_LIMIT {
        if g.wipes() > 0 {
            break;
        }
        if g.rock(0).2 && g.wipes() == 0 && g.scroll() > g.rock(0).0 + 20.0 {
            panic!("slid past a rock dead ahead from the water — no wipeout");
        }
        let want_l = g.canoe_x() - lane > 1.0;
        let want_r = lane - g.canoe_x() > 1.0;
        let ev = h.diff((want_l, want_r, false));
        sim.frame(&mut g, &ev);
    }
    assert_eq!(g.wipes(), 1);
    assert!(g.stun() > 0, "the swimmer is stunned in the wash");
    assert!(
        g.scroll() < g.rock(0).0 - 10.0,
        "the river spat the canoe back downstream"
    );
}

#[test]
fn the_banks_hold_the_canoe() {
    let mut g = River::new(7);
    let mut sim = Sim::new();
    let mut h = Held::default();
    sim.idle(&mut g, PRESENT + 1);
    let ev = h.diff((true, false, false));
    sim.frame(&mut g, &ev);
    sim.idle(&mut g, 250);
    assert_eq!(g.canoe_x(), 46.0);
    let ev = h.diff((false, true, false));
    sim.frame(&mut g, &ev);
    sim.idle(&mut g, 350);
    assert_eq!(g.canoe_x(), 274.0);
}

#[test]
fn pike_boats_sweep_and_obey_the_seed() {
    let a = River::new(11);
    let b = River::new(11);
    // boat 0 (mark 550) has no rock within 130 — it owns the whole stream
    let (mid0, half0) = a.boat_sweep(0);
    assert!((mid0 - 170.0..=mid0 + 170.0).contains(&a.boat_x(0, 250)));
    assert!(half0 > 110.0);
    // the boats shadowed by a rock hug that rock's lane instead
    for i in 1..4 {
        let (mid, half) = a.boat_sweep(i);
        assert!(half <= 25.0, "boat {i} must huddle in the rock's wake");
        for t in [100, 400, 900, 1500] {
            let x = a.boat_x(i, t);
            assert!(
                (mid - half..=mid + half).contains(&x),
                "boat {i} off its row"
            );
        }
    }
    let x0 = a.boat_x(0, 100);
    assert!((x0 - a.boat_x(0, 400)).abs() > 1.0);
    assert_eq!(x0, b.boat_x(0, 100));
}

#[test]
fn drifting_still_ends_at_the_falls_and_scores_by_wipeouts() {
    let mut g = River::new(3);
    let mut sim = Sim::new();
    sim.idle(&mut g, PRESENT + 1);
    for _ in 0..TIME_LIMIT {
        if g.state() != MgState::Running {
            break;
        }
        sim.frame(&mut g, &[]);
    }
    assert!(
        g.scroll() >= 2600.0,
        "even the idle drift reaches the falls"
    );
    let want = (40 - 8 * g.wipes() as i32).clamp(0, 100) as u32;
    assert_eq!(g.score(), want);
    if want >= 40 {
        assert!(matches!(g.state(), MgState::Won(Tier::Bronze)));
    } else {
        assert_eq!(g.state(), MgState::Lost);
    }
}
