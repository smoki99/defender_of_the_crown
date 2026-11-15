//! T108: music manager — one looping MOD track plus a pool of one-shot
//! SFX voices mixed into the same output stream. Pure logic: the web sink
//! just pumps [`Music::render`] at the AudioContext rate.

use crate::engine::audio::modplayer::Player;
use crate::engine::audio::synth::{self, Sfx};

struct Voice {
    buf: Vec<f32>,
    pos: usize,
    amp: f32,
}

pub struct Music {
    track: Option<Player>,
    /// Gain ramp of the current bed: 0 -> 1 over ~25 ms after set_track,
    /// so a fresh bed never switches on mid-sample (T902 click gate).
    track_fade: f32,
    /// Bed just replaced, ramping out over the same window (T902).
    tail: Option<Player>,
    tail_gain: f32,
    /// One-shot MOD payload over the bed, dropped when done (T308).
    sting: Option<Player>,
    voices: Vec<Voice>,
}

impl Default for Music {
    fn default() -> Self {
        Self::new()
    }
}

impl Music {
    pub fn new() -> Self {
        Self {
            track: None,
            track_fade: 0.0,
            tail: None,
            tail_gain: 0.0,
            sting: None,
            voices: Vec::new(),
        }
    }

    /// Replace the looping background track (None stops it). The outgoing
    /// bed keeps playing as a tail and fades out on an equal-power curve
    /// while the new bed fades in over 20 ms: the summed level is
    /// continuous at the swap, no mid-sample click (T902 mix gate).
    pub fn set_track(&mut self, p: Option<Player>) {
        self.tail = self.track.take();
        self.tail_gain = self.track_fade;
        self.track = p;
        self.track_fade = 0.0;
    }

    pub fn track(&self) -> Option<&Player> {
        self.track.as_ref()
    }
    /// Fire a parsed one-shot module (Norman turn-call, T308).
    pub fn sting(&mut self, p: Player) {
        self.sting = Some(p);
    }
    pub fn sting_playing(&self) -> bool {
        self.sting.is_some()
    }

    /// Fire a generated effect.
    pub fn sfx(&mut self, kind: Sfx) {
        // 0.55 bed + 0.4 one-shot: even a peak-aligned sample stays under
        // full scale; sfx_never_blows_the_bed_past_full_scale pins it.
        self.stinger(synth::play(kind), 0.4);
    }

    /// Mix an arbitrary stereo buffer over the track, then drop it.
    pub fn stinger(&mut self, buf: Vec<f32>, amp: f32) {
        if buf.is_empty() {
            return;
        }
        self.voices.push(Voice { buf, pos: 0, amp });
    }

    pub fn voices(&self) -> usize {
        self.voices.len()
    }

    /// Render `out.len()/2` stereo frames; track fills the bed, voices mix
    /// on top, result hard-clamped to [-1, 1]. Finished voices are dropped.
    pub fn render(&mut self, out: &mut [f32], sr: u32) {
        out.fill(0.0);
        // Bed master 0.55 (raw beds peak near full scale, T902 headroom)
        // with a per-sample equal-power curve: a fresh bed fades in over
        // 20 ms (click gate); the replaced bed rides out over 300 ms,
        // bridging the near-silent opening bars of the incoming module so
        // the 5 ms envelope never dips (T902 mix gate).
        const BED: f32 = 0.55;
        const HALF_PI: f32 = core::f32::consts::FRAC_PI_2;
        let step = 1.0 / (0.02 * sr as f32);
        let tail_step = 1.0 / (0.3 * sr as f32);
        if let Some(p) = self.track.as_mut() {
            p.render(out, sr);
            let n = out.len() as f32;
            let f0 = self.track_fade;
            for (i, s) in out.iter_mut().enumerate() {
                let f = (f0 + step * i as f32 / n).min(1.0);
                *s *= BED * (f * HALF_PI).sin();
            }
            self.track_fade = (f0 + step).min(1.0);
        }
        if self.tail.is_some() {
            let mut tmp = vec![0.0f32; out.len()];
            if let Some(t) = self.tail.as_mut() {
                t.render(&mut tmp, sr);
            }
            let n = out.len() as f32;
            let f0 = self.tail_gain;
            for (i, (o, x)) in out.iter_mut().zip(tmp).enumerate() {
                let f = (f0 - tail_step * i as f32 / n).max(0.0);
                *o += x * BED * (f * HALF_PI).sin();
            }
            self.tail_gain = (f0 - tail_step).max(0.0);
            if self.tail_gain <= 0.0 || self.tail.as_ref().is_some_and(|t| t.done()) {
                self.tail = None;
            }
        }
        if self.sting.is_some() {
            let mut tmp = vec![0.0f32; out.len()];
            if let Some(p) = self.sting.as_mut() {
                p.render(&mut tmp, sr);
            }
            for (o, t) in out.iter_mut().zip(tmp) {
                *o += t * 0.45;
            }
            if self.sting.as_ref().is_some_and(|p| p.done()) {
                self.sting = None;
            }
        }
        for v in self.voices.iter_mut() {
            let mut i = 0;
            while i < out.len() && v.pos < v.buf.len() {
                out[i] += v.buf[v.pos] * v.amp;
                v.pos += 1;
                i += 1;
            }
        }
        self.voices.retain(|v| v.pos < v.buf.len());
        for s in out.iter_mut() {
            *s = s.clamp(-1.0, 1.0);
        }
    }
}
