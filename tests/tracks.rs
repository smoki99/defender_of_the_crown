//! T401: every composed track decodes, carries song data, and stays within
//! the payload budget (7 embedded tracks ≤ 128 KiB raw).
use defender_of_the_crown::engine::audio::modplayer::Mod;
use defender_of_the_crown::ui::{DEFEAT_MOD, NORMAN_STING, VICTORY_MOD};

const TITLE: &[u8] = include_bytes!("../assets/music/title.mod");
const SAXON: &[u8] = include_bytes!("../assets/music/saxon.mod");
const BATTLE: &[u8] = include_bytes!("../assets/music/battle.mod");
const MINIGAME: &[u8] = include_bytes!("../assets/music/minigame.mod");

fn check(name: &str, bytes: &'static [u8], min_order: usize) {
    let m = Mod::parse(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(
        m.song_len >= min_order,
        "{name}: order length {}",
        m.song_len
    );
    assert!(!m.patterns.is_empty(), "{name}: no patterns");
    let budget = 128 * 1024 / 7;
    assert!(
        bytes.len() <= budget,
        "{name}: {} B exceeds per-track budget",
        bytes.len()
    );
}

#[test]
fn all_seven_campaign_tracks_decodeth() {
    check("title", TITLE, 4);
    check("saxon", SAXON, 2);
    check("norman", NORMAN_STING, 1);
    check("battle", BATTLE, 2);
    check("minigame", MINIGAME, 2);
    check("victory", VICTORY_MOD, 1);
    check("defeat", DEFEAT_MOD, 1);
}

#[test]
fn the_overture_loopeth_back_to_the_a_section() {
    let m = Mod::parse(TITLE).expect("title parses");
    assert!(
        m.restart > 0,
        "title must carry a restart byte into the A-section"
    );
    assert!(
        m.restart < m.song_len,
        "restart {} beyond song_len {}",
        m.restart,
        m.song_len
    );
}
