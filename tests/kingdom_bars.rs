//! T202 proofs: kingdom bars - income scaling, popularity discount,
//! provisions upkeep with hunger, loyalty-zero revolts, clamping.
use defender_of_the_crown::campaign::game::{starting_bars, Faction, GameState, BAR_MAX, PLAYER};
use defender_of_the_crown::campaign::units::{Unit, UnitKind};

fn garrison_of(g: &GameState, sid: usize) -> usize {
    g.strongholds[sid].garrison.len()
}

#[test]
fn loyalty_scales_income() {
    let g = GameState::new(1337);
    let raw: u32 = g
        .strongholds
        .iter()
        .filter(|s| s.owner == PLAYER)
        .map(|s| s.income)
        .sum();
    let mut lo = g.clone();
    lo.bars[0].loyalty = 0;
    assert_eq!(lo.income_of(PLAYER), raw / 2);
    let mut hi = g.clone();
    hi.bars[0].loyalty = BAR_MAX;
    assert_eq!(hi.income_of(PLAYER), raw * 3 / 2);
    // starting loyalty 70 => +20%
    assert_eq!(g.income_of(PLAYER), raw * 120 / 100);
}

#[test]
fn bars_start_and_clamp() {
    let mut g = GameState::new(1337);
    assert_eq!(g.bars_of(PLAYER), starting_bars());
    g.add_loyalty(PLAYER, 1000);
    assert_eq!(g.bars_of(PLAYER).loyalty, BAR_MAX);
    g.add_loyalty(PLAYER, -1000);
    assert_eq!(g.bars_of(PLAYER).loyalty, 0);
    g.add_popularity(PLAYER, 1000);
    assert_eq!(g.bars_of(PLAYER).popularity, BAR_MAX);
    g.add_provisions(PLAYER, 9999);
    assert_eq!(g.bars_of(PLAYER).provisions, g.provisions_cap(PLAYER));
    g.add_provisions(PLAYER, -9999);
    assert_eq!(g.bars_of(PLAYER).provisions, 0);
}

#[test]
fn popularity_discounts_recruits() {
    let mut g = GameState::new(1337);
    let sid = g.ids_of(PLAYER)[1];
    g.gold[0] = 500;
    // pop 0: full price
    g.bars[0].popularity = 0;
    assert_eq!(g.effective_cost(PLAYER, UnitKind::Levy), UnitKind::Levy.cost());
    // pop max: exactly half, and the charge is the discounted price
    g.bars[0].popularity = BAR_MAX;
    let half = UnitKind::Levy.cost() / 2;
    assert_eq!(g.effective_cost(PLAYER, UnitKind::Levy), half);
    let before = g.gi(PLAYER);
    let men = garrison_of(&g, sid);
    g.recruit(PLAYER, sid, UnitKind::Levy).unwrap();
    assert_eq!(before - g.gi(PLAYER), half);
    assert_eq!(garrison_of(&g, sid), men + 1);
}

#[test]
fn market_sells_provisions_up_to_cap() {
    let mut g = GameState::new(1337);
    g.bars[0].provisions = 0;
    g.gold[0] = 500;
    let cap = g.provisions_cap(PLAYER);
    let before = g.gi(PLAYER);
    g.buy_provisions(PLAYER, 1000).unwrap();
    assert_eq!(g.bars_of(PLAYER).provisions, cap);
    assert_eq!(before - g.gi(PLAYER), 2 * cap);
}

#[test]
fn hungry_armies_desert_and_loyalty_falls() {
    let mut g = GameState::new(1337);
    g.bars[0].provisions = 0;
    g.bars[0].loyalty = 50;
    g.bars[1].provisions = 999; // keep the Normans fed for a clean read
    let before = g.troops_of(PLAYER);
    g.end_player_turn();
    assert!(g
        .log
        .iter()
        .any(|l| l.contains("HUNGER TAKES")),
        "no desertion log: {:?}",
        g.log);
    assert!(g.troops_of(PLAYER) <= before);
    assert_eq!(g.bars_of(PLAYER).provisions, 0);
    assert!(g.bars_of(PLAYER).loyalty < 50);
}

#[test]
fn zero_loyalty_castle_revolts_to_neutral() {
    let mut g = GameState::new(1337);
    g.bars[0].loyalty = 0;
    g.bars[0].provisions = 999; // isolate the revolt from hunger
    g.bars[1].provisions = 999;
    g.bars[1].loyalty = 50;
    g.end_player_turn();
    assert!(g.log.iter().any(|l| l.contains("REVOLTS")));
    let rebels: Vec<usize> = (0..g.strongholds.len())
        .filter(|&i| g.strongholds[i].owner == Faction::Neutral)
        .collect();
    assert_eq!(rebels.len(), 1);
    let rid = rebels[0];
    assert_eq!(garrison_of(&g, rid), 0);
    // the rebels harvest for nobody: excluded from both payrolls
    assert!(g.strongholds[rid].income > 0);
    let raw_s: u32 = g
        .strongholds
        .iter()
        .filter(|s| s.owner == PLAYER)
        .map(|s| s.income)
        .sum();
    let raw_n: u32 = g
        .strongholds
        .iter()
        .filter(|s| s.owner == Faction::Norman)
        .map(|s| s.income)
        .sum();
    assert_eq!(g.income_of(PLAYER), raw_s / 2);
    assert_eq!(
        g.income_of(Faction::Norman),
        raw_n * (50 + g.bars_of(Faction::Norman).loyalty) / 100
    );
}

#[test]
fn rebel_castles_are_claimable_without_a_fight() {
    let mut g = GameState::new(7);
    let home = 0usize; // Buckingham, capital, garrisoned
    let target = g.adj(home)[0];
    g.strongholds[target].owner = Faction::Neutral;
    g.strongholds[target].garrison.clear();
    let pop = g.bars_of(PLAYER).popularity;
    g.march(PLAYER, home, target).unwrap();
    assert_eq!(g.strongholds[target].owner, PLAYER);
    assert_eq!(g.bars_of(PLAYER).popularity, pop + 2);
    assert!(g.log.iter().any(|l| l.contains("IS YOURS")));
}

#[test]
fn failed_attack_sours_loyalty() {
    let mut g = GameState::new(7);
    let home = 0usize;
    let target = g.adj(home)[0];
    // wall the target up with Norman knights; send two levies
    g.strongholds[target].owner = Faction::Norman;
    g.strongholds[target].garrison.clear();
    for _ in 0..32 {
        g.strongholds[target].garrison.push(Unit::new(UnitKind::Knight));
    }
    g.strongholds[home].garrison.clear();
    g.strongholds[home].garrison.push(Unit::new(UnitKind::Levy));
    g.strongholds[home].garrison.push(Unit::new(UnitKind::Levy));
    let loy = g.bars_of(PLAYER).loyalty;
    let _ = g.march(PLAYER, home, target);
    assert_eq!(g.strongholds[target].owner, Faction::Norman);
    assert_eq!(g.bars_of(PLAYER).loyalty, loy - 3);
    assert!(g.log.iter().any(|l| l.contains("REPULSED")));
}

#[test]
fn neutral_slot_never_acts_or_earns() {
    let g = GameState::new(1337);
    assert_eq!(g.income_of(Faction::Neutral), 0);
    assert_eq!(g.ids_of(Faction::Neutral).len(), 0);
    assert_eq!(g.gi(Faction::Neutral), 0);
    let mut h = g.clone();
    assert!(h.march(Faction::Neutral, 0, h.adj(0)[0]).is_err());
    assert!(h.recruit(Faction::Neutral, 0, UnitKind::Levy).is_err());
}

#[test]
fn bar_rules_stay_deterministic() {
    let run = || {
        let mut g = GameState::new(20261114);
        for t in 0..12 {
            let owned = g.ids_of(PLAYER);
            if t % 3 == 0 {
                let _ = g.recruit(PLAYER, owned[t % owned.len()], UnitKind::Bowman);
            }
            if t % 4 == 0 {
                let _ = g.buy_provisions(PLAYER, 10);
            }
            g.end_player_turn();
        }
        (g.turn, g.gold, g.bars, g.log.clone())
    };
    assert_eq!(run(), run());
}
