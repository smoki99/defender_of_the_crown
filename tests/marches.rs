//! SPEC 5.3 marches: one march per castle per turn, whole garrisons only,
//! no two-leg days, captured castles already spent their day - and dawn
//! (end_player_turn) resets every flag.

use defender_of_the_crown::campaign::game::{Faction, GameState, PLAYER};

fn own_with_garrison(g: &GameState, min: usize) -> usize {
    (0..g.strongholds.len())
        .find(|&i| g.strongholds[i].owner == PLAYER && g.strongholds[i].garrison.len() >= min)
        .unwrap()
}

fn own_neighbour(g: &GameState, sid: usize) -> usize {
    g.adj(sid)
        .iter()
        .copied()
        .find(|&a| g.strongholds[a].owner == PLAYER)
        .unwrap()
}

#[test]
fn a_march_moves_the_whole_garrison_and_spends_the_day() {
    let mut g = GameState::new(7);
    let from = own_with_garrison(&g, 3);
    let to = own_neighbour(&g, from);
    let (a, b) = (
        g.strongholds[from].garrison.len(),
        g.strongholds[to].garrison.len(),
    );
    let turn = g.turn;
    assert!(g.march(PLAYER, from, to).is_ok());
    assert!(g.strongholds[from].garrison.is_empty());
    assert_eq!(g.strongholds[to].garrison.len(), a + b);
    assert!(g.strongholds[from].moved, "the marching castle is spent");
    assert!(g.strongholds[to].moved, "no two-leg day for the receiver");
    assert_eq!(g.turn, turn, "a march is not an end-of-turn");
}

#[test]
fn a_spent_castle_cannot_march_again_same_turn() {
    let mut g = GameState::new(7);
    let from = own_with_garrison(&g, 3);
    let to = own_neighbour(&g, from);
    assert!(g.march(PLAYER, from, to).is_ok());
    let elsewhere = g
        .adj(from)
        .iter()
        .copied()
        .find(|&a| a != to && g.strongholds[a].owner == PLAYER);
    if let Some(other) = elsewhere {
        assert!(g.march(PLAYER, from, other).is_err(), "the day is spent");
    }
    assert!(g.march(PLAYER, to, own_neighbour(&g, to)).is_err());
}

#[test]
fn no_two_leg_day_for_the_arriving_army() {
    let mut g = GameState::new(7);
    let a = own_with_garrison(&g, 3);
    let b = own_neighbour(&g, a);
    assert!(g.march(PLAYER, a, b).is_ok()); // reinforcements arrive
    if let Some(c) = g
        .adj(b)
        .iter()
        .copied()
        .find(|&x| x != a && g.strongholds[x].owner == PLAYER)
    {
        assert!(
            g.march(PLAYER, b, c).is_err(),
            "armies march one road a day"
        );
    } else {
        panic!("{b} needs a second own neighbour");
    }
}

#[test]
fn dawn_resets_every_march_flag() {
    let mut g = GameState::new(7);
    let from = own_with_garrison(&g, 3);
    let to = own_neighbour(&g, from);
    assert!(g.march(PLAYER, from, to).is_ok());
    g.end_player_turn();
    assert!(
        g.strongholds.iter().all(|s| !s.moved),
        "a new day, a new march"
    );
}

#[test]
fn the_same_castle_marches_again_next_dawn() {
    let mut g = GameState::new(7);
    let a = own_with_garrison(&g, 3);
    let b = own_neighbour(&g, a);
    assert!(g.march(PLAYER, a, b).is_ok());
    g.end_player_turn();
    let c = g
        .adj(b)
        .iter()
        .copied()
        .find(|&x| x != a && g.strongholds[x].owner == PLAYER);
    if let Some(c) = c {
        assert!(g.march(PLAYER, b, c).is_ok(), "the army is rested and free");
    }
}

#[test]
fn a_capturing_march_leaves_the_winner_spent() {
    let mut g = GameState::new(7);
    let from = own_with_garrison(&g, 4);
    let target = g.adj(from)[0];
    // turn it into an undefended rebel hold
    g.strongholds[target].owner = Faction::Neutral;
    g.strongholds[target].garrison.clear();
    g.strongholds[target].walls = 0;
    assert!(g.march(PLAYER, from, target).is_ok());
    assert_eq!(g.strongholds[target].owner, PLAYER);
    assert!(
        g.strongholds[target].moved,
        "the victors already fought their way in"
    );
    assert!(g.march(PLAYER, target, from).is_err());
}

#[test]
fn march_gates_refuse_illegal_moves() {
    let mut g = GameState::new(7);
    let a = own_with_garrison(&g, 2);
    let far = (0..g.strongholds.len())
        .find(|&i| i != a && !g.adj(a).contains(&i) && g.strongholds[i].owner == PLAYER)
        .expect("a non-adjacent own castle");
    assert!(g.march(PLAYER, a, far).is_err(), "roads only");
    let b = own_neighbour(&g, a);
    g.strongholds[a].garrison.clear();
    assert!(g.march(PLAYER, a, b).is_err(), "no empty marches");
    let c = g
        .adj(b)
        .iter()
        .copied()
        .find(|&i| {
            i != a && g.strongholds[i].owner == PLAYER && g.strongholds[i].garrison.len() >= 2
        })
        .expect("a garrisoned own neighbour of the rebel hold");
    g.strongholds[b].owner = Faction::Neutral;
    g.strongholds[b].garrison.clear();
    g.strongholds[b].walls = 0;
    assert!(g.march(PLAYER, c, b).is_ok(), "rebel holds may be seized");
    g.over = Some(Faction::Norman);
    assert!(
        g.march(PLAYER, c, own_neighbour(&g, c)).is_err(),
        "the game is done"
    );
}
