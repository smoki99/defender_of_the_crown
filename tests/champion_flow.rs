//! T707 champion triple-gold flow: the proclamation modal opens over a
//! deaf map and only its OK dismisses it; it stacks above an open omen.
use defender_of_the_crown::campaign::events::EventKind;
use defender_of_the_crown::campaign::game::{Difficulty, PLAYER};
use defender_of_the_crown::engine::gfx::{Framebuffer, DARKRED, GOLD, LINEN, NAVY};
use defender_of_the_crown::ui::{
    castle_screen, end_turn_rect, event_ok_rect, Btn, CampaignUi, PlayScene, SceneEvent,
};

fn tap(x: i32, y: i32) -> SceneEvent {
    SceneEvent::Tap { x, y }
}
fn tapc(b: &Btn) -> SceneEvent {
    tap(b.x + b.w as i32 / 2, b.y + b.h as i32 / 2)
}

#[test]
fn the_triple_crown_proclaimeth_and_ok_closenth() {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    ui.announce_champion();
    let mut fb = Framebuffer::new();
    ui.draw(&mut fb);
    // omen plate reused: LINEN field, NAVY plaque, gold crown crest,
    // raised OK button under the label.
    assert_eq!(fb.get(74, 70), LINEN, "linen plate");
    assert_eq!(fb.get(80, 72), NAVY, "crown plaque ground");
    assert_eq!(fb.get(83, 79), GOLD, "crown band");
    assert_eq!(fb.get(87, 80), DARKRED, "crown jewel");
    assert_eq!(fb.get(146, 129), DARKRED, "OK button face");
    // the map is deaf: a castle tap selects nothing, END TURN stalls
    let (bx, by) = castle_screen(&ui.g.strongholds[0]);
    ui.event(&tap(bx, by), true);
    assert_eq!(ui.sel, None, "no selection behind the proclamation");
    let turn = ui.g.turn;
    ui.event(&tapc(&end_turn_rect()), true);
    assert_eq!(ui.g.turn, turn, "the crown out-ranks the turn");
    // only OK dismisses
    ui.event(&tapc(&event_ok_rect()), true);
    assert!(!ui.champ_open);
    ui.draw(&mut fb);
    assert_ne!(fb.get(74, 70), LINEN, "plate gone after OK");
    // and the map hearkens again
    ui.event(&tap(bx, by), true);
    assert_eq!(ui.sel, Some(0), "the ear returns with the crown");
}

#[test]
fn the_crown_sitteth_above_the_omen() {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    ui.event_kind = Some(EventKind::Plague); // an omen left open...
    ui.event_open = true;
    ui.announce_champion(); // ...and glory crowns over it
    let mut fb = Framebuffer::new();
    ui.draw(&mut fb);
    assert_eq!(fb.get(87, 80), DARKRED, "crown jewel paints last");
    ui.event(&tapc(&event_ok_rect()), true);
    assert!(!ui.champ_open, "OK drops the crown first");
    assert!(ui.event_open, "the omen waits beneath");
    ui.event(&tapc(&event_ok_rect()), true);
    assert!(!ui.event_open, "and a second OK clears the omen");
}

#[test]
fn the_champion_ridth_with_the_garrison() {
    // the panel's unit rows are data-driven off ALL_KINDS; a champion
    // pushed by the harness hook must show up in the player's own view.
    use defender_of_the_crown::campaign::units::UnitKind;
    let scene = PlayScene::new(7, Difficulty::Normal).with_champion(true);
    assert!(scene.ui.champ_open, "champ=1 proclaims the crown");
    let g = &scene.ui.g.strongholds[PLAYER.index()].garrison;
    assert_eq!(g.iter().filter(|u| u.kind == UnitKind::Champion).count(), 1);
}
