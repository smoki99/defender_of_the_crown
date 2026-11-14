use crate::campaign::game::{GameState, Faction, PLAYER};
use crate::campaign::units::UnitKind;

/// The Norman war council, and also usable as an automated Saxon side in
/// tests: spend gold, attack where clearly stronger, feed the fronts.
pub fn take_turn(g: &mut GameState, f: Faction) {
    recruit_phase(g, f);
    attack_phase(g, f);
    reinforce_phase(g, f);
}

fn is_frontier(g: &GameState, sid: usize, f: Faction) -> bool {
    g.adj(sid).iter().any(|&t| g.strongholds[t].owner != f)
}

fn recruit_phase(g: &mut GameState, f: Faction) {
    loop {
        let mut bought = false;
        for sid in g.ids_of(f) {
            let c = g.counts(sid);
            let frontier = is_frontier(g, sid, f);
            let kind = if g.gi(f) >= 60 && c[UnitKind::Siege.index()] == 0 && frontier {
                UnitKind::Siege
            } else if g.gi(f) >= 45 && frontier && c[UnitKind::Knight.index()] < 4 {
                UnitKind::Knight
            } else if g.gi(f) >= 20 && c[UnitKind::Bowman.index()] < 8 {
                UnitKind::Bowman
            } else if g.gi(f) >= 10 {
                UnitKind::Levy
            } else {
                continue;
            };
            if g.recruit(f, sid, kind).is_ok() {
                bought = true;
            }
        }
        if !bought {
            break;
        }
    }
}

fn attack_phase(g: &mut GameState, f: Faction) {
    let mut attacks: Vec<(f32, usize, usize)> = Vec::new();
    for &s in &g.ids_of(f) {
        if g.strongholds[s].moved || g.strongholds[s].garrison.is_empty() {
            continue;
        }
        let p = g.army_power(s) as f32;
        for t in g.adj(s) {
            if g.strongholds[t].owner != f {
                let ratio = p / (g.defense_power(t) as f32 + 1.0);
                if ratio > 1.45 {
                    attacks.push((ratio, s, t));
                }
            }
        }
    }
    attacks.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    for (_, s, t) in attacks {
        let _ = g.march(f, s, t);
    }
}

fn reinforce_phase(g: &mut GameState, f: Faction) {
    for &s in &g.ids_of(f) {
        let alive = g.strongholds[s].garrison.len() as u32;
        if g.strongholds[s].moved || alive < 12 || is_frontier(g, s, f) {
            continue;
        }
        let best = g
            .adj(s)
            .into_iter()
            .filter(|&t| {
                g.strongholds[t].owner == f
                    && is_frontier(g, t, f)
                    && !g.strongholds[t].moved
                    && g.army_power(t) < g.army_power(s) / 2
            })
            .max_by_key(|&t| g.army_power(t));
        if let Some(t) = best {
            let _ = g.march(f, s, t);
        }
    }
}

/// Used by tests: let the AI run a faction end-to-end inside a turn.
pub fn play_full_turn(g: &mut GameState, f: Faction) {
    if f == PLAYER {
        recruit_phase(g, f);
        attack_phase(g, f);
        reinforce_phase(g, f);
        g.end_player_turn();
    } else {
        take_turn(g, f);
    }
}
