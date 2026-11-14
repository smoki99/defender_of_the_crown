//! SPEC 5.4: one weighted omen a turn, both factions feeling it, and the
//! Danish axe waking at turn 12. Effects are rolled in isolation via
//! `roll_events` (the AI must not muddy omen tests); one seeded full game
//! proves the whole loop is deterministic.

use defender_of_the_crown::campaign::events::EventKind;
use defender_of_the_crown::campaign::game::{GameState, PLAYER};
use defender_of_the_crown::campaign::units::{Unit, UnitKind};

fn force(g: &mut GameState, e: EventKind) {
    g.tip = Some(e);
    g.roll_events();
}

fn count_log(g: &GameState, needle: &str) -> usize {
    g.log.iter().filter(|l| l.contains(needle)).count()
}

#[test]
fn omens_are_seeded_and_land_every_turn() {
    // Every live turn mints exactly one omen; a full game of them must be
    // reproducible from the seed alone. (The log itself is trimmed, so the
    // per-tick vector is the ledger.)
    let run = |seed: u64| -> Vec<&'static str> {
        let mut g = GameState::new(seed);
        let mut v = Vec::new();
        for _ in 0..15 {
            g.end_player_turn();
            if g.over.is_some() {
                break;
            }
            v.push(g.event.unwrap().title());
        }
        v
    };
    let a = run(11);
    let b = run(11);
    assert_eq!(a, b, "same seed, same fates");
    assert!(a.len() >= 5, "omens keep landing while the war lives");
}

#[test]
fn a_spy_tip_overrides_the_dice_once() {
    let mut g = GameState::new(5);
    force(&mut g, EventKind::KingsLevy);
    assert_eq!(g.event, Some(EventKind::KingsLevy));
    assert_eq!(g.tip, None, "the whisper is spent");
}

#[test]
fn plague_halves_income_and_burns_two_turns() {
    let mut g = GameState::new(7);
    let full = g.income_of(PLAYER);
    g.plague[0] = 1;
    assert_eq!(g.income_of(PLAYER), full / 2);
    g.plague[0] = 0;
    assert_eq!(g.income_of(PLAYER), full);

    force(&mut g, EventKind::Plague);
    assert_eq!(g.plague[0], 2, "two turns of half harvest");
    assert_eq!(count_log(&g, "A PLEAGUE STRIKETH"), 2);
}

#[test]
fn kings_levy_garrisons_the_capital() {
    let mut g = GameState::new(3);
    let cap = (0..g.strongholds.len())
        .find(|&i| g.strongholds[i].owner == PLAYER && g.strongholds[i].capital)
        .unwrap();
    let n0 = g.strongholds[cap].garrison.len();
    force(&mut g, EventKind::KingsLevy);
    assert_eq!(g.strongholds[cap].garrison.len(), n0 + 3);
    assert_eq!(count_log(&g, "HARBOURS"), 2, "both kings get men");
}

#[test]
fn deserters_thin_the_largest_host() {
    let mut g = GameState::new(3);
    let big = (0..g.strongholds.len())
        .filter(|&i| g.strongholds[i].owner == PLAYER)
        .max_by_key(|&i| (g.strongholds[i].garrison.len(), std::cmp::Reverse(i)))
        .unwrap();
    let n0 = g.strongholds[big].garrison.len();
    force(&mut g, EventKind::Deserters);
    assert_eq!(g.strongholds[big].garrison.len(), n0 - 2);
    assert_eq!(count_log(&g, "SLINK"), 2);
}

#[test]
fn caravan_enricheth_and_harvest_faileth() {
    let mut g = GameState::new(9);
    let g0 = g.gi(PLAYER);
    force(&mut g, EventKind::MerchantCaravan);
    assert!(g.gi(PLAYER) > g0);
    assert_eq!(count_log(&g, "SWELL"), 2);

    let mut g = GameState::new(9);
    let p0 = g.bars_of(PLAYER).provisions;
    force(&mut g, EventKind::BadHarvest);
    assert!(g.bars_of(PLAYER).provisions < p0);
    assert_eq!(count_log(&g, "BAD HARVEST IN"), 2);
}

#[test]
fn the_danish_axe_sleeps_until_turn_twelve() {
    use defender_of_the_crown::campaign::rng::Xor;
    for turn in 1..12u32 {
        assert_eq!(EventKind::DanishRaiders.weight(turn), 0);
        for seed in 1..=400u64 {
            let mut r = Xor::new(seed);
            assert_ne!(EventKind::roll(&mut r, turn), EventKind::DanishRaiders);
        }
    }
    assert!(EventKind::DanishRaiders.weight(12) > 0);
    let woke = (1..=400u64).any(|seed| {
        let mut r = Xor::new(seed);
        EventKind::roll(&mut r, 12) == EventKind::DanishRaiders
    });
    assert!(woke, "raiders must be reachable at turn 12");
}

/// Put a bare coast before the pirates: it burns.
#[test]
fn raiders_sack_a_bare_coast() {
    let mut g = GameState::new(7);
    let coast: Vec<usize> = (0..g.strongholds.len())
        .filter(|&i| g.strongholds[i].coast)
        .collect();
    assert!(!coast.is_empty(), "map needs a coast");
    for &i in &coast {
        g.strongholds[i].garrison = vec![Unit::new(UnitKind::Levy); 12];
    }
    let soft = coast
        .iter()
        .copied()
        .find(|&i| g.strongholds[i].owner == PLAYER)
        .expect("saxons own a coast");
    g.strongholds[soft].garrison = vec![Unit::new(UnitKind::Levy); 1];
    force(&mut g, EventKind::DanishRaiders);
    assert!(count_log(&g, "DANISH RAIDERS SACK") >= 1);
    assert!(
        g.strongholds[soft].garrison.is_empty(),
        "the lone levy died"
    );
    assert_eq!(count_log(&g, "PUT BACK TO SEA"), 1, "the Norman shore held");
}

#[test]
fn a_stout_shore_sendeth_pirates_home() {
    let mut g = GameState::new(7);
    for i in 0..g.strongholds.len() {
        if g.strongholds[i].coast {
            g.strongholds[i].garrison = vec![Unit::new(UnitKind::Knight); 12];
        }
    }
    force(&mut g, EventKind::DanishRaiders);
    assert_eq!(count_log(&g, "SACK"), 0);
    assert_eq!(count_log(&g, "PUT BACK TO SEA"), 2, "both shores held");
}

#[test]
fn a_herald_raises_a_tourney_banner() {
    let mut g = GameState::new(4);
    force(&mut g, EventKind::TournamentHerald);
    let t = g.tournament[PLAYER.index()].expect("herald tendeth somewhere");
    assert_eq!(
        g.strongholds[t].owner, PLAYER,
        "a herald camps on home soil"
    );
    assert_eq!(count_log(&g, "HERALD CRIES"), 2);
    assert!(g.play_tournament(PLAYER, 999, 10, 50).is_ok(), "free entry");
    assert!(
        g.play_tournament(PLAYER, 999, 10, 50).is_err(),
        "one meet per herald"
    );
}
