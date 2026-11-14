// T502 browser playtest: at the TURN 5 draw the SPORTS meet hands out the
// joust; the harness actually plays it - reads the rival's rail from the
// lane pips, steers with arrows, throws the lance off the pixel-tracked
// gap - and rides home to the campaign map.
import { coords, tap, key, px, snap, regionDiff, shot, ok, finish, problems, decodeTurn } from "./jsdom-harness.mjs";

const blood = (x, y) => {
  const [r, g, b] = px(x, y);
  return Math.abs(r - 153) <= 30 && g <= 30 && Math.abs(b - 34) <= 26;
};
const LANE_Y = [118, 150, 182];

// the rail the BLOOD pip marks on the HUD strip (y 246..253)
function pipLane() {
  for (let l = 0; l < 3; l++) if (blood(108 + l * 12, 250)) return l;
  return null;
}
// left edge of the rival's caparison stripe on his rail: rx-14, rx = 86+gap
function rivalLeft(lane) {
  for (const y of [LANE_Y[lane] + 5, LANE_Y[lane] + 6, LANE_Y[lane] + 7]) {
    for (let x = 88; x < 306; x++) if (blood(x, y)) return x;
  }
  return null;
}
// the results screen's gold border frame - the ONLY proof we are looking at
// the plate; a rival galloping down the top rail crosses the white-sampling
// band and would fake the crown tableau.
const goldFrame = () => { const p = px(40, 56); return p[0] === 204 && p[1] === 170 && p[2] === 0; };
let lane = 1; // the horse starts on the middle rail
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

const linAt = (x, y) => { const f = snap(); const i = (y * 320 + x) * 4; return f[i] === 204 && f[i + 1] === 204 && f[i + 2] === 153; };
async function dismissOverlay() {
  for (let i = 0; i < 20 && !linAt(64, 100) && !linAt(74, 70); i++) await Bun.sleep(100);
  if (linAt(64, 100)) { tap(10, 10); await Bun.sleep(150); tap(10, 10); await Bun.sleep(150); }
  if (linAt(74, 70)) { tap(160, 128); await Bun.sleep(150); }
}
async function waitTurn(target, budgetMs = 25000) {
  const t0 = Date.now();
  while (Date.now() - t0 < budgetMs) {
    if (decodeTurn(snap()) === target) return true;
    await dismissQuick();
    await Bun.sleep(100);
  }
  return false;
}
async function dismissQuick() {
  if (linAt(64, 100)) { tap(10, 10); await Bun.sleep(120); tap(10, 10); await Bun.sleep(120); }
  else if (linAt(74, 70)) { tap(160, 128); await Bun.sleep(120); }
}

// the deterministic draw offers the joust at TURN 22 (splitmix-scrambled)
let want = decodeTurn(snap());
while (want < 22) {
  await dismissOverlay();
  tap(...coords.END);
  want += 1;
  if (!(await waitTurn(want))) break;
}
const turn = decodeTurn(snap());
ok(turn === 22, `reached turn 22 for the joust draw (got ${turn})`);
await dismissOverlay(); // a levy/omen modal may have landed with turn 22
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
  // crown tableau: WHITE text on the BLACK band at y 116..129
  let white = 0;
  for (let y = 119; y < 127; y += 2) for (let x = 80; x < 240; x += 2) {
    const i = (y * 320 + x) * 4;
    if (f[i] > 200 && f[i + 1] > 200 && f[i + 2] > 200) white++;
  }
  if (white > 8 && goldFrame()) crownSeen++;
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
