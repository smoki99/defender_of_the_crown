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
            voices: Vec::new(),
        }
    }

    /// Replace the looping background track (None stops it).
    pub fn set_track(&mut self, p: Option<Player>) {
        self.track = p;
    }

    pub fn track(&self) -> Option<&Player> {
        self.track.as_ref()
    }

    /// Fire a generated effect.
    pub fn sfx(&mut self, kind: Sfx) {
        self.stinger(synth::play(kind), 1.0);
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
        if let Some(p) = self.track.as_mut() {
            p.render(out, sr);
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
