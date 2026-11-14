//! SPEC 5.5: the minigame economy. SPORTS costs 10g and pays by tier;
//! event tournaments are free, double, and a triple GOLD recruits a
//! CHAMPION (max three, SPEC 5.3). Rewards clamp at bar and garrison caps.

use defender_of_the_crown::campaign::game::{
    Faction, GameState, Reward, Tier, MAX_CHAMPIONS, PLAYER, SPORTS_FEE,
};
use defender_of_the_crown::campaign::units::{Unit, UnitKind, N_KINDS};

fn buck(g: &GameState) -> usize {
    g.homes[PLAYER.index()]
}

#[test]
fn tiers_cut_at_their_thresholds() {
    assert_eq!(Tier::of(9, 10, 50), Tier::Bronze);
    assert_eq!(Tier::of(10, 10, 50), Tier::Silver);
    assert_eq!(Tier::of(50, 10, 50), Tier::Gold);
}

#[test]
fn sports_cost_gold_and_pay_by_tier() {
    let mut g = GameState::new(2);
    let g0 = g.gi(PLAYER);
    let (tier, r) = g.play_sports(PLAYER, buck(&g), 0, 10, 50).unwrap();
    assert_eq!(tier, Tier::Bronze);
    assert_eq!(g.gi(PLAYER), g0 - SPORTS_FEE + r.gold);
    assert_eq!(r.troops[0], 2, "bronze brings levies");
    let (_, r2) = g.play_sports(PLAYER, buck(&g), 50, 10, 50).unwrap();
    assert_eq!(r2.troops[2], 1, "gold brings a knight");
}

#[test]
fn sports_deny_strangers_and_beggars() {
    let mut g = GameState::new(2);
    let norman = g.homes[Faction::Norman.index()];
    let g0 = g.gi(PLAYER);
    assert!(g.play_sports(PLAYER, norman, 99, 10, 50).is_err());
    assert_eq!(g.gi(PLAYER), g0, "refused play is free");
    let mut poor = GameState::new(2);
    poor.gold[0] = SPORTS_FEE - 1;
    assert!(poor.play_sports(PLAYER, buck(&poor), 99, 10, 50).is_err());
}

#[test]
fn popularity_thickens_the_purse() {
    let mut plain = GameState::new(2);
    plain.bars[0].popularity = 0;
    let mut famed = GameState::new(2);
    famed.bars[0].popularity = 100;
    let b = buck(&plain);
    let (_, r) = plain.play_sports(PLAYER, b, 0, 10, 50).unwrap();
    let (_, r2) = famed.play_sports(PLAYER, b, 0, 10, 50).unwrap();
    assert_eq!(r2.gold, r.gold + 10, "+pop/10 gold");
}

#[test]
fn rewards_respect_the_provisions_cap() {
    let mut g = GameState::new(2);
    let cap = g.provisions_cap(PLAYER);
    g.bars[0].provisions = cap;
    let (_, r) = g.play_sports(PLAYER, buck(&g), 50, 10, 50).unwrap();
    assert!(r.provisions > 0);
    assert_eq!(
        g.bars_of(PLAYER).provisions,
        cap,
        "the barn is already full"
    );
}

#[test]
fn troops_march_into_an_empty_barracks_only() {
    let mut g = GameState::new(2);
    let b = buck(&g);
    g.strongholds[b].garrison.clear();
    for _ in 0..38 {
        g.strongholds[b].garrison.push(Unit::new(UnitKind::Levy));
    }
    let (_, r) = g.play_sports(PLAYER, b, 0, 10, 50).unwrap();
    assert_eq!(r.troops[0], 2, "the purse promises two levies");
    assert_eq!(g.strongholds[b].garrison.len(), 40, "garrison caps at 40");
}

#[test]
fn tournaments_are_free_and_double() {
    let mut g = GameState::new(2);
    let b = buck(&g);
    let (_, plain) = g.play_sports(PLAYER, b, 99, 10, 50).unwrap();
    g.tournament[0] = Some(b);
    let g0 = g.gi(PLAYER);
    let (tier, r) = g.play_tournament(PLAYER, 99, 10, 50).unwrap();
    assert_eq!(tier, Tier::Gold);
    assert_eq!(r.gold, plain.gold * 2);
    assert_eq!(r.provisions, plain.provisions * 2);
    assert_eq!(g.gi(PLAYER), g0 + r.gold, "no fee at a herald's castle");
    assert!(
        g.play_tournament(PLAYER, 99, 10, 50).is_err(),
        "herald spent"
    );
}

#[test]
fn a_triple_goldcrown_recruiteth_a_champion() {
    let mut g = GameState::new(2);
    let b = buck(&g);
    for i in 1..=3 {
        g.tournament[0] = Some(b);
        let (_, r) = g.play_tournament(PLAYER, 99, 10, 50).unwrap();
        assert_eq!(g.golds[0], if i == 3 { 0 } else { i }, "GOLDs banked");
        assert_eq!(r.champion, i == 3, "the champion arrives on the third");
    }
    assert_eq!(g.count_champions(PLAYER), 1);
    assert!(g.log.iter().any(|l| l.contains("TRIPLE CROWN")));
}

#[test]
fn a_silver_crown_winneth_no_champion() {
    let mut g = GameState::new(2);
    let b = buck(&g);
    for _ in 0..3 {
        g.tournament[0] = Some(b);
        g.play_tournament(PLAYER, 10, 10, 50).unwrap();
    }
    assert_eq!(g.golds[0], 0, "no bank, no crown");
    assert_eq!(g.count_champions(PLAYER), 0);
}

#[test]
fn the_round_table_seateth_three_at_most() {
    let mut g = GameState::new(2);
    let b = buck(&g);
    for i in 1..=3 {
        g.golds[0] = 2;
        g.tournament[0] = Some(b);
        let (_, r) = g.play_tournament(PLAYER, 99, 10, 50).unwrap();
        assert!(r.champion, "crown {i} fills a seat");
    }
    assert_eq!(g.count_champions(PLAYER), MAX_CHAMPIONS);
    g.golds[0] = 2;
    g.tournament[0] = Some(b);
    let (_, r) = g.play_tournament(PLAYER, 99, 10, 50).unwrap();
    assert!(!r.champion, "no fourth champion");
    assert!(r.gold >= 200, "the full table pays out in gold: {r:?}");
    assert_eq!(g.count_champions(PLAYER), MAX_CHAMPIONS);
}

#[test]
fn the_ledger_of_a_reward_is_the_purse_itself() {
    let mut g = GameState::new(2);
    let b = buck(&g);
    let l0 = g.bars_of(PLAYER).loyalty;
    let p0 = g.bars_of(PLAYER).popularity;
    let (_, r) = g.play_sports(PLAYER, b, 10, 10, 50).unwrap();
    assert_eq!(g.bars_of(PLAYER).loyalty, l0 + r.loyalty as u32);
    assert_eq!(g.bars_of(PLAYER).popularity, p0 + r.popularity as u32);
    let owned: usize = g.strongholds[b]
        .garrison
        .iter()
        .filter(|u| u.kind == UnitKind::Bowman)
        .count();
    assert!(owned >= r.troops[1] as usize);
    assert_eq!(r.troops.len(), N_KINDS);
    let _ = Reward::default();
}
