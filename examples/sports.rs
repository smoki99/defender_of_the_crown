//! Seasonal-meet oracle for the headless harness. Bare run prints the
//! deterministic sport draw per turn (T706 pacing); run with turn args it
//! additionally emits the exact 8x8 glyph bitmap the SPORTS button paints
//! for that season, so a driver can verify the on-canvas label pixel-exact.
use defender_of_the_crown::engine::fonts::glyph;
use defender_of_the_crown::minigames::framework::pick;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        for t in 1..=30 {
            let seed = ((t as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15;
            let s = pick(t, seed);
            println!("turn {t}: {}", s.map_or("-".into(), |s| s.id.to_string()));
        }
        return;
    }
    for a in &args {
        let t: u32 = a.parse().expect("turn number");
        let seed = ((t as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15;
        let s = pick(t, seed);
        let label = s.map_or("NO MEET", |s| s.label);
        println!("@{t} {} {label}", s.map_or("-", |s| s.id));
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
}
