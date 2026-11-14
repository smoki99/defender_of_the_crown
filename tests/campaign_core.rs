//! T201 data-layer tests: castle graph, unit definitions, event table.

use defender_of_the_crown::campaign::events::{EventKind, ALL_EVENTS};
use defender_of_the_crown::campaign::game::{GameState, PLAYER};
use defender_of_the_crown::campaign::map;
use defender_of_the_crown::campaign::rng::Xor;
use defender_of_the_crown::campaign::units::{UnitKind, ALL_KINDS, N_KINDS};

#[test]
fn graph_is_symmetric_and_connected() {
    let defs = map::defs();
    assert_eq!(defs.len(), 12);
    for (i, d) in defs.iter().enumerate() {
        assert!(!d.adj.contains(&i), "{i} has a self-loop");
        for &j in d.adj {
            assert!(j < defs.len(), "dangling edge {i}->{j}");
            assert!(defs[j].adj.contains(&i), "asymmetric edge {i}->{j}");
        }
    }
    // BFS connectivity
    let mut seen = vec![false; defs.len()];
    let mut queue = vec![0usize];
    seen[0] = true;
    let mut n = 1;
    while let Some(i) = queue.pop() {
        for &j in defs[i].adj {
            if !seen[j] {
                seen[j] = true;
                n += 1;
                queue.push(j);
            }
        }
    }
    assert_eq!(n, 12, "graph not connected");
}

#[test]
fn each_side_has_exactly_one_capital_and_six_castles() {
    let defs = map::defs();
    let saxon: Vec<_> = defs.iter().filter(|d| d.is_saxon).collect();
    let norman: Vec<_> = defs.iter().filter(|d| !d.is_saxon).collect();
    assert_eq!(saxon.len(), 6);
    assert_eq!(norman.len(), 6);
    assert_eq!(defs.iter().filter(|d| d.is_saxon && d.capital).count(), 1);
    assert_eq!(defs.iter().filter(|d| !d.is_saxon && d.capital).count(), 1);
}

#[test]
fn roads_are_the_deduped_edge_set() {
    let roads = map::roads();
    let edges: usize = map::defs().iter().map(|d| d.adj.len()).sum();
    assert_eq!(roads.len(), edges / 2);
    for (a, b) in &roads {
        assert!(a < b);
    }
    let mut sorted = roads.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), roads.len(), "duplicate road edge");
}

#[test]
fn coastal_raid_axis_and_port_exist() {
    let defs = map::defs();
    let coast: Vec<&str> = defs.iter().filter(|d| d.coast).map(|d| d.name).collect();
    assert_eq!(coast, vec!["Wallingford", "London", "Dover"]);
    assert_eq!(
        defs.iter().filter(|d| d.port).map(|d| d.name).collect::<Vec<_>>(),
        vec!["Dover"]
    );
}

#[test]
fn five_unit_kinds_with_unique_indexes() {
    assert_eq!(ALL_KINDS.len(), N_KINDS);
    let mut idx = [usize::MAX; N_KINDS];
    for k in ALL_KINDS {
        let i = k.index();
        assert!(i < N_KINDS);
        assert_eq!(idx[i], usize::MAX, "duplicate index for {k:?}");
        idx[i] = i;
        assert_eq!(UnitKind::from_index(i), Some(k));
        assert!(k.hp() > 0 && k.atk() > 0);
    }
    assert_eq!(UnitKind::Champion.index(), 4);
    assert!(UnitKind::Champion.special());
    assert!(!UnitKind::Levy.special());
}

#[test]
fn champions_cannot_be_bought() {
    let mut g = GameState::new(1);
    let buck = 0usize; // player capital
    assert_eq!(g.recruit(PLAYER, buck, UnitKind::Champion).unwrap_err(), "CHAMPIONS ARE EARNED, NOT BOUGHT.");
    assert_eq!(g.gi(PLAYER), 70, "rejected recruit must not spend gold");
}

#[test]
fn event_weights_positive_and_danes_gated() {
    assert_eq!(ALL_EVENTS.len(), 7);
    for &e in &ALL_EVENTS {
        assert!(e.weight(12) > 0);
    }
    assert_eq!(EventKind::DanishRaiders.weight(11), 0);
    assert!(EventKind::DanishRaiders.weight(12) > 0);
}

#[test]
fn event_roll_is_deterministic_and_turn_gated() {
    let a = EventKind::roll(&mut Xor::new(42), 3);
    let b = EventKind::roll(&mut Xor::new(42), 3);
    assert_eq!(a, b);
    // No Danish raids before turn 12 across many seeds.
    for seed in 0..500 {
        let e = EventKind::roll(&mut Xor::new(seed + 1), 5);
        assert_ne!(e, EventKind::DanishRaiders, "seed {seed} raided too early");
    }
    // And they do appear from turn 12 on.
    let mut hits = 0;
    for seed in 0..500 {
        if EventKind::roll(&mut Xor::new(seed + 1), 20) == EventKind::DanishRaiders {
            hits += 1;
        }
    }
    assert!(hits > 0, "Danes never raid");
}
