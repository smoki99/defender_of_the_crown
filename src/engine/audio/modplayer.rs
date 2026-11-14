//! Minimal ProTracker (.MOD, "M.K." 4-channel) player.
//!
//! Cell layout (verified against xmrs `patternslot.rs`):
//! `IIII PPPPPPPPPPPP IIIF DDDDDDDD` — instrument split across the two
//! high nibbles, 12-bit period, effect nibble + 8-bit data. There is **no**
//! volume column; volume comes from `C` (set volume, 0-64) and the sample.
//!
//! Pitch: Amiga Paula plays sample data at `PAULA / period` Hz where the
//! effective clock is 7 093 792 / 2. Timing: `bpm / 5` ticks per second.
//! Pure, deterministic, dependency-free — testable on host, runs in wasm.

/// Effective Amiga sample-rate clock (Hz per 1/period).
pub const PAULA: f32 = 3_546_896.0;

pub struct Sample {
    pub data: Vec<i8>,
    /// 0-64 (ProTracker sample volume).
    pub volume: u8,
    /// Loop region in sample frames; `loop_len == 0` means one-shot.
    pub loop_start: usize,
    pub loop_len: usize,
}

#[derive(Clone, Copy, Default)]
pub struct Cell {
    pub period: u16,
    pub sample: u8,
    pub fx: u8,
    pub fp: u8,
}

pub struct Mod {
    pub title: String,
    pub samples: Vec<Sample>,
    /// Parsed pattern slots; classic MODs store exactly 15.
    pub patterns: Vec<Vec<[Cell; 4]>>,
    pub order: Vec<u8>,
    pub song_len: usize,
}

impl Mod {
    pub fn parse(bytes: &[u8]) -> Result<Mod, String> {
        if bytes.len() < 1084 || &bytes[1080..1084] != b"M.K." {
            return Err("not an M.K. module".into());
        }
        let title = String::from_utf8_lossy(&bytes[0..20]).trim_end_matches(|c| c <= ' ').to_string();
        let mut samples = Vec::with_capacity(31);
        let mut pcm_sizes = Vec::with_capacity(31);
        for s in 0..31 {
            let o = 20 + s * 30;
            let len_words = u16::from_be_bytes([bytes[o + 22], bytes[o + 23]]) as usize;
            samples.push(Sample {
                data: Vec::new(),
                volume: bytes[o + 25].min(64),
                loop_start: u16::from_be_bytes([bytes[o + 28], bytes[o + 29]]) as usize * 2,
                loop_len: u16::from_be_bytes([bytes[o + 26], bytes[o + 27]]) as usize * 2,
            });
            pcm_sizes.push(len_words * 2);
        }
        let song_len = bytes[950] as usize;
        if song_len == 0 || song_len > 128 {
            return Err("bad song length".into());
        }
        let order = bytes[952..1080].to_vec();
        let mut pat_base = 1084usize;
        let mut patterns = Vec::with_capacity(15);
        for _ in 0..15 {
            let end = pat_base + 1024;
            if end > bytes.len() {
                return Err("truncated pattern data".into());
            }
            let mut rows = Vec::with_capacity(64);
            for r in 0..64 {
                let row: [Cell; 4] = std::array::from_fn(|c| {
                    let o = pat_base + r * 16 + c * 4;
                    Cell {
                        period: (((bytes[o] & 0x0f) as u16) << 8) | bytes[o + 1] as u16,
                        sample: (bytes[o] & 0xf0) | (bytes[o + 2] >> 4),
                        fx: bytes[o + 2] & 0x0f,
                        fp: bytes[o + 3],
                    }
                });
                rows.push(row);
            }
            patterns.push(rows);
            pat_base = end;
        }
        for (s, size) in pcm_sizes.iter().enumerate() {
            let end = pat_base + size;
            if end > bytes.len() {
                return Err("truncated sample data".into());
            }
            samples[s].data = bytes[pat_base..end].iter().map(|b| *b as i8).collect();
            pat_base = end;
        }
        Ok(Mod { title, samples, patterns, order, song_len })
    }
}

#[derive(Clone, Copy)]
struct Channel {
    period: u16,
    vol: u8,
    sample: usize,
    /// Read position in sample data, in frames (fractional).
    pos: f32,
    active: bool,
    vib_phase: u8,
}

const CH_INIT: Channel =
    Channel { period: 0, vol: 64, sample: 0, pos: 0.0, active: false, vib_phase: 0 };

static VIB_SIN: [f32; 32] = [
    0.0, 0.195, 0.383, 0.556, 0.707, 0.831, 0.924, 0.981, 1.0, 0.981, 0.924, 0.831, 0.707, 0.556,
    0.383, 0.195, 0.0, -0.195, -0.383, -0.556, -0.707, -0.831, -0.924, -0.981, -1.0, -0.981,
    -0.924, -0.831, -0.707, -0.556, -0.383, -0.195,
];

pub struct Player {
    m: Mod,
    pos: usize,
    row: usize,
    /// Index of the *current* tick within the row (during rendering).
    tick: usize,
    speed: u8,
    bpm: u8,
    ch: [Channel; 4],
    /// Held per-channel effect (fx, param) driving tick-rate modulation.
    row_fx: [(u8, u8); 4],
    /// Frames accumulated toward the next tick boundary.
    frame_acc: f32,
    spt: f32,
    sr: f32,
    jmp: Option<(u8, usize)>,
}

impl Player {
    pub fn new(m: Mod) -> Player {
        Player {
            m,
            pos: 0,
            row: 0,
            tick: 0,
            speed: 6,
            bpm: 125,
            ch: [CH_INIT; 4],
            row_fx: [(0, 0); 4],
            frame_acc: 0.0,
            spt: 1.0,
            sr: 0.0,
            jmp: None,
        }
    }

    pub fn module(&self) -> &Mod {
        &self.m
    }
    pub fn position(&self) -> (usize, usize, usize) {
        (self.pos, self.row, self.tick.wrapping_sub(1))
    }

    /// Render stereo-interleaved f32 into `out` at `sr` Hz.
    pub fn render(&mut self, out: &mut [f32], sr: u32) {
        self.sr = sr as f32;
        self.spt = (sr as f32) * 5.0 / self.bpm as f32;
        if self.frame_acc <= 0.0 {
            self.frame_acc = self.spt; // fire the first tick at frame 0
        }
        for frame in out.chunks_exact_mut(2) {
            if self.frame_acc >= self.spt {
                self.frame_acc -= self.spt;
                self.do_tick();
            }
            let v = self.mix_frame();
            frame[0] = v;
            frame[1] = v;
            self.frame_acc += 1.0;
        }
    }

    fn cur_pattern(&self) -> &[[Cell; 4]] {
        let pat = self.m.order[self.pos % self.m.song_len.max(1)] as usize;
        &self.m.patterns[pat % self.m.patterns.len()]
    }

    /// Start of one tick: on tick 0 process the row cells (notes, one-shot
    /// effects), then advance tick/row/position counters.
    fn do_tick(&mut self) {
        if self.tick == 0 {
            if let Some((pos, row)) = self.jmp.take() {
                self.pos = (pos as usize).min(self.m.song_len - 1);
                self.row = row.min(63);
            }
            let row_cells = self.cur_pattern()[self.row % 64];
            for (c, &cell) in row_cells.iter().enumerate() {
                if cell.sample > 0 && (cell.sample as usize) <= self.m.samples.len() {
                    self.ch[c].sample = (cell.sample - 1) as usize;
                }
                if cell.period != 0 || cell.fx != 0 {
                    self.row_fx[c] = (cell.fx, cell.fp);
                }
                match cell.fx {
                    0x0c => self.ch[c].vol = cell.fp.min(64),
                    0x0f => {
                        if cell.fp > 0 && cell.fp < 0x20 {
                            self.speed = cell.fp;
                        } else if cell.fp >= 0x20 {
                            self.bpm = cell.fp;
                        }
                    }
                    0x0b => self.jmp = Some((cell.fp, 0)),
                    0x0d => {
                        let r = (cell.fp >> 4) * 10 + (cell.fp & 0x0f);
                        self.jmp = Some(((self.pos + 1) as u8, r as usize));
                    }
                    _ => {}
                }
                if cell.period != 0 {
                    let start = if cell.fx == 0x09 { (cell.fp as f32) * 256.0 } else { 0.0 };
                    let ch = &mut self.ch[c];
                    ch.period = cell.period;
                    ch.pos = start;
                    ch.active = true;
                    ch.vib_phase = 0;
                }
            }
        }
        self.tick += 1;
        if self.tick as u32 >= self.speed.max(1) as u32 {
            self.tick = 0;
            self.row += 1;
            if self.row >= 64 {
                self.row = 0;
                self.pos = (self.pos + 1) % self.m.song_len.max(1); // v1: loop forever
            }
        }
    }

    /// Mix one stereo frame; returns the mono value (duplicated).
    fn mix_frame(&mut self) -> f32 {
        let mut acc = 0.0f32;
        let ti = self.tick.wrapping_sub(1); // tick index within the row
        for c in 0..4 {
            let ch = &mut self.ch[c];
            if !ch.active || ch.period == 0 {
                continue;
            }
            let (fx, fp) = self.row_fx[c];
            let mut period = ch.period as f32;
            match fx {
                0x01 if ti > 0 => {
                    ch.period = ch.period.saturating_sub(fp as u16).max(32);
                    period = ch.period as f32;
                }
                0x02 if ti > 0 => {
                    ch.period = (ch.period.saturating_add(fp as u16)).min(3418);
                    period = ch.period as f32;
                }
                0x04 => {
                    let depth = (fp & 0x0f) as f32;
                    ch.vib_phase = ch.vib_phase.wrapping_add(fp >> 4);
                    period *= 1.0 + 0.0075 * depth * VIB_SIN[ch.vib_phase as usize & 31];
                }
                0x0a => {
                    let semi = match ti % 3 {
                        1 => (fp >> 4) as f32,
                        2 => (fp & 0x0f) as f32,
                        _ => 0.0,
                    };
                    period *= (2.0f32).powf(semi / 12.0);
                }
                _ => {}
            }
            let smp = &self.m.samples[ch.sample];
            if smp.data.is_empty() {
                continue;
            }
            let step = (PAULA / period / self.sr.max(1.0)).min(16.0);
            let len = smp.data.len();
            let mut pos = ch.pos + step;
            if smp.loop_len > 0 {
                let start = smp.loop_start.min(len - 1) as f32;
                let end = (smp.loop_start + smp.loop_len).min(len) as f32;
                if pos >= end {
                    pos = start + (pos - start) % (end - start);
                }
            } else if pos >= len as f32 {
                ch.active = false;
                continue;
            }
            ch.pos = pos;
            let i0 = pos as usize;
            let i1 = (i0 + 1).min(len - 1);
            let frac = pos - i0 as f32;
            let s = smp.data[i0] as f32 * (1.0 - frac) + smp.data[i1] as f32 * frac;
            acc += s * (ch.vol as f32 / 64.0) * (smp.volume as f32 / 64.0) / 128.0;
        }
        acc.clamp(-1.0, 1.0)
    }
}
