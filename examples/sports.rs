use defender_of_the_crown::minigames::framework::pick;
fn main() {
    for t in 1..=30 {
        let seed = ((t as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15;
        let s = pick(t, seed);
        println!("turn {t}: {}", s.map_or("-".into(), |s| s.id.to_string()));
    }
}
