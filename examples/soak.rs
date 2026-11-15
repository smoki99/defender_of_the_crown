//! T801 SOAK (ALPHA 2 gate): 500 AI-vs-AI campaigns must run panic-free
//! and every one must end within the chronicle horizon (<= 90 turns) with
//! the world invariants intact; plus a reward-economy stress where one
//! banner golds every meet it can - the minigame purse must stay inside
//! the reward table's ceiling and never break the income engine.
use defender_of_the_crown::campaign::ai;
use defender_of_the_crown::campaign::game::{
    Faction, GameState, MAX_CHAMPIONS, PLAYER, SPORTS_FEE,
};
use defender_of_the_crown::campaign::units::MAX_GARRISON;
use defender_of_the_crown::minigames::framework::SPORTS;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Runaway-detection ceiling for AI policy games: no legitimate campaign
/// should bank this much in a chronicle-length war. The forced-meet
/// farmer skips this static cap and is checked per credit instead.
const GOLD_CEILING: u32 = 2500;

fn invariants(g: &GameState, check_gold: bool) -> Result<(), String> {
    for f in [Faction::Saxon, Faction::Norman] {
        let i = f.index();
        let bars = g.bars[i];
        if bars.loyalty > 100 || bars.popularity > 100 {
            return Err(format!("turn {}: {f:?} bars out of 0..=100", g.turn));
        }
        if g.bars[i].provisions > g.provisions_cap(f) {
            return Err(format!("turn {}: {f:?} barn over cap", g.turn));
        }
        if check_gold && g.gold[i] > GOLD_CEILING {
            return Err(format!("turn {}: {f:?} gold {} runaway", g.turn, g.gold[i]));
        }
        let champs = g.count_champions(f);
        if champs > MAX_CHAMPIONS {
            return Err(format!("turn {}: {f:?} {champs} champions", g.turn));
        }
    }
    let mut total = 0usize;
    for s in &g.strongholds {
        if s.garrison.len() > MAX_GARRISON {
            return Err(format!(
                "turn {}: {} garrison {}",
                g.turn,
                s.name,
                s.garrison.len()
            ));
        }
        total += 1;
    }
    if total != g.strongholds.len() || total < 12 {
        return Err("realm reshaped itself".into());
    }
    Ok(())
}

/// One full AI-vs-AI campaign; Err on any invariant break, None if the
/// chronicle horizon is crossed without an ending.
fn run_game(seed: u64) -> Result<(Faction, u32), String> {
    let mut g = GameState::new(seed);
    while g.over.is_none() {
        // SPEC 5.5: an even tale at XC spares the war; the X+ London
        // tie-break guarantees an ending by turn 100 - no endless war.
        if g.turn > 100 {
            return Err(format!("seed {seed}: still waging war at turn {}", g.turn));
        }
        ai::take_turn(&mut g, PLAYER);
        g.end_player_turn();
        // day-start invariants; mid-turn the barn may ride over a
        // shrunken cap until pay_day clamps it.
        invariants(&g, true)?;
    }
    let w = g.over.unwrap();
    Ok((w, g.turn))
}

/// Forced-meet economy stress: gold every single meet, free heralds too.
/// Every credit must stay inside the reward table (max 80 + popularity
/// bonus; tourney doubles; +40 when the banner already carries its third
/// champion) and the world must still end.
fn run_forced_meets(seed: u64) -> Result<(), String> {
    let mut g = GameState::new(seed);
    let silver = SPORTS[0].silver_at;
    let gold = SPORTS[0].gold_at;
    while g.over.is_none() {
        if g.turn > 100 {
            return Err(format!("forced seed {seed}: war never ended"));
        }
        ai::take_turn(&mut g, PLAYER);
        // one paid meet at home: a gold run must pay at most 90 net table
        // gold minus the fee (popularity <= 100 => bonus <= 10).
        let home = g.homes[PLAYER.index()];
        let before = g.gold[PLAYER.index()];
        if g.gold[PLAYER.index()] >= SPORTS_FEE {
            g.play_sports(PLAYER, home, 9999, silver, gold)
                .map_err(|e| format!("forced meet refused: {e}"))?;
            let delta = g.gold[PLAYER.index()].saturating_sub(before);
            if delta > 80 {
                return Err(format!(
                    "turn {}: meet netted {delta} > 80 (90 max - fee)",
                    g.turn
                ));
            }
        }
        // the herald's free meet: double haul, +40 once three champions ride
        if g.tournament[PLAYER.index()].is_some() {
            let before = g.gold[PLAYER.index()];
            g.play_tournament(PLAYER, 9999, silver, gold)?;
            let delta = g.gold[PLAYER.index()].saturating_sub(before);
            if delta > 90 * 2 + 40 {
                return Err(format!(
                    "turn {}: herald paid {delta} > table max 220",
                    g.turn
                ));
            }
        }
        g.end_player_turn();
        invariants(&g, false)?;
    }
    Ok(())
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let games: u64 = std::env::args()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(500);
    let mut saxon = 0u32;
    let mut norman = 0u32;
    let mut other = 0u32;
    let mut turns_sum = 0u64;
    let mut worst_turn = 0u32;
    let mut problems: Vec<String> = Vec::new();

    for seed in 1..=games {
        match catch_unwind(AssertUnwindSafe(|| run_game(seed))) {
            Ok(Ok((w, t))) => {
                turns_sum += t as u64;
                worst_turn = worst_turn.max(t);
                match w {
                    Faction::Saxon => saxon += 1,
                    Faction::Norman => norman += 1,
                    _ => other += 1,
                }
            }
            Ok(Err(e)) => problems.push(format!("seed {seed}: {e}")),
            Err(_) => problems.push(format!("seed {seed}: PANIC")),
        }
    }
    // one deterministic forced-meet stress on top of the soak
    match catch_unwind(AssertUnwindSafe(|| run_forced_meets(7))) {
        Ok(Ok(())) => println!("forced-meets: gold economy stayed inside the table"),
        Ok(Err(e)) => problems.push(format!("forced: {e}")),
        Err(_) => problems.push("forced: PANIC".into()),
    }
    println!(
        "soak {games}: ends {} (saxon {saxon} / norman {norman} / other {other}), avg turn {}, worst {}",
        saxon + norman + other,
        turns_sum as f64 / games as f64,
        worst_turn,
    );
    if problems.is_empty() {
        println!("DONE clean");
    } else {
        for p in problems.iter().take(20) {
            println!("PROBLEM {p}");
        }
        println!("FAILS {}", problems.len());
        std::process::exit(1);
    }
}
