//! T203 proofs: fog of war and the spy action.
use defender_of_the_crown::campaign::game::{GameState, Intel, Faction, SPY_COST, PLAYER};

fn dark_castle(g: &GameState) -> usize {
    (0..g.strongholds.len())
        .find(|&i| g.strongholds[i].owner == Faction::Norman && !g.can_see(PLAYER, i))
        .expect("map must have a fogged Norman castle")
}

fn truth_counts(g: &GameState, sid: usize) -> [u32; 5] {
    g.counts(sid)
}

#[test]
fn own_and_adjacent_castles_are_visible() {
    let g = GameState::new(1337);
    for &sid in &g.ids_of(PLAYER) {
        let i = g.intel(PLAYER, sid).unwrap();
        assert_eq!(i.counts, truth_counts(&g, sid));
        assert_eq!(i.seen, g.turn);
        for adj in g.adj(sid) {
            // neighbour of an owned castle: seen too
            assert!(g.can_see(PLAYER, adj));
        }
    }
}

#[test]
fn distant_enemy_is_fogged() {
    let g = GameState::new(1337);
    let dark = dark_castle(&g);
    assert!(g.intel(PLAYER, dark).is_none());
}

#[test]
fn spy_lifts_fog_and_charges() {
    let mut g = GameState::new(1337);
    let dark = dark_castle(&g);
    g.gold[0] = 50;
    let rep = g.spy(PLAYER, dark).unwrap();
    assert_eq!(rep.counts, truth_counts(&g, dark));
    assert_eq!(rep.walls, g.strongholds[dark].walls);
    assert_eq!(rep.seen, 1);
    assert_eq!(g.gi(PLAYER), 50 - SPY_COST);
    assert_eq!(g.intel(PLAYER, dark), Some(rep));
    assert!(g.log.iter().any(|l| l.contains("SPIES REPORT")));
}

#[test]
fn spy_refuses_own_and_broke() {
    let mut g = GameState::new(1337);
    let mine = g.ids_of(PLAYER)[0];
    assert!(g.spy(PLAYER, mine).is_err());
    let dark = dark_castle(&g);
    g.gold[0] = SPY_COST - 1;
    assert!(g.spy(PLAYER, dark).is_err());
    assert!(g.intel(PLAYER, dark).is_none());
}

#[test]
fn stale_reports_keep_their_stamp_and_their_lies() {
    let mut g = GameState::new(1337);
    let dark = dark_castle(&g);
    g.gold[0] = 200;
    let rep = g.spy(PLAYER, dark).unwrap();
    // army moves on while the spies sleep
    g.strongholds[dark].garrison.clear();
    let moved = g.spy(PLAYER, dark);
    assert!(matches!(moved, Ok(Intel { seen: 1, .. })));
    // a report from turn 1 must not mutate when the truth does
    let mut h = GameState::new(1337);
    h.gold[0] = 200;
    let old = h.spy(PLAYER, dark).unwrap();
    h.strongholds[dark].garrison.clear();
    assert_eq!(h.intel(PLAYER, dark), Some(old));
    assert_ne!(old.counts, truth_counts(&h, dark));
}

#[test]
fn battle_reports_survive_losing_the_staging_castle() {
    let mut g = GameState::new(7);
    let home = 0usize;
    let target = g.adj(home)[0];
    g.strongholds[target].owner = Faction::Norman;
    // an attack guaranteed to be repelled: two levies vs thirty-two knights
    g.strongholds[target].garrison.clear();
    for _ in 0..32 {
        g.strongholds[target]
            .garrison
            .push(defender_of_the_crown::campaign::units::Unit::new(
                defender_of_the_crown::campaign::units::UnitKind::Knight,
            ));
    }
    g.strongholds[home].garrison.clear();
    g.strongholds[home]
        .garrison
        .push(defender_of_the_crown::campaign::units::Unit::new(
            defender_of_the_crown::campaign::units::UnitKind::Levy,
        ));
    g.march(PLAYER, home, target).unwrap();
    assert_eq!(g.strongholds[target].owner, Faction::Norman);
    // now the Normans retake the staging castle: live eyes are gone
    // Normans retake the whole ring: live eyes are gone everywhere
    let ring: Vec<usize> = g.adj(target);
    for a in ring {
        g.strongholds[a].owner = Faction::Norman;
        g.strongholds[a].garrison.clear();
    }
    assert!(!g.can_see(PLAYER, target));
    let rep = g.intel(PLAYER, target).expect("battle record must persist");
    assert_eq!(rep.counts, truth_counts(&g, target));
    assert_eq!(rep.seen, g.turn);
}

#[test]
fn intel_state_is_deterministic() {
    let run = || {
        let mut g = GameState::new(4242);
        let dark = dark_castle(&g);
        g.gold[0] = 300;
        g.spy(PLAYER, dark).unwrap();
        for _ in 0..5 {
            g.end_player_turn();
        }
        (g.known.clone(), g.intel(PLAYER, dark))
    };
    assert_eq!(run(), run());
}
