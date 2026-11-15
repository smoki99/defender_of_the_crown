//! T302/T304 pixel kit: three-tone bevels, garrison list scrolling, hover
//! tooltips and the SPORTS gate (season meet + entry fee), asserted on the
//! framebuffer.
use defender_of_the_crown::campaign::game::{Difficulty, PLAYER, SPORTS_FEE};
use defender_of_the_crown::engine::gfx::{
    Framebuffer, BLACK, DARKGRAY, DARKRED, EMBER, GRAY, LINEN, ORANGE, SILVER, TAN,
};
use defender_of_the_crown::minigames::framework::pick as sport_pick;
use defender_of_the_crown::ui::{action_rects, end_turn_rect, kit, Btn, CampaignUi, SceneEvent};

fn tap(x: i32, y: i32) -> SceneEvent {
    SceneEvent::Tap { x, y }
}

fn move_to(x: i32, y: i32) -> SceneEvent {
    SceneEvent::Move { x, y }
}

fn center(b: &Btn) -> (i32, i32) {
    (b.x + b.w as i32 / 2, b.y + b.h as i32 / 2)
}

fn btn<'a>(rects: &'a [Btn], id: &str) -> &'a Btn {
    rects.iter().find(|b| b.id == id).expect("rect")
}

fn select(ui: &mut CampaignUi, name: &str) {
    let s =
        ui.g.strongholds
            .iter()
            .find(|s| s.name == name)
            .expect("castle");
    let (x, y) = defender_of_the_crown::ui::castle_screen(s);
    ui.event(&tap(x, y), true);
    assert_eq!(ui.sel.map(|i| ui.g.strongholds[i].name), Some(name));
}

#[test]
fn buttons_carry_three_tones_per_state() {
    let mut fb = Framebuffer::new();
    let b = btn(&action_rects(true, false), "rec:0").clone();
    kit::button(&mut fb, &b, false, true);
    assert_eq!(fb.get(b.x as u32, b.y as u32), BLACK, "rim");
    assert_eq!(
        fb.get((b.x + 1) as u32, (b.y + 1) as u32),
        EMBER,
        "lit edge"
    );
    assert_eq!(
        fb.get((b.x + 1) as u32, (b.y + b.h as i32 - 2) as u32),
        BLACK,
        "shadow edge"
    );
    assert_eq!(fb.get(b.x as u32 + 34, b.y as u32 + 7), DARKRED, "face");
    kit::button(&mut fb, &b, true, true);
    assert_eq!(fb.get(b.x as u32 + 34, b.y as u32 + 7), ORANGE, "hot face");
    kit::button(&mut fb, &b, false, false);
    assert_eq!(fb.get(b.x as u32 + 34, b.y as u32 + 7), GRAY, "dark face");
    assert_eq!(
        fb.get((b.x + 1) as u32, (b.y + 1) as u32),
        SILVER,
        "dark lit edge"
    );
}

#[test]
fn panels_rim_and_bevel_like_amiga() {
    let mut fb = Framebuffer::new();
    kit::panel(&mut fb, 4, 86, 292, 86);
    assert_eq!(fb.get(4, 86), TAN, "rim corner");
    assert_eq!(fb.get(5, 87), GRAY, "lit inner top");
    assert_eq!(fb.get(5, 170), BLACK, "shadow inner bottom");
    assert_eq!(fb.get(150, 130), DARKGRAY, "face");
}

#[test]
fn hover_over_a_button_paints_a_tooltip() {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    select(&mut ui, "Buckingham");
    let levy = btn(&action_rects(true, false), "rec:0").clone();
    let (bx, by) = center(&levy);
    ui.event(&move_to(bx, by), true);
    assert_eq!(ui.hover, Some("rec:0"));
    let mut fb = Framebuffer::new();
    ui.draw(&mut fb);
    // plate = tip text width + 2px padding, top-left at (bx+8, by+10),
    // clamped inside the 320px screen.
    let w = ("MILITIA: CHEAP, MANY, WEAK".len() as u32) * 6 + 4;
    let tx = (bx as u32 + 8).min(320 - w);
    let ty = (by + 10) as u32;
    assert_eq!(fb.get(tx, ty), BLACK, "plate rim");
    assert_eq!(fb.get(tx + 1, ty + 1), LINEN, "plate face");
    assert_eq!(fb.get(tx + w - 1, ty), BLACK, "plate rim right");
    assert_eq!(fb.get(tx + 1, ty + 5), LINEN, "left margin");
    // the hovered button rides hot (ORANGE face beside the label)
    assert_eq!(fb.get(levy.x as u32 + 2, (levy.y + 2) as u32), ORANGE);
    // pointer off the controls: no hover, no plate
    ui.event(&move_to(10, 10), true);
    assert_eq!(ui.hover, None);
}

#[test]
fn garrison_roll_scroll_when_it_overflows() {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    select(&mut ui, "Buckingham");
    ui.g.gold[PLAYER.index()] = 500;
    for id in ["rec:1", "rec:2", "rec:3"] {
        let (x, y) = center(btn(&action_rects(true, false), id));
        ui.event(&tap(x, y), true);
    }
    // four unit rows + walls row overflow the 4-row window
    let mut fb = Framebuffer::new();
    ui.draw(&mut fb);
    // thumb (TAN) sits at the top of the track while unscrolled
    assert_eq!(fb.get(202, 110), TAN, "thumb top");
    let scr_y = 100 + 36 - 8; // scrollbar down-arrow band
    ui.event(&tap(204, scr_y + 4), true);
    assert_eq!(ui.list_top, 1, "down arrow did not scroll");
    let mut fb2 = Framebuffer::new();
    ui.draw(&mut fb2);
    assert_ne!(fb.get(202, 110), fb2.get(202, 110), "thumb never moved");
    assert_eq!(fb2.get(202, 110), DARKGRAY, "track shows above thumb");
    // up arrow scrolls back
    ui.event(&tap(204, 104), true);
    assert_eq!(ui.list_top, 0);
}

#[test]
fn sports_gate_is_the_meet_and_the_fee() {
    // Every posted on-season meet now ships, so the gate is exactly:
    // a drawn meet plus a payable fee. The button tone follows the
    // meet, a light purse greys it, and a payable tap launches the
    // meet - the fee itself only settles when the host returns.
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    select(&mut ui, "Buckingham");
    ui.g.turn = 12;
    let sports = btn(&action_rects(true, false), "sports").clone();
    let (cx, cy) = center(&sports);
    let mut fb = Framebuffer::new();
    let held = sport_pick(
        ui.g.turn,
        ((ui.g.turn as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15,
    )
    .is_some();
    assert!(held, "the season posts a meet");
    ui.draw(&mut fb);
    assert_ne!(
        fb.get(cx as u32 - 20, (sports.y + 2) as u32),
        GRAY,
        "a payable meet keeps the button lit"
    );
    // purse below the fee -> greyed whatever the season
    ui.g.gold[PLAYER.index()] = SPORTS_FEE - 1;
    ui.draw(&mut fb);
    assert_eq!(
        fb.get(cx as u32 - 20, (sports.y + 2) as u32),
        GRAY,
        "broke player sees a greyed button"
    );
    // payable purse: the tap launches the meet and charges nothing yet
    ui.g.gold[PLAYER.index()] = 100;
    ui.event(&tap(cx, cy), true);
    assert!(ui.launch_req.is_some(), "a payable meet launches");
    assert_eq!(ui.g.gi(PLAYER), 100, "launch defers the fee to the host");
}

#[test]
fn end_turn_still_ends_under_the_new_hit_order() {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    let end = end_turn_rect();
    let (x, y) = center(&end);
    ui.event(&move_to(x, y), true);
    assert_eq!(ui.hover, Some("end"));
    let before = ui.g.turn;
    ui.event(&tap(x, y), true);
    assert!(ui.g.turn > before, "end turn regressed");
}
