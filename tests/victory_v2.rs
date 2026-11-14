//! SPEC 5.6: how the war ends. Coronation (London + 8 shires), annihilation,
//! despair at a fallen home keep, and the turn-90 chronicle judgment.
//! Garrisons of thirty knights keep the AI from meddling in scripted states.

use defender_of_the_crown::campaign::game::{Bars, Faction, GameState, PLAYER};
use defender_of_the_crown::campaign::units::{Unit, UnitKind};

fn by_name(g: &GameState, name: &str) -> usize {
    g.strongholds.iter().find(|s| s.name == name).unwrap().id
}

fn give(g: &mut GameState, names: &[&str], f: Faction) {
    for n in names {
        let i = by_name(g, n);
        g.strongholds[i].owner = f;
    }
}

fn count_log(g: &GameState, needle: &str) -> usize {
    g.log.iter().filter(|l| l.contains(needle)).count()
}

/// Ten knights a castle: the provisions cap (10 a castle) feeds no more,
/// and ten is exactly the raid-repel line - ownership freezes (with full
/// loyalty: no hunger, no desertion, no revolt).
fn fortify(g: &mut GameState) {
    for s in g.strongholds.iter_mut() {
        s.garrison = vec![Unit::new(UnitKind::Knight); 10];
    }
    for b in g.bars.iter_mut() {
        *b = Bars {
            provisions: 99_999,
            loyalty: 100,
            popularity: 60,
        };
    }
}

fn count(g: &GameState, f: Faction) -> usize {
    g.strongholds.iter().filter(|s| s.owner == f).count()
}

#[test]
fn holding_london_with_eight_shires_crowneth_the_player() {
    let mut g = GameState::new(7);
    give(&mut g, &["London", "Norwich"], PLAYER);
    assert_eq!(count(&g, PLAYER), 8);
    fortify(&mut g);
    g.end_player_turn();
    assert_eq!(g.over, Some(PLAYER));
    assert!(g.log.iter().any(|l| l.contains("CROWNED")), "{:?}", g.log);
}

#[test]
fn seven_shires_and_london_are_not_yet_a_kingdom() {
    let mut g = GameState::new(7);
    give(&mut g, &["London"], PLAYER);
    assert_eq!(count(&g, PLAYER), 7);
    fortify(&mut g);
    g.end_player_turn();
    assert_eq!(g.over, None, "the archbishop waiteth for the eighth");
}

#[test]
fn the_norman_likewise_crowneth_himself() {
    let mut g = GameState::new(8);
    give(&mut g, &["Oxford", "Salisbury"], Faction::Norman);
    assert_eq!(count(&g, Faction::Norman), 8);
    fortify(&mut g);
    g.end_player_turn();
    assert_eq!(g.over, Some(Faction::Norman));
    assert!(g.log.iter().any(|l| l.contains("CROWNED")));
}

#[test]
fn losing_every_castle_is_defeat() {
    let mut g = GameState::new(9);
    for s in g.strongholds.iter_mut() {
        s.owner = Faction::Norman;
    }
    g.end_player_turn();
    assert_eq!(g.over, Some(Faction::Norman));
    assert!(g.log.iter().any(|l| l.contains("DEFEAT")));
}

/// Loyalty delta one turn after Buckingham falls, versus the same turn
/// without the loss - isolates the -30 despair from payday upkeep.
fn loyalty_delta(capture: bool) -> i64 {
    let mut g = GameState::new(4);
    give(&mut g, &["Oxford"], Faction::Norman); // 5 Saxon / 7 Norman
    if capture {
        let buck = g.homes[0];
        assert_eq!(g.strongholds[buck].name, "Buckingham");
        g.strongholds[buck].owner = Faction::Norman;
    }
    fortify(&mut g);
    let before = g.bars_of(PLAYER).loyalty as i64;
    g.end_player_turn();
    let after = g.bars_of(PLAYER).loyalty as i64;
    if capture {
        assert_eq!(
            g.log.iter().filter(|l| l.contains("DESPAIR")).count(),
            1,
            "despair striketh once"
        );
        assert!(g.home_lost[PLAYER.index()]);
    }
    after - before
}

#[test]
fn a_fallen_home_keep_bringeth_despair_once() {
    let quiet = loyalty_delta(false);
    let fallen = loyalty_delta(true);
    assert_eq!(fallen, quiet - 30, "despair is exactly -30 loyalty");
}

#[test]
fn the_chronicle_judgeth_at_turn_ninety() {
    let mut g = GameState::new(3);
    g.turn = 89;
    give(&mut g, &["Dover", "Colchester"], PLAYER); // 8 Saxon v 4 Norman
    fortify(&mut g);
    g.end_player_turn();
    assert_eq!(g.turn, 90);
    assert_eq!(g.over, Some(PLAYER));
    assert!(g
        .log
        .iter()
        .any(|l| l.contains("TURN XC") && l.contains("VICTORY")));
}
#[test]
fn an_even_tale_at_turn_ninety_spareth_the_war() {
    let mut g = GameState::new(3);
    g.turn = 89;
    give(&mut g, &["Oxford"], Faction::Norman); // 5 Saxon / 6 Norman... make even:
    give(&mut g, &["Dover"], PLAYER); // 6 Saxon / 6 Norman
    assert_eq!(count(&g, PLAYER), 6);
    assert_eq!(count(&g, Faction::Norman), 6);
    fortify(&mut g);
    g.end_player_turn();
    assert_eq!(g.over, None, "a draw writeth no ending");
    assert!(g.log.iter().any(|l| l.contains("EVEN TALE")));
    g.end_player_turn();
    assert_eq!(
        count_log(&g, "TURN XC."),
        1,
        "the chronicle speaketh only at turn ninety"
    );
}
#[test]
fn an_even_tale_rejudgeth_when_a_later_decade_tipeth() {
    let mut g = GameState::new(3);
    g.turn = 89;
    give(&mut g, &["Oxford"], Faction::Norman);
    give(&mut g, &["Dover"], PLAYER); // 6 Saxon / 6 Norman
    fortify(&mut g);
    g.end_player_turn(); // turn 90: even tale, the war is spared
    assert_eq!(g.over, None);
    g.turn = 99;
    give(&mut g, &["York"], PLAYER); // the scales tip: 7 / 5
    fortify(&mut g);
    g.end_player_turn(); // turn 100: re-judged
    assert_eq!(g.over, Some(PLAYER), "a tipped decade must write the ending");
    assert!(g
        .log
        .iter()
        .any(|l| l.contains("THE CHRONICLE DECIDETH") && !l.contains("TURN XC.")),
        "only the ninetieth verdict weareth the TURN XC prefix");
}
#[test]
fn an_unbroken_even_tale_falleth_to_london() {
    let mut g = GameState::new(3);
    g.turn = 99; // the ninety was already spoken; reach the hundred
    fortify(&mut g);
    assert_eq!(count(&g, PLAYER), count(&g, Faction::Norman), "the tale is even");
    g.end_player_turn(); // turn 100: still even - London decideth
    assert_eq!(g.over, Some(Faction::Norman), "London breaketh the second even tale");
    assert!(g
        .log
        .iter()
        .any(|l| l.contains("LONDON IS HIS") && l.contains("DEFEAT")));
}
