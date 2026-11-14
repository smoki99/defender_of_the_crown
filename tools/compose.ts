// compose.ts — ProTracker .MOD tracker (writer). SPEC §3: all music authored
// here, shipped as real .mod assets; playback by our own Rust player.
// Format: "M.K." 4-channel, 15 pattern slots, 31 sample slots, big-endian.
//
// usage: bun tools/compose.ts <out.mod> [tone]
//   tone mode writes a minimal single-note file used by the pitch round-trip test.

// ---------------------------------------------------------------- periods
// Amiga period table, index 0 = C-1. Tracker octave naming (C-1..C-7).
const PERIODS = [
  856, 808, 762, 720, 678, 640, 604, 570, 538, 508, 480, 454,
  428, 404, 381, 360, 340, 320, 302, 285, 269, 254, 240, 227,
  214, 202, 191, 180, 170, 160, 151, 143, 135, 127, 120, 113,
  107, 101, 95, 90, 85, 80, 75, 71, 67, 64, 60, 57,
  54, 50, 47, 45, 42, 40, 38, 35, 33, 32, 30, 28,
];
const SEMI: Record<string, number> = { C: 0, "C#": 1, D: 2, "D#": 3, E: 4, F: 5, "F#": 6, G: 7, "G#": 8, A: 9, "A#": 10, B: 11 };

export function noteToPeriod(note: string | null): number {
  if (!note || note === "---" || note === "...") return 0;
  const m = /^([A-G]#?)-?([1-7])$/.exec(note);
  if (!m) throw new Error(`bad note: ${note}`);
  const semi = SEMI[m[1]] + (parseInt(m[2]) - 1) * 12;
  return PERIODS[semi];
}

// ---------------------------------------------------------------- samples
export interface SampleDef {
  name: string;
  data: Int8Array;   // 8-bit signed, looped if loopLen > 0
  volume: number;    // 1..64
  loopStart: number; // samples
  loopLen: number;   // samples
}

// one full cycle, `len` samples -> plays at any pitch (single-cycle loop)
export function cycle(wave: "square" | "pulse" | "tri" | "saw", len = 128, amp = 100, duty = 0.5): Int8Array {
  const d = new Int8Array(len);
  for (let i = 0; i < len; i++) {
    const p = i / len;
    let v: number;
    switch (wave) {
      case "square": v = p < 0.5 ? 1 : -1; break;
      case "pulse": v = p < duty ? 1 : -0.6; break;
      case "tri": v = 4 * Math.abs(p - 0.5) - 1; break;
      case "saw": v = 2 * p - 1; break;
    }
    d[i] = Math.round(v * amp);
  }
  return d;
}

export function kick(len = 128, amp = 110): Int8Array {
  const d = new Int8Array(len);
  for (let i = 0; i < len; i++) {
    const t = i / 44100;
    const f = 180 * Math.exp(-t * 28) + 45;      // pitch sweep down
    const env = Math.exp(-t * 30);
    d[i] = Math.round(Math.sin(2 * Math.PI * f * t) * env * amp);
  }
  return d;
}

export function hat(len = 256, amp = 70): Int8Array {
  const d = new Int8Array(len);
  let x = 1;
  for (let i = 0; i < len; i++) {
    x = (x * 1103515245 + 12345) & 0x7fffffff;    // deterministic LCG
    const env = Math.exp(-(i / len) * 5);
    d[i] = Math.round(((x % 251) / 125 - 1) * env * amp);
  }
  return d;
}

// ---------------------------------------------------------------- patterns
// cell: [note, sample(0=keep), volume(0=keep), fx, fparam]
type Cell = [string | null, number, number, number, number];
const N: Cell = [null, 0, 0, 0, 0];
export type Pattern = Cell[][]; // exactly 64 rows x 4 channels

function rows(fill: Cell[][]): Pattern {
  const p: Pattern = Array.from({ length: 64 }, (_, r) => (r < fill.length ? fill[r] : [N, N, N, N]));
  return p;
}
function line(...cells: Cell[]): Cell[] {
  while (cells.length < 4) cells.push(N);
  return cells;
}
const C = (note: string | null, s = 0, v = 0, fx = 0, fp = 0): Cell => [note, s, v, fx, fp];

// ---------------------------------------------------------------- writer
export interface Song {
  title: string;
  samples: SampleDef[];   // <= 31
  patterns: Pattern[];    // <= 15, all written into the 15 slots
  order: number[];        // song length
  restart?: number;       // pattern index the song loops back to (T401)
  speed?: number;         // encoded nowhere in MOD (player default), kept for meta
}

export function buildMod(song: Song): Uint8Array {
  if (song.samples.length > 31) throw new Error("max 31 samples");
  if (song.patterns.length > 15) throw new Error("max 15 patterns (M.K.)");
  const out: number[] = [];
  const be16 = (v: number) => out.push((v >> 8) & 0xff, v & 0xff);

  // header: title 20 bytes, then 31 sample records; "M.K." lives at 1080
  const title = song.title.padEnd(20, "\x00").slice(0, 20);
  for (let i = 0; i < 20; i++) out.push(title.charCodeAt(i) & 0x7f);

  // 31 sample entries x 30 bytes
  for (let s = 0; s < 31; s++) {
    const sd = song.samples[s];
    const name = (sd?.name ?? "").padEnd(22, "\x00").slice(0, 22);
    for (let i = 0; i < 22; i++) out.push(name.charCodeAt(i) & 0x7f);
    let data = sd ? sd.data : new Int8Array(0);
    if (data.length % 2) data = new Int8Array([...data, 0]); // word-aligned
    be16(data.length / 2);
    out.push(0);                 // finetune
    out.push(sd ? sd.volume : 0);
    be16((sd?.loopLen ?? 0) / 2);
    be16((sd?.loopStart ?? 0) / 2);
  }

  // song length / restart / order table, then format tag, then patterns
  out.push(song.order.length, song.restart ?? 0x7f);
  for (let i = 0; i < 128; i++) out.push(i < song.order.length ? song.order[i] : 0);
  out.push(...[0x4d, 0x2e, 0x4b, 0x2e]); // "M.K." @1080
  for (let p = 0; p < 15; p++) {
    const pat = song.patterns[p];
    for (let r = 0; r < 64; r++) {
      for (let ch = 0; ch < 4; ch++) {
        const [note, samp, vol, fx, fp] = pat ? pat[r][ch] : N;
        const period = noteToPeriod(note);
        const [efx, efp] = vol > 0 && fx === 0 ? [0x0c, vol] : [fx, fp]; // no vol column: express as C-effect
        out.push(
          ((samp & 0xf0) | (period >> 8)) & 0xff,
          period & 0xff,
          (((samp & 0x0f) << 4) | (efx & 0x0f)) & 0xff,
          efp & 0xff,
        );
      }
    }
  }

  // sample pcm data
  for (const sd of song.samples) for (let i = 0; i < sd.data.length; i++) out.push(sd.data[i] & 0xff);

  return new Uint8Array(out);
}

// ---------------------------------------------------------------- reader (self-check)
export function readMod(buf: Uint8Array) {
  const tag = String.fromCharCode(...buf.slice(1080, 1084));
  if (tag !== "M.K.") throw new Error(`bad tag ${tag}`);
  const songLen = buf[950];
  const order = [...buf.slice(952, 1080)];
  const samples = [];
  for (let s = 0; s < 31; s++) {
    const o = 20 + s * 30;
    samples.push({
      name: String.fromCharCode(...buf.slice(o, o + 22)).trim(),
      length: ((buf[o + 22] << 8) | buf[o + 23]) * 2,
      volume: buf[o + 25],
      loopLen: ((buf[o + 26] << 8) | buf[o + 27]) * 2,
      loopStart: ((buf[o + 28] << 8) | buf[o + 29]) * 2,
    });
  }
  const patBase = 1084;
  const cell = (p: number, r: number, ch: number) => {
    const o = patBase + p * 1024 + r * 16 + ch * 4;
    return { period: ((buf[o] & 0x0f) << 8) | buf[o + 1], sample: (buf[o] & 0xf0) | (buf[o + 2] >> 4), fx: buf[o + 2] & 0x0f, fp: buf[o + 3] };
  };
  return { tag, songLen, order, samples, cell };
}

// ---------------------------------------------------------------- demo song
function demoSong(): Song {
  const samples: SampleDef[] = [
    { name: "LEAD PULSE", data: cycle("pulse", 128, 95, 0.3), volume: 54, loopStart: 0, loopLen: 128 },
    { name: "BASS SQUARE", data: cycle("square", 128, 110), volume: 60, loopStart: 0, loopLen: 128 },
    { name: "KICK", data: kick(), volume: 64, loopStart: 0, loopLen: 0 },
    { name: "HAT", data: hat(), volume: 40, loopStart: 0, loopLen: 0 },
  ];
  const leadSeq = ["A-4", "C-5", "E-5", "C-5", "D-5", "C-5", "A-4", "G-4"];
  const bassSeq = ["A-2", "A-2", "E-2", "E-2", "F-2", "F-2", "G-2", "G-2"];
  const pA = rows([]);
  const pB = rows([]);
  for (let i = 0; i < 8; i++) {
    pA[i * 4] = line(C(leadSeq[i], 1, 64), C(bassSeq[i], 2, 52), i % 2 === 0 ? C("C-2", 3, 64) : N, i % 2 === 1 ? C("C-5", 4, 48) : N);
    pB[i * 4] = line(C(leadSeq[7 - i], 1, 60, 0x0a, 0x07), C(bassSeq[7 - i], 2, 56), i % 4 === 0 ? C("C-2", 3, 64) : N, C("C-5", 4, 44));
    if (i % 2 === 0) pB[i * 4 + 2] = line(N, N, N, C("C-5", 4, 36));
  }
  return { title: "DOC TEST TRACK", samples, patterns: [pA, pB], order: [0, 1, 0, 1] };
}

function toneSong(): Song {
  const p = rows([]);
  p[0] = [C("C-4", 1, 64), N, N, N];   // period 107 -> 3546896/107/128 ~ 259 Hz
  p[16] = [C("C-4", 1, 64), N, N, N];  // retrigger
  p[32] = [C("C-4", 1, 64), N, N, N];
  p[48] = [C("C-4", 1, 64), N, N, N];
  return {
    title: "DOC TONE TEST",
    samples: [{ name: "TONE", data: cycle("tri", 128, 100), volume: 64, loopStart: 0, loopLen: 128 }],
    patterns: [p],
    order: [0],
  };
}

// The Norman turn-call (T308): a war-horn fanfare D-F-A-D over a
// war-drum, then silence - played as a one-shot stinger when the AI
// turn begins. Rows 32..63 rest so the 2 s stinger render ends clean.
function normanSong(): Song {
  const p: Pattern = rows([]);
  const s = (r: number, ch: number, cell: Cell) => { p[r][ch] = cell; };
  s(0, 3, C(null, 0, 0, 15, 1)); // F01: one tick per row (40 ms) - fast sting
  // horn calls, rising
  s(0, 0, C("D-3", 1, 54)); s(6, 0, C("D-3", 1, 54)); s(12, 0, C("F-3", 1, 56));
  s(18, 0, C("A-3", 1, 56)); s(24, 0, C("D-4", 1, 60));
  // fifth harmony under the last two calls
  s(18, 1, C("E-3", 1, 44)); s(24, 1, C("A-3", 1, 48));
  // war drum
  s(0, 2, C("C-2", 2, 64)); s(6, 2, C("C-2", 2, 56)); s(12, 2, C("C-2", 2, 64));
  s(18, 2, C("C-2", 2, 56)); s(21, 2, C("C-2", 2, 48)); s(24, 2, C("C-2", 2, 64));
  // C00 volume-off: the looped horn has no natural release (row 36 = 1.44 s)
  s(36, 0, C(null, 0, 0, 12, 0));
  s(36, 1, C(null, 0, 0, 12, 0));
  return {
    title: "NORMAN STING",
    samples: [
      { name: "HORN", data: cycle("square", 128, 100, 0.3), volume: 56, loopStart: 0, loopLen: 128 },
      { name: "DRUM", data: kick(128, 118), volume: 64, loopStart: 0, loopLen: 0 },
    ],
    patterns: [p],
    order: [0],
  };
}
// ---------------------------------------------------------------- T401 tracks
// Event: [row, durRows, note|null, volume]. A note sustains until the next
// event; dur < 64 emits a C00 note-off so short articulations stay crisp.
type Ev = [number, number, string, number];
type Ch = { smp: number; evs: Ev[] } | null;

function mkPat(chans: [Ch, Ch, Ch, Ch]): Pattern {
  const p: Pattern = rows([]);
  chans.forEach((c, ch) => {
    if (!c) return;
    for (const [r, d, note, vol] of c.evs) {
      p[r][ch] = C(note, c.smp, vol);
      if (d > 0 && r + d < 64) p[r + d][ch] = C(null, 0, 0, 12, 0);
    }
  });
  return p;
}

// rolled chord: notes fire dur rows apart and the last one holds
const roll = (start: number, dur: number, notes: string[], vol: number): Ev[] =>
  notes.map((n, i) => [start + i * dur, i === notes.length - 1 ? -1 : dur, n, vol] as Ev);

// 16th grid run (dur 4 rows) starting at row r0
const run = (r0: number, notes: string[], vol: number): Ev[] =>
  notes.map((n, i) => [r0 + i * 4, 4, n, vol] as Ev);

const S = {
  horn: (v = 56): SampleDef => ({ name: "HORN", data: cycle("square", 128, 100, 0.3), volume: v, loopStart: 0, loopLen: 128 }),
  pulse: (n: string, v = 44): SampleDef => ({ name: n, data: cycle("pulse", 128, 90, 0.25), volume: v, loopStart: 0, loopLen: 128 }),
  bass: (v = 52): SampleDef => ({ name: "BASS", data: cycle("square", 128, 110, 0.5), volume: v, loopStart: 0, loopLen: 128 }),
  tri: (n: string, v = 48): SampleDef => ({ name: n, data: cycle("tri", 128, 95, 0.5), volume: v, loopStart: 0, loopLen: 128 }),
  drum: (v = 64): SampleDef => ({ name: "DRUM", data: kick(128, 118), volume: v, loopStart: 0, loopLen: 0 }),
  snr: (v = 50): SampleDef => ({ name: "SNR", data: kick(96, 96), volume: v, loopStart: 0, loopLen: 0 }),
  hat: (v = 40): SampleDef => ({ name: "HAT", data: hat(256, 70), volume: v, loopStart: 0, loopLen: 0 }),
};
const D = (rv: [number, number][]): Ev[] => rv.map(([r, v]) => [r, 1, "C-2", v] as Ev);
const R = (rv: [number, number][]): Ev[] => rv.map(([r, v]) => [r, 1, "C-3", v] as Ev);
const Hh = (rv: [number, number][]): Ev[] => rv.map(([r, v]) => [r, 1, "C-4", v] as Ev);

// galloping 8th-pair bass over one bar of chord roots
const gallop = (roots: [string, string][]): Ev[] => {
  const evs: Ev[] = [];
  roots.forEach(([lo, hi], i) => {
    const r = i * 16;
    evs.push([r, 3, lo, 50], [r + 4, 3, hi, 44], [r + 8, 3, lo, 50], [r + 12, 3, hi, 44]);
  });
  return evs;
};

// "THE CROWN ENDURES" (SPEC §3): 8-bar brass fanfare over pulse triads ->
// heroic Dorian march with singable lead over galloping bass -> lyrical
// bridge -> reprise with drums + arpeggio sparkle. 24 bars ~= 74 s; the
// restart byte loops the song back to the march (order 8), so the fanfare
// only crowns the first pass.
function titleSong(): Song {
  // fanfare bars (D major)
  const f1 = mkPat([
    { smp: 1, evs: [[0, 20, "D-5", 60], [24, 12, "A-4", 54], [36, 8, "B-4", 54], [44, 20, "D-5", 60]] },
    { smp: 2, evs: [...roll(0, 4, ["D-3", "A-3", "D-4"], 44), ...roll(16, 4, ["F#-3", "A-3", "D-4"], 44), ...roll(32, 4, ["D-3", "A-3", "D-4"], 44), ...roll(48, 4, ["A-2", "D-3", "F#-3"], 44)] },
    { smp: 3, evs: [[0, 16, "D-2", 50], [16, 16, "D-2", 50], [32, 16, "A-2", 50], [48, 16, "D-2", 50]] },
    { smp: 5, evs: D([[0, 60]]) },
  ]);
  const f2 = mkPat([
    { smp: 1, evs: [[0, 8, "E-5", 58], [8, 8, "F#-5", 58], [16, 8, "E-5", 58], [24, 24, "D-5", 60], [52, 12, "A-4", 54]] },
    { smp: 2, evs: [...roll(0, 4, ["A-2", "E-3", "A-3"], 44), ...roll(16, 4, ["C#-4", "E-4", "A-4"], 40), ...roll(32, 4, ["G-2", "D-3", "G-3"], 44), ...roll(48, 4, ["A-2", "E-3", "A-3"], 44)] },
    { smp: 3, evs: [[0, 16, "A-2", 50], [16, 16, "A-2", 50], [32, 16, "G-2", 50], [48, 16, "A-2", 50]] },
    null,
  ]);
  const f3 = mkPat([
    { smp: 1, evs: [[0, 20, "A-5", 62], [24, 8, "F#-5", 58], [32, 8, "A-5", 62], [40, 24, "B-5", 62]] },
    { smp: 2, evs: [...roll(0, 4, ["D-3", "A-3", "D-4"], 44), ...roll(24, 4, ["G-2", "D-3", "G-3"], 44), ...roll(40, 4, ["B-2", "F#-3", "B-3"], 44)] },
    { smp: 3, evs: [[0, 16, "D-2", 50], [16, 16, "D-2", 50], [24, 16, "G-2", 50], [40, 24, "A-2", 50]] },
    { smp: 5, evs: D([[0, 60], [32, 56]]) },
  ]);
  const f4 = mkPat([
    { smp: 1, evs: [[0, 8, "A-5", 62], [8, 8, "G-5", 60], [16, 20, "F#-5", 58], [40, 24, "D-5", 62]] },
    { smp: 2, evs: [...roll(0, 4, ["G-2", "D-3", "G-3"], 44), ...roll(16, 4, ["A-2", "E-3", "A-3"], 44), ...roll(40, 4, ["D-3", "A-3", "D-4"], 46)] },
    { smp: 3, evs: [[0, 16, "G-2", 50], [16, 16, "A-2", 50], [40, 24, "D-2", 52]] },
    { smp: 5, evs: D([[0, 60], [40, 64]]) },
  ]);
  // march bars (D Dorian: F naturals, C chord colours)
  const m1 = mkPat([
    { smp: 1, evs: [[0, 8, "A-4", 56], [8, 8, "D-5", 60], [16, 16, "F-5", 60], [32, 24, "A-5", 62], [56, 8, "G-5", 58]] },
    { smp: 3, evs: gallop([["D-2", "D-3"], ["F-2", "F-3"], ["G-2", "G-3"], ["A-2", "A-3"]]) },
    { smp: 2, evs: [[6, 8, "A-3", 38], [14, 16, "C-4", 38], [22, 8, "A-3", 38], [38, 8, "B-3", 38], [46, 16, "D-4", 38], [54, 8, "A-3", 38]] },
    { smp: 5, evs: [...D([[0, 62], [32, 60]]), ...R([[16, 44], [48, 44]])] },
  ]);
  const m2 = mkPat([
    { smp: 1, evs: [[0, 24, "A-5", 62], [24, 8, "G-5", 58], [32, 16, "F-5", 60], [48, 16, "E-5", 58]] },
    { smp: 3, evs: gallop([["D-2", "D-3"], ["D-2", "D-3"], ["A-2", "A-3"], ["A-2", "A-3"]]) },
    { smp: 2, evs: [[0, 30, "F-3", 36], [32, 30, "E-3", 36]] },
    { smp: 5, evs: [...D([[0, 62]]), ...Hh([[8, 36], [24, 36], [40, 36], [56, 36]])] },
  ]);
  const m3 = mkPat([
    { smp: 1, evs: [[0, 16, "D-5", 62], [16, 8, "E-5", 60], [24, 8, "F-5", 60], [32, 32, "A-5", 64]] },
    { smp: 3, evs: [...gallop([["D-2", "D-3"], ["D-2", "D-3"]]), [48, 16, "A-2", 54]] },
    { smp: 2, evs: [...roll(0, 4, ["D-3", "A-3", "D-4"], 44), [32, 30, "F#-3", 40]] },
    { smp: 5, evs: [...D([[0, 64], [32, 64]]), ...R([[44, 48], [48, 48], [52, 48], [56, 52]])] },
  ]);
  // lyrical bridge: sparkle + soft lead, no drums
  const b1 = mkPat([
    { smp: 1, evs: [[0, 32, "F-5", 48], [32, 16, "E-5", 46], [48, 16, "D-5", 48]] },
    { smp: 4, evs: run(0, ["D-4", "F-4", "A-4", "F-4", "D-4", "F-4", "A-4", "C-5"], 34) },
    { smp: 3, evs: [[0, 32, "D-2", 44], [32, 32, "A-2", 44]] },
    { smp: 2, evs: [[0, 30, "A-3", 30], [32, 30, "A-3", 30]] },
  ]);
  const b2 = mkPat([
    { smp: 1, evs: [[0, 24, "E-5", 48], [24, 8, "D-5", 46], [32, 16, "C-5", 46], [48, 16, "D-5", 50]] },
    { smp: 4, evs: run(0, ["C-4", "E-4", "G-4", "E-4", "C-4", "E-4", "G-4", "A-4"], 34) },
    { smp: 3, evs: [[0, 32, "A-2", 44], [32, 32, "D-2", 44]] },
    { smp: 2, evs: [[0, 30, "F-3", 30], [32, 30, "D-3", 30]] },
  ]);
  // reprise: march lead + 16th sparkle + full drums
  const r1 = mkPat([
    { smp: 1, evs: [[0, 8, "A-4", 58], [8, 8, "D-5", 62], [16, 16, "F-5", 62], [32, 24, "A-5", 64], [56, 8, "G-5", 60]] },
    { smp: 4, evs: run(0, ["D-5", "A-4", "F-4", "A-4", "D-5", "A-4", "F-4", "A-4", "F-5", "C-5", "A-4", "C-5", "G-5", "D-5", "B-4", "D-5"], 30) },
    { smp: 3, evs: gallop([["D-2", "D-3"], ["F-2", "F-3"], ["G-2", "G-3"], ["A-2", "A-3"]]) },
    { smp: 5, evs: [...D([[0, 64], [16, 60], [32, 64], [48, 60]]), ...R([[8, 46], [24, 46], [40, 46], [56, 46]])] },
  ]);
  const r2 = mkPat([
    { smp: 1, evs: [[0, 20, "A-5", 64], [20, 8, "G-5", 60], [28, 12, "F-5", 60], [40, 8, "E-5", 58], [48, 16, "D-5", 64]] },
    { smp: 4, evs: run(0, ["F-5", "D-5", "A-4", "D-5", "F-5", "D-5", "A-4", "D-5", "A-5", "F-5", "C-5", "F-5", "A-5", "F-5", "C-5", "F-5"], 30) },
    { smp: 3, evs: gallop([["D-2", "D-3"], ["D-2", "D-3"], ["A-2", "A-3"], ["A-2", "A-3"]]) },
    { smp: 5, evs: [...D([[0, 64], [24, 58], [48, 64]]), ...R([[12, 48], [40, 48], [56, 52]])] },
  ]);
  return {
    title: "THE CROWN ENDURES",
    samples: [S.horn(58), S.pulse("TRIAD", 44), S.bass(52), S.tri("SPARK", 38), S.drum(64), S.snr(50)],
    patterns: [f1, f2, f3, f4, m1, m2, m3, b1, b2, r1, r2],
    order: [0, 1, 2, 3, 0, 1, 2, 3, 4, 5, 4, 6, 4, 5, 4, 6, 7, 8, 7, 8, 9, 10, 9, 10],
    restart: 8,
  };
}

// warm major castle theme (G), harp-style rolls under a flute lead
function saxonSong(): Song {
  const s1 = mkPat([
    { smp: 1, evs: run(0, ["G-3", "B-3", "D-4", "G-4", "B-4", "D-4", "B-3", "G-3"], 44).map(([r, , n, v]) => [r, 7, n, v] as Ev) },
    { smp: 2, evs: [[0, 24, "B-4", 46], [24, 16, "A-4", 44], [40, 24, "G-4", 46]] },
    { smp: 3, evs: [[0, 32, "G-2", 46], [32, 32, "D-3", 44]] },
    { smp: 4, evs: [[0, 63, "D-4", 26]] },
  ]);
  const s2 = mkPat([
    { smp: 1, evs: run(0, ["D-4", "F#-4", "A-4", "D-5", "A-4", "F#-4", "A-4", "D-4"], 44).map(([r, , n, v]) => [r, 7, n, v] as Ev) },
    { smp: 2, evs: [[0, 24, "D-5", 46], [24, 8, "B-4", 44], [32, 32, "A-4", 44]] },
    { smp: 3, evs: [[0, 32, "D-2", 46], [32, 32, "A-2", 44]] },
    { smp: 4, evs: [[0, 63, "F#-3", 26]] },
  ]);
  const s3 = mkPat([
    { smp: 1, evs: run(0, ["E-3", "G-3", "B-3", "E-4", "B-3", "G-3", "B-3", "E-3"], 42).map(([r, , n, v]) => [r, 7, n, v] as Ev) },
    { smp: 2, evs: [[0, 16, "G-4", 44], [16, 16, "A-4", 44], [32, 16, "B-4", 46], [48, 16, "D-5", 46]] },
    { smp: 3, evs: [[0, 64, "E-2", 44]] },
    { smp: 4, evs: [[0, 63, "G-3", 26]] },
  ]);
  const s4 = mkPat([
    { smp: 1, evs: [...run(0, ["D-4", "A-4", "F#-4", "A-4"], 44).map(([r, , n, v]) => [r, 7, n, v] as Ev), ...run(32, ["G-3", "B-3", "D-4", "G-4"], 44).map(([r, , n, v]) => [r, 7, n, v] as Ev)] },
    { smp: 2, evs: [[0, 16, "C-5", 46], [16, 16, "B-4", 44], [32, 16, "A-4", 44], [48, 16, "G-4", 46]] },
    { smp: 3, evs: [[0, 16, "D-2", 46], [16, 16, "A-2", 44], [32, 32, "G-2", 46]] },
    { smp: 4, evs: [[0, 63, "B-3", 26]] },
  ]);
  return {
    title: "SAXON HALL",
    samples: [S.tri("HARP", 46), S.horn(46), S.bass(46), S.pulse("PAD", 28)],
    patterns: [s1, s2, s3, s4],
    order: [0, 1, 2, 3, 0, 1, 2, 3],
  };
}

// driving 6/8 combat cue (A minor) for battle reports
function battleSong(): Song {
  const ost = (a: string, b: string): Ev[] =>
    Array.from({ length: 16 }, (_, i) => [i * 4, 3, i % 2 ? b : a, 40] as Ev);
  const b1 = mkPat([
    { smp: 1, evs: [[0, 8, "A-4", 56], [8, 8, "C-5", 58], [16, 12, "A-4", 56], [28, 12, "E-5", 60], [40, 16, "D-5", 58], [56, 8, "C-5", 56]] },
    { smp: 2, evs: ost("A-3", "E-4") },
    { smp: 3, evs: [[0, 64, "A-2", 48]] },
    { smp: 4, evs: [...D([[0, 62], [8, 58], [16, 62], [24, 58], [32, 62], [40, 58], [48, 62], [56, 58]]), ...R([[12, 50], [28, 50], [44, 50], [60, 50]])] },
  ]);
  const b2 = mkPat([
    { smp: 1, evs: [[0, 12, "E-5", 60], [12, 8, "G-5", 60], [20, 12, "E-5", 58], [32, 8, "C-5", 56], [40, 12, "D-5", 58], [52, 12, "C-5", 56]] },
    { smp: 2, evs: ost("G-3", "D-4") },
    { smp: 3, evs: [[0, 64, "G-2", 48]] },
    { smp: 4, evs: [...D([[0, 62], [8, 58], [16, 62], [24, 58], [32, 62], [40, 58], [48, 62], [56, 58]]), ...R([[12, 50], [28, 50], [44, 50], [60, 50]])] },
  ]);
  const b3 = mkPat([
    { smp: 1, evs: [[0, 4, "A-5", 62], [4, 4, "G-5", 60], [8, 4, "F-5", 60], [12, 4, "E-5", 58], [16, 4, "D-5", 58], [20, 4, "C-5", 56], [24, 4, "B-4", 54], [28, 36, "A-4", 58]] },
    { smp: 2, evs: ost("A-3", "E-4") },
    { smp: 3, evs: [[0, 64, "A-2", 48]] },
    { smp: 4, evs: [...D([[0, 62], [8, 58], [16, 62], [24, 58]]), ...D([[32, 60], [36, 56], [40, 60], [44, 56], [48, 60], [52, 56], [56, 64], [60, 60]]), ...R([[12, 50], [28, 50]])] },
  ]);
  return {
    title: "TO THE WALLS",
    samples: [S.horn(58), S.bass(42), S.bass(48), S.drum(64), S.snr(52)],
    patterns: [b1, b2, b3],
    order: [0, 1, 0, 2],
  };
}

// playful tavern reel (C major) for the minigame rooms
function minigameSong(): Song {
  const g1 = mkPat([
    { smp: 1, evs: [[0, 6, "C-5", 54], [6, 2, "D-5", 50], [8, 8, "E-5", 54], [16, 8, "G-5", 56], [24, 8, "E-5", 54], [32, 8, "C-5", 54], [40, 8, "D-5", 54], [48, 16, "E-5", 56]] },
    { smp: 2, evs: [[0, 3, "C-2", 48], [4, 3, "G-3", 42], [8, 3, "C-2", 48], [12, 3, "E-3", 42], [16, 3, "C-2", 48], [20, 3, "G-3", 42], [24, 3, "E-2", 48], [28, 3, "G-3", 42], [32, 3, "C-2", 48], [36, 3, "G-3", 42], [40, 3, "C-2", 48], [44, 3, "E-3", 42], [48, 3, "A-2", 48], [52, 3, "G-3", 42], [56, 3, "G-2", 48], [60, 3, "D-3", 42]] },
    { smp: 4, evs: [...D([[0, 52], [16, 48], [32, 52], [48, 48]]), ...Hh([[8, 34], [24, 34], [40, 34], [56, 34]])] },
    null,
  ]);
  const g2 = mkPat([
    { smp: 1, evs: [[0, 6, "F-5", 56], [6, 2, "E-5", 50], [8, 8, "D-5", 54], [16, 8, "C-5", 52], [24, 8, "G-4", 50], [32, 8, "B-4", 52], [40, 8, "D-5", 54], [48, 16, "G-5", 56]] },
    { smp: 2, evs: [[0, 3, "F-2", 48], [4, 3, "C-3", 42], [8, 3, "F-2", 48], [12, 3, "A-3", 42], [16, 3, "C-2", 48], [20, 3, "G-3", 42], [24, 3, "C-2", 48], [28, 3, "E-3", 42], [32, 3, "G-2", 48], [36, 3, "B-3", 42], [40, 3, "G-2", 48], [44, 3, "D-3", 42], [48, 3, "G-2", 48], [52, 3, "B-3", 42], [56, 3, "G-2", 48], [60, 3, "D-3", 42]] },
    { smp: 4, evs: [...D([[0, 52], [16, 48], [32, 52], [48, 48]]), ...Hh([[8, 34], [24, 34], [40, 34], [56, 34]])] },
    null,
  ]);
  const g3 = mkPat([
    { smp: 1, evs: [[0, 4, "C-6", 58], [4, 4, "G-5", 56], [8, 4, "E-5", 54], [12, 4, "C-5", 52], [16, 8, "D-5", 54], [24, 8, "E-5", 54], [32, 16, "G-5", 56], [48, 12, "C-6", 60]] },
    { smp: 2, evs: [[0, 3, "C-2", 48], [4, 3, "G-3", 42], [8, 3, "C-2", 48], [12, 3, "E-3", 42], [16, 3, "F-2", 48], [20, 3, "C-3", 42], [24, 3, "F-2", 48], [28, 3, "A-3", 42], [32, 3, "G-2", 48], [36, 3, "B-3", 42], [40, 3, "G-2", 48], [44, 3, "D-3", 42], [48, 3, "C-2", 52], [52, 3, "G-3", 44], [56, 3, "C-3", 52]] },
    { smp: 4, evs: [...D([[0, 52], [16, 48], [32, 52], [48, 56]])] },
    null,
  ]);
  return {
    title: "TAVERN REEL",
    samples: [S.horn(52), S.bass(46), S.tri("WIPE", 30), S.drum(54), S.hat(38)],
    patterns: [g1, g2, g3],
    order: [0, 1, 0, 2],
  };
}

// 6-bar victory stinger: drum crack, triumphant D arpeggio, held high D
function victorySong(): Song {
  const v1 = mkPat([
    { smp: 1, evs: [...roll(0, 4, ["D-4", "A-4", "D-5"], 62), [24, 40, "F#-5", 62]] },
    { smp: 2, evs: run(0, ["D-3", "A-3", "D-4", "F#-4", "A-4", "F#-4", "D-4", "A-3"], 46).map(([r, , n, v]) => [r, 7, n, v] as Ev) },
    { smp: 3, evs: [[0, 32, "D-2", 54], [32, 32, "A-2", 52]] },
    { smp: 4, evs: [...D([[0, 64], [8, 56], [16, 60], [24, 56], [32, 62], [48, 60]]), ...R([[40, 52], [56, 52]])] },
  ]);
  const v2 = mkPat([
    { smp: 1, evs: [[0, 32, "A-5", 64], [32, 24, "B-5", 62], [56, 8, "A-5", 60]] },
    { smp: 2, evs: run(0, ["F#-3", "D-4", "F#-4", "A-4", "F#-4", "D-4", "F#-4", "A-4"], 46).map(([r, , n, v]) => [r, 7, n, v] as Ev) },
    { smp: 3, evs: [[0, 64, "D-2", 54]] },
    { smp: 4, evs: [...D([[0, 58], [16, 58], [32, 58], [48, 58]]), ...R([[12, 48], [28, 48], [44, 48], [60, 48]])] },
  ]);
  const v3 = mkPat([
    { smp: 1, evs: [[0, 48, "D-6", 64]] },
    { smp: 2, evs: run(0, ["D-4", "F#-4", "A-4", "D-5", "A-4", "F#-4", "D-4", "F#-4"], 44).map(([r, , n, v]) => [r, 7, n, v] as Ev) },
    { smp: 3, evs: [[0, 48, "D-2", 56]] },
    { smp: 4, evs: [...D([[0, 64], [32, 56]]), [48, 1, "C-2", 64], [56, 1, "C-2", 52]] },
  ]);
  return {
    title: "CROWN CLAIMED",
    samples: [S.horn(60), S.pulse("TRIAD", 46), S.bass(54), S.drum(64), S.snr(50)],
    patterns: [v1, v2, v3],
    order: [0, 1, 2, 0, 1, 2],
  };
}

// 6-bar defeat stinger: tolling drum, descending D-minor lament
function defeatSong(): Song {
  const lamen = (note: string, vol: number): Pattern =>
    mkPat([
      { smp: 1, evs: [[0, 56, note, vol]] },
      { smp: 2, evs: [[0, 62, "A-3", 26]] },
      { smp: 3, evs: [[0, 63, "D-2", 40]] },
      { smp: 4, evs: D([[0, 60]]) },
    ]);
  const last = mkPat([
    { smp: 1, evs: [[0, 40, "A-3", 38]] },
    { smp: 2, evs: [[0, 62, "F-3", 24]] },
    { smp: 3, evs: [[0, 63, "D-2", 36]] },
    { smp: 4, evs: [...D([[0, 58]]), [48, 1, "C-2", 44]] },
  ]);
  return {
    title: "THE CROWN FALLS",
    samples: [S.horn(44), S.pulse("PAD", 28), S.bass(42), S.drum(60)],
    patterns: [lamen("D-4", 50), lamen("C-4", 46), lamen("A#-3", 42), last],
    order: [0, 0, 1, 2, 3, 3],
  };
}

// ---------------------------------------------------------------- cli
const SONGS: Record<string, () => Song> = {
  demo: demoSong,
  tone: toneSong,
  norman: normanSong,
  title: titleSong,
  saxon: saxonSong,
  battle: battleSong,
  minigame: minigameSong,
  victory: victorySong,
  defeat: defeatSong,
};
if (import.meta.main) {
  const args = process.argv.slice(2);
  if (args[0] === "all") {
    for (const [name, make] of Object.entries(SONGS)) {
      if (name === "demo" || name === "tone") continue;
      const song = make();
      const buf = buildMod(song);
      const dest = `assets/music/${name}.mod`;
      await Bun.write(dest, buf);
      console.log(`${dest}: ${buf.length} bytes "${song.title}" order=${song.order.length} restart=${song.restart ?? "-"}`);
    }
  } else {
    const out = args[0] ?? "/tmp/test.mod";
    const which = Object.keys(SONGS).find((k) => args.includes(k)) ?? "demo";
    const buf = buildMod(SONGS[which]());
    await Bun.write(out, buf);
    const back = readMod(buf);
    const n = back.cell(back.order[0], 0, 0);
    console.log(`wrote ${out} (${buf.length} bytes) tag=${back.tag} songLen=${back.songLen} firstCell=${JSON.stringify(n)}`);
  }
}

