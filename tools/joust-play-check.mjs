// T502 browser playtest: at the TURN 5 draw the SPORTS meet hands out the
// joust; the harness actually plays it - reads the rival's rail from the
// lane pips, steers with arrows, throws the lance off the pixel-tracked
// gap - and rides home to the campaign map.
import { coords, tap, key, px, snap, regionDiff, shot, ok, finish, problems, decodeTurn } from "./jsdom-harness.mjs";

const blood = (x, y) => {
  const [r, g, b] = px(x, y);
  return Math.abs(r - 153) <= 55 && g <= 30 && Math.abs(b - 34) <= 26;
};
const LANE_Y = [118, 150, 182];

// the rail the BLOOD pip marks on the HUD strip (y 246..253)
function pipLane() {
  for (let l = 0; l < 3; l++) if (blood(108 + l * 12, 250)) return l;
  return null;
}
// left edge of the rival's caparison stripe on his rail: rx-14, rx = 86+gap
function rivalLeft(pl) {
  for (const y of [LANE_Y[pl] + 5, LANE_Y[pl] + 6, LANE_Y[pl] + 7]) {
    for (let x = 88; x < 306; x++) if (blood(x, y)) return x;
  }
  return null;
}
let lane = 1; // my rail index; steerTo keeps it in sync with the arrows sent
function steerTo(target) {
  while (lane !== target) {
    if (lane < target) { key("ArrowRight"); lane++; } else { key("ArrowLeft"); lane--; }
  }
}

await Bun.sleep(700);
key(" "); await Bun.sleep(120);        // boot herald
key("Enter"); await Bun.sleep(120);    // BEGIN CAMPAIGN
key("ArrowDown"); key("Enter");        // NORMAL
await Bun.sleep(900);
key("Enter"); key("Enter"); key("Enter"); // crawl -> campaign
await Bun.sleep(900);

// Exact-plate matchers. T705 reworked the parchment palette: LINEN is
// (187,170,119) and the CREAM map labels (221,204,153) collide with any
// loose matcher - the Lincoln label sits on the omen probe point and fakes
// an open modal. Match the real plate (and its scanline-dimmed rows) only.
const linen = (x, y) => { const p = px(x, y); return (Math.abs(p[0] - 187) <= 14 && Math.abs(p[1] - 170) <= 14 && Math.abs(p[2] - 119) <= 14) || (Math.abs(p[0] - 124) <= 12 && Math.abs(p[1] - 113) <= 12 && Math.abs(p[2] - 79) <= 12); };
const reportOpen = () => linen(64, 100) || linen(70, 92);
const omenOpen = () => linen(74, 70) || linen(80, 76);
// After END the map is deaf for the Norman march (ai_anim, 60 ticks that
// stretch over seconds under jsdom's throttled rAF); taps in that window
// die. The dawn modal opens the very tick the march ends, so the walk only
// has to keep tapping: while deaf the taps are harmlessly ignored, once the
// plate shows they aim at OK, and the first tap on the bare map ends the
// round again.
async function walkTo(target, budgetMs = 120000) {
  const t0 = Date.now();
  let cur = decodeTurn(snap());
  while (cur < target && Date.now() - t0 < budgetMs) {
    if (reportOpen()) { tap(10, 10); await Bun.sleep(150); if (reportOpen()) tap(10, 10); }
    else if (omenOpen()) tap(160, 128);
    else tap(...coords.END);
    await Bun.sleep(260);
    cur = decodeTurn(snap());
  }
  return cur;
}

// the deterministic draw offers the joust at TURN 22 (splitmix-scrambled)
const turn = await walkTo(22);
ok(turn === 22, `reached turn 22 for the joust draw (got ${turn})`);
// a levy/omen modal may still be on its way with turn 22: give the march
// animation time to surface it, then clear it before touching the map.
for (let i = 0; i < 40; i++) {
  if (reportOpen()) { tap(10, 10); await Bun.sleep(150); if (reportOpen()) tap(10, 10); }
  else if (omenOpen()) tap(160, 128);
  else if (i > 14) break;
  await Bun.sleep(150);
}
tap(...coords.BUCK);   // own-castle roll
await Bun.sleep(150);
tap(...coords.SPORTS); // the tilt opens
await Bun.sleep(250);
const map = snap();

// the BLOOD pip on the HUD strip telegraphs the rival's rail once the
// gallop runs; the caparison stripe on that same rail times the lance
shot("joust-present");

let lanced = 0;
let armed = false; // rider seen at the far end: window entry may fire once
let pipsSeen = 0;
let steerOk = 0;
let crownSeen = 0;
let home = false;
const t0 = Date.now();
while (Date.now() - t0 < 60000 && !home) {
  await Bun.sleep(20);
  const f = snap();
  // results tableau (T706 art): NAVY plate framed by full-width GOLD rows
  // at y56/y59 and y196/y199 (the scanline between each pair is dimmed,
  // not gold); the gallop screen scores ~3 against this, results ~470.
  let span = 0;
  for (const y of [56, 59, 196, 199]) {
    for (let x = 20; x < 300; x += 2) {
      const i = (y * 320 + x) * 4;
      const r = f[i], g = f[i + 1], b = f[i + 2];
      if (r >= 150 && r - g >= 20 && r - g <= 55 && g - b >= 60) span++;
    }
  }
  if (span > 150) crownSeen++;
  const pl = pipLane();
  if (pl !== null) {
    pipsSeen++;
    if (pl !== lane && Date.now() > steerOk) {
      steerTo(pl);
      steerOk = Date.now() + 150;
    }
  }
  const rl = pl !== null ? rivalLeft(pl) : null;
  if (rl !== null) {
    if (rl > 200) armed = true; // pass live (opening or return gallop)
    // sweet spot: stripe left edge 92..103 (gap 20..31, |off| <= 6)
    if (armed && rl >= 92 && rl <= 103 && lanced < 2) {
      key(" ");
      lanced++;
      armed = false;
    }
  }
  if (crownSeen > 2) key("Enter"); // results screen: ride home
  // home: campaign HUD back (no GOLD "TOURNAMENT" word top-left) on turn 22
  if (lanced >= 2 && crownSeen > 2 && decodeTurn(f) === 22) {
    let gpix = 0;
    for (let y = 2; y < 11; y++) for (let x = 4; x < 90; x++) {
      const i = (y * 320 + x) * 4;
      if (f[i] > 180 && f[i + 1] > 140 && f[i + 2] < 90) gpix++;
    }
    if (gpix < 8) home = true;
  }
}

shot("joust-end");
ok(pipsSeen > 20, "lane pips telegraph the rival's rail throughout");
ok(lanced === 2, `both lances thrown off pixel timing (thrown ${lanced})`);
ok(crownSeen > 2, "the crown tableau painted a banner");
ok(home, "ENTER leaves the results screen back on the campaign map");
ok(problems.length === 0, `no errors (${problems.slice(0, 3).join(" | ") || "clean"})`);
await finish();
