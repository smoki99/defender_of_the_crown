//! T606: the RHYTHM smithy. Three tempo groups (40/28/18), space judged
//! against the sweep line: ±3 perfect, ±6 good, early swings waste the
//! beat, and an unstruck beat walks away as a miss. All perfects plus the
//! clean-chain bounty is the 95 gold line. Pure logic via synthetic Input.

use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::{Input, Raw, KEY_SPACE};
use defender_of_the_crown::minigames::forge::Forge;
use defender_of_the_crown::minigames::framework::{MgState, Minigame, Tier};

const PRESENT: u32 = 90;
/// (interval, beats) — mirror of the private TEMPOS table.
const TEMPOS: [(u32, u32); 3] = [(40, 6), (28, 6), (18, 6)];

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
    fn frame(&mut self, g: &mut Forge, events: &[Raw]) {
        for e in events {
            self.inp.push(*e);
        }
        self.frame += 1;
        self.inp.begin_frame(self.frame);
        g.tick(1.0 / 60.0, &self.inp, &mut self.fb);
    }
    fn idle(&mut self, g: &mut Forge, ticks: u32) {
        for _ in 0..ticks {
            self.frame(g, &[]);
        }
    }
    /// One fresh key edge (press frame judges, release frame follows).
    fn tap(&mut self, g: &mut Forge, code: u32) {
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

fn open_run(seed: u64) -> (Sim, Forge) {
    let mut sim = Sim::new();
    let mut g = Forge::new(seed);
    sim.idle(&mut g, PRESENT + 1); // burn the banner; Run active, t_local == 1
    (sim, g)
}

/// Hammer the armed beat at `t_local == t_at` (afterwards t_local == 1
/// again on the next beat, so scripts chain without bookkeeping).
fn strike(sim: &mut Sim, g: &mut Forge, t_at: u32) {
    sim.idle(g, t_at - 1);
    sim.tap(g, KEY_SPACE);
}

/// Play the whole 18-beat run; `mode` picks the per-beat offset from the
/// line: 0 = perfect, ±5 = good, -100 = wasted swing (clamps to t=1).
fn play_all(sim: &mut Sim, g: &mut Forge, mode: i32) {
    for &(iv, n) in TEMPOS.iter() {
        for _ in 0..n {
            let t = (iv as i32 + mode).max(1) as u32;
            strike(sim, g, t);
        }
    }
    sim.idle(g, 60); // last flash settles -> verdict
}

#[test]
fn banner_then_run() {
    let (_sim, g) = open_run(1);
    assert_eq!(g.state(), MgState::Running);
    assert_eq!(g.score(), 0);
    assert_eq!(g.beat_no(), 0);
    assert_eq!(g.current_interval(), 40);
}

#[test]
fn perfect_chain_forges_gold() {
    let (mut sim, mut g) = open_run(7);
    play_all(&mut sim, &mut g, 0); // every blow dead centre
    assert_eq!(g.score(), 95); // 18 x 5 + 5 clean bounty
    assert_eq!(g.state(), MgState::Won(Tier::Gold));
}

#[test]
fn goods_only_is_bronze_work() {
    let (mut sim, mut g) = open_run(7);
    play_all(&mut sim, &mut g, 5); // all late GOODS, chain unbroken
    assert_eq!(g.score(), 41); // 18 x 2 + 5
    assert_eq!(g.state(), MgState::Won(Tier::Bronze));
}

#[test]
fn one_good_denies_the_gold_line() {
    let (mut sim, mut g) = open_run(7);
    let mut first = true;
    for &(iv, n) in TEMPOS.iter() {
        for _ in 0..n {
            let mode = if first {
                first = false;
                -5 // one early GOOD, everything else perfect
            } else {
                0
            };
            strike(&mut sim, &mut g, (iv as i32 + mode) as u32);
        }
    }
    sim.idle(&mut g, 60);
    assert_eq!(g.score(), 92); // 17 x 5 + 2 + 5 < 95
    assert_eq!(g.state(), MgState::Won(Tier::Silver));
}

#[test]
fn early_swing_wastes_the_beat() {
    let (mut sim, mut g) = open_run(3);
    strike(&mut sim, &mut g, 1); // hammer falls into thin air
    assert_eq!(g.score(), 0);
    assert_eq!(g.combo(), 0);
    assert_eq!(g.beat_no(), 1);
}

#[test]
fn miss_breaks_the_clean_chain() {
    let (mut sim, mut g) = open_run(3);
    for (k, &(iv, n)) in TEMPOS.iter().enumerate() {
        for i in 0..n {
            let global = k as u32 * 6 + i; // groups are 6 beats each
            let mode = if global == 2 { -100 } else { 0 }; // beat 2: wasted
            strike(&mut sim, &mut g, (iv as i32 + mode).max(1) as u32);
        }
    }
    sim.idle(&mut g, 60);
    assert_eq!(g.score(), 85); // 17 perfects, no bounty
    assert_eq!(g.state(), MgState::Won(Tier::Silver));
}

#[test]
fn tempos_ramp_slow_fast_fury() {
    let (mut sim, mut g) = open_run(9);
    assert_eq!(g.current_interval(), 40);
    for _ in 0..6 {
        strike(&mut sim, &mut g, 39);
    }
    assert_eq!(g.current_interval(), 28);
    for _ in 0..6 {
        strike(&mut sim, &mut g, 27);
    }
    assert_eq!(g.current_interval(), 18);
    for _ in 0..6 {
        strike(&mut sim, &mut g, 17);
    }
    sim.idle(&mut g, 60);
    assert_eq!(g.state(), MgState::Won(Tier::Gold));
}

#[test]
fn unstruck_beats_walk_away_as_misses() {
    let (mut sim, mut g) = open_run(5);
    sim.idle(&mut g, 760); // every beat's deadline passes
    assert_eq!(g.score(), 0);
    assert_eq!(g.beat_no(), 18);
    assert_eq!(g.state(), MgState::Lost);
}

#[test]
fn only_space_swing_the_hammer() {
    let (mut sim, mut g) = open_run(5);
    for _ in 0..20 {
        sim.tap(&mut g, b'w' as u32); // walking does nothing here
    }
    assert_eq!(g.score(), 0);
    assert_eq!(g.beat_no(), 0); // beat 0 still armed
    assert_eq!(g.current_interval(), 40);
}

#[test]
fn perfect_script_is_deterministic() {
    let (mut sim, mut g) = open_run(11);
    play_all(&mut sim, &mut g, 0);
    let a = g.score();
    let (mut sim2, mut g2) = open_run(999); // seed must not matter
    play_all(&mut sim2, &mut g2, 0);
    assert_eq!(a, g2.score());
    assert_eq!(g.state(), g2.state());
}
