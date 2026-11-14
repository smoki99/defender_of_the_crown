//! T108: procedural SFX synth — every effect is a deterministic burst of
//! stereo f32 PCM at [`SR`], one allocation per effect.
//! Pure Rust: renders and asserts on host, ships to wasm unchanged.

/// Reference sample rate for generated effects.
pub const SR: u32 = 44_100;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    Blip,
    Coin,
    Sword,
    Hoof,
    Boom,
    Crowd,
}

/// Generate a stereo-interleaved effect buffer.
pub fn play(kind: Sfx) -> Vec<f32> {
    match kind {
        Sfx::Blip => blip(),
        Sfx::Coin => coin(),
        Sfx::Sword => sword(),
        Sfx::Hoof => hoof(),
        Sfx::Boom => boom(),
        Sfx::Crowd => crowd(),
    }
}

/// Linear attack/decay envelope, 0 outside `[0, dur)`.
#[inline]
fn env_ad(t: f32, attack: f32, dur: f32) -> f32 {
    if t < 0.0 || t >= dur {
        0.0
    } else if t < attack {
        t / attack
    } else {
        1.0 - (t - attack) / (dur - attack)
    }
}

/// xorshift PRNG — deterministic "noise" so tests are reproducible.
struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as i32 as f32) / 2_147_483_648.0
    }
}

fn build<F>(dur: f32, mut f: F) -> Vec<f32>
where
    F: FnMut(f32, usize) -> (f32, f32),
{
    let n = (dur * SR as f32) as usize;
    let mut out = vec![0.0f32; n * 2];
    for i in 0..n {
        let (l, r) = f(i as f32 / SR as f32, i);
        out[2 * i] = l;
        out[2 * i + 1] = r;
    }
    out
}

/// UI confirm: square blip stepping up in pitch.
fn blip() -> Vec<f32> {
    build(0.07, |t, _| {
        let f = if t < 0.035 { 660.0 } else { 990.0 };
        let s = if (t * f) % 1.0 < 0.5 { 1.0 } else { -1.0 };
        let a = 0.35 * env_ad(t, 0.004, 0.07);
        (s * a, s * a)
    })
}

/// Treasure pickup: two-tone bell arpeggio.
fn coin() -> Vec<f32> {
    build(0.28, |t, _| {
        let f = if t < 0.09 { 988.0 } else { 1319.0 };
        let a = 0.4 * env_ad(t, 0.006, 0.28);
        let s = (t * f).sin() * a;
        (s, s)
    })
}

/// Sword clash: noise sweep down plus metallic ring.
fn sword() -> Vec<f32> {
    let mut nz = Noise(0x9E37_79B9);
    build(0.16, move |t, _| {
        let sweep = ((2400.0 - 14000.0 * t) * t).sin();
        let ring = (7400.0 * t).sin() * 0.4;
        let a = 0.45 * env_ad(t, 0.002, 0.16);
        let s = (nz.next() * 0.6 + sweep * 0.5 + ring) * a;
        (s, s * 0.85)
    })
}

/// Cavalry step: soft thud plus initial click.
fn hoof() -> Vec<f32> {
    build(0.09, |t, i| {
        let thud = (110.0 * t).sin() * env_ad(t, 0.002, 0.09);
        let click = if i < 64 { 0.3 * (i as f32 * 2.1).sin() } else { 0.0 };
        let a = 0.5 * (thud + click);
        (a, a)
    })
}

/// Catapult impact: descending sub thump plus noise tail.
fn boom() -> Vec<f32> {
    let mut nz = Noise(0x51ED_2701);
    build(0.55, move |t, _| {
        let sub = ((80.0 - 40.0 * t) * t).sin() * env_ad(t, 0.004, 0.3);
        let tail = nz.next() * env_ad(t, 0.004, 0.55);
        let a = 0.55 * (sub + tail * 0.7);
        (a, a * 0.95)
    })
}

/// Distant army murmur: lowpassed, amplitude-modulated noise bed.
fn crowd() -> Vec<f32> {
    let mut nz = Noise(0xC0FF_EE11);
    let mut lp = 0.0f32;
    build(1.5, move |t, _| {
        lp = 0.95 * lp + 0.05 * nz.next(); // one-pole lowpass
        let modu = 0.5 + 0.5 * (1.7 * t).sin();
        let a = lp * modu * env_ad(t, 0.25, 1.5);
        (a, a)
    })
}
