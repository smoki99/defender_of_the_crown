// T505 reward-loop integration: campaign -> joust -> GOLD applied to the
// purse -> hall-of-fame entry written to localStorage. Plays the TURN 5
// joust draw for real (pixel solver from T502), then checks the three
// links of the loop the player never sees joined: HUD purse, engine gold,
// and the doc.hof chronicle the title screen's HALL OF FAME reads.
import { coords, tap, key, px, snap, regionDiff, shot, ok, finish, problems, decodeTurn } from "./jsdom-harness.mjs";

const GOLD = [204, 170, 0];
const eq = (p, q) => p[0] === q[0] && p[1] === q[1] && p[2] === q[2];
const blood = (x, y) => {
  const [r, g, b] = px(x, y);
  return Math.abs(r - 153) <= 30 && g <= 30 && Math.abs(b - 34) <= 26;
};
const LANE_Y = [118, 150, 182];

function pipLane() {
  for (let l = 0; l < 3; l++) if (blood(108 + l * 12, 250)) return l;
  return null;
}
function rivalLeft(lane) {
  for (const y of [LANE_Y[lane] + 5, LANE_Y[lane] + 6, LANE_Y[lane] + 7]) {
    for (let x = 88; x < 306; x++) if (blood(x, y)) return x;
  }
  return null;
}
function laurelHead() {
  // results plate head line: GOLD glyph pixels only on a gold laurel
  let n = 0;
  for (let y = 72; y < 79; y++) for (let x = 104; x < 204; x += 2) if (eq(px(x, y), GOLD)) n++;
  return n > 8;
}
let lane = 1;
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
async function dismissQuick() {
  if (linAt(64, 100)) { tap(10, 10); await Bun.sleep(120); tap(10, 10); await Bun.sleep(120); }
  else if (linAt(74, 70)) { tap(160, 128); await Bun.sleep(120); }
}
async function dismissOverlay() {
  for (let i = 0; i < 20 && !linAt(64, 100) && !linAt(74, 70); i++) await Bun.sleep(100);
  await dismissQuick();
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

// the deterministic draw offers the joust at TURN 5
let want = decodeTurn(snap());
while (want < 5) {
  await dismissOverlay();
  tap(...coords.END);
  want += 1;
  if (!(await waitTurn(want))) break;
}
const turn = decodeTurn(snap());
ok(turn === 5, `reached turn 5 for the joust draw (got ${turn})`);
await dismissOverlay();

ok(localStorage.getItem("doc.hof") === null || localStorage.getItem("doc.hof") === "",
  "the hall of fame starts blank");

tap(...coords.BUCK);
await Bun.sleep(150);
const purseBefore = snap();
tap(...coords.SPORTS);
await Bun.sleep(400);
shot("loop-present");

let lanced = 0;
let armed = false;
let pipsSeen = 0;
let steerOk = 0;
let crownSeen = 0;
let gold = false;
let home = false;
const t0 = Date.now();
while (Date.now() - t0 < 60000 && !home) {
  await Bun.sleep(20);
  const f = snap();
  let white = 0;
  for (let y = 119; y < 127; y += 2) for (let x = 80; x < 240; x += 2) {
    const i = (y * 320 + x) * 4;
    if (f[i] > 200 && f[i + 1] > 200 && f[i + 2] > 200) white++;
  }
  if (white > 8) crownSeen++;
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
    if (rl > 200) armed = true;
    if (armed && rl >= 92 && rl <= 103 && lanced < 2) {
      key(" ");
      lanced++;
      armed = false;
    }
  }
  // the plate scrolls in over ~1 s: ride home once the head line is
  // painted gold, or once any banner has sat painted long enough.
  if (crownSeen > 2 && laurelHead()) gold = true;
  if (crownSeen > 2 && (gold || crownSeen > 40)) key("Enter"); // results -> map
  if (lanced >= 2 && crownSeen > 2 && decodeTurn(f) === 5) {
    await Bun.sleep(200); // a calm repainted map frame
    home = true;
  }
}

shot("loop-home");
ok(lanced === 2, `both lances thrown off pixel timing (thrown ${lanced})`);
ok(gold, "the results plate head gleamed GOLD LAUREL");
ok(home, "ENTER returned to the campaign map");

const purse = snap();
ok(regionDiff(purseBefore, purse, 4, 204, 90, 212) > 0, "the purse line moved with the fee and prize");

const hof = localStorage.getItem("doc.hof") ?? "";
ok(hof.startsWith("TOURNAMENT GOLD - SCORE "), `hall of fame chronicles the meet ("${hof}")`);
ok(problems.length === 0, `no errors (${problems.slice(0, 3).join(" | ") || "clean"})`);
await finish();
