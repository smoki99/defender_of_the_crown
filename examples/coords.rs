//! Emits the screen coordinates the headless browser harness taps.
use defender_of_the_crown::campaign::game::Difficulty;
use defender_of_the_crown::ui::{action_rects, castle_screen, end_turn_rect, CampaignUi};

use defender_of_the_crown::engine::fonts::glyph;
fn main() {
    let ui = CampaignUi::new(7, Difficulty::Normal);
    let s =
        ui.g.strongholds
            .iter()
            .find(|s| s.name == "Buckingham")
            .expect("buckingham");
    let (bx, by) = castle_screen(s);
    let levy = action_rects(true)
        .into_iter()
        .find(|b| b.id == "rec:0")
        .unwrap();
    let end = end_turn_rect();
    println!("BUCK {bx} {by}");
    let lon = castle_screen(&ui.g.strongholds[6]); // London - Norman capital, Buckingham-adjacent
    println!("LON {lx} {ly}", lx = lon.0, ly = lon.1);
    println!(
        "LEVY {} {}",
        levy.x + levy.w as i32 / 2,
        levy.y + levy.h as i32 / 2
    );
    println!(
        "END {} {}",
        end.x + end.w as i32 / 2,
        end.y + end.h as i32 / 2
    );
    // Title-screen NORMAL banner center (web.rs diff_rect(1)).
    println!("NORMAL {} {}", 46 + 80 + 72 / 2, 170 + 14 / 2);
    // Digit glyph bitmaps so the harness can decode the HUD turn counter.
    for d in b'0'..=b'9' {
        println!(
            "DIGIT {} {}",
            d - b'0',
            glyph(d as char)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
}
