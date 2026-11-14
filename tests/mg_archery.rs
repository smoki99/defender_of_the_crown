//! T601: mg-archery core - pop windows, scoring tiers, fool penalty,
//! finite shafts. Drives the pure logic through a synthetic Input so the
//! web host stays a thin adapter.

use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::{
    Input, Raw, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_SPACE, KEY_UP,
};
use defender_of_the_crown::minigames::archery::Archery;
use defender_of_the_crown::minigames::framework::{MgState, Minigame, Tier};

struct Sim {
    inp: Input,
    fb: Framebuffer,
    frame: u64,
}

fn key(code: u32, pressed: bool) -> Raw {
    Raw::Key { code, pressed }
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
    fn frame(&mut self, g: &mut Archery, events: &[Raw]) {
        for e in events {
            self.inp.push(*e);
        }
        self.frame += 1;
        self.inp.begin_frame(self.frame);
        g.tick(1.0 / 60.0, &self.inp, &mut self.fb);
    }
    fn idle(&mut self, g: &mut Archery, ticks: u32) {
        for _ in 0..ticks {
            self.frame(g, &[]);
        }
    }
    fn tap_space(&mut self, g: &mut Archery) {
        self.frame(g, &[key(KEY_SPACE, true), key(KEY_SPACE, false)]);
    }
    /// Steer the sight to (x, y): holds arrow keys with proper press
    /// bookkeeping (release any axis that leaves the want-set).
    fn steer(&mut self, g: &mut Archery, x: f32, y: f32) -> bool {
        let keys_at = |g: &Archery| {
            let (cx, cy) = g.cross();
            let mut k = Vec::new();
            // 9 px steps: a narrow deadband limit-cycles; keep > step/2
            if cx - x > 4.8 {
                k.push(KEY_LEFT);
            } else if x - cx > 4.8 {
                k.push(KEY_RIGHT);
            }
            if cy - y > 4.8 {
                k.push(KEY_UP);
            } else if y - cy > 4.8 {
                k.push(KEY_DOWN);
            }
            k
        };
        let mut held: Vec<u32> = Vec::new();
        let mut ok = false;
        for _ in 0..200 {
            let k = keys_at(g);
            if k.is_empty() {
                ok = true;
                break;
            }
            let mut ev: Vec<Raw> = Vec::new();
            for c in held.iter().filter(|c| !k.contains(c)) {
                ev.push(key(*c, false));
            }
            for c in k.iter().filter(|c| !held.contains(c)) {
                ev.push(key(*c, true));
            }
            self.frame(g, &ev);
            held = k;
        }
        if !held.is_empty() {
            let ev: Vec<Raw> = held.iter().map(|&c| key(c, false)).collect();
            self.frame(g, &ev);
        }
        ok
    }
    /// Steer onto the hole and loose while its target is up.
    fn snipe(&mut self, g: &mut Archery, hole: usize) -> bool {
        let (hx, hy) = Archery::hole_xy(hole);
        for _ in 0..60 {
            let live = g
                .targets()
                .into_iter()
                .any(|(h, vis)| h == hole && vis >= 0.7);
            if !live {
                return false; // ducked away before we could loose
            }
            if !self.steer(g, hx, hy) {
                return false;
            }
            self.tap_space(g);
            let vis = g
                .targets()
                .into_iter()
                .find(|(h, _)| *h == hole)
                .map(|(_, v)| v)
                .unwrap_or(0.0);
            if vis == 0.0 {
                return true; // collected: struck targets drop at once
            }
        }
        false
    }
}

#[test]
fn a_sharp_shooter_takes_the_gold() {
    let mut g = Archery::new(7);
    let mut s = Sim::new();
    s.idle(&mut g, 95); // Present banner, then Run opens
    assert_eq!(g.state(), MgState::Running);

    // pre-aim each volley's scheduled hole before it pops; loose on all pops
    while g.state() == MgState::Running && g.volley() < 7 {
        let live: Vec<usize> = g
            .targets()
            .into_iter()
            .filter(|(_, vis)| *vis >= 0.7)
            .map(|(h, _)| h)
            .collect();
        if live.is_empty() {
            // aim at the next hole still to pop: remaining slots of this
            // volley (slot s pops at local 6 + 12s), else the next volley
            let v = g.volley();
            let loc = g.tick() % 48;
            let mut aim = None;
            for slt in 0..3 {
                if let Some(h) = g.scheduled(v).into_iter().nth(slt).flatten() {
                    if 6 + 12 * slt as u32 > loc {
                        aim = Some(h);
                        break;
                    }
                }
            }
            if aim.is_none() {
                aim = g.scheduled(v + 1).into_iter().flatten().next();
            }
            if let Some(h) = aim {
                let (hx, hy) = Archery::hole_xy(h);
                let _ = s.steer(&mut g, hx, hy);
            }
            s.idle(&mut g, 1);
            continue;
        }
        for h in live {
            s.snipe(&mut g, h);
        }
    }
    // the apple: chase it while it swings (still volleys >= 3)
    let mut shots = 0;
    while g.apple().is_some() && g.arrows() > 0 && shots < 60 {
        if let Some((ax, ay)) = g.apple() {
            let (cx, cy) = g.cross();
            if (cx - ax).abs() + (cy - ay).abs() > 6.0 && !s.steer(&mut g, ax, ay) {
                break;
            }
            if let Some((ax, ay)) = g.apple() {
                let (cx, cy) = g.cross();
                if (cx - ax).abs() + (cy - ay).abs() <= 6.0 {
                    s.tap_space(&mut g);
                }
            }
        }
        shots += 1;
    }

    // let the field run out and the tableau resolve
    for _ in 0..900 {
        if g.state() != MgState::Running {
            break;
        }
        s.idle(&mut g, 1);
    }
    assert_eq!(g.state(), MgState::Won(Tier::Gold), "score {}", g.score());
    assert!(g.score() >= 95);
}

#[test]
fn a_shot_fool_draws_boos_not_glory() {
    let mut g = Archery::new(11);
    let mut s = Sim::new();
    s.idle(&mut g, 95);
    let before = g.tally();
    // plant the sight on the fool's path and loose when he runs under it
    let fx = g.fool_x().clamp(60.0, 260.0);
    assert!(s.steer(&mut g, fx, 196.0));
    for _ in 0..300 {
        let (cx, _) = g.cross();
        if (cx - g.fool_x()).abs() <= 4.0 {
            s.tap_space(&mut g);
            break;
        }
        s.idle(&mut g, 1);
    }
    assert_eq!(g.tally(), before - 15, "the crowd remembers fool shots");
    assert_eq!(g.hits(), 0);
}

#[test]
fn an_empty_galley_reward_is_zero() {
    let mut g = Archery::new(3);
    let mut s = Sim::new();
    for _ in 0..1500 {
        if g.state() != MgState::Running {
            break;
        }
        s.idle(&mut g, 1);
    }
    assert_eq!(g.score(), 0);
    assert_eq!(g.state(), MgState::Lost, "no shafts, no wreath");
}

#[test]
fn the_fourteenth_shaft_is_the_last() {
    let mut g = Archery::new(5);
    let mut s = Sim::new();
    s.idle(&mut g, 95);
    for _ in 0..14 {
        s.tap_space(&mut g);
        s.idle(&mut g, 3);
    }
    assert_eq!(g.arrows(), 0);
    s.tap_space(&mut g); // the fifteenth is air
    assert_eq!(g.arrows(), 0);
}

#[test]
fn factory_arms_the_gallery() {
    use defender_of_the_crown::ui::host::make_game;
    let g = make_game("archery", 1).expect("gallery shipped");
    assert_eq!(g.id(), "archery");
}
