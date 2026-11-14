use crate::campaign::combat::FULL_WALLS;
use crate::campaign::game::{Difficulty, Faction, GameState, View, PLAYER};
use crate::campaign::units::UnitKind;

/// Weight of one soldier of each kind for the council's back-of-hand math.
/// Must mirror `Unit::power` = hp * atk, or the council misreads every odds.
fn kind_power(k: UnitKind) -> f32 {
    (k.hp() * k.atk()) as f32
}
/// A DARK keep is assumed at full strength until a spy says otherwise.
const DARK_PRIOR: f32 = 700.0;

/// The Norman war council (and any automated seat): feed the realm, spend
/// gold, strike on estimated odds, hand the fog to the spies. What it DARES
/// and what it KNOWS follow `g.difficulty` (SPEC 6.1).
pub fn take_turn(g: &mut GameState, f: Faction) {
    let diff = g.difficulty[f.index()];
    feed(g, f, diff);
    recruit_phase(g, f, diff);
    let every = diff.spy_every();
    if every > 0 && g.turn.is_multiple_of(every) {
        spy_phase(g, f);
    }
    attack_phase(g, f, diff);
    if diff.reinforces() {
        reinforce_phase(g, f);
    }
}

fn is_frontier(g: &GameState, sid: usize, f: Faction) -> bool {
    g.adj(sid).iter().any(|&t| g.strongholds[t].owner != f)
}

/// Castles whose walls see an enemy banner — neutral keeps do not count,
/// so a host behind the marches can still be sent to the real front.
fn touches_enemy(g: &GameState, sid: usize, f: Faction) -> bool {
    g.adj(sid)
        .iter()
        .any(|&t| g.strongholds[t].owner == f.other())
}

/// Market bread before walls (SPEC 6.1: the AI buys too). Surplus gold past
/// the difficulty's reserve goes into the barn, capped by the barn itself.
fn feed(g: &mut GameState, f: Faction, diff: Difficulty) {
    let cap = g.provisions_cap(f);
    let prov = g.bars_of(f).provisions.min(cap);
    // The army eats first; only then comes a buffer (SPEC 4: hunger
    // sinks loyalty, and loyalty sinks income).
    let need = (g.troops_of(f) as u32 + 8 + diff.food_reserve()).min(cap);
    if prov >= need {
        return;
    }
    let want = (need - prov).min(g.gi(f) / 2);
    let _ = g.buy_provisions(f, want);
}

fn recruit_phase(g: &mut GameState, f: Faction, diff: Difficulty) {
    let tax = diff.recruit_tax();
    let limit = g.provisions_cap(f) * diff.army_pct() / 100;
    loop {
        // Only soldiers the yearly income can feed AND pay are levied
        // (SPEC 4: an army you cannot feed melts).
        let troops = g.troops_of(f) as u32;
        if troops >= limit || troops * 3 >= g.income_of(f) {
            break;
        }
        // Never spend the army's bread or the spy purse on new men.
        if g.gi(f) < troops * 2 + 30 {
            break;
        }
        let mut bought = false;
        for sid in g.ids_of(f) {
            let c = g.counts(sid);
            let frontier = is_frontier(g, sid, f);
            let gold = g.gi(f);
            let kind = if gold >= 60 + tax && c[UnitKind::Siege.index()] == 0 && frontier {
                UnitKind::Siege
            } else if gold >= 45 + tax && frontier && c[UnitKind::Knight.index()] < 4 {
                UnitKind::Knight
            } else if gold >= 20 + tax && c[UnitKind::Bowman.index()] < 8 {
                UnitKind::Bowman
            } else if gold >= 10 + tax {
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

/// One covert push per turn: the frontier keep whose defences are least
/// certain (DARK or only a rough sighting), if affordable.
fn spy_phase(g: &mut GameState, f: Faction) {
    if g.gi(f) < 25 + g.troops_of(f) as u32 * 2 {
        return;
    }
    let target = g
        .ids_of(f)
        .iter()
        .flat_map(|&s| g.adj(s).into_iter())
        .filter(|&t| g.strongholds[t].owner != f && !matches!(g.view(f, t), View::Spied(_)))
        .max_by_key(|&t| (est_defense(g, f, t) as u32, g.strongholds[t].id));
    if let Some(t) = target {
        let _ = g.spy(f, t);
    }
}

/// What the council thinks defends `t`: a fresh spy report counts walls,
/// a darkness is a shrug.
fn est_defense(g: &GameState, f: Faction, t: usize) -> f32 {
    match g.view(f, t) {
        View::Spied(rep) => {
            let men: f32 = rep
                .counts
                .iter()
                .zip([
                    UnitKind::Levy,
                    UnitKind::Bowman,
                    UnitKind::Knight,
                    UnitKind::Siege,
                    UnitKind::Champion,
                ])
                .map(|(&n, k)| n as f32 * kind_power(k))
                .sum();
            men + rep.walls as f32
        }
        // A "SEEN: ~N" sighting can hide up to double the truth (SPEC 5.3
        // +-50%), battlements included - plan against the worst reading.
        View::Rough { total } => {
            total as f32 * 2.0 * kind_power(UnitKind::Levy) + FULL_WALLS as f32
        }
        View::Dark => DARK_PRIOR,
        View::Own(_) => 0.0,
    }
}

fn attack_phase(g: &mut GameState, f: Faction, diff: Difficulty) {
    let mut tried: Vec<(usize, usize)> = Vec::new();
    loop {
        // Pick ONE strike per pass and re-read the world after each march:
        // the first repulse thins the target, and stale odds are graveyards.
        let mut best: Option<(f32, usize, usize)> = None;
        for &s in &g.ids_of(f) {
            if g.strongholds[s].moved || g.strongholds[s].garrison.is_empty() {
                continue;
            }
            let p = g.army_power(s) as f32;
            let with_siege = g.counts(s)[UnitKind::Siege.index()] > 0;
            for t in g.adj(s) {
                if g.strongholds[t].owner != f && !tried.contains(&(s, t)) {
                    // Walls eat hosts that bring no catapults (SPEC 5.3):
                    // never storm a garrisoned keep without siege trains.
                    let mass = match g.view(f, t) {
                        View::Spied(rep) => rep.counts.iter().sum::<u32>(),
                        View::Rough { total } => total * 2,
                        View::Dark => u32::MAX,
                        View::Own(_) => 0,
                    };
                    if !with_siege && mass >= 6 {
                        continue;
                    }
                    let ratio = p / (est_defense(g, f, t) + 1.0);
                    let req = match g.view(f, t) {
                        View::Spied(_) => diff.confident_ratio(),
                        _ => diff.min_ratio(),
                    };
                    if ratio > req && best.is_none_or(|(b, _, _)| ratio > b) {
                        best = Some((ratio, s, t));
                    }
                }
            }
        }
        let Some((_, s, t)) = best else { break };
        if g.march(f, s, t).is_err() {
            // e.g. an empty purse grounds the host: never ask twice a turn.
            tried.push((s, t));
        }
    }
}

/// How many friendly marches an army at `from` needs to reach a castle that
/// faces the enemy; 0 if it already does, `u32::MAX` if unreachable.
fn front_dist(g: &GameState, f: Faction, from: usize) -> u32 {
    if touches_enemy(g, from, f) {
        return 0;
    }
    let mut frontier = vec![from];
    let mut seen = vec![false; g.strongholds.len()];
    seen[from] = true;
    for d in 1u32.. {
        let mut next = Vec::new();
        for &s in &frontier {
            for t in g.adj(s) {
                if seen[t] || g.strongholds[t].owner != f {
                    continue;
                }
                if touches_enemy(g, t, f) {
                    return d;
                }
                seen[t] = true;
                next.push(t);
            }
        }
        if next.is_empty() {
            return u32::MAX;
        }
        frontier = next;
    }
    u32::MAX
}

fn reinforce_phase(g: &mut GameState, f: Faction) {
    for &s in &g.ids_of(f) {
        let alive = g.strongholds[s].garrison.len() as u32;
        let ds = front_dist(g, f, s);
        if g.strongholds[s].moved || alive < 8 || ds == 0 || ds == u32::MAX {
            continue;
        }
        // Mass on the marches: every host steps one hop strictly nearer the
        // front, piling onto the thinnest castle on that road.
        let best = g
            .adj(s)
            .into_iter()
            .filter(|&t| {
                g.strongholds[t].owner == f && !g.strongholds[t].moved && front_dist(g, f, t) < ds
            })
            .min_by_key(|&t| (front_dist(g, f, t), g.army_power(t)));
        if let Some(t) = best {
            let _ = g.march_part(f, s, t, (alive - 4) as usize);
        }
    }
}

/// Used by tests: let the AI run a faction end-to-end inside a turn.
pub fn play_full_turn(g: &mut GameState, f: Faction) {
    if f == PLAYER {
        take_turn(g, f);
        g.end_player_turn();
    } else {
        take_turn(g, f);
    }
}
