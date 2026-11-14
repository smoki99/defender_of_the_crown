//! T501 minigame framework tests: SPORTS table integrity, unlock pacing,
//! deterministic draws, the cheer clock, and tier resolution.

use defender_of_the_crown::campaign::game::Tier;
use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::Input;
use defender_of_the_crown::minigames::framework::{
    pick, resolve, unlocked, Cheer, MgOutcome, MgState, Minigame, SportSpec, SPORTS,
};

/// SPEC 6 catalogue order.
const SPEC_ORDER: [&str; 13] = [
    "joust", "catapult", "sword", "archery", "rampart", "maze", "river", "cloak", "forge", "herbs",
    "falcon", "tavern", "dragon",
];

const SPEC_LABELS: [&str; 13] = [
    "TOURNAMENT",
    "SIEGE",
    "DUEL",
    "GALLERY",
    "DEFEND",
    "RESCUE",
    "CANOE",
    "STEALTH",
    "RHYTHM",
    "MEMORY",
    "FALCONER",
    "BRAWL",
    "HOARD",
];

fn spec_of(id: &str) -> &'static SportSpec {
    SPORTS.iter().find(|s| s.id == id).expect("spec row exists")
}

#[test]
fn mg_ids_unique_and_in_spec_order() {
    let ids: Vec<&str> = SPORTS.iter().map(|s| s.id).collect();
    assert_eq!(ids.as_slice(), SPEC_ORDER.as_slice());
    let labels: Vec<&str> = SPORTS.iter().map(|s| s.label).collect();
    assert_eq!(labels.as_slice(), SPEC_LABELS.as_slice());
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len(), "duplicate sport ids");
}

#[test]
fn mg_thresholds_strictly_ordered() {
    for s in SPORTS {
        assert!(
            s.bronze_at > 0,
            "{}: bronze must be reachable-only-by-play",
            s.id
        );
        assert!(s.bronze_at < s.silver_at, "{}: bronze >= silver", s.id);
        assert!(s.silver_at < s.gold_at, "{}: silver >= gold", s.id);
    }
}

#[test]
fn mg_unlock_boundaries() {
    // SPEC 5.5 pacing: 1-4 from the start, 5-9 from turn 5, 10-12 from 10.
    assert_eq!(unlocked(0).len(), 4);
    assert_eq!(unlocked(4).len(), 4);
    assert_eq!(unlocked(5).len(), 9);
    assert_eq!(unlocked(9).len(), 9);
    assert_eq!(unlocked(10).len(), 12);
    for turn in [11, 50, 90, u32::MAX - 1] {
        assert_eq!(unlocked(turn).len(), 12);
    }
    let t5: Vec<&str> = unlocked(5).iter().map(|s| s.id).collect();
    assert_eq!(t5, SPEC_ORDER[..9]);
    let t10: Vec<&str> = unlocked(10).iter().map(|s| s.id).collect();
    assert_eq!(t10, SPEC_ORDER[..12]);
}

#[test]
fn mg_dragon_is_never_offered() {
    for &turn in &[0u32, 5, 10, 20, 42, u32::MAX] {
        assert!(
            unlocked(turn).iter().all(|s| s.id != "dragon"),
            "dragon leaked into unlocked() at turn {turn}"
        );
        for seed in 0..200u64 {
            if let Some(s) = pick(turn, seed) {
                assert_ne!(s.id, "dragon", "dragon leaked from pick() seed {seed}");
            }
        }
    }
}

#[test]
fn mg_pick_is_deterministic() {
    for seed in 0..50u64 {
        let a = pick(7, seed).map(|s| s.id);
        let b = pick(7, seed).map(|s| s.id);
        assert_eq!(a, b, "seed {seed} drifted between calls");
        assert!(a.is_some(), "something is always unlocked");
        assert!(
            unlocked(7).iter().any(|s| Some(s.id) == a),
            "pick outside pool"
        );
    }
}

#[test]
fn mg_pick_covers_all_starters() {
    let mut seen: Vec<&str> = Vec::new();
    for seed in 0..500u64 {
        if let Some(s) = pick(0, seed) {
            assert_eq!(s.unlock_turn, 0, "locked sport drawn at turn 0");
            if !seen.contains(&s.id) {
                seen.push(s.id);
            }
        }
    }
    for id in ["joust", "catapult", "sword", "archery"] {
        assert!(seen.contains(&id), "{id} never picked over 500 seeds");
    }
}

#[test]
fn mg_cheer_ends_at_60s_with_monotonic_frac() {
    assert_eq!(Cheer::SECONDS, 60.0);
    let mut c = Cheer::new();
    assert!(!c.done());
    assert_eq!(c.frac(), 0.0);
    // 1/128 is exact in binary floats, so the accumulated sum is exact too.
    let dt = 1.0 / 128.0;
    let mut spent = 0.0f32;
    let mut prev = 0.0f32;
    while !c.done() {
        c.step(dt);
        spent += dt;
        let f = c.frac();
        assert!(f >= prev, "frac regressed at {spent}s");
        assert!((0.0..=1.0).contains(&f));
        prev = f;
    }
    assert!(
        (spent - Cheer::SECONDS).abs() < 1e-3,
        "cheer done at {spent}s, want 60.0"
    );
    assert!((c.frac() - 1.0).abs() < 1e-6);
    // The clock never overflows and never rewinds.
    c.step(10.0);
    assert_eq!(c.frac(), 1.0);
    c.step(-5.0);
    assert_eq!(c.frac(), 1.0);
    assert!(c.done());
}

#[test]
fn mg_resolve_hits_every_tier_boundary() {
    for s in SPORTS {
        assert_eq!(resolve(0, s), MgState::Lost);
        assert_eq!(resolve(s.bronze_at - 1, s), MgState::Lost);
        assert_eq!(resolve(s.bronze_at, s), MgState::Won(Tier::Bronze));
        assert_eq!(resolve(s.bronze_at + 1, s), MgState::Won(Tier::Bronze));
        assert_eq!(resolve(s.silver_at - 1, s), MgState::Won(Tier::Bronze));
        assert_eq!(resolve(s.silver_at, s), MgState::Won(Tier::Silver));
        assert_eq!(resolve(s.gold_at - 1, s), MgState::Won(Tier::Silver));
        assert_eq!(resolve(s.gold_at, s), MgState::Won(Tier::Gold));
        assert_eq!(resolve(s.gold_at + 1000, s), MgState::Won(Tier::Gold));
    }
}

/// Minimal sport implementing the trait shape every game module adopts.
struct Dummy {
    score: u32,
    state: MgState,
}

impl Minigame for Dummy {
    fn id(&self) -> &'static str {
        "joust"
    }
    fn tick(&mut self, _dt: f32, _input: &Input, _fb: &mut Framebuffer) {
        self.score += 1;
        if self.score >= 95 {
            self.state = MgState::Won(Tier::Gold);
        }
    }
    fn score(&self) -> u32 {
        self.score
    }
    fn state(&self) -> MgState {
        self.state
    }
}

#[test]
fn mg_trait_runner_and_outcome_roundtrip() {
    let spec = spec_of("joust");
    let mut game = Dummy {
        score: 0,
        state: MgState::Running,
    };
    let input = Input::new();
    let mut fb = Framebuffer::new();
    let mut cheer = Cheer::new();
    // The runner loop shape the scene wrapper will use: play until the game
    // decides or the crowd's cheer runs out, then resolve the score.
    while game.state() == MgState::Running && !cheer.done() {
        game.tick(1.0 / 60.0, &input, &mut fb);
        cheer.step(1.0 / 60.0);
    }
    assert_eq!(game.state(), MgState::Won(Tier::Gold));
    assert!(
        !cheer.done(),
        "script-perfect 95-point run must land well inside the cheer"
    );
    let tier = match resolve(game.score(), spec) {
        MgState::Won(t) => Some(t),
        _ => None,
    };
    let outcome = MgOutcome {
        id: game.id(),
        score: game.score(),
        tier,
    };
    assert_eq!(
        outcome,
        MgOutcome {
            id: "joust",
            score: 95,
            tier: Some(Tier::Gold),
        }
    );
    // A stalled-out low score pays nothing.
    let dud = Dummy {
        score: 0,
        state: MgState::Running,
    };
    assert_eq!(resolve(dud.score(), spec), MgState::Lost);
}
