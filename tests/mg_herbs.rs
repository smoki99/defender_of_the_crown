//! T701: the MEMORY herb salve. Sixteen tiles, eight kinds; two clicks
//! turn a pair, matched kinds stay down, mismatches re-hide after a short
//! show. The cauldron boils for BOWL ticks - finish the brew in time, and
//! finish it FAST: the speed bonus is what carries a flawless run over
//! the 90 gold line. Layout is the only seed variance (and a test seam).

use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::{Input, Raw};
use defender_of_the_crown::minigames::framework::{MgState, Minigame, Tier};
use defender_of_the_crown::minigames::herbs::Herbs;

const PRESENT: u32 = 90;
const BOWL: u32 = 2400;
const SHOW: u32 = 30;

// mirror of the module geometry
const TILE: i32 = 36;
const GAP: i32 = 6;
const X0: i32 = (320 - (4 * TILE + 3 * GAP)) / 2;
const Y0: i32 = 76;

fn center(i: usize) -> (i32, i32) {
    (
        X0 + (i as i32 % 4) * (TILE + GAP) + TILE / 2,
        Y0 + (i as i32 / 4) * (TILE + GAP) + TILE / 2,
    )
}

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
    fn frame(&mut self, g: &mut Herbs, events: &[Raw]) {
        for e in events {
            self.inp.push(*e);
        }
        self.frame += 1;
        self.inp.begin_frame(self.frame);
        g.tick(1.0 / 60.0, &self.inp, &mut self.fb);
    }
    fn idle(&mut self, g: &mut Herbs, ticks: u32) {
        for _ in 0..ticks {
            self.frame(g, &[]);
        }
    }
    /// One click: hover, press frame (judged), release frame.
    fn click(&mut self, g: &mut Herbs, i: usize) {
        let (x, y) = center(i);
        self.frame(g, &[Raw::Move { x, y }, Raw::Down { x, y }]);
        self.frame(g, &[Raw::Up { x, y }]);
    }
}

fn open_run(seed: u64) -> (Sim, Herbs) {
    let mut sim = Sim::new();
    let mut g = Herbs::new(seed);
    sim.idle(&mut g, PRESENT + 1); // burn the banner; Run active
    (sim, g)
}

fn find_pair(layout: &[u8; 16], kind: u8) -> (usize, usize) {
    let mut got = [usize::MAX; 2];
    let mut n = 0;
    for (i, k) in layout.iter().enumerate() {
        if *k == kind {
            got[n] = i;
            n += 1;
        }
    }
    assert_eq!(n, 2, "kind {kind} must appear exactly twice");
    (got[0], got[1])
}

/// Click a known pair and let the show elapse; returns nothing - the
/// caller reads `pairs`.
fn bank_pair(sim: &mut Sim, g: &mut Herbs, kind: u8, extra_idle: u32) {
    let (a, b) = find_pair(&g.layout(), kind);
    sim.click(g, a);
    sim.click(g, b);
    sim.idle(g, SHOW + extra_idle);
}

#[test]
fn banner_gates_the_board() {
    let mut sim = Sim::new();
    let mut g = Herbs::new(7);
    assert!(matches!(g.state(), MgState::Running));
    sim.click(&mut g, 0); // clicked during PRESENT: nothing turns
    sim.idle(&mut g, PRESENT - 1);
    assert_eq!(g.score(), 0);
    assert!(!g.is_up(0));
    sim.idle(&mut g, 2); // Run begins, and 0 is still face-down
    assert!(!g.is_up(0));
}

#[test]
fn mismatch_shakes_then_rehides() {
    let (mut sim, mut g) = open_run(3);
    let lay = g.layout();
    let a = 0usize;
    // any tile of a different kind than tile 0
    let b = (1..16).find(|&i| lay[i] != lay[0]).unwrap();
    sim.click(&mut g, a);
    sim.click(&mut g, b);
    assert!(g.is_up(a) && g.is_up(b), "both show during the mismatch");
    // locked out: a third click is ignored while the pair is on display
    let c = (1..16).find(|&i| lay[i] != lay[0] && i != b).unwrap();
    sim.click(&mut g, c);
    assert!(!g.is_up(c));
    sim.idle(&mut g, SHOW);
    assert!(!g.is_up(a) && !g.is_up(b), "mismatch re-hides");
    assert_eq!(g.pairs(), 0);
    // a lone reveal stays up indefinitely (no memory tax)
    sim.click(&mut g, a);
    sim.idle(&mut g, SHOW + 60);
    assert!(g.is_up(a));
}

#[test]
fn match_banks_and_locks() {
    let (mut sim, mut g) = open_run(11);
    bank_pair(&mut sim, &mut g, 0, 0);
    assert_eq!(g.pairs(), 1);
    let (a, b) = find_pair(&g.layout(), 0);
    assert!(g.is_up(a) && g.is_up(b), "matched tiles stay down");
    sim.click(&mut g, a); // spent slots ignore clicks
    assert_eq!(g.pairs(), 1);
}

#[test]
fn fast_flawless_brew_is_gold() {
    let (mut sim, mut g) = open_run(5);
    for kind in 0..8u8 {
        bank_pair(&mut sim, &mut g, kind, 0);
    }
    assert!(
        matches!(g.state(), MgState::Won(Tier::Gold)),
        "8 pairs in ~{} ticks must plate gold: {:?}",
        g.elapsed(),
        g.state()
    );
}

#[test]
fn dawdling_brew_drops_to_silver() {
    let (mut sim, mut g) = open_run(5);
    for kind in 0..8u8 {
        bank_pair(&mut sim, &mut g, kind, 150); // slow hands
    }
    // 80 pairs + a thin speed bonus: over silver, short of gold.
    assert_eq!(g.state(), MgState::Won(Tier::Silver));
}

#[test]
fn boil_over_ends_the_brew() {
    let (mut sim, mut g) = open_run(9);
    sim.idle(&mut g, BOWL + 5);
    assert_eq!(g.state(), MgState::Lost, "nothing matched, pot boiled over");
    assert_eq!(g.score(), 0);
}

#[test]
fn partial_brew_plates_bronze() {
    let (mut sim, mut g) = open_run(13);
    for kind in 0..4u8 {
        bank_pair(&mut sim, &mut g, kind, 0);
    }
    sim.idle(&mut g, BOWL + 5); // the rest of the herbs burn
    assert_eq!(g.state(), MgState::Won(Tier::Bronze)); // 40 = bronze line
}

#[test]
fn layout_is_a_pair_bag_and_seed_true() {
    for seed in [1u64, 2, 99] {
        let lay = Herbs::new(seed).layout();
        for kind in 0..8u8 {
            assert_eq!(lay.iter().filter(|&&k| k == kind).count(), 2);
        }
    }
    assert_eq!(
        Herbs::new(4).layout(),
        Herbs::new(4).layout(),
        "same seed, same board"
    );
    assert_ne!(Herbs::new(1).layout(), Herbs::new(2).layout());
}

#[test]
fn hover_and_gutters_never_flip() {
    let (mut sim, mut g) = open_run(6);
    for i in 0..16 {
        let (x, y) = center(i);
        sim.frame(&mut g, &[Raw::Move { x, y }]); // hover only
    }
    // gutter coordinates inside the grid bounds
    sim.click_at(&mut g, X0 + TILE + 2, Y0 + 2);
    sim.click_at(&mut g, X0 - 1, Y0 - 1);
    assert_eq!(g.pairs(), 0);
    assert!((0..16).all(|i| !g.is_up(i)));
}

impl Sim {
    fn click_at(&mut self, g: &mut Herbs, x: i32, y: i32) {
        self.frame(g, &[Raw::Move { x, y }, Raw::Down { x, y }]);
        self.frame(g, &[Raw::Up { x, y }]);
    }
}
