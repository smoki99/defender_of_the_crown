//! T605: the STEALTH keep. Gaolers walk fixed beats with sight cones, bells
//! escalate the alarm, the lever draws the portcullis onto the hidden nook,
//! and three sightings put the thief in irons. Pure logic driven through a
//! synthetic Input; scripts prove a clean 100 run and the time-up loss.

use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::{Input, Raw};
use defender_of_the_crown::minigames::cloak::Cloak;
use defender_of_the_crown::minigames::framework::{MgState, Minigame, Tier};

const PRESENT: u32 = 90;
const TIME_LIMIT: u32 = 3600;
const CATCH_FREEZE: u32 = 90;
const SPEEDUP: u32 = 4;

/// Mirror of the private [`KEEP`] for geometry assertions in test 1.
const KEEP: [&str; 16] = [
    "####################",
    "#.#...............##",
    "#.#######...#####..#",
    "#.......#.#.#...#.##",
    "#######.#.##..#...##",
    "#.......#.....#...##",
    "#..###.##.######...#",
    "#.....#.........#..#",
    "####..####...#..#.##",
    "#...#.....#.....#.##",
    "#.##.####..#####...#",
    "#..................#",
    "#.###.###########..#",
    "#...#.............##",
    "#######...##..#.####",
    "####################",
];
const GATE: (i32, i32) = (1, 13);
const TORCHES: [(i32, i32); 5] = [(5, 7), (13, 3), (1, 1), (17, 7), (18, 2)];
const BELLS: [(i32, i32); 3] = [(6, 13), (11, 11), (16, 11)];
const LEVER: (i32, i32) = (14, 9);
const PORTCULLIS: (i32, i32) = (17, 2);

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
    fn frame(&mut self, g: &mut Cloak, events: &[Raw]) {
        for e in events {
            self.inp.push(*e);
        }
        self.frame += 1;
        self.inp.begin_frame(self.frame);
        g.tick(1.0 / 60.0, &self.inp, &mut self.fb);
    }
    fn idle(&mut self, g: &mut Cloak, ticks: u32) {
        for _ in 0..ticks {
            self.frame(g, &[]);
        }
    }
    /// One fresh key edge (press next frame, release after).
    fn tap(&mut self, g: &mut Cloak, code: u32) {
        self.frame(
            g,
            &[Raw::Key {
                code,
                pressed: true,
            }],
        );
        self.frame(
            g,
            &[Raw::Key {
                code,
                pressed: false,
            }],
        );
    }
}

fn open_run(seed: u64) -> (Sim, Cloak) {
    let mut sim = Sim::new();
    let mut g = Cloak::new(seed);
    sim.idle(&mut g, PRESENT + 1); // burn the banner; Run active
    (sim, g)
}

fn floor(x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < 20 && y < 16 && KEEP[y as usize].as_bytes()[x as usize] != b'#'
}

/// BFS over the keep honouring the portcullis state.
fn reachable(gate_open: bool) -> Vec<(i32, i32)> {
    let mut seen = vec![GATE];
    let mut i = 0;
    while i < seen.len() {
        let (x, y) = seen[i];
        i += 1;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if !floor(nx, ny) || (!gate_open && (nx, ny) == PORTCULLIS) {
                continue;
            }
            if !seen.contains(&(nx, ny)) {
                seen.push((nx, ny));
            }
        }
    }
    seen
}

#[test]
fn keep_geometry_loops_and_nook() {
    let shut = reachable(false);
    for t in TORCHES.iter().take(4) {
        assert!(shut.contains(t), "torch {:?} unreachable", t);
    }
    for b in BELLS.iter() {
        assert!(shut.contains(b), "bell {:?} unreachable", b);
    }
    assert!(shut.contains(&LEVER));
    assert!(!shut.contains(&TORCHES[4]), "nook must be sealed while shut");
    let open = reachable(true);
    assert!(open.contains(&TORCHES[4]), "lever must open the nook");
}

#[test]
fn run_is_deterministic() {
    let (mut a, mut g1) = open_run(0xC0FFEE);
    let (mut b, mut g2) = open_run(0xC0FFEE);
    let (mut c, mut g3) = open_run(0xBADF00D);
    a.idle(&mut g1, 300);
    b.idle(&mut g2, 300);
    c.idle(&mut g3, 300);
    for i in 0..5 {
        assert_eq!(g1.guard(i), g2.guard(i), "beat {} drifted", i);
    }
    let same = (0..5).all(|i| g1.guard(i) == g3.guard(i));
    assert!(!same, "seed must vary the watch");
}

#[test]
fn bell_ringing_speeds_the_watch() {
    let (mut sim, mut g) = open_run(7);
    let before = g.period(0);
    sim.idle(&mut g, 1);
    g.debug_place(BELLS[0].0, BELLS[0].1);
    sim.idle(&mut g, 1);
    assert_eq!(g.bells(), 1);
    assert_eq!(g.period(0), before - SPEEDUP);
    g.debug_place(BELLS[1].0, BELLS[1].1);
    sim.idle(&mut g, 1);
    g.debug_place(BELLS[2].0, BELLS[2].1);
    sim.idle(&mut g, 1);
    assert_eq!(g.bells(), 3);
    assert!(g.period(0) >= 12, "escalation must floor at MIN_PERIOD");
}

#[test]
fn portcullis_gates_the_nook_walk() {
    let (mut sim, mut g) = open_run(11);
    g.debug_place(16, 2);
    sim.idle(&mut g, 1);
    sim.tap(&mut g, b'd' as u32); // walk at the shut gate
    sim.idle(&mut g, 1);
    assert_eq!(g.thief_cell(), (16, 2), "shut gate must block");
    g.debug_open_gate();
    sim.tap(&mut g, b'd' as u32);
    sim.idle(&mut g, 1);
    assert_eq!(g.thief_cell(), (17, 2));
    sim.tap(&mut g, b'd' as u32); // into the nook, fifth torch
    sim.idle(&mut g, 1);
    assert_eq!(g.thief_cell(), (18, 2));
    assert_eq!(g.torches_out(), 1);
}

#[test]
fn three_sightings_shackle() {
    let (mut sim, mut g) = open_run(3);
    for n in 1..=3 {
        let (gx, gy, _) = g.guard(0);
        g.debug_place(gx, gy); // stand on a gaoler: touched
        sim.idle(&mut g, 1);
        assert_eq!(g.seen(), n, "standing on a beat must be seen");
        assert_eq!(g.thief_cell(), GATE, "a sighting re-sents the thief");
        assert_eq!(g.cloaks(), 3 - n);
        if n < 3 {
            assert_eq!(g.state(), MgState::Running);
            sim.idle(&mut g, CATCH_FREEZE + 5); // eyes blink, then re-arm
        }
    }
    assert_eq!(g.state(), MgState::Lost);
    assert_eq!(g.score(), 0);
}

#[test]
fn clean_sweep_scores_hundred() {
    let (mut sim, mut g) = open_run(42);
    for t in [TORCHES[2], TORCHES[0], TORCHES[1], TORCHES[3]] {
        wait_safe(&mut sim, &mut g, t);
    }
    wait_safe(&mut sim, &mut g, LEVER);
    assert!(g.gate_open());
    assert_eq!(g.torches_out(), 4);
    wait_safe(&mut sim, &mut g, TORCHES[4]);
    assert_eq!(g.torches_out(), 5);
    wait_safe(&mut sim, &mut g, GATE); // in at the postern, all dark
    assert_eq!(g.bells(), 0);
    assert_eq!(g.score(), 100, "5x15 + escape 10 + clean 15");
    assert!(matches!(g.state(), MgState::Won(Tier::Gold)), "clean sweep must be GOLD");
}

#[test]
fn escape_needs_every_torch() {
    let (mut sim, mut g) = open_run(9);
    g.debug_place(TORCHES[0].0, TORCHES[0].1);
    sim.idle(&mut g, 1);
    g.debug_place(GATE.0, GATE.1);
    sim.idle(&mut g, 1);
    assert_eq!(g.state(), MgState::Running, "one torch is not a win");
    assert_eq!(g.score(), 15, "only the torch paid");
}

#[test]
fn time_up_ends_the_night() {
    let (mut sim, mut g) = open_run(5);
    sim.idle(&mut g, TIME_LIMIT + 2);
    assert_eq!(g.state(), MgState::Lost, "the relief finds an empty keep");
    assert_eq!(g.score(), 0);
}

#[test]
fn wasd_moves_the_thief() {
    let (mut sim, mut g) = open_run(1);
    g.debug_place(8, 13);
    sim.idle(&mut g, 1);
    sim.tap(&mut g, b's' as u32);
    assert_eq!(g.thief_cell(), (8, 14), "S steps down");
    sim.tap(&mut g, b'D' as u32);
    assert_eq!(g.thief_cell(), (9, 14), "shifted D steps right");
    assert_eq!(g.seen(), 0);
}

#[test]
fn predicted_sight_tracks_the_beats() {
    let (mut sim, mut g) = open_run(0xC0FFEE);
    // A cell just outside every cone right now: predicted vision 0..k must
    // equal what actually eyes the cell at tick k — the bot-safety contract.
    let probe = (8, 13);
    g.debug_place(probe.0, probe.1);
    sim.idle(&mut g, 1);
    assert_eq!(g.seen(), 0);
    for ahead in [1u32, 5, 40, 160] {
        let pred = g.predicted_sight(ahead);
        for _ in 0..ahead {
            sim.idle(&mut g, 1);
        }
        let now = g.predicted_sight(0);
        // prediction from the earlier state must equal reality after the
        // same number of ticks — the bot-safety contract
        let (mut p, mut n) = (pred, now);
        p.sort();
        n.sort();
        assert_eq!(p, n, "predicted vision diverged at {} ticks", ahead);
        assert!(!p.is_empty(), "a keep with five gaolers does see cells");
        assert_eq!(g.seen(), 0, "probe cell sat outside every real cone");
        assert_eq!(g.thief_cell(), probe);
    }
}

/// A thief waits for the predicted eyes to clear before stepping in.
fn wait_safe(sim: &mut Sim, g: &mut Cloak, cell: (i32, i32)) {
    for _ in 0..400 {
        if !g.predicted_sight(1).contains(&cell) {
            break;
        }
        sim.idle(g, 1);
    }
    assert!(
        !g.predicted_sight(1).contains(&cell),
        "watch never cleared {:?}",
        cell
    );
    g.debug_place(cell.0, cell.1);
    sim.idle(g, 1);
    assert_eq!(g.seen(), 0, "predicted-safe step into {:?} was seen", cell);
}
