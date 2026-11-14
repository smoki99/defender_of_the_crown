//! T303 kingdom bars HUD: WEALTH bar tracks gold (200 = full), all bars glide
//! toward their true meters and show a WHITE tick at the unreached target
//! until they settle. Pixel-pinned on the framebuffer.
use defender_of_the_crown::campaign::game::{Difficulty, PLAYER};
use defender_of_the_crown::engine::gfx::{Framebuffer, GOLD, GRAY, GREEN, NAVY, WHITE};
use defender_of_the_crown::ui::CampaignUi;

// WEALTH track: rect(38,214,42,6), fill starts x=39 row 215..=219.
const WX: u32 = 39;
const WY: u32 = 216;

#[test]
fn wealth_bar_tracks_gold_and_rests_without_marker() {
    let mut ui = CampaignUi::new(5, Difficulty::Normal);
    let gold = ui.g.gi(PLAYER);
    let mut fb = Framebuffer::new();
    ui.draw(&mut fb);
    assert_eq!(ui.bars_disp[0], gold.min(200) / 2, "disp starts settled");
    assert_eq!(fb.get(WX, WY), GOLD, "fill present");
    // where a marker WOULD sit if the bar were moving: still track interior
    let mx = WX + (gold.min(200) / 2 * 40) / 100;
    assert_ne!(fb.get(mx, 214), WHITE, "settled bar carries no marker");
}

#[test]
fn drained_gold_glides_to_target_with_white_tick() {
    let mut ui = CampaignUi::new(5, Difficulty::Normal);
    ui.g.gold[PLAYER.index()] = 10; // wealth target drops to 5 -> w = 2
    ui.tick(); // one step of the ~8-tick glide
    assert!(ui.bars_disp[0] > 5, "fill jumped straight to target");
    let mut fb = Framebuffer::new();
    ui.draw(&mut fb);
    let x45 = WX + 6;
    assert_eq!(fb.get(x45, WY), GOLD, "glide shows the OLD longer fill");
    let mx = WX + (5 * 40) / 100;
    assert_eq!(fb.get(mx, 214), WHITE, "unreached target unmarked");

    for _ in 0..40 {
        ui.tick();
    }
    assert_eq!(ui.bars_disp[0], 5, "never converged");
    let mut fb2 = Framebuffer::new();
    ui.draw(&mut fb2);
    assert_ne!(fb2.get(x45, WY), GOLD, "stale fill outstayed the glide");
    assert_eq!(fb2.get(WX, WY), GOLD, "settled fill remains");
    assert_eq!(fb2.get(mx, 214), GRAY, "marker sticks after settling");
}

#[test]
fn provisions_bar_glides_like_the_rest() {
    let mut ui = CampaignUi::new(5, Difficulty::Normal);
    let start = ui.bars_disp[1];
    ui.g.bars[PLAYER.index()].provisions = 7;
    ui.tick();
    assert!(
        ui.bars_disp[1] < start && ui.bars_disp[1] > 7,
        "food fill neither lagged nor snapped"
    );
    let mut fb = Framebuffer::new();
    ui.draw(&mut fb);
    // FOOD track: rect(118,214,42,6), marker at 119 + 7*40/100 = 121.
    assert_eq!(fb.get(119, WY), GREEN, "food fill gone too early");
    assert_eq!(fb.get(121, 214), WHITE, "food target unmarked");
    for _ in 0..40 {
        ui.tick();
    }
    assert_eq!(ui.bars_disp[1], 7);
    let mut fb2 = Framebuffer::new();
    ui.draw(&mut fb2);
    assert_eq!(fb2.get(119, WY), GREEN, "settled fill missing");
    assert_eq!(fb2.get(121, WY), NAVY, "food fill overran its glide");
    assert_eq!(fb2.get(121, 214), GRAY, "marker sticks after settling");
}
