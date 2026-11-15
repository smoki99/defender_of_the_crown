//! T801 SOAK gate: every AI-vs-AI campaign must end inside the
//! chronicle horizon with the world intact, and forced minigame farming
//! must never push gold past the reward table. The 500-game run doubles
//! as the panic net for the whole campaign engine (release: ~0.3 s).
use defender_of_the_crown::campaign::ai;
use defender_of_the_crown::campaign::game::{
    Faction, GameState, MAX_CHAMPIONS, PLAYER, SPORTS_FEE,
};
use defender_of_the_crown::campaign::units::MAX_GARRISON;
use defender_of_the_crown::minigames::framework::SPORTS;

const GOLD_CEILING: u32 = 2500;

fn invariants(g: &GameState, check_gold: bool) -> Result<(), String> {
    for f in [Faction::Saxon, Faction::Norman] {
        let i = f.index();
        if g.bars[i].loyalty > 100 || g.bars[i].popularity > 100 {
            return Err(format!("turn {}: {f:?} bars out of 0..=100", g.turn));
        }
        if g.bars[i].provisions > g.provisions_cap(f) {
            return Err(format!("turn {}: {f:?} barn over cap", g.turn));
        }
        if check_gold && g.gold[i] > GOLD_CEILING {
            return Err(format!("turn {}: {f:?} gold runaway", g.turn));
        }
        if g.count_champions(f) > MAX_CHAMPIONS {
            return Err(format!("turn {}: {f:?} too many champions", g.turn));
        }
    }
    if g.strongholds.len() < 12 {
        return Err("realm reshaped itself".into());
    }
    for s in &g.strongholds {
        if s.garrison.len() > MAX_GARRISON {
            return Err(format!(
                "turn {}: {} garrison {}",
                g.turn,
                s.name,
                s.garrison.len()
            ));
        }
    }
    Ok(())
}

#[test]
fn five_hundred_councils_wage_ended_wars() {
    for seed in 1..=500u64 {
        let mut g = GameState::new(seed);
        while g.over.is_none() {
            // SPEC 5.5: an even tale at XC may drag the war to the X+
            // London tie-break, but turn 100 must close the chronicle.
            assert!(g.turn <= 100, "seed {seed}: endless war at turn {}", g.turn);
            ai::take_turn(&mut g, PLAYER);
            g.end_player_turn();
            // day-start view; mid-day the barn may ride over a cap that
            // conquest shrank until pay_day clamps it.
            invariants(&g, true).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        }
    }
}

#[test]
fn farmed_meets_never_break_the_purse() {
    let mut g = GameState::new(7);
    let silver = SPORTS[0].silver_at;
    let gold = SPORTS[0].gold_at;
    while g.over.is_none() {
        assert!(g.turn <= 100, "farmer: endless war");
        ai::take_turn(&mut g, PLAYER);
        let home = g.homes[PLAYER.index()];
        let p = PLAYER.index();
        if g.gold[p] >= SPORTS_FEE {
            let before = g.gold[p];
            g.play_sports(PLAYER, home, 9999, silver, gold).unwrap();
            // 80 + popularity bonus (<=10) minus the 10g fee
            assert!(
                g.gold[p] - before <= 80,
                "turn {}: meet netted {} > 80",
                g.turn,
                g.gold[p] - before
            );
        }
        if g.tournament[p].is_some() {
            let before = g.gold[p];
            g.play_tournament(PLAYER, 9999, silver, gold).unwrap();
            // double gold (<=180) or the third-champion fallback (+40)
            assert!(
                g.gold[p] - before <= 220,
                "turn {}: herald netted {} > 220",
                g.turn,
                g.gold[p] - before
            );
        }
        g.end_player_turn();
        invariants(&g, false).unwrap_or_else(|e| panic!("farmer: {e}"));
    }
}
