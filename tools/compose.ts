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
  out.push(song.order.length, 0x7f);
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
// ---------------------------------------------------------------- cli
if (import.meta.main) {
  const args = process.argv.slice(2);
  const out = args[0] ?? "/tmp/test.mod";
  const song = args.includes("norman")
    ? normanSong()
    : args.includes("tone")
      ? toneSong()
      : demoSong();
  const buf = buildMod(song);
  await Bun.write(out, buf);
  const back = readMod(buf);
  const n = back.cell(back.order[0], 0, 0);
  console.log(`wrote ${out} (${buf.length} bytes) tag=${back.tag} songLen=${back.songLen} firstCell=${JSON.stringify(n)}`);
}
