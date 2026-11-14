//! SPEC 5.3 combat v2: battlement favour, catapult sieges, the champion's
//! +25%, the duel that turns a lost day, and armies that retreat when
//! repelled instead of evaporating.

use defender_of_the_crown::campaign::combat::{self, FULL_WALLS};
use defender_of_the_crown::campaign::game::{Faction, GameState, PLAYER};
use defender_of_the_crown::campaign::rng::Xor;
use defender_of_the_crown::campaign::units::{KindCounts, Unit, UnitKind};

fn u(kind: UnitKind, n: usize) -> Vec<Unit> {
    vec![Unit::new(kind); n]
}

fn sum(c: &KindCounts) -> u32 {
    c.iter().sum()
}

#[test]
fn battlements_favour_the_defender() {
    let with = {
        let mut r = Xor::new(3);
        combat::simulate(
            &u(UnitKind::Levy, 12),
            &u(UnitKind::Bowman, 5),
            FULL_WALLS,
            &mut r,
        )
    };
    let naked = {
        let mut r = Xor::new(3);
        combat::simulate(&u(UnitKind::Levy, 12), &u(UnitKind::Bowman, 5), 0, &mut r)
    };
    // +50% from the battlements: more attacker dead, none of the defenders.
    assert!(
        sum(&with.atk_loss) > sum(&naked.atk_loss),
        "walls: atk loss {} vs {}",
        sum(&with.atk_loss),
        sum(&naked.atk_loss)
    );
    assert!(sum(&with.def_loss) < sum(&naked.def_loss));
}

#[test]
fn catapults_breach_the_walls_fastest() {
    let siege = {
        let mut r = Xor::new(9);
        let a = [u(UnitKind::Siege, 2), u(UnitKind::Knight, 3)].concat();
        combat::simulate(&a, &u(UnitKind::Bowman, 2), FULL_WALLS, &mut r)
    };
    let cavalry = {
        let mut r = Xor::new(9);
        combat::simulate(
            &u(UnitKind::Knight, 4),
            &u(UnitKind::Bowman, 2),
            FULL_WALLS,
            &mut r,
        )
    };
    let s = siege.walls_broken_round.expect("catapults breach");
    let c = cavalry
        .walls_broken_round
        .expect("knights eventually chip through");
    assert!(s < c, "siege breached in {s}, cavalry in {c}");
    assert_eq!(siege.walls_left, 0, "a breached wall is a dead wall");
}

#[test]
fn wall_damage_persists_after_a_failed_storm() {
    let mut r = Xor::new(11);
    let out = combat::simulate(
        &u(UnitKind::Levy, 1),
        &u(UnitKind::Bowman, 1),
        FULL_WALLS,
        &mut r,
    );
    assert!(!out.attacker_won, "one levy must not take a keep");
    assert!(out.walls_left > 0 && out.walls_left < FULL_WALLS);
}

#[test]
fn a_champion_lifts_his_army() {
    let plain = {
        let mut r = Xor::new(3);
        combat::simulate(&u(UnitKind::Levy, 8), &u(UnitKind::Bowman, 8), 0, &mut r)
    };
    let led = {
        let mut r = Xor::new(3);
        let army = [u(UnitKind::Champion, 1), u(UnitKind::Levy, 7)].concat();
        combat::simulate(&army, &u(UnitKind::Bowman, 8), 0, &mut r)
    };
    assert!(
        sum(&led.def_loss) > sum(&plain.def_loss),
        "the +25% must bite: {} vs {}",
        sum(&led.def_loss),
        sum(&plain.def_loss)
    );
    assert!(sum(&led.atk_loss) <= sum(&plain.atk_loss));
}

/// Attacker always wins this melee; the duel must fire for roughly a quarter
/// of the seeds (deterministic set, wide sanity window).
#[test]
fn the_duel_turns_the_day_about_one_time_in_four() {
    let mut duels = 0;
    let mut wins = 0;
    for seed in 1..=200u64 {
        let mut r = Xor::new(seed);
        let def = [u(UnitKind::Champion, 1), u(UnitKind::Levy, 2)].concat();
        let out = combat::simulate(&u(UnitKind::Knight, 8), &def, 0, &mut r);
        if out.duel_won {
            duels += 1;
            assert!(!out.attacker_won, "a duel day is never a storm day");
        } else if out.attacker_won {
            wins += 1;
        }
    }
    assert_eq!(duels + wins, 200, "knights must always take the day");
    assert!(
        (25..=75).contains(&duels),
        "duel fired {duels} times in 200"
    );
}

#[test]
fn a_repelled_attack_retreats_the_survivors_home() {
    let mut g = GameState::new(7);
    let from = (0..g.strongholds.len())
        .filter(|&i| {
            g.strongholds[i].owner == PLAYER
                && g.adj(i).iter().any(|&a| g.strongholds[a].owner != PLAYER)
        })
        .min_by_key(|&i| g.adj(i).len())
        .unwrap();
    let target = g
        .adj(from)
        .iter()
        .copied()
        .find(|&a| g.strongholds[a].owner != PLAYER)
        .unwrap();
    // a hopelessly small mob: 2 levies against a full keep
    g.strongholds[from].garrison = u(UnitKind::Levy, 2);
    let n0 = g.strongholds[from].garrison.len();
    let walls0 = g.strongholds[target].walls;
    let owner0 = g.strongholds[target].owner;
    assert!(
        g.march(PLAYER, from, target).is_ok(),
        "the march itself is legal"
    );
    if g.strongholds[target].owner == owner0 {
        // repelled: survivors come home, walls stay chewed, log says retreat
        assert!(g.strongholds[from].garrison.len() <= n0);
        assert!(g.strongholds[target].walls < walls0);
        assert!(
            g.strongholds[target].walls > 0,
            "one levy mob must not breach"
        );
        assert!(g.log.iter().any(|l| l.contains("RETREAT")));
    } else {
        panic!("a levy mob against a full keep must be repelled");
    }
}

#[test]
fn a_duel_wins_reports_the_turned_day() {
    for seed in 1..=3000u64 {
        let mut g = GameState::new(seed);
        let from = (0..g.strongholds.len())
            .filter(|&i| g.strongholds[i].owner == PLAYER)
            .find(|&i| {
                g.adj(i)
                    .iter()
                    .any(|&a| g.strongholds[a].owner == Faction::Norman)
            })
            .unwrap();
        let target = g
            .adj(from)
            .iter()
            .copied()
            .find(|&a| g.strongholds[a].owner == Faction::Norman)
            .unwrap();
        g.strongholds[from].garrison = vec![Unit::new(UnitKind::Knight); 8];
        g.strongholds[target].garrison = [u(UnitKind::Champion, 1), u(UnitKind::Levy, 2)].concat();
        g.strongholds[target].walls = 0;
        assert!(g.march(PLAYER, from, target).is_ok());
        if g.log.iter().any(|l| l.contains("TURNED THE DAY")) {
            assert_eq!(
                g.strongholds[target].owner,
                Faction::Norman,
                "duel keeps the keep"
            );
            assert!(
                g.strongholds[target].garrison.is_empty(),
                "the duel cost the champions their lives"
            );
            assert!(
                !g.strongholds[from].garrison.is_empty(),
                "the army comes home"
            );
            return;
        }
    }
    panic!("no duel in 3000 seeds at a 25% rate");
}
