//! Emits the exact 8x8 glyph bitmap `draw_text`/`draw_text_shadow` paints
//! for a label, so browser drivers can verify on-canvas text pixel-exact.
//! Format: "@<label>" then 8 rows of '#'/'.' (one cell per char, MSB first).
use defender_of_the_crown::engine::fonts::glyph;

fn main() {
    let label = std::env::args().nth(1).expect("label");
    println!("@{label}");
    for row in 0..8 {
        let mut line = String::new();
        for ch in label.chars() {
            let bits = glyph(ch)[row];
            for col in 0..8 {
                line.push(if bits & (0x80 >> col) != 0 { '#' } else { '.' });
            }
        }
        println!("{line}");
    }
}
