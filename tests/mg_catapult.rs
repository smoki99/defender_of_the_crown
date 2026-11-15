//! T503: mg-catapult — ballistics parity, judge bands, ship lead, and the
//! SPEC 6 #2 tier scaling. Driven through the same Input and Framebuffer
//! the web host feeds it, exactly like the joust suite.

use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::{Input, Raw, KEY_LEFT, KEY_RIGHT, KEY_SPACE};
use defender_of_the_crown::minigames::catapult::{predict, Catapult, Landing, CHARGE_STEP, TOWERS};
use defender_of_the_crown::minigames::framework::{MgState, Minigame, Tier};

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
    fn frame(&mut self, g: &mut Catapult, events: &[Raw]) {
        for e in events {
            self.inp.push(*e);
        }
        self.frame += 1;
        self.inp.begin_frame(self.frame);
        g.tick(1.0 / 60.0, &self.inp, &mut self.fb);
    }
    /// Tap the aim keys until the arm reads `target` (a multiple of the
    /// 0.75 px/frame detent away from the default 45 degrees).
    fn aim_angle(&mut self, g: &mut Catapult, target: f32) {
        for _ in 0..120 {
            if (g.angle() - target).abs() < 0.01 {
                return;
            }
            let k = if g.angle() > target {
                KEY_LEFT
            } else {
                KEY_RIGHT
            };
            self.frame(
                g,
                &[Raw::Key {
                    code: k,
                    pressed: true,
                }],
            );
            self.frame(
                g,
                &[Raw::Key {
                    code: k,
                    pressed: false,
                }],
            );
        }
        panic!("angle {} unreachable", target);
    }
    /// Hold SPACE for `frames` frames and loose. The meter adds
    /// CHARGE_STEP on every frame from press through release, starting at
    /// zero, so the power is exactly CHARGE_STEP * frames below 100.
    fn fire(&mut self, g: &mut Catapult, frames: u32) {
        assert!(g.aiming(), "can only loose while aiming");
        let down = Raw::Key {
            code: KEY_SPACE,
            pressed: true,
        };
        let up = Raw::Key {
            code: KEY_SPACE,
            pressed: false,
        };
        if frames <= 1 {
            self.frame(g, &[down, up]);
        } else {
            self.frame(g, &[down]);
            for _ in 1..frames - 1 {
                self.frame(g, &[]);
            }
            self.frame(g, &[up]);
        }
    }
    /// Run frames until the stone resolves and the next stone opens.
    fn wait_next(&mut self, g: &mut Catapult, shot: usize) -> Landing {
        for _ in 0..900 {
            self.frame(g, &[]);
            if g.shot_no() > shot {
                return g.landed();
            }
        }
        panic!("stone {} never resolved", shot);
    }
    /// Wait out the intro banner.
    fn wait_for_aim(&mut self, g: &mut Catapult) {
        for _ in 0..90 {
            self.frame(g, &[]);
        }
        assert!(g.aiming());
    }
}

enum Want {
    Tower(f32),
    Gap(f32),
    Ship,
}

/// Aim detents the solver may dial with one key tap per 0.75 degrees.
const ANGLES: [f32; 41] = [
    30.0, 30.75, 31.5, 32.25, 33.0, 33.75, 34.5, 35.25, 36.0, 36.75, 37.5, 38.25, 39.0, 39.75,
    40.5, 41.25, 42.0, 42.75, 43.5, 44.25, 45.0, 45.75, 46.5, 47.25, 48.0, 48.75, 49.5, 50.25,
    51.0, 51.75, 52.5, 53.25, 54.0, 54.75, 55.5, 56.25, 57.0, 57.75, 58.5, 59.25, 60.0,
];

/// The best (angle, charge frames) for the current stone, scored with the
/// game's own public `predict` — the same solver the browser harness
/// reimplements. `Want::Ship` keeps 45 degrees and leads the deck via
/// `ship_x_in(flight_ticks)`.
fn best_charge(g: &Catapult, want: Want) -> Option<(f32, u32, f32)> {
    let mut best: Option<(f32, u32, f32)> = None; // (angle, frames, error)
    let ship = matches!(want, Want::Ship);
    for &a in if ship { &[45.0f32][..] } else { &ANGLES[..] } {
        for n in 1..=63u32 {
            let p = (CHARGE_STEP * n as f32).min(100.0);
            let (x, ticks, kind) = predict(a, p, g.wind(), ship, g.down());
            let (err, reach) = match want {
                Want::Tower(tx) => (
                    (x - tx).abs(),
                    kind == Landing::Tower || kind == Landing::Direct,
                ),
                Want::Gap(wx) => ((x - wx).abs(), kind == Landing::Wall),
                // The deck keeps sailing while the meter fills: the ship
                // position at splash is ship_x_in(charge frames + flight).
                Want::Ship => ((x - g.ship_x_in(n + ticks)).abs(), true),
            };
            if reach && err < best.map_or(f32::MAX, |(_, _, e)| e) {
                best = Some((a, n, err));
            }
        }
    }
    best
}

#[test]
fn predict_and_live_flight_agree() {
    // A stone dropped into open ground must stop at the very x the public
    // ballistics promise — the harness solves with predict, so this pins
    // that shared contract (1/60 Euler predict vs 1/240 live substeps).
    for seed in [1u64, 12, 77] {
        let mut g = Catapult::new(seed);
        let mut s = Sim::new();
        s.wait_for_aim(&mut g);
        let charge = 12u32; // power 19.2: always falls short of the wall
        let (px, _ticks, kind) =
            predict(45.0, CHARGE_STEP * charge as f32, g.wind(), false, g.down());
        assert_eq!(
            kind,
            Landing::Ground,
            "seed {}: charge should fall short",
            seed
        );
        let shot = g.shot_no();
        s.fire(&mut g, charge);
        assert_eq!(s.wait_next(&mut g, shot), Landing::Ground);
        let dx = (g.last_x() - px).abs();
        assert!(
            dx <= 3.0,
            "seed {}: live stop {:.1} vs predicted {:.1}",
            seed,
            g.last_x(),
            px
        );
    }
}

#[test]
fn three_towers_fall_to_a_computed_battery() {
    // predict-solved angle+power against each tower: every stone at least
    // a Tower hit; the wall remembers nothing it should.
    let mut g = Catapult::new(0xC0FFEE);
    let mut s = Sim::new();
    s.wait_for_aim(&mut g);
    for (i, tx) in TOWERS.iter().enumerate() {
        let (a, n, err) = best_charge(&g, Want::Tower(*tx as f32)).expect("tower reachable");
        assert!(
            err <= 2.0,
            "tower {} solved to {:.1}px (angle {a})",
            tx,
            err
        );
        let shot = g.shot_no();
        assert_eq!(shot, i);
        s.aim_angle(&mut g, a);
        s.fire(&mut g, n);
        let landed = s.wait_next(&mut g, shot);
        assert!(
            landed == Landing::Direct || landed == Landing::Tower,
            "stone {}: {:?}",
            i,
            landed
        );
    }
    assert_eq!(g.down(), [true, true, true]);
}

#[test]
fn thud_on_the_wall_and_a_mile_short_pay_nothing() {
    let mut g = Catapult::new(4);
    let mut s = Sim::new();
    s.wait_for_aim(&mut g);
    s.fire(&mut g, 10); // feeble heave: falls short in the field
    assert_eq!(s.wait_next(&mut g, 0), Landing::Ground);
    // aim the bare wall face between towers 206 and 250 (gap center ~228)
    let (a, n, err) = best_charge(&g, Want::Gap(228.0)).expect("wall reachable");
    assert!(err <= 4.0, "solver can pick the gap: {err:.1}");
    s.aim_angle(&mut g, a);
    s.fire(&mut g, n);
    let landed = s.wait_next(&mut g, 1);
    assert_eq!(landed, Landing::Wall, "a stone into bare wall is a THUD");
    assert_eq!(g.score(), 0, "misses pay nothing");
}

#[test]
fn the_supply_ship_can_be_led_and_holed() {
    // Spend four stones fast, then hole the sailing ship on both shots.
    let mut g = Catapult::new(9);
    let mut s = Sim::new();
    s.wait_for_aim(&mut g);
    for shot in 0..4 {
        s.fire(&mut g, 8); // feeble heaves resolve quickly
        s.wait_next(&mut g, shot);
    }
    assert!(g.ship_phase());
    s.aim_angle(&mut g, 45.0);
    let mut holes = 0;
    for shot in 4..6 {
        let (_a, n, err) = best_charge(&g, Want::Ship).expect("ship solvable");
        assert!(err < 10.0, "lead solver found a window (err {err:.1})");
        s.fire(&mut g, n);
        let landed = s.wait_next(&mut g, shot);
        if landed == Landing::Ship || landed == Landing::Broadside {
            holes += 1;
        }
    }
    assert_eq!(holes, 2, "both led stones must hole the ship");
}

#[test]
fn full_battery_and_ship_hits_sail_into_gold() {
    // 3 tower hits (>=45, direct 66) + 2 ship hits (>=40) -> past Gold (92).
    let mut g = Catapult::new(0xC0FFEE);
    let mut s = Sim::new();
    s.wait_for_aim(&mut g);
    for _i in 0..3 {
        let down = g.down();
        let tx = TOWERS
            .iter()
            .enumerate()
            .find(|(i, _)| !down[*i])
            .map(|(_, t)| *t as f32)
            .expect("a tower still stands");
        let (a, n, _) = best_charge(&g, Want::Tower(tx)).expect("reachable");
        let shot = g.shot_no();
        s.aim_angle(&mut g, a);
        s.fire(&mut g, n);
        s.wait_next(&mut g, shot);
    }
    // 4th stone: nothing left standing — a throwaway heave.
    s.fire(&mut g, 30);
    s.wait_next(&mut g, 3);
    s.aim_angle(&mut g, 45.0);
    for shot in 4..6 {
        let (_a, n, _) = best_charge(&g, Want::Ship).expect("ship solvable");
        s.fire(&mut g, n);
        let landed = s.wait_next(&mut g, shot);
        assert!(
            landed == Landing::Ship || landed == Landing::Broadside,
            "shot {}",
            shot
        );
    }
    assert_eq!(g.state(), MgState::Won(Tier::Gold), "score {}", g.score());
}

#[test]
fn factory_arms_catapult_too() {
    use defender_of_the_crown::ui::host::make_game;
    assert!(make_game("catapult", 1).is_some(), "catapult shipped");
    assert!(make_game("dragon", 1).is_some(), "dragon shipped");
    assert!(
        make_game("dragon", 7).is_some(),
        "dragon seedeth by sport no"
    );
    assert!(make_game("wyvern", 1).is_none(), "unshipped ides stay void");
}
