use crate::campaign::rng::Xor;
use crate::campaign::units::{KindCounts, Unit, UnitKind, ALL_KINDS, N_KINDS};

pub const MAX_ROUNDS: usize = 30;
pub const FULL_WALLS: u32 = 60;

#[derive(Clone, Debug)]
pub struct BattleOutcome {
    pub attacker_won: bool,
    /// Attrition defeat (round limit) rather than annihilation.
    pub repelled_by_walls: bool,
    pub rounds: usize,
    pub walls_broken_round: Option<usize>,
    /// Losses and survivors indexed by `UnitKind::index()`.
    pub atk_loss: KindCounts,
    pub def_loss: KindCounts,
    pub atk_survivors: Vec<Unit>,
    pub def_survivors: Vec<Unit>,
}

fn alive_count(units: &[Unit]) -> usize {
    units.iter().filter(|u| u.hp > 0).count()
}

/// Pick the k-th living unit (k < alive_count).
fn nth_alive(units: &mut [Unit], k: usize) -> Option<&mut Unit> {
    let mut seen = 0usize;
    for u in units.iter_mut() {
        if u.hp > 0 {
            if seen == k {
                return Some(u);
            }
            seen += 1;
        }
    }
    None
}

/// Round-based melee: the attacker storms the gates; while the walls stand,
/// attacker blows go against the walls (catapults x3, others at half damage),
/// defenders shoot back from the battlements. Fought to annihilation or the
/// round limit (limit = the attack is repelled).
pub fn simulate(attacker: &[Unit], defender: &[Unit], walls: u32, rng: &mut Xor) -> BattleOutcome {
    let mut atk: Vec<Unit> = attacker.to_vec();
    let mut def: Vec<Unit> = defender.to_vec();
    let atk_start = counts(&atk);
    let def_start = counts(&def);

    let mut walls_hp = walls;
    let mut walls_broken_round = None;
    let mut round = 0usize;
    let mut attacker_won = false;
    let mut repelled_by_walls = false;

    while round < MAX_ROUNDS {
        round += 1;

        // Attacker phase
        for a in atk.iter() {
            if a.hp == 0 {
                continue;
            }
            let dmg = a.kind.atk();
            let is_siege = a.kind == UnitKind::Siege;
            if walls_hp > 0 {
                let wall_dmg = if is_siege { dmg * 3 } else { (dmg / 2).max(1) };
                if wall_dmg >= walls_hp {
                    walls_hp = 0;
                    walls_broken_round = Some(round);
                } else {
                    walls_hp -= wall_dmg;
                    continue;
                }
            }
            let n = alive_count(&def);
            if n == 0 {
                break;
            }
            let k = rng.below(n as u32) as usize;
            if let Some(t) = nth_alive(&mut def, k) {
                t.hp = t.hp.saturating_sub(dmg);
            }
        }
        if alive_count(&def) == 0 {
            attacker_won = true;
            break;
        }

        // Defender phase
        for d in def.iter() {
            if d.hp == 0 {
                continue;
            }
            let dmg = d.kind.atk();
            let n = alive_count(&atk);
            if n == 0 {
                break;
            }
            let k = rng.below(n as u32) as usize;
            if let Some(t) = nth_alive(&mut atk, k) {
                t.hp = t.hp.saturating_sub(dmg);
            }
        }
        if alive_count(&atk) == 0 {
            break;
        }
    }

    if !attacker_won && alive_count(&atk) > 0 && round >= MAX_ROUNDS {
        repelled_by_walls = true;
    }

    let mut atk_survivors: Vec<Unit> = atk.into_iter().filter(|u| u.hp > 0).collect();
    let mut def_survivors: Vec<Unit> = def.into_iter().filter(|u| u.hp > 0).collect();
    // Stable order for reports and reproducibility.
    let ord = |u: &Unit| u.kind.index();
    atk_survivors.sort_by_key(ord);
    def_survivors.sort_by_key(ord);

    let atk_end = counts(&atk_survivors);
    let def_end = counts(&def_survivors);
    let atk_loss = std::array::from_fn(|i| atk_start[i] - atk_end[i]);
    let def_loss = std::array::from_fn(|i| def_start[i] - def_end[i]);

    BattleOutcome {
        attacker_won,
        repelled_by_walls,
        rounds: round,
        walls_broken_round,
        atk_loss,
        def_loss,
        atk_survivors,
        def_survivors,
    }
}

pub fn counts(units: &[Unit]) -> KindCounts {
    let mut c = [0u32; N_KINDS];
    for u in units {
        c[u.kind.index()] += 1;
    }
    c
}

/// "2 Knights, 1 Levies" — compact loss/garrison phrasing.
pub fn fmt_counts(c: &KindCounts) -> String {
    let parts: Vec<String> = ALL_KINDS
        .iter()
        .filter(|k| c[k.index()] > 0)
        .map(|k| format!("{} {}", c[k.index()], k.name()))
        .collect();
    if parts.is_empty() {
        "none".to_string()
    } else {
        parts.join(", ")
    }
}
