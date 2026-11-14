//! T307 event modals: the dawn omen opens a pixel-icon dialog that deafens
//! the map until its OK button dismisses it; a tourney year flies a yellow
//! pennant on the hosting keep.
use defender_of_the_crown::campaign::events::EventKind;
use defender_of_the_crown::campaign::game::{Difficulty, View, PLAYER};
use defender_of_the_crown::engine::gfx::{Framebuffer, DARKRED, LINEN, WHITE, YELLOW};
use defender_of_the_crown::ui::{
    castle_screen, end_turn_rect, event_ok_rect, Btn, CampaignUi, SceneEvent,
};

fn tap(x: i32, y: i32) -> SceneEvent {
    SceneEvent::Tap { x, y }
}

fn tapc(b: &Btn) -> SceneEvent {
    let (x, y) = center(b);
    tap(x, y)
}
fn center(b: &Btn) -> (i32, i32) {
    (b.x + b.w as i32 / 2, b.y + b.h as i32 / 2)
}

/// Force one omen through the spy-tip override, then end the turn.
fn announce(ui: &mut CampaignUi, e: EventKind) {
    ui.g.tip = Some(e);
    ui.event(&tapc(&end_turn_rect()), true);
}

#[test]
fn the_omen_openeth_a_modal_and_ok_closenth_it() {
    let mut ui = CampaignUi::new(13, Difficulty::Normal);
    announce(&mut ui, EventKind::Plague);
    assert_eq!(ui.event_kind, Some(EventKind::Plague));
    assert!(ui.event_open, "the omen must announce itself");
    let mut fb = Framebuffer::new();
    ui.draw(&mut fb);
    // LINEN plate at a text-free corner, plague skull head in WHITE,
    // raised OK button face under the label.
    assert_eq!(fb.get(74, 70), LINEN);
    assert_eq!(fb.get(88, 78), WHITE);
    assert_eq!(fb.get(146, 129), DARKRED);
    ui.event(&tapc(&event_ok_rect()), true);
    assert!(!ui.event_open);
    // plate gone from screen
    ui.draw(&mut fb);
    assert_ne!(fb.get(74, 70), LINEN);
}

#[test]
fn the_omen_deafeneth_the_map() {
    let mut ui = CampaignUi::new(13, Difficulty::Normal);
    announce(&mut ui, EventKind::Plague);
    // announce's own END tap moved us two turns; take that as baseline
    let turn0 = ui.g.turn;
    let id =
        ui.g.strongholds
            .iter()
            .position(|s| s.owner == PLAYER)
            .unwrap();
    let (cx, cy) = castle_screen(&ui.g.strongholds[id]);
    ui.event(&tap(cx, cy), true);
    assert_eq!(ui.sel, None, "keeps are unreachable behind the modal");
    ui.event(&tapc(&end_turn_rect()), true);
    assert_eq!(ui.g.turn, turn0, "END is swallowed while the omen stands");
    assert!(ui.event_open);
    ui.event(&tapc(&event_ok_rect()), true);
    ui.event(&tapc(&end_turn_rect()), true);
    assert_eq!(ui.g.turn, turn0 + 2);
    assert!(ui.event_open, "every dawn rolls a fresh omen");
}

#[test]
fn the_tournary_year_pennanteth_the_keep() {
    let mut ui = CampaignUi::new(5, Difficulty::Normal);
    announce(&mut ui, EventKind::TournamentHerald);
    ui.event(&tapc(&event_ok_rect()), true);
    let pick = ui.g.tournament[PLAYER.index()].expect("herald picks a keep");
    let (cx, cy) = castle_screen(&ui.g.strongholds[pick]);
    let mut fb = Framebuffer::new();
    ui.draw(&mut fb);
    // yellow pennant flying from the right crenel (anim 0 frame)
    assert_eq!(fb.get(cx as u32 + 7, cy as u32 - 13), YELLOW);
    // a keep with no tourney (dark for us: no YELLOW garrison digits there
    // either) flies none
    let other =
        ui.g.strongholds
            .iter()
            .find(|s| s.id != pick && matches!(ui.g.view(PLAYER, s.id), View::Dark))
            .expect("a dark keep exists");
    let (ox, oy) = castle_screen(other);
    assert_ne!(fb.get(ox as u32 + 7, oy as u32 - 13), YELLOW);
    assert_ne!(fb.get(ox as u32 + 6, oy as u32 - 14), YELLOW);
}
