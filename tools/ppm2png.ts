// ppm2png.ts — minimal P6 (binary RGB8) PPM -> PNG encoder. No deps beyond
// node:zlib (deflateSync exists in Bun). General-purpose, e.g. previewing
// framebuffer dumps; usage: bun tools/ppm2png.ts in.ppm out.png

import { readFileSync, writeFileSync } from "node:fs";
import { deflateSync } from "node:zlib";

const [src, dst] = process.argv.slice(2);
if (!src || !dst) {
  console.error("usage: bun tools/ppm2png.ts in.ppm out.png");
  process.exit(2);
}

const buf = readFileSync(src);
if (buf[0] !== 0x50 || buf[1] !== 0x36) throw new Error("not a P6 PPM");

// header: P6 <ws> w <ws> h <ws> maxval <single ws>
let p = 2;
const skipWs = () => {
  while (p < buf.length && (buf[p] === 0x20 || buf[p] === 0x0a || buf[p] === 0x0d || buf[p] === 0x09)) p++;
  if (buf[p] === 0x23) {
    while (p < buf.length && buf[p] !== 0x0a) p++;
    skipWs();
  }
};
const token = (): string => {
  skipWs();
  const s = p;
  while (p < buf.length && !/[ \n\r\t]/.test(String.fromCharCode(buf[p]))) p++;
  return buf.subarray(s, p).toString("ascii");
};
const w = parseInt(token(), 10);
const h = parseInt(token(), 10);
const maxval = parseInt(token(), 10);
if (!w || !h || maxval !== 255) throw new Error(`unsupported header: P6 ${w}x${h} ${maxval}`);
p++; // single whitespace char terminating the header
const need = w * h * 3;
if (buf.length < p + need) throw new Error(`truncated pixel data: ${buf.length - p} < ${need}`);
const px = buf.subarray(p, p + need);

// ---------------------------------------------------------------- png emit
const CRC = new Int32Array(256, (_v, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c;
});
const crc32 = (b: Uint8Array): number => {
  let c = ~0;
  for (const x of b) c = CRC[(c ^ x) & 255] ^ (c >>> 8);
  return ~c >>> 0;
};
const chunks: Buffer[] = [];
const chunk = (type: string, data: Buffer) => {
  const td = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(td));
  chunks.push(len, td, crc);
};

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(w, 0);
ihdr.writeUInt32BE(h, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 2; // color type: truecolor RGB
chunk("IHDR", ihdr);

const stride = w * 3;
const raw = Buffer.allocUnsafe(h * (stride + 1));
for (let y = 0; y < h; y++) {
  raw[y * (stride + 1)] = 0; // filter: None
  px.copy(raw, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
}
chunk("IDAT", deflateSync(raw));
chunk("IEND", Buffer.alloc(0));

writeFileSync(dst, Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), ...chunks]));
console.log(`${src} -> ${dst} (${w}x${h})`);
