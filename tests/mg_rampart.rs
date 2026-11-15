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

/// A flawless watch: stones on fat parties, lead-shots on every climber.
/// The siege is scripted, so "perfect" is just the greedy-optimal loop.
fn perfect_watch(g: &mut Rampart, sim: &mut Sim) {
    // bow-perfect: every man loosed at - the marching runners first (a
    // dead runner never plants a ladder), then every climber. The rocks
    // stay in the cart: the module's own note is that this run "lands
    // just above Gold" (24 runners + 72 climbers + 6 repels).
    let march = 2.4_f32; // MARCH_SPD mirrored
    sim.idle(g, PRESENT + 1);
    for _ in 0..END_TICKS + 400 {
        if !matches!(g.state(), MgState::Running) {
            break;
        }
        let mut acted = false;
        if g.quiver() > 0 {
            // the man nearest the battlements and his remaining climb
            let mut best: Option<(f32, usize, usize, f32)> = None; // y,w,c,left
            for w in 0..6 {
                for c in 0..3 {
                    if let Some((cx, cy)) = g.climber_xy(w, c) {
                        let lane = g.runner_xy(w, 0).map(|(_, y)| y).unwrap_or(cy + 60.0);
                        let rate = ((lane - TOP_Y) / CLIMB_TICKS).max(0.05);
                        let left = (cy - TOP_Y) / rate;
                        if best.is_none_or(|(by, _, _, _)| cy < by) {
                            best = Some((cy, w, c, left));
                        }
                        let _ = cx;
                    }
                }
            }
            // while no climber is inside an arrow's grace, spend the
            // beat on the marching runners - that is where gold hides
            if best.is_none_or(|(_, _, _, left)| left >= 40.0) {
                'march: for w in 0..6 {
                    // marching men, and the team waiting at a planted
                    // foot, are fair game while no climb is in peril
                    let st = g.ladder(w).0;
                    if st != 1 && st != 3 {
                        continue;
                    }
                    for r in 0..2 {
                        if let Some((rx, ry)) = g.runner_xy(w, r) {
                            let flight =
                                ((rx - BOW.0).powi(2) + (ry - BOW.1).powi(2)).sqrt() / ARROW_SPD;
                            // foot teams stand still; marching men don't
                            let lead = if st == 1 { march * flight } else { 0.0 };
                            sim.loose(g, (rx - lead) as i32, ry as i32);
                            acted = true;
                            break 'march;
                        }
                    }
                }
            }
            // else lead the most urgent climber up his run
            if !acted {
                if let Some((_, w, c, _)) = best {
                    let (cx, cy) = g.climber_xy(w, c).unwrap();
                    let lane = g.runner_xy(w, 0).map(|(_, y)| y).unwrap_or(cy + 60.0);
                    let rate = (lane - TOP_Y) / CLIMB_TICKS;
                    let flight =
                        ((cx - BOW.0).powi(2) + (cy - BOW.1).powi(2)).sqrt() / ARROW_SPD;
                    sim.loose(g, cx as i32, (cy - flight * rate) as i32);
                    acted = true;
                }
            }
        }
        if !acted {
            sim.idle(g, 1);
        }
    }
}

#[test]
fn a_flawless_watch_earneth_gold() {
    use defender_of_the_crown::campaign::game::Tier;
    let mut g = Rampart::new(7);
    let mut sim = Sim::new();
    perfect_watch(&mut g, &mut sim);
    assert!(!matches!(g.state(), MgState::Lost), "flawless watch lost ({} breaches)", g.breaches());
    assert!(g.score() >= 90, "flawless watch scored {} < gold line", g.score());
    assert!(
        matches!(g.state(), MgState::Won(Tier::Gold)),
        "flawless watch did not resolve Gold: {:?}",
        g.state()
    );
}

#[test]
fn a_slack_watch_bringeth_bronze_not_worse() {
    // Casual read of the siege: the eye lands on the works every 16
    // ticks, the shaft flies at where a man was last seen (he has
    // since moved up), over a third of looses are bobbled, and the
    // rocks stay in the cart. The plan's Q6 bar: casual lands in
    // [BRONZE, SILVER) - it must still win, never at gold (that is
    // the flawless watch's claim), never in the sack.
    use defender_of_the_crown::campaign::game::Tier;
    let mut g = Rampart::new(7);
    let mut sim = Sim::new();
    sim.idle(&mut g, PRESENT + 1);
    let mut ghost: Option<(f32, f32)> = None;
    let mut t = 0u32;
    let mut rng = 0x5DEECE66Du64; // the watch's fidgety finger
    for _ in 0..END_TICKS + 400 {
        if !matches!(g.state(), MgState::Running) {
            break;
        }
        t += 1;
        if t.is_multiple_of(16) {
            // the eye settles on the lowest man - the easy target
            ghost = (0..6)
                .filter_map(|w| {
                    (0..3)
                        .filter_map(|c| g.climber_xy(w, c))
                        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                })
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .or(ghost);
        }
        let mut acted = false;
        if t % 16 == 15 && g.quiver() > 0 {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let flinch = rng >> 61 & 7 < 3; // over a third are bobbled
            if let Some((x, y)) = ghost {
                if !flinch {
                    sim.loose(&mut g, x as i32, y as i32); // at the memory
                }
                acted = true;
            }
        }
        if !acted {
            sim.idle(&mut g, 1);
        }
    }
    assert!(
        matches!(g.state(), MgState::Won(Tier::Bronze)),
        "slack watch did not place bronze: {:?} ({})",
        g.state(),
        g.score()
    );
}
