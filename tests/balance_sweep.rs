//! T802 balance sweep: the FINAL reward tables for all 13 sports must be
//! monotone, land on clean tier boundaries, and keep visible skill slack
//! between bands (MG-QA Q6 normalised across the whole catalogue on the
//! final engine).
use defender_of_the_crown::campaign::game::Tier;
use defender_of_the_crown::minigames::framework::{resolve, MgState, SPORTS};

fn tier_of(s: &defender_of_the_crown::minigames::framework::SportSpec, score: u32) -> String {
    match resolve(score, s) {
        MgState::Won(t) => format!("{t:?}"),
        MgState::Lost => "NONE".into(),
        _ => unreachable!("resolve only ends"),
    }
}

#[test]
fn every_reward_table_is_monotone_and_gated() {
    assert_eq!(SPORTS.len(), 13, "the catalogue grew silently");
    for s in SPORTS {
        assert!(
            s.bronze_at < s.silver_at && s.silver_at < s.gold_at && s.gold_at <= 100,
            "{}: thresholds {} < {} < {} <= 100 broken",
            s.id,
            s.bronze_at,
            s.silver_at,
            s.gold_at
        );
        // skill slack: each band edge must be a visible jump, not 1 px
        // of timing (a casual script that misreads one beat changes tier)
        assert!(
            s.silver_at - s.bronze_at >= 10,
            "{}: bronze->silver slack only {}",
            s.id,
            s.silver_at - s.bronze_at
        );
        assert!(
            s.gold_at - s.silver_at >= 10,
            "{}: silver->gold slack only {}",
            s.id,
            s.gold_at - s.silver_at
        );
        // exact boundaries: the line itself promotes, one short demotes
        let cases = [
            (s.gold_at, "Gold"),
            (s.gold_at - 1, "Silver"),
            (s.silver_at, "Silver"),
            (s.silver_at - 1, "Bronze"),
            (s.bronze_at, "Bronze"),
            (s.bronze_at - 1, "NONE"),
            (0, "NONE"),
            (100, "Gold"), // gold_at <= 100, so the ceiling is always Gold
        ];
        for (score, want) in cases {
            assert_eq!(tier_of(s, score), want, "{}: score {score}", s.id);
        }
        // the purse-side resolver agrees on the promotion lines
        assert_eq!(Tier::of(s.gold_at, s.silver_at, s.gold_at), Tier::Gold);
        assert_eq!(
            Tier::of(s.gold_at - 1, s.silver_at, s.gold_at),
            Tier::Silver
        );
        assert_eq!(Tier::of(s.silver_at, s.silver_at, s.gold_at), Tier::Silver);
        assert_eq!(
            Tier::of(s.silver_at - 1, s.silver_at, s.gold_at),
            Tier::Bronze
        );
    }
}
