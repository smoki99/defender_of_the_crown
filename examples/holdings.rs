use defender_of_the_crown::campaign::{ai, game::{Faction, GameState}};
fn main() {
    for seed in [7u64, 11, 23, 101] {
        let mut g = GameState::new(seed);
        for _ in 0..400 {
            if g.over.is_some() { break; }
            ai::take_turn(&mut g, Faction::Saxon);
            g.end_player_turn();
        }
        println!("seed {seed}: turn={} over={:?} s={} n={}", g.turn, g.over, g.ids_of(Faction::Saxon).len(), g.ids_of(Faction::Norman).len());
    }
}
