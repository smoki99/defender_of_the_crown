// GATE ALPHA 1 (A3) minigame baselines, one capture per process: the jsdom
// URL (and thus the ?scene=mg&mg=<id>[&autoplay=1] deep link) is frozen at
// boot, so tools/baseline-sports.sh forks one run per (sport, mode).
//   STATE=<name> [TOL=n] [SETTLE=ms] bun tools/sports-baseline-one.mjs
//   UPDATE=1 ... recaptures; otherwise diffs against shots/<name>.rgba.zz.
import { mkdirSync, existsSync, readFileSync } from "node:fs";
import { inflateSync, deflateSync } from "node:zlib";
import { snap, shot, png, ok, finish, W, H } from "./jsdom-harness.mjs";

const name = process.env.STATE ?? "sport";
const tol = Number(process.env.TOL ?? "1300"); // gold checker ring blinks on tick
const UPDATE = process.env.UPDATE === "1";
const BASE_DIR = new URL("../shots/", import.meta.url).pathname;
if (UPDATE) mkdirSync(`${BASE_DIR}png`, { recursive: true });

await Bun.sleep(Number(process.env.SETTLE ?? "600"));
const cur = snap();

let captured = 0;
let failed = 0;
const path = `${BASE_DIR}${name}.rgba.zz`;
if (UPDATE) {
  await Bun.write(path, deflateSync(Buffer.from(cur.buffer)));
  await Bun.write(`${BASE_DIR}png/${name}.png`, png(cur, W, H));
  shot(name); // run copy in SHOTS_DIR for eyeballing
  console.log(`pass  ${name}: recaptured`);
  captured = 1;
} else {
  if (!existsSync(path)) {
    console.log(`FAIL  ${name}: no baseline at ${path} (UPDATE=1 to capture)`);
    failed = 1;
  } else {
    const base = Uint8Array.from(inflateSync(readFileSync(path)));
    let d = 0;
    for (let i = 0; i < cur.length; i++) if (cur[i] !== base[i]) d++;
    console.log(`${d <= tol ? "pass" : "FAIL"}  ${name}: ${d} px (tol ${tol})`);
    if (d > tol) {
      await Bun.write(`/tmp/${name}-cur.png`, png(cur, W, H));
    }
    captured = 1;
  }
}
ok(captured === 1 && failed === 0, `${name}: baseline check`);
finish();
