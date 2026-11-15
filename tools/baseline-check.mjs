// GATE ALPHA 1 proof (A3): committed screenshot baselines + pixel-diff
// re-run. Drives the deterministic jsdom flow through its checkpoint
// states, captures each framebuffer, and diffs against the zlib-compressed
// RGBA baselines committed under shots/ (human-viewable PNGs in
// shots/png/). Animated states carry a tolerance for tick-phase jitter;
// static panels demand near-purity. Recapture with UPDATE=1.
//   bun tools/baseline-check.mjs            # diff vs committed baselines
import { key, tap, coords, px, snap, shot, png, ok, finish, decodeTurn, PAL, W, H } from "./jsdom-harness.mjs";
import { mkdirSync, existsSync } from "node:fs";
import { inflateSync, deflateSync } from "node:zlib";

const BASE_DIR = new URL("../shots/", import.meta.url).pathname;
const UPDATE = process.env.UPDATE === "1";
if (UPDATE) {
  mkdirSync(BASE_DIR, { recursive: true });
  mkdirSync(`${BASE_DIR}png`, { recursive: true });
}

// [name, interior-diff tolerance (px)]. The diff ignores the 8-px gold
// checker RING (its blink toggles ~1144 border px with tick phase - pure
// noise) and compares the interior 304x240. Interior jitter sources:
// torch flicker + crown rays on the how-to pages (<=182 px), the crawl
// flame vignette (~1000), banner art mid-unfurl on boot. Everything
// else is bit-pure across runs; measured over three re-runs.
const STATES = [
  ["boot", 1400],
  ["menu", 150],
  ["options", 150],
  ["howto-1", 300],
  ["howto-2", 300],
  ["howto-3", 300],
  ["fame", 150],
  ["difficulty", 150],
  ["crawl-1", 1200],
  ["panel", 150], // own-castle roll, static after slide-in
  ["report", 400], // battle report, text scrolls in then rests
  ["campaign", 125], // turn-1 map is static
];

const frames = {};
async function cap(name) {
  await Bun.sleep(60); // let the fixed step land one full draw
  frames[name] = snap();
}

await Bun.sleep(700); // boot: herald unfurl completes ~0.6 s after load
await cap("boot");

key("Enter"); // audio unlock + herald rolls up -> menu
await Bun.sleep(150);
await cap("menu");

key("ArrowDown"); key("ArrowDown"); key("ArrowDown"); // -> OPTIONS
key("Enter");
await Bun.sleep(150);
await cap("options");
key("Escape"); // back to menu (selection resets to top)
await Bun.sleep(150);

key("ArrowDown"); // HOW TO PLAY
key("Enter");
await Bun.sleep(150);
await cap("howto-1");
key("ArrowRight");
await Bun.sleep(150);
await cap("howto-2");
key("ArrowRight");
await Bun.sleep(150);
await cap("howto-3");
key("Escape"); // menu
await Bun.sleep(150);

key("ArrowDown"); key("ArrowDown"); // HALL OF FAME
key("Enter");
await Bun.sleep(150);
await cap("fame");
key("Escape");
await Bun.sleep(150);

key("Enter"); // BEGIN CAMPAIGN -> difficulty pick (EASY highlighted)
await Bun.sleep(150);
key("ArrowDown"); // NORMAL
await Bun.sleep(120);
await cap("difficulty");
key("Enter"); // -> crawl leaf 1
await Bun.sleep(250);
await cap("crawl-1");

// skip the crawl: NEXT through the leaves; the third NEXT commits the war
key("Enter"); await Bun.sleep(150);
key("Enter"); await Bun.sleep(150);
key("Enter"); await Bun.sleep(700);
await cap("campaign");

// own-castle roll: the Buckingham panel (SPORTS/LEVY/etc buttons visible)
tap(...coords.BUCK);
await Bun.sleep(350);
await cap("panel");
tap(40, 40); // sea tap deselects, roll closes
await Bun.sleep(200);

// Force a battle: order Buckingham's host on London - the combat log rises
// as the auto-opening BATTLE REPORT (LINEN plate at x62..258,y84..; the
// omen modal never reaches x64/y88, so (64,88) is a report-only marker).
const [LR, LG, LB] = PAL.LINEN;
const lin = (x, y) => { const p = px(x, y); return p[0] === LR && p[1] === LG && p[2] === LB; };
tap(...coords.BUCK);
await Bun.sleep(250);
tap(...coords.LON); // march order
await Bun.sleep(600);
let report = false;
for (let t = 0; t < 6 && !report; t++) {
  if (lin(64, 88)) {
    await Bun.sleep(500); // lines scroll in over ~0.4 s, then the plate rests
    await cap("report");
    report = true;
    tap(10, 10); await Bun.sleep(250); // close
    break;
  }
  if (lin(240, 70)) {
    tap(160, 128); await Bun.sleep(250); // omen OK out of the way
  }
  tap(264, 217); // END TURN: combat resolves, dawn marches ~1 s
  await Bun.sleep(1400);
}
ok(report, "battle report plate captured after the march on London");

let captured = 0;
for (const [name, tol] of STATES) {
  const cur = frames[name];
  ok(cur !== undefined, `${name}: captured`);
  const path = `${BASE_DIR}${name}.rgba.zz`;
  if (UPDATE) {
    await Bun.write(path, deflateSync(Buffer.from(cur.buffer)));
    const p = shot(name); // run copy in SHOTS_DIR
    await Bun.write(`${BASE_DIR}png/${name}.png`, png(cur, W, H));
    captured++;
    continue;
  }
  if (!existsSync(path)) {
    ok(false, `${name}: baseline missing (run UPDATE=1 to capture)`);
    continue;
  }
  const base = new Uint8Array(inflateSync(await Bun.file(path).arrayBuffer()));
  let d = 0;
  for (let y = 8; y < 248; y++)
    for (let x = 8; x < 312; x++) {
      const i = (y * W + x) * 4;
      if (cur[i] !== base[i] || cur[i + 1] !== base[i + 1] || cur[i + 2] !== base[i + 2]) d++;
    }
  ok(d <= tol, `${name}: ${d} interior px differ vs baseline (tol ${tol})`);
}
if (captured) console.log(`captured ${captured} baselines -> shots/`);
finish();
