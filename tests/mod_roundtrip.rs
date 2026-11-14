//! T004/T003 proof: compose.ts-written .MOD files parse with our player and
//! render real PCM at the correct pitch.

use defender_of_the_crown::engine::audio::modplayer::{Mod, Player};

const TONE: &[u8] = include_bytes!("../assets/music/tone.mod");
const SONG: &[u8] = include_bytes!("../assets/music/test.mod");

fn render(bytes: &[u8], secs: f32, sr: u32) -> Vec<f32> {
    let m = Mod::parse(bytes).expect("parse");
    let mut p = Player::new(m);
    let mut out = vec![0.0f32; (secs * sr as f32) as usize * 2];
    p.render(&mut out, sr);
    out
}

#[test]
fn parse_rejects_garbage() {
    assert!(Mod::parse(&[0u8; 64]).is_err());
    assert!(Mod::parse(&[]).is_err());
}

#[test]
fn demo_song_structure() {
    let m = Mod::parse(SONG).expect("parse demo");
    assert_eq!(m.title, "DOC TEST TRACK");
    assert_eq!(m.song_len, 4);
    assert_eq!(m.samples.len(), 31); // all slots parsed
    assert_eq!(m.samples[0].data.len(), 128); // LEAD PULSE one-cycle loop
    assert_eq!(m.samples[0].loop_len, 128);
    assert_eq!(m.samples[2].loop_len, 0); // KICK one-shot
    assert_eq!(m.patterns[0][0][0].sample, 1);
    assert_eq!(m.patterns[0][0][0].fx, 0x0c); // volume expressed as C-effect
    assert_eq!(m.patterns[0][0][0].fp, 64);
}

#[test]
fn tone_renders_audible_pcm() {
    let out = render(TONE, 1.0, 44100);
    let peak = out.iter().fold(0.0f32, |a, &v| a.max(v.abs()));
    let rms = (out.iter().map(|v| v * v).sum::<f32>() / out.len() as f32).sqrt();
    assert!(peak > 0.15 && peak < 0.55, "peak {peak}");
    assert!(rms > 0.07 && rms < 0.3, "rms {rms}");
}

#[test]
fn tone_pitch_matches_period_table() {
    // C-4 => period 107 => 3 546 896 / 107 / 128-frame cycle ~= 259 Hz.
    let out = render(TONE, 1.0, 44100);
    let mut crossings = 0usize;
    let mut prev = 0.0f32;
    for (i, &v) in out.iter().enumerate().step_by(2) {
        if i > 2000 && v.signum() != prev.signum() && v != 0.0 {
            crossings += 1;
        }
        prev = v;
    }
    let hz = crossings as f32 / 2.0;
    assert!((250.0..=270.0).contains(&hz), "estimated {hz} Hz");
}

#[test]
fn demo_song_renders_mixture() {
    let out = render(SONG, 2.0, 44100);
    let peak = out.iter().fold(0.0f32, |a, &v| a.max(v.abs()));
    assert!(peak > 0.05, "demo peak {peak}");
}
