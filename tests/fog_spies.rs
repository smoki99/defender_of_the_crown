//! SPEC 5.3 fog: own eyes are truth, neighbours give "SEEN: ~N" ±50% scouts,
//! spies (70%) buy exact reports of a castle and its ring, failure just costs
//! gold, and battle reports outlive the eyes that saw them.
use defender_of_the_crown::campaign::game::{Faction, GameState, View, PLAYER, SPY_COST};
use defender_of_the_crown::campaign::rng::Xor;
use defender_of_the_crown::campaign::units::{Unit, UnitKind};

fn dark_castle(g: &GameState) -> usize {
    (0..g.strongholds.len())
        .find(|&i| matches!(g.view(PLAYER, i), View::Dark))
        .expect("a fogged castle")
}

fn enemy_next_door(g: &GameState) -> usize {
    (0..g.strongholds.len())
        .find(|&i| {
            g.strongholds[i].owner != PLAYER && matches!(g.view(PLAYER, i), View::Rough { .. })
        })
        .expect("an enemy in scout range")
}

fn levy(n: usize) -> Vec<Unit> {
    vec![Unit::new(UnitKind::Levy); n]
}

/// The spy action draws exactly two `below(100)` rolls from a fresh game.
fn seed_where(pred: impl Fn(u32, u32) -> bool) -> u64 {
    (1..100_000u64)
        .find(|&s| {
            let mut r = Xor::new(s);
            pred(r.below(100), r.below(100))
        })
        .expect("a seed with the wanted spy fate")
}

#[test]
fn own_castles_report_exact_truth() {
    let g = GameState::new(7);
    let own = (0..g.strongholds.len())
        .find(|&i| g.strongholds[i].owner == PLAYER)
        .unwrap();
    match g.view(PLAYER, own) {
        View::Own(rep) => {
            assert_eq!(
                rep.counts.iter().sum::<u32>(),
                g.strongholds[own].garrison.len() as u32
            );
            assert_eq!(rep.walls, g.strongholds[own].walls);
            assert_eq!(rep.seen, g.turn);
        }
        v => panic!("own castle gave {v:?}"),
    }
}

#[test]
fn neighbour_garrisons_are_rough_eyes_only() {
    let g = GameState::new(7);
    let t = enemy_next_door(&g);
    let truth = g.strongholds[t].garrison.len() as u32;
    let est = match g.view(PLAYER, t) {
        View::Rough { total } => total,
        v => panic!("expected a scout estimate, got {v:?}"),
    };
    // SPEC: "SEEN: ~N" is within ±50% of the truth and stable per view.
    assert!(
        (truth / 2..=truth / 2 + truth).contains(&est),
        "est {est} vs truth {truth}"
    );
    assert_eq!(
        g.view(PLAYER, t),
        g.view(PLAYER, t),
        "the lie must be stable"
    );
}

#[test]
fn distant_enemy_castle_is_fully_fogged() {
    let g = GameState::new(7);
    let v = dark_castle(&g);
    assert_eq!(g.view(PLAYER, v), View::Dark);
}

#[test]
fn a_successful_spy_reveals_the_castle_and_its_ring() {
    let seed = seed_where(|a, b| a < 70 && b >= 10);
    let mut g = GameState::new(seed);
    let t = dark_castle(&g);
    let gold = g.gi(PLAYER);
    assert_eq!(g.spy(PLAYER, t), Ok(true));
    assert_eq!(g.gi(PLAYER), gold - SPY_COST);
    for sid in std::iter::once(t).chain(g.strongholds[t].adj.iter().copied()) {
        if g.strongholds[sid].owner == PLAYER {
            continue;
        }
        match g.view(PLAYER, sid) {
            View::Spied(rep) => {
                assert_eq!(
                    rep.counts.iter().sum::<u32>(),
                    g.strongholds[sid].garrison.len() as u32
                )
            }
            v => panic!("spy should have cleared {sid}, got {v:?}"),
        }
    }
    assert!(g.log.iter().any(|l| l.contains("SPIES REPORT")));
}

#[test]
fn a_caught_spy_only_costs_the_gold() {
    let seed = seed_where(|a, _| a >= 70);
    let mut g = GameState::new(seed);
    let t = dark_castle(&g);
    let gold = g.gi(PLAYER);
    assert_eq!(g.spy(PLAYER, t), Ok(false));
    assert_eq!(g.gi(PLAYER), gold - SPY_COST);
    assert_eq!(
        g.view(PLAYER, t),
        View::Dark,
        "no intel survives the capture"
    );
    assert!(g.log.iter().any(|l| l.contains("CAUGHT")));
    // SPEC: failure is repeatable - the spy slot is open again.
    assert!(g.spy(PLAYER, t).is_ok());
}

#[test]
fn lucky_spies_whisper_tomorrows_event() {
    let seed = seed_where(|a, b| a < 70 && b < 10);
    let mut g = GameState::new(seed);
    let t = dark_castle(&g);
    assert_eq!(g.spy(PLAYER, t), Ok(true));
    assert!(g.tip.is_some(), "one time in ten the servant talks");
}

#[test]
fn spy_refusals_cost_nothing() {
    let mut g = GameState::new(7);
    let own = (0..g.strongholds.len())
        .find(|&i| g.strongholds[i].owner == PLAYER)
        .unwrap();
    let gold = g.gi(PLAYER);
    assert!(g.spy(PLAYER, own).is_err());
    assert_eq!(g.gi(PLAYER), gold, "refusal must not bill you");
    g.gold[PLAYER.index()] = SPY_COST - 1;
    let t = dark_castle(&g);
    assert!(g.spy(PLAYER, t).is_err());
    assert_eq!(g.gi(PLAYER), SPY_COST - 1);
}

#[test]
fn spied_reports_go_stale_but_never_lie_about_the_past() {
    let seed = seed_where(|a, b| a < 70 && b >= 10);
    let mut g = GameState::new(seed);
    let t = dark_castle(&g);
    assert_eq!(g.spy(PLAYER, t), Ok(true));
    let truth_at_spy = g.strongholds[t].garrison.len() as u32;
    g.strongholds[t].garrison = levy(3);
    match g.view(PLAYER, t) {
        View::Spied(rep) => {
            assert_eq!(
                rep.counts.iter().sum::<u32>(),
                truth_at_spy,
                "the stamp must hold its value"
            );
            assert_ne!(rep.counts.iter().sum::<u32>(), 3);
        }
        v => panic!("expected a stale report, got {v:?}"),
    }
}

#[test]
fn own_eyes_always_beat_a_stale_report() {
    let seed = seed_where(|a, b| a < 70 && b >= 10);
    let mut g = GameState::new(seed);
    let t = dark_castle(&g);
    assert_eq!(g.spy(PLAYER, t), Ok(true));
    g.strongholds[t].owner = PLAYER; // recaptured
    g.strongholds[t].garrison = levy(3);
    match g.view(PLAYER, t) {
        View::Own(rep) => assert_eq!(rep.counts.iter().sum::<u32>(), 3),
        v => panic!("own eyes must win, got {v:?}"),
    }
}

#[test]
fn battle_reports_survive_losing_the_staging_castle() {
    let mut g = GameState::new(7);
    let home = (0..g.strongholds.len())
        .find(|&i| {
            g.strongholds[i].owner == PLAYER
                && g.strongholds[i].garrison.len() >= 2
                && g.adj(i).iter().any(|&a| g.strongholds[a].owner != PLAYER)
        })
        .unwrap();
    let target = g
        .adj(home)
        .iter()
        .copied()
        .find(|&a| g.strongholds[a].owner != PLAYER)
        .unwrap();
    assert!(g.march(PLAYER, home, target).is_ok()); // a battle was fought
                                                    // Normans retake it; Saxon eyes are gone everywhere nearby.
    g.strongholds[target].owner = Faction::Norman;
    let ring: Vec<usize> = g.adj(target).to_vec();
    for a in ring {
        g.strongholds[a].owner = Faction::Norman;
        g.strongholds[a].garrison.clear();
    }
    assert!(!g.can_see(PLAYER, target), "the ring must really be blind");
    let rep = match g.view(PLAYER, target) {
        View::Spied(rep) => rep,
        v => panic!("the battle report must persist, got {v:?}"),
    };
    assert_eq!(rep.seen, g.turn);
    assert!(
        rep.counts.iter().sum::<u32>() > 0,
        "a garrison was watched at combat time"
    );
}

#[test]
fn spy_fate_is_deterministic() {
    let seed = seed_where(|a, b| a < 70 && b < 10);
    let run = || {
        let mut g = GameState::new(seed);
        let t = dark_castle(&g);
        let r = g.spy(PLAYER, t);
        (r, g.known.clone(), g.tip)
    };
    let a = run();
    let b = run();
    assert_eq!(a.0, b.0);
    assert_eq!(a.1, b.1);
    assert_eq!(a.2, b.2);
}
