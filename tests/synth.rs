//! T108 proofs: synth effects are well-formed (finite, audible, quiet
//! tails, deterministic); the music manager mixes, clamps and reaps voices.
use defender_of_the_crown::engine::audio::modplayer::{Mod, Player};
use defender_of_the_crown::engine::audio::music::Music;
use defender_of_the_crown::engine::audio::synth::{play, Sfx, SR};

const ALL: [Sfx; 6] = [Sfx::Blip, Sfx::Coin, Sfx::Sword, Sfx::Hoof, Sfx::Boom, Sfx::Crowd];

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32).sqrt()
}

fn zcr(x: &[f32]) -> usize {
    x.windows(2).filter(|w| w[0].signum() != w[1].signum() && w[0] != 0.0).count()
}

#[test]
fn every_effect_is_finite_audible_and_starts_quiet() {
    for k in ALL {
        let a = play(k);
        let b = play(k);
        assert_eq!(a, b, "{k:?} must be deterministic");
        assert!(a.len().is_multiple_of(2) && !a.is_empty());
        assert!(a.iter().all(|s| s.is_finite()), "{k:?} NaN/inf");
        assert!(rms(&a) > 0.01, "{k:?} inaudible: rms {}", rms(&a));
        assert!(
            a.iter().all(|s| s.abs() <= 1.0001),
            "{k:?} clipped beyond full scale"
        );
        let tail = &a[a.len() - 64..];
        assert!(
            tail.iter().all(|s| s.abs() < 0.02),
            "{k:?} tail not quiet: peak {}",
            tail.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
        );
    }
}

#[test]
fn coin_steps_up_in_pitch() {
    let a = play(Sfx::Coin);
    let w1 = &a[(SR as usize * 20 / 1000) * 2..(SR as usize * 80 / 1000) * 2]; // 20..80 ms tone 1
    let w2 = &a[(SR as usize * 100 / 1000) * 2..(SR as usize * 160 / 1000) * 2]; // 100..160 ms tone 2
    let (z1, z2) = (zcr(w1), zcr(w2));
    let r = z2 as f32 / z1 as f32;
    assert!((r - 1.335).abs() < 0.2, "pitch ratio {r} not 1319/988");
}

#[test]
fn stinger_mixes_over_silence_and_is_reaped() {
    let mut m = Music::new();
    m.stinger(vec![0.5; 100], 0.5);
    assert_eq!(m.voices(), 1);
    let mut out = vec![0.0f32; 40];
    m.render(&mut out, SR);
    assert!(out.iter().all(|s| (*s - 0.25).abs() < 1e-6)); // 0.5 * 0.5
    let mut out2 = vec![0.0f32; 100];
    m.render(&mut out2, SR);
    assert_eq!(m.voices(), 0);
    assert!(!out2.iter().all(|s| *s == 0.0)); // remainder consumed
    let mut out3 = vec![0.0f32; 16];
    m.render(&mut out3, SR);
    assert!(out3.iter().all(|s| *s == 0.0)); // silence again
}

#[test]
fn overlapping_voices_hard_clamp() {
    let mut m = Music::new();
    m.stinger(vec![0.9; 64], 1.0);
    m.stinger(vec![0.9; 64], 1.0);
    let mut out = vec![0.0f32; 64];
    m.render(&mut out, SR);
    assert!(out.iter().all(|s| *s <= 1.0 && *s >= -1.0));
    assert!(out.contains(&1.0)); // actually clamped
}

#[test]
fn sfx_rides_on_top_of_the_track() {
    let bytes = include_bytes!("../assets/music/test.mod");
    let mk = || {
        let mut m = Music::new();
        m.set_track(Some(Player::new(Mod::parse(bytes).unwrap())));
        m
    };
    // two identical managers: one gets an stinger -> output must differ
    let mut a = mk();
    let mut b = mk();
    b.sfx(Sfx::Blip);
    let mut oa = vec![0.0f32; 2048 * 2];
    let mut ob = vec![0.0f32; 2048 * 2];
    a.render(&mut oa, SR);
    b.render(&mut ob, SR);
    assert!(oa.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
    let diff = oa
        .iter()
        .zip(&ob)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max);
    assert!(diff > 0.05, "stinger inaudible over track");
    assert_eq!(b.voices(), 1);
    // drain the blip (0.07 s @ 44.1k = ~3087 samples > one 2048 block)
    for _ in 0..4 {
        b.render(&mut ob, SR);
    }
    assert_eq!(b.voices(), 0);
}
