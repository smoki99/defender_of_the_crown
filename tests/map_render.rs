//! T301 map renderer: coast shading, scenery sprites, fluttering banners
//! and the breathing selection pulse - pinned to exact framebuffer pixels.
use defender_of_the_crown::campaign::game::Difficulty;
use defender_of_the_crown::engine::gfx::{
    Framebuffer, DARKGREEN, DEEPWATER, GOLD, SAXON_BLUE, SILVER, SKY, TEAL, YELLOW,
};
use defender_of_the_crown::ui::CampaignUi;

fn ui(anim: u32) -> CampaignUi {
    let mut u = CampaignUi::new(1, Difficulty::Normal);
    u.anim = anim;
    u
}

#[test]
fn sea_shore_and_drifting_waves() {
    let mut fb = Framebuffer::new();
    ui(0).draw(&mut fb);
    // North Sea at y=100: coastline sits near x=209, so x=300 is water.
    assert_eq!(fb.get(300, 100), DEEPWATER);
    // Hatched shoreline at y=50 (Norfolk coast x~219, clear of keeps):
    // SKY interleaves the coast on even checker cells.
    assert_eq!(fb.get(220, 50), SKY);
    assert_eq!(fb.get(221, 50), DEEPWATER, "no water cell beside hatch");
    // Wave dashes drift with anim: TEAL at y=0 for anim 0, gone at anim 9.
    assert_eq!(fb.get(235, 0), TEAL);
    let mut fb2 = Framebuffer::new();
    ui(9).draw(&mut fb2);
    assert_eq!(fb2.get(235, 0), DEEPWATER, "wave did not drift");
    assert_eq!(fb2.get(235, 1), TEAL);
}

#[test]
fn forest_and_hill_scenery() {
    let mut fb = Framebuffer::new();
    ui(0).draw(&mut fb);
    assert!(fb.count(DARKGREEN) > 60, "forest canopies missing");
    assert_eq!(fb.get(129, 50), SILVER, "Midlands hill highlight missing");
}

#[test]
fn banner_flutter_is_two_frames() {
    // Buckingham (own, visible) keep centre: 125,100.
    let mut a = Framebuffer::new();
    ui(0).draw(&mut a);
    let mut b = Framebuffer::new();
    ui(8).draw(&mut b);
    assert_eq!(a.get(121, 87), SAXON_BLUE, "frame 0 flag high");
    assert_ne!(b.get(121, 87), SAXON_BLUE, "frame 1 flag must leave row 87");
    assert_eq!(b.get(121, 88), SAXON_BLUE, "frame 1 flag low");
}

#[test]
fn selection_pulse_breathes() {
    // York (175,17): outside the panel band so the ring stays visible.
    let mut a = Framebuffer::new();
    let mut u = ui(0);
    u.sel = Some(9);
    u.draw(&mut a);
    assert_eq!(a.get(166, 8), YELLOW);
    let mut b = Framebuffer::new();
    let mut u = ui(8);
    u.sel = Some(9);
    u.draw(&mut b);
    assert_eq!(b.get(165, 7), GOLD, "expanded gold ring");
    assert_ne!(b.get(166, 8), YELLOW);
}
