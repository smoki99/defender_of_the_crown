//! T603: the RESCUE dungeon — corridors clamp the runner, gaolers catch
//! and are routed by torch-fright, the baron follows his own rescue and
//! both slip out the postern. Pure logic driven through a synthetic Input;
//! a scripted runner proves a GOLD run is possible: it times its departures
//! by predicting every gaoler's exact future beat position, and rides the
//! torch-fright for the dash in to the baron.

use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::{Input, Raw, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_UP};
use defender_of_the_crown::minigames::framework::{MgState, Minigame, Tier};
use defender_of_the_crown::minigames::maze::Maze;

const PRESENT: u32 = 90;
const TIME_LIMIT: u32 = 3300;
const FRIGHT_TICKS: u32 = 260;

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
    fn frame(&mut self, g: &mut Maze, events: &[Raw]) {
        for e in events {
            self.inp.push(*e);
        }
        self.frame += 1;
        self.inp.begin_frame(self.frame);
        g.tick(1.0 / 60.0, &self.inp, &mut self.fb);
    }
    fn idle(&mut self, g: &mut Maze, ticks: u32) {
        for _ in 0..ticks {
            self.frame(g, &[]);
        }
    }
}

/// Walk the held-key set to exactly `want` (arrows), one frame per step.
fn walk(sim: &mut Sim, g: &mut Maze, want: &[u32], held: &mut Vec<u32>) -> bool {
    if *held == want {
        sim.frame(g, &[]);
        return false; // advanced one tick, nothing re-pressed
    }
    let mut ev = Vec::new();
    for k in held.iter() {
        if !want.contains(k) {
            ev.push(Raw::Key {
                code: *k,
                pressed: false,
            });
        }
    }
    for k in want.iter() {
        if !held.contains(k) {
            ev.push(Raw::Key {
                code: *k,
                pressed: true,
            });
        }
    }
    *held = want.to_vec();
    sim.frame(g, &ev);
    true
}

/// Straight-line keys toward `to`, snapping one axis at a time.
fn steer(from: (f32, f32), to: (f32, f32)) -> Vec<u32> {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let mut k = Vec::new();
    if dx.abs() > 1.0 {
        k.push(if dx < 0.0 { KEY_LEFT } else { KEY_RIGHT });
    } else if dy.abs() > 1.0 {
        k.push(if dy < 0.0 { KEY_UP } else { KEY_DOWN });
    }
    k
}

#[test]
fn stone_clamps_the_runner() {
    let mut sim = Sim::new();
    let mut g = Maze::new(7);
    sim.idle(&mut g, PRESENT);
    assert_eq!(g.state(), MgState::Running);
    assert_eq!(g.player(), (24.0, 216.0));

    // north up the postern column: the stone at row 8 stops the box at 148.8
    let mut held = Vec::new();
    for _ in 0..200 {
        walk(&mut sim, &mut g, &[KEY_UP], &mut held);
    }
    assert!((g.player().1 - 148.8).abs() < 0.01, "held against the stone");
    assert_eq!(g.player().0, 24.0, "no drift");

    // west into the outer wall: the box clamps at 20.8
    let mut held = Vec::new();
    for _ in 0..50 {
        walk(&mut sim, &mut g, &[KEY_LEFT], &mut held);
    }
    assert!((g.player().0 - 20.8).abs() < 0.01);
}

#[test]
fn a_torch_freezes_and_routes_the_watch() {
    let mut sim = Sim::new();
    let mut g = Maze::new(11);
    sim.idle(&mut g, PRESENT);

    // step onto the second torch: the spell lights
    g.debug_place(216.0, 184.0);
    sim.idle(&mut g, 2);
    assert!(g.fright() > FRIGHT_TICKS - 10, "torch lit the fright");
    assert_eq!(g.score(), 5, "torch loot");

    // frozen gaolers hold their beat while the spell burns
    let before = g.guard(2);
    sim.idle(&mut g, 30);
    assert_eq!(g.guard(2), before, "cowering guards do not jog");

    // press the cowering man: he quits the beat for good
    let (gx, gy, _) = before;
    g.debug_place(gx, gy + 3.0);
    sim.idle(&mut g, 2);
    assert!(g.guard(2).2, "guard routed");
    assert_eq!(g.caught(), 0, "a frightened guard never catches");
}

#[test]
fn a_catch_seizes_champion_and_baron() {
    let mut sim = Sim::new();
    let mut g = Maze::new(13);
    sim.idle(&mut g, PRESENT);

        // free the baron — the upper-middle street stays clear of his door
        sim.idle(&mut g, 30);
    g.debug_place(280.0, 24.0);
    sim.idle(&mut g, 2);
    assert!(g.freed() && g.score() == 20);

    // drop the champion onto the right-hand column beat and let him jog in
    let (gx, gy, _) = g.guard(1);
    g.debug_place(gx, gy);
    let mut caught = false;
    for _ in 0..400 {
        sim.idle(&mut g, 1);
        if g.caught() == 1 {
            caught = true;
            break;
        }
    }
    assert!(caught, "the street gaoler takes the champion");
    assert!(!g.freed(), "the baron is back in chains");
    assert_eq!(g.player(), (24.0, 216.0), "dumped back at the postern");

    // the grace window: standing on the beat again must not re-catch
    let (gx, gy, _) = g.guard(1);
    g.debug_place(gx, gy);
    sim.idle(&mut g, 60);
    assert_eq!(g.caught(), 1, "invulnerable for a beat");
}

#[test]
fn the_watch_change_ends_a_slow_run() {
    let mut sim = Sim::new();
    let mut g = Maze::new(3);
    sim.idle(&mut g, PRESENT + TIME_LIMIT + 5);
    assert_eq!(g.state(), MgState::Lost, "idle in the dark: no rescue");
}

// --- the scripted gold run ---------------------------------------------------

/// Cell centre in pixels.
fn c(cx: f32, cy: f32) -> (f32, f32) {
    (cx * 16.0 + 8.0, cy * 16.0 + 8.0)
}

/// The four gaoler beats in pixels — must mirror `GUARDS` in the module.
const BEATS: [((f32, f32), (f32, f32)); 4] = [
    ((120.0, 120.0), (248.0, 120.0)), // mid street
    ((216.0, 56.0), (248.0, 56.0)), // upper middle street
    ((88.0, 216.0), (264.0, 216.0)),  // bottom street
    ((56.0, 24.0), (248.0, 24.0)),    // top street
];

/// A gaoler's exact position `k` ticks on: ping-pong folds onto the beat.
fn guard_future(i: usize, pos: (f32, f32), v: (f32, f32), k: u32) -> (f32, f32) {
    let ((ax, ay), (bx, by)) = BEATS[i];
    let horiz = bx != ax;
    let len = (bx - ax).abs() + (by - ay).abs();
    let s0 = if horiz { pos.0 - ax } else { pos.1 - ay };
    let vs = if horiz { v.0 } else { v.1 };
    let period = 2.0 * len;
    let mut u = (s0 + vs * k as f32).rem_euclid(period);
    if u > len {
        u = period - u;
    }
    let t = if len == 0.0 { 0.0 } else { u / len };
    (ax + (bx - ax) * t, ay + (by - ay) * t)
}

fn on_polyline(pts: &[(f32, f32)], mut d: f32) -> (f32, f32) {
    for w in pts.windows(2) {
        let l = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        if d <= l {
            let t = if l == 0.0 { 0.0 } else { d / l };
            return (
                w[0].0 + (w[1].0 - w[0].0) * t,
                w[0].1 + (w[1].1 - w[0].1) * t,
            );
        }
        d -= l;
    }
    *pts.last().unwrap()
}

/// May the runner sprint `start -> way` at full speed?  The dash is legal
/// iff no catch (radius 7) can land on any tick: 11 px predicted clearance.
fn window_ok(g: &Maze, start: (f32, f32), way: &[(f32, f32)], vels: &[(f32, f32); 4]) -> bool {
    let mut pts = vec![start];
    pts.extend_from_slice(way);
    let total: f32 = pts
        .windows(2)
        .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
        .sum();
    let dur = (total / 1.6) as u32 + 2;
    for k in 0..=dur {
        let pp = on_polyline(&pts, 1.6 * k as f32);
        for (i, &vel) in vels.iter().enumerate() {
            let (gx, gy, routed) = g.guard(i);
            if routed {
                continue;
            }
            let gp = guard_future(i, (gx, gy), vel, k);
            if (pp.0 - gp.0).hypot(pp.1 - gp.1) < 11.0 {
                return false;
            }
        }
    }
    true
}

#[test]
fn scripted_rescue_rides_gold() {
    // run 1, timed sprint: postern -> y11 bridge street -> torch II
    // run 2, fright dash: up the right column, grabbing torch I en route,
    //                  past the gaolers' mouths to the baron, into the nook
    // run 3, timed sprint: column -> y11 bridge street -> postern
    let runs: Vec<(Vec<(f32, f32)>, bool)> = vec![
        (vec![c(1.0, 11.0), c(13.0, 11.0)], true),
        (
            vec![
                c(17.0, 11.0),
                c(17.0, 1.0),  // up the column — torch I sits on it at (17,7)
                c(17.0, 2.0),
                c(18.0, 2.0),  // the nook: clear of every beat
            ],
            false,
        ),
        (vec![c(17.0, 2.0), c(17.0, 11.0), c(1.0, 11.0), c(1.0, 13.0)], true),
    ];

    let mut sim = Sim::new();
    let mut g = Maze::new(5);
    sim.idle(&mut g, PRESENT);

    let mut prev: Vec<(f32, f32, bool)> = (0..4).map(|i| g.guard(i)).collect();
    let mut held = Vec::new();
    let mut run = 0usize;
    let mut wpi = 0usize;
    let mut departing = false;
    let mut finished = 0u32; // ticks idling at the postern while the baron catches up

    for _step in 0..TIME_LIMIT + PRESENT {
        let p = g.player();
        let mut vels = [(0.0f32, 0.0f32); 4];
        for i in 0..4 {
            let cur = g.guard(i);
            vels[i] = (cur.0 - prev[i].0, cur.1 - prev[i].1);
            prev[i] = cur;
        }

        let mut want: Vec<u32> = Vec::new();
        if run < runs.len() {
            let (way, predictive) = &runs[run];
            if !departing {
                let ready = if *predictive {
                    g.fright() == 0 && window_ok(&g, p, way, &vels)
                } else {
                    g.fright() > 0 // ride the torch spell in to the baron
                };
                if ready {
                    departing = true;
                    wpi = 0;
                }
            }
            if departing {
                want = steer(p, way[wpi]);
            }
        } else {
            finished += 1;
        }
        walk(&mut sim, &mut g, &want, &mut held);

        if departing {
            let p2 = g.player();
            let w = runs[run].0[wpi];
            if (p2.0 - w.0).hypot(p2.1 - w.1) < 2.0 {
                wpi += 1;
                if wpi >= runs[run].0.len() {
                    departing = false;
                    run += 1;
                    wpi = 0;
                }
            }
        }
        if _step % 200 == 0 {
            eprintln!("t{_step} run{run} wpi{wpi} dep{departing} fright{} score{} pos{:?}", g.fright(), g.score(), g.player());
        }
        match g.state() {
            MgState::Won(_) | MgState::Lost => break,
            _ => {}
        }
    }

    assert_eq!(g.caught(), 0, "no catches on the scripted route");
    assert!(g.freed(), "the baron was freed");
    match g.state() {
        MgState::Won(Tier::Gold) => {}
        s => panic!(
            "scripted rescue must ride Gold, got {s:?} (score {}, finished {finished})",
            g.score()
        ),
    }
    assert!(g.score() >= 90);
}
