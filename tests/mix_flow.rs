//! T902 audio mix gates, offline and deterministic: every shipped bed
//! must sit at an audible, unclipped level; a bed change must be
//! click-free (the old bed fades out, the new fades in - no silent gap,
//! no spike); and the loudest SFX one-shots stay inside the headroom.
//!
//! Note on metric: sample-to-sample deltas are useless here - the MOD
//! waveforms are saw/pulse, whose reset edges ARE the sound. Continuity
//! is judged on a 5 ms RMS envelope instead.
use defender_of_the_crown::engine::audio::{Mod, Music, Player, Sfx};

const SR: usize = 48_000;

fn player(name: &str, all: &[(&str, &[u8])]) -> Player {
    let bytes = all.iter().find(|(n, _)| *n == name).map(|(_, b)| *b).unwrap();
    Player::new(Mod::parse(bytes).unwrap())
}

const TRACKS: &[(&str, &[u8])] = &[
    ("title", include_bytes!("../assets/music/title.mod")),
    ("saxon", include_bytes!("../assets/music/saxon.mod")),
    ("battle", include_bytes!("../assets/music/battle.mod")),
    ("minigame", include_bytes!("../assets/music/minigame.mod")),
    ("victory", include_bytes!("../assets/music/victory.mod")),
    ("defeat", include_bytes!("../assets/music/defeat.mod")),
];

/// Render `secs` starting `pre` seconds before a set_track(from -> to)
/// event (from == "" means cold start). Returns (peak, env_min, env_med):
/// peak over everything; the 5 ms envelope stats over a 600 ms window
/// around the swap - a bed change must be click-free THERE, while quiet
/// composition bars elsewhere are no transition gap. Cold start: from
/// the first 250 ms on (skip the intro fade).
fn run(from: &str, to: &str, secs: usize) -> (f32, f32, f32) {
    let mut music = Music::new();
    if !from.is_empty() {
        music.set_track(Some(player(from, TRACKS)));
    }
    const CHUNK: usize = 256;
    let mut buf = vec![0.0f32; CHUNK * 2];
    let total = secs * SR;
    let switch_at = if from.is_empty() { 0 } else { 2 * SR };
    let mut peak = 0.0f32;
    let mut env: Vec<f32> = vec![];
    let mut acc = 0.0f64;
    let mut c = 0usize;
    let w = SR / 200; // 5 ms
    for f in 0..total {
        if f == switch_at && !to.is_empty() {
            music.set_track(Some(player(to, TRACKS)));
        }
        if f % CHUNK == 0 {
            music.render(&mut buf, SR as u32);
        }
        let s = buf[(f % CHUNK) * 2]; // left channel
        peak = peak.max(s.abs());
        if f >= SR / 4
            && (from.is_empty() || (switch_at - SR / 5..switch_at + 2 * SR / 5).contains(&f))
        {
            acc += (s as f64) * (s as f64);
            c += 1;
            if c == w {
                env.push((acc / w as f64).sqrt() as f32);
                acc = 0.0;
                c = 0;
            }
        }
    }
    env.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (peak, env[0], env[env.len() / 2])
}

#[test]
fn every_bed_sits_at_an_audible_unclipped_level() {
    for (name, _) in TRACKS {
        let (peak, _, med) = run("", name, 30);
        assert!(peak > 0.05, "{name}: bed inaudible (peak {peak:.3})");
        assert!(peak < 0.995, "{name}: bed clips (peak {peak:.3})");
        assert!(med > 0.02, "{name}: bed too quiet (median {med:.3})");
    }
}

#[test]
fn bed_changes_never_gap_or_spike() {
    // A cut mid-sample leaves either a dead 5 ms window or a step
    // spike; the crossfade must keep both envelopes alive through the
    // swap. Exercise every directed pair between different beds.
    for (a, _) in TRACKS {
        for (b, _) in TRACKS {
            if a == b {
                continue;
            }
            let (peak, emin, emed) = run(a, b, 4);
            assert!(peak < 0.995, "{a} -> {b}: transition spikes (peak {peak:.3})");
            assert!(
                emin > 0.15 * emed,
                "{a} -> {b}: transition gap (env_min {emin:.4}, median {emed:.4})"
            );
        }
    }
}

#[test]
fn sfx_never_blows_the_bed_past_full_scale() {
    for (name, _) in TRACKS {
        for kind in [Sfx::Sword, Sfx::Coin, Sfx::Hoof, Sfx::Blip] {
            let peak = with_sfx(name, kind);
            assert!(peak < 0.995, "{name} + {kind:?} clips (peak {peak:.3})");
        }
    }
}

/// Peak of `bed` while all four SFX fire together at the 2 s mark -
/// worst realistic stack (turn resolution jingle).
fn with_sfx(bed: &str, extra: Sfx) -> f32 {
    let mut music = Music::new();
    music.set_track(Some(player(bed, TRACKS)));
    const CHUNK: usize = 256;
    let mut buf = vec![0.0f32; CHUNK * 2];
    let mut peak = 0.0f32;
    for f in 0..4 * SR {
        if f % CHUNK == 0 {
            if f == 2 * SR {
                music.sfx(Sfx::Sword);
                music.sfx(Sfx::Coin);
                music.sfx(Sfx::Hoof);
                music.sfx(extra);
            }
            music.render(&mut buf, SR as u32);
        }
        for s in buf.iter() {
            peak = peak.max(s.abs());
        }
    }
    peak
}
