// Throwaway helper: decode a PNG to RGBA and sample pixels/rows for matcher work.
// usage: bun tools/png-decode.mjs <file.png> [predRow|px:x,y] ...
import { inflateSync } from "node:zlib";

export function decodePng(buf) {
  const b = new Uint8Array(buf);
  let off = 8, w = 0, h = 0, colt = 0;
  const idat = [];
  while (off < b.length) {
    const len = ((b[off] << 24) | (b[off + 1] << 16) | (b[off + 2] << 8) | b[off + 3]) >>> 0;
    const type = String.fromCharCode(b[off + 4], b[off + 5], b[off + 6], b[off + 7]);
    const data = b.subarray(off + 8, off + 8 + len);
    if (type === "IHDR") {
      w = ((data[0] << 24) | (data[1] << 16) | (data[2] << 8) | data[3]) >>> 0;
      h = ((data[4] << 24) | (data[5] << 16) | (data[6] << 8) | data[7]) >>> 0;
      colt = data[9];
    } else if (type === "IDAT") idat.push(data);
    off += 12 + len;
  }
  const raw = inflateSync(Buffer.concat(idat.map((d) => Buffer.from(d))));
  const ch = colt === 6 ? 4 : colt === 2 ? 3 : 1;
  const stride = w * ch;
  const img = new Uint8Array(w * h * ch);
  let prev = new Uint8Array(stride), rp = 0;
  for (let y = 0; y < h; y++) {
    const ft = raw[rp++];
    const line = raw.subarray(rp, rp + stride); rp += stride;
    const cur = new Uint8Array(stride);
    for (let x = 0; x < stride; x++) {
      const a = x >= ch ? cur[x - ch] : 0, bb = prev[x], c = x >= ch ? prev[x - ch] : 0;
      let v = line[x];
      if (ft === 1) v = (v + a) & 255;
      else if (ft === 2) v = (v + ((a + bb) >> 1)) & 255;
      else if (ft === 3) v = (v + (((a + bb + c) / 3) | 0)) & 255;
      else if (ft === 4) {
        const p = a + bb - c, pa = Math.abs(p - a), pb = Math.abs(p - bb), pc = Math.abs(p - c);
        v = (v + (pa <= pb && pa <= pc ? a : pb <= pc ? bb : c)) & 255;
      }
      cur[x] = v;
    }
    img.set(cur, y * stride); prev = cur;
  }
  return { w, h, ch, img };
}

export function sampler(rgba) {
  const { w, h, ch, img } = rgba;
  return (x, y) => { const i = (y * w + x) * ch; return [img[i], img[i + 1], img[i + 2]]; };
}

if (import.meta.main) {
  const [file, ...args] = process.argv.slice(2);
  const rgba = decodePng(await Bun.file(file).arrayBuffer());
  const px = sampler(rgba);
  console.log(`${file} ${rgba.w}x${rgba.h}`);
  for (const a of args) {
    if (a.startsWith("px:")) {
      const [x, y] = a.slice(3).split(",").map(Number);
      console.log(`(${x},${y}) =`, px(x, y).join(","));
    } else if (a.startsWith("gold")) {
      for (let y = 0; y < rgba.h; y++) {
        let n = 0;
        for (let x = 0; x < rgba.w; x++) {
          const p = px(x, y);
          if (p[0] >= 150 && p[0] - p[1] >= 20 && p[0] - p[1] <= 55 && p[1] - p[2] >= 60) n++;
        }
        if (n > 4) console.log(`goldrow y=${y} n=${n}`);
      }
    }
  }
}
