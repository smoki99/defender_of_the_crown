//! T706 - unlock pacing integration: the seasonal meet draw, the button
//! label it drives, and the launch must all stay inside the era pool.
use defender_of_the_crown::campaign::game::{Difficulty, PLAYER, SPORTS_FEE};
use defender_of_the_crown::minigames::framework::{unlocked, SPORTS};
use defender_of_the_crown::ui::{action_rects, season_sport, CampaignUi, SportMode};

fn at_turn(t: u32) -> CampaignUi {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    ui.g.turn = t;
    ui
}

#[test]
fn the_season_draw_never_leaves_the_unlocked_pool() {
    for t in 1..=40 {
        let ui = at_turn(t);
        let sp = season_sport(&ui.g).expect("a pool is always open");
        assert!(
            unlocked(t).iter().any(|s| s.id == sp.id),
            "turn {t} offered {} outside its era pool",
            sp.id
        );
        // the rare-event hoard is never part of the seasonal draw
        assert_ne!(sp.id, "dragon", "turn {t} drew the rare-event sport");
    }
}

#[test]
fn early_seasons_cannot_offer_mid_or_late_sports() {
    let starters: Vec<&str> = SPORTS
        .iter()
        .filter(|s| s.unlock_turn == 0)
        .map(|s| s.id)
        .collect();
    assert_eq!(starters.len(), 4);
    for t in 1..=4 {
        let ui = at_turn(t);
        let id = season_sport(&ui.g).unwrap().id;
        assert!(
            starters.contains(&id),
            "turn {t} broke the 1-4 starter lockout with {id}"
        );
    }
    let late: Vec<&str> = SPORTS
        .iter()
        .filter(|s| s.unlock_turn == 10)
        .map(|s| s.id)
        .collect();
    for t in 5..=9 {
        let ui = at_turn(t);
        let id = season_sport(&ui.g).unwrap().id;
        assert!(!late.contains(&id), "turn {t} offered late-era {id}");
    }
}

#[test]
fn every_era_actually_reaches_the_table() {
    // Deterministic in the turn, so this sweep is a stable contract: over
    // one long campaign the mid-era and late-era sports must all be held
    // at least once, or the pacing tiers are dead weight.
    let mut seen: Vec<&str> = Vec::new();
    for t in 1..=60 {
        let ui = at_turn(t);
        let id = season_sport(&ui.g).unwrap().id;
        if !seen.contains(&id) {
            seen.push(id);
        }
    }
    for s in SPORTS.iter() {
        if s.unlock_turn != u32::MAX {
            assert!(
                seen.contains(&s.id),
                "{} (t{}) never drew across 60 seasons",
                s.id,
                s.unlock_turn
            );
        }
    }
}

#[test]
fn the_sports_button_carries_the_season_and_fits_its_frame() {
    // Layout invariant: every draw is centred inside the sports Btn with
    // breathing room, so adding a long-labelled sport cannot overflow.
    let b = action_rects(true, false)
        .into_iter()
        .find(|b| b.id == "sports")
        .unwrap();
    for t in [1, 3, 5, 7, 10, 12, 20] {
        let ui = at_turn(t);
        let sp = season_sport(&ui.g).unwrap();
        assert!(
            sp.label.len() * 8 + 4 <= b.w as usize,
            "turn {t}: label {} overflows the {}px button",
            sp.label,
            b.w
        );
        // two calls in one frame must agree (label, gate and launch share
        // the draw, so a flicker-free frame depends on it)
        assert_eq!(season_sport(&ui.g).unwrap().id, sp.id);
    }
}

#[test]
fn launching_a_meet_boards_the_season_sport_of_the_current_era() {
    let mut ui = at_turn(11);
    ui.g.gold[PLAYER.index()] = 100;
    let want = season_sport(&ui.g).unwrap().id;
    ui.act("sports", 0);
    let (spec, mode) = ui.launch_req.expect("the meet boards");
    assert_eq!(spec.id, want, "launch drifted from the button label");
    assert!(spec.unlock_turn <= 11, "launch broke the pacing gate");
    assert!(matches!(
        mode,
        SportMode::Sports { .. } | SportMode::Herald { .. }
    ));
    assert!(ui.g.gi(PLAYER) >= SPORTS_FEE);
}
