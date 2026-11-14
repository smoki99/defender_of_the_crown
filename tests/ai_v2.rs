//! AI v2 contract (SPEC 6.1): the Norman war council must beat an idle
//! player within the chronicle's horizon, a harder council must beat an
//! easier one head-to-head, and the same seed must replay the same war.
use defender_of_the_crown::campaign::ai;
use defender_of_the_crown::campaign::game::{Difficulty, Faction, GameState};

fn norman_beats_idle(diff: Difficulty, seed: u64) -> bool {
    let mut g = GameState::new(seed);
    g.difficulty[1] = diff;
    let mut t = 0;
    while g.over.is_none() && t < 100 {
        g.end_player_turn();
        t += 1;
    }
    g.over == Some(Faction::Norman)
}

#[test]
fn the_council_conquereth_a_dormant_crown() {
    for diff in [Difficulty::Normal, Difficulty::Hard] {
        let wins = (1..=10u64).filter(|s| norman_beats_idle(diff, *s)).count();
        assert!(wins >= 7, "{diff:?} wins only {wins}/10 vs an idle throne");
    }
}

#[test]
fn a_harder_council_outwinneth_an_easier_one() {
    let mut hwins = 0;
    for seed in 1..=10u64 {
        let mut g = GameState::new(seed);
        g.difficulty[1] = Difficulty::Hard;
        g.difficulty[0] = Difficulty::Easy;
        let mut t = 0;
        while g.over.is_none() && t < 100 {
            ai::take_turn(&mut g, Faction::Saxon);
            g.end_player_turn();
            t += 1;
        }
        if g.over == Some(Faction::Norman) {
            hwins += 1;
        }
    }
    assert!(hwins >= 7, "Hard beat Easy only {hwins}/10 head-to-head");
}

fn fingerprint(seed: u64) -> (u32, u8, u64, [u8; 12]) {
    let mut g = GameState::new(seed);
    g.difficulty[1] = Difficulty::Hard;
    while g.over.is_none() && g.turn < 100 {
        g.end_player_turn();
    }
    let mut owners = [0u8; 12];
    for (i, s) in g.strongholds.iter().enumerate() {
        owners[i] = s.owner.index() as u8 * 40 + s.garrison.len().min(39) as u8;
    }
    let mut sum: u64 = 0;
    for f in [Faction::Saxon, Faction::Norman] {
        sum = sum
            .wrapping_mul(31)
            .wrapping_add(g.gi(f) as u64)
            .wrapping_add(g.troops_of(f) as u64);
    }
    (g.turn, g.over.map_or(9, |f| f.index() as u8), sum, owners)
}

#[test]
fn the_same_seed_playeth_the_same_war() {
    for seed in 1..=10u64 {
        assert_eq!(fingerprint(seed), fingerprint(seed), "seed {seed} drifts");
    }
}
