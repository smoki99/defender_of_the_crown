// Shared jsdom harness: boots the real wasm bundle against a synthetic DOM
// (canvas putImageData -> framebuffer readback, fake WebAudio) so scenario
// scripts can click/key the game headlessly and assert on pixels.
// LS_SEED="k=v;k=v" pre-seeds localStorage (simulates a returning player).
import { JSDOM, VirtualConsole } from "jsdom";
import { deflateSync } from "node:zlib";
import { resolve } from "node:path";
import { mkdirSync } from "node:fs";
const ROOT = resolve(import.meta.dirname, "..");
const SHOTS = process.env.SHOTS_DIR ?? "/tmp/crown-shots";
mkdirSync(SHOTS, { recursive: true });

export const W = 320, H = 256;

// ---- tap coordinates from `cargo run --example coords`
export const coords = {};
export const digits = {};
const proc = Bun.spawn([`${process.env.HOME}/.cargo/bin/cargo`, "run", "-q", "--manifest-path", `${ROOT}/Cargo.toml`, "--example", "coords"], { stdout: "pipe" });
for (const line of (await new Response(proc.stdout).text()).trim().split("\n")) {
  const parts = line.split(" ");
  if (parts[0] === "DIGIT") digits[Number(parts[1])] = parts[2].split(",").map((h) => parseInt(h, 16));
  else coords[parts[0]] = [Number(parts[1]), Number(parts[2])];
}

// ---- PNG writer (deflate via node:zlib)
const CRC = Array.from({ length: 256 }, (_, n) => { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 8) : c >>> 1; return c >>> 0; });
function crc32(buf) { let c = 0xffffffff; for (const b of buf) c = CRC[(c ^ b) & 255] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; }
function chunk(type, data) { const len = new Uint32Array([data.length]); const l = new Uint8Array(4); for (let i = 0; i < 4; i++) l[i] = (data.length >>> (24 - 8 * i)) & 255; const t = new TextEncoder().encode(type); const body = new Uint8Array(t.length + data.length); body.set(t); body.set(data, t.length); const c = new Uint32Array([crc32(body)]); const cl = new Uint8Array(4); for (let i = 0; i < 4; i++) cl[i] = (crc32(body) >>> (24 - 8 * i)) & 255; return new Uint8Array([...l, ...body, ...cl]); }
export function png(rgba, w, h) {
  const raw = new Uint8Array(h * (w * 4 + 1));
  for (let y = 0; y < h; y++) { raw[y * (w * 4 + 1)] = 0; raw.set(rgba.subarray(y * w * 4, (y + 1) * w * 4), y * (w * 4 + 1) + 1); }
  const ihdr = new Uint8Array(13);
  const dv = new DataView(ihdr.buffer);
  dv.setUint32(0, w); dv.setUint32(4, h); ihdr[8] = 8; ihdr[9] = 6;
  const sig = new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]);
  const chunks = [chunk("IHDR", ihdr), chunk("IDAT", deflateSync(raw)), chunk("IEND", new Uint8Array(0))];
  const out = new Uint8Array(sig.length + chunks.reduce((a, c) => a + c.length, 0));
  out.set(sig); let o = sig.length; for (const c of chunks) { out.set(c, o); o += c.length; }
  return out;
}

// ---- DOM
export const problems = [];
const vc = new VirtualConsole();
vc.on("jsdomError", (e) => { if (!/Could not parse CSS/.test(String(e.message))) problems.push("jsdom: " + e.message); });
vc.on("error", (...a) => problems.push("console.error: " + a.join(" ")));
export const dom = new JSDOM(`<!doctype html><html><body><canvas id="screen"></canvas></body></html>`, {
  url: `http://127.0.0.1:8123/index.html${process.env.CROWN_QS ? "?" + process.env.CROWN_QS : ""}`, pretendToBeVisual: true, virtualConsole: vc,
});
export const { window } = dom;
window.addEventListener("error", (e) => problems.push("window: " + (e.error?.stack ?? e.message)));

let frameCount = 0;
let frame = null;
class Ctx2D {
  constructor(canvas) { this.canvas = canvas; }
  putImageData(img) { frame = new Uint8Array(img.data); frameCount++; }
  getImageData() { return { data: new Uint8ClampedArray(W * H * 4) }; }
}
Object.defineProperty(Ctx2D, Symbol.hasInstance, { value: (o) => Ctx2D.prototype.isPrototypeOf(o), configurable: true });
window.CanvasRenderingContext2D = Ctx2D;
globalThis.CanvasRenderingContext2D = Ctx2D;
window.HTMLCanvasElement.prototype.getContext = function () { return new Ctx2D(this); };
class FakeImageData { constructor(d, w, h) { this.data = d; this.width = w; this.height = h ?? 1; } }
window.ImageData = FakeImageData;
const param = () => ({ value: 0, setValueAtTime() {}, linearRampToValueAtTime() {}, setTargetAtTime() {}, cancelScheduledValues() {} });
const node = () => ({ connect(x) { return x; }, disconnect() {}, start() {}, stop() {}, addEventListener() {}, removeEventListener() {}, postMessage() {}, port: { postMessage() {}, onmessage: null }, gain: param(), frequency: param(), playbackRate: param(), detune: param(), buffer: null, loop: false, getChannelData: () => new Float32Array(4096), copyToChannel() {}, copyFromChannel() {}, onaudioprocess: null });
class FakeAudioCtx {
  constructor() { this.destination = node(); this.sampleRate = 48000; this.state = "running"; this._t = 0; }
  get currentTime() { return (this._t += 0.016); }
  resume() { return Promise.resolve(); }
  createGain() { return { gain: param(), connect: (x) => x, disconnect() {} }; }
  createOscillator() { return node(); }
  createBufferSource() { return node(); }
  createBiquadFilter() { return node(); }
  createBuffer(ch, len, rate) { return { length: len, numberOfChannels: ch, sampleRate: rate, getChannelData: () => new Float32Array(len), copyToChannel() {} }; }
  createScriptProcessor(b) { const n = node(); n.bufferSize = b; return n; }
  get audioWorklet() { return { addModule: () => Promise.reject(new Error("no worklet in node")) }; }
}
window.AudioContext = FakeAudioCtx;

globalThis.window = window;
globalThis.document = window.document;
globalThis.navigator = window.navigator;
for (const k of ["HTMLElement", "HTMLCanvasElement", "Element", "Event", "MouseEvent", "KeyboardEvent", "WheelEvent", "Image", "CSS", "getComputedStyle", "requestAnimationFrame", "cancelAnimationFrame", "localStorage", "devicePixelRatio", "ImageData", "location", "addEventListener", "removeEventListener", "setTimeout", "clearTimeout", "innerWidth", "innerHeight", "self"]) globalThis[k] = window[k];
class PerfShim { now() { return Number(process.hrtime.bigint()) / 1e6; } }
Object.defineProperty(PerfShim, Symbol.hasInstance, { value: (o) => o === window.performance || (o && typeof o.now === "function"), configurable: true });
Object.defineProperty(window, "performance", { value: new PerfShim(), configurable: true });
globalThis.performance = window.performance;
window.Performance = PerfShim;
globalThis.Performance = PerfShim;
globalThis.AudioContext = FakeAudioCtx;
// web_sys::window() = global().dyn_into::<Window>() - make the node global
// pass `instanceof Window` without touching jsdom's own class.
const OrigWindow = window.Window;
class CrownWindow extends OrigWindow {}
Object.defineProperty(CrownWindow, Symbol.hasInstance, {
  value: (o) => o === globalThis || o instanceof OrigWindow,
  configurable: true,
});
globalThis.Window = CrownWindow;

// returning-player simulation: seed persisted prefs before boot
if (process.env.LS_SEED) {
  for (const kv of process.env.LS_SEED.split(";")) {
    const eq = kv.indexOf("=");
    if (eq > 0) window.localStorage.setItem(kv.slice(0, eq), kv.slice(eq + 1));
  }
}

// ---- boot the real bundle
const wasmBytes = await Bun.file(`${ROOT}/web/pkg/defender_of_the_crown_bg.wasm`).arrayBuffer();
const init = (await import(`${ROOT}/web/pkg/defender_of_the_crown.js`)).default;
await init({ module_or_path: wasmBytes });

export const canvas = () => window.document.getElementById("screen");
export function tap(x, y) {
  const c = canvas();
  for (const type of ["pointerdown", "pointerup"]) c.dispatchEvent(new window.MouseEvent(type, { clientX: x, clientY: y, bubbles: true }));
}
/// Semantic key press: the app maps KeyboardEvent.key on window.
export function key(k) {
  window.dispatchEvent(new window.KeyboardEvent("keydown", { key: k, bubbles: true }));
  window.dispatchEvent(new window.KeyboardEvent("keyup", { key: k, bubbles: true }));
}
export function keyDown(k) {
  window.dispatchEvent(new window.KeyboardEvent("keydown", { key: k, bubbles: true }));
}
export function keyUp(k) {
  window.dispatchEvent(new window.KeyboardEvent("keyup", { key: k, bubbles: true }));
}
export const getFrame = () => frame;
export const getFrameCount = () => frameCount;
export const px = (x, y) => { const i = (y * W + x) * 4; return [frame[i], frame[i + 1], frame[i + 2]]; };
export function regionDiff(a, b, x0, y0, x1, y1) { let d = 0; for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) { const i = (y * W + x) * 4; if (a[i] !== b[i] || a[i + 1] !== b[i + 1] || a[i + 2] !== b[i + 2]) d++; } return d; }
let shotN = 0;
export function shot(name) {
  const path = `${SHOTS}/${String(shotN++).padStart(2, "0")}-${name}.png`;
  Bun.write(path, png(frame, W, H));
  return path;
}

/// HUD turn counter read straight from the framebuffer (8x8 glyphs at (160,204)).
export function decodeTurn(fr) {
  const white = (x, y) => { const i = (y * W + x) * 4; return fr[i] > 150 && fr[i + 1] > 150 && fr[i + 2] > 150; };
  const cellMask = (cx, cy) => { const rows = []; for (let r = 0; r < 8; r++) { let m = 0; for (let c = 0; c < 8; c++) if (white(cx + c, cy + r)) m |= 0x80 >> c; rows.push(m); } return rows; };
  let s = "";
  for (let k = 0; k < 2; k++) {
    const m = cellMask(160 + k * 8, 204);
    if (m.every((r) => r === 0)) break;
    let f = null;
    for (let d = 0; d < 10; d++) if (m.every((v, i) => v === digits[d][i])) { f = d; break; }
    if (f === null) break;
    s += f;
  }
  return s === "" ? null : parseInt(s, 10);
}

export const asserts = [];
export const ok = (cond, msg) => { if (!cond) { asserts.push(msg); console.log("FAIL " + msg); } else console.log("pass  " + msg); };
export const snap = () => frame.slice();
export const finish = () => {
  console.log(`DONE frames=${frameCount} shots=${shotN} assertsFailed=${asserts.length} problems=${problems.length}`);
  process.exit(asserts.length === 0 ? 0 : 1);
};
