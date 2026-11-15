//! T301 map renderer: coast shading, scenery sprites, fluttering banners
//! and the breathing selection pulse - pinned to exact framebuffer pixels.
use defender_of_the_crown::campaign::game::Difficulty;
use defender_of_the_crown::engine::gfx::{
    Framebuffer, DARKGREEN, DEEPWATER, GOLD, GRAY3, SAXON_BLUE, TAN, TEAL, YELLOW,
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
    // Dry sand band lines the coast (T901) - pin the feature, not the
    // exact column: keep sprites may overlap individual coastline pixels.
    assert!(fb.count(TAN) > 100, "sand strip missing");
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
    assert_eq!(fb.get(129, 50), GRAY3, "Midlands hill highlight missing");
}

#[test]
fn banner_flutter_is_two_frames() {
    // Buckingham (own, visible) keep: pennant area around 125,100 top.
    // The keep sprite owns the exact pennant pixels, so pin the
    // invariant: Saxon-blue flag pixels exist and MOVE between the
    // two flutter frames.
    let blues = |anim: u32| -> Vec<(i32, i32)> {
        let mut fb = Framebuffer::new();
        ui(anim).draw(&mut fb);
        let mut v = Vec::new();
        for y in 76..102u32 {
            for x in 110..142u32 {
                if fb.get(x, y) == SAXON_BLUE {
                    v.push((x as i32, y as i32));
                }
            }
        }
        v
    };
    let a = blues(0);
    let b = blues(8);
    assert!(!a.is_empty() && !b.is_empty(), "pennant missing");
    assert_ne!(a, b, "flag does not flutter");
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
