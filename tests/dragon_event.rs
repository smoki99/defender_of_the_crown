//! T704 (campaign layer): THE DRAGON'S HOARD end to end. The omen may
//! only stir from turn 20 and never twice in a campaign; the HOARD row
//! openeth beside an own castle while the offer standeth; the heist pays
//! the DOUBLE tourney haul and closes the offer; a failed or timed-out
//! heist still burneth it. The season's SPORTS draw never toucheth the
//! dragon - the lair is omen-only.

use defender_of_the_crown::campaign::game::{Difficulty, GameState, Tier, PLAYER};
use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::minigames::dragon::Dragon;
use defender_of_the_crown::minigames::framework::{pick as sport_pick, unlocked, SPORTS};
use defender_of_the_crown::engine::scene::Scene;
use defender_of_the_crown::ui::host::{MgHost, OutcomeCell, SportMode};
use defender_of_the_crown::ui::{action_rects, castle_screen, Btn, CampaignUi, SceneEvent};
use std::rc::Rc;

fn btn<'a>(rects: &'a [Btn], id: &str) -> &'a Btn {
    rects.iter().find(|b| b.id == id).expect("rect")
}

#[test]
fn the_hills_never_stir_before_twenty() {
    for seed in 0..300u64 {
        let mut g = GameState::new(seed);
        for t in 1..=19u32 {
            g.turn = t;
            g.roll_events();
            assert!(
                !g.dragon_offer,
                "the hoard stirred at turn {t} (seed {seed})"
            );
        }
    }
}

#[test]
fn the_stir_waketh_once_in_a_campaign() {
    let mut g = GameState::new(77);
    let mut hits = 0;
    for t in 1..=600u32 {
        g.turn = t;
        g.roll_events();
        if g.dragon_offer {
            hits += 1;
            assert!(g.dragon_seen, "an offer must mark itself spent");
        }
    }
    assert_eq!(hits, 1, "the dragon stirreth exactly once per campaign");
}

#[test]
fn the_paying_heist_doubles_the_haul() {
    let mut g = GameState::new(11);
    g.dragon_offer = true;
    let before = g.gold[PLAYER.index()];
    let (tier, r) = g
        .play_dragon(PLAYER, 100, SPORTS[12].silver_at, SPORTS[12].gold_at)
        .expect("the offer standeth");
    assert_eq!(tier, Tier::Gold);
    assert!(r.gold >= 160, "gold tier doubles to at least 160, saw {}", r.gold);
    assert!(r.provisions >= 60, "provisions double too, saw {}", r.provisions);
    assert_eq!(g.gold[PLAYER.index()], before + r.gold, "purse credited");
    assert!(!g.dragon_offer, "one heist per campaign");
    assert!(g.log.iter().any(|l| l.contains("HOARD IS ROBBED")), "log narrates");
    assert!(
        g.play_dragon(PLAYER, 100, 80, 100).is_err(),
        "the hoard granteth no second try"
    );
}

#[test]
fn a_burnt_offer_payeth_nothing() {
    let mut g = GameState::new(13);
    g.dragon_offer = true;
    let before = g.gold[PLAYER.index()];
    g.forfeit_dragon();
    assert!(!g.dragon_offer, "the failed heist burneth the offer");
    assert_eq!(g.gold[PLAYER.index()], before, "nought credited to failure");
    assert!(g.play_dragon(PLAYER, 100, 80, 100).is_err(), "offer is spent");
    g.forfeit_dragon(); // silence on an already-burnt offer
}

#[test]
fn the_chime_of_sixty_silenceth_the_hoard() {
    // timeout path: the host force-resolves a Running heist at 0 points,
    // tier None - the campaign flow answereth with forfeit_dragon().
    let cell: OutcomeCell = Rc::default();
    let mut h = MgHost::new(
        &SPORTS[12],
        SportMode::Hoard,
        Box::new(Dragon::new(3)),
        cell.clone(),
        false,
    );
    let mut fb = Framebuffer::new();
    for _ in 0..62 * 60 {
        h.draw(&mut fb);
        if cell.borrow().is_some() {
            break;
        }
    }
    let o = cell.borrow().expect("the cheer timer resolves the heist");
    assert_eq!(o.score, 0, "caught inside, the score is naught");
    assert_eq!(o.tier, None, "a heist nae ended at the door payeth no tier");
    let mut g = GameState::new(17);
    g.dragon_offer = true;
    if o.tier.is_none() {
        g.forfeit_dragon();
    }
    assert!(!g.dragon_offer, "timeout burneth the one offer");
}

#[test]
fn the_hoard_row_appeareth_only_with_the_omen() {
    let ids = |own: bool, hoard: bool| -> Vec<&'static str> {
        action_rects(own, hoard).iter().map(|b| b.id).collect()
    };
    assert!(!ids(true, false).contains(&"hoard"), "no omen, no row");
    assert!(!ids(false, true).contains(&"hoard"), "an omen on anoither's castle openeth no row");
    assert!(ids(true, true).contains(&"hoard"), "the omen openeth the row");
}

#[test]
fn tapping_hoard_openeth_the_lair_free() {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    let s = ui
        .g
        .strongholds
        .iter()
        .find(|s| s.name == "Buckingham")
        .expect("castle");
    let (x, y) = castle_screen(s);
    ui.event(&SceneEvent::Tap { x, y }, true);
    assert_eq!(ui.sel.map(|i| ui.g.strongholds[i].name), Some("Buckingham"));
    let gold = ui.g.gold[PLAYER.index()];
    // no omen: the row is simply absent
    assert!(btn(&action_rects(true, false), "sports").w > 0);
    ui.g.dragon_offer = true;
    let b = btn(&action_rects(true, true), "hoard").clone();
    ui.event(
        &SceneEvent::Tap {
            x: b.x + b.w as i32 / 2,
            y: b.y + b.h as i32 / 2,
        },
        true,
    );
    let (spec, mode) = ui.launch_req.expect("the lair openeth");
    assert_eq!(spec.id, "dragon");
    assert_eq!(mode, SportMode::Hoard);
    assert_eq!(ui.g.gold[PLAYER.index()], gold, "the heist asketh no fee");
}

#[test]
fn the_meet_never_draweth_the_dragon() {
    for t in 0..=60u32 {
        assert!(
            unlocked(t).iter().all(|s| s.id != "dragon"),
            "turn {t}: the season meet listed the dragon"
        );
        for seed in 0..300u64 {
            let s = sport_pick(t, seed ^ 0xABCD_0000);
            assert!(s.is_none_or(|s| s.id != "dragon"), "turn {t} seed {seed}");
        }
    }
}
