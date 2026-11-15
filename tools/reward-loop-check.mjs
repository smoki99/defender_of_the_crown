// T505 reward-loop integration: campaign -> joust -> GOLD applied to the
// purse -> hall-of-fame entry written to localStorage. Plays the TURN 22
// joust draw for real (pixel solver from T502), then checks the three
// links of the loop the player never sees joined: HUD purse, engine gold,
// and the doc.hof chronicle the title screen's HALL OF FAME reads.
import { coords, tap, key, px, snap, regionDiff, shot, ok, finish, problems, decodeTurn } from "./jsdom-harness.mjs";

const blood = (x, y) => {
  const [r, g, b] = px(x, y);
  return Math.abs(r - 153) <= 55 && g <= 30 && Math.abs(b - 34) <= 26;
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
// results tableau (T706): NAVY plate framed by full-width GOLD rows at
// y56/y59/y196/y199 (T706 gold is (187,153,51), dimmed (124,102,34)); the
// gallop screen scores ~3 against this, the results plate ~470.
const resultsPlate = (f) => {
  let span = 0;
  for (const y of [56, 59, 196, 199]) for (let x = 20; x < 300; x += 2) {
    const i = (y * 320 + x) * 4;
    const r = f[i], g = f[i + 1], b = f[i + 2];
    if (r >= 150 && r - g >= 20 && r - g <= 55 && g - b >= 60) span++;
  }
  return span > 150;
};
function laurelHead(f) {
  // "GOLD LAUREL!" head line (y76..83): gold glyph pixels, only on results
  let n = 0;
  for (let y = 76; y < 84; y++) for (let x = 100; x < 224; x++) {
    const i = (y * 320 + x) * 4;
    const r = f[i], g = f[i + 1], b = f[i + 2];
    if (r >= 110 && r - g >= 20 && r - g <= 55 && g - b >= 30) n++;
  }
  return n > 6;
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

const linAt = (x, y) => { const f = snap(); const i = (y * 320 + x) * 4; const r = f[i], g = f[i + 1], b = f[i + 2]; return (Math.abs(r - 187) <= 14 && Math.abs(g - 170) <= 14 && Math.abs(b - 119) <= 14) || (Math.abs(r - 124) <= 12 && Math.abs(g - 113) <= 12 && Math.abs(b - 79) <= 12); };
// Converging walk: the march animation deafens the map after END and the
// omen/report plate lands seconds later at jsdom rAF speed, so fixed
// schedules drop taps (turn stuck forever). Exact LINEN plates decide:
// tap OK/skip on a real plate, tap END only on the bare map.
async function walkTo(target, budgetMs = 120000) {
  const t0 = Date.now();
  let cur = decodeTurn(snap());
  while (cur < target && Date.now() - t0 < budgetMs) {
    if (linAt(64, 100)) {
      tap(10, 10); await Bun.sleep(150);
      if (linAt(64, 100)) tap(10, 10);
      await Bun.sleep(150);
    } else if (linAt(74, 70)) {
      tap(160, 128); await Bun.sleep(150);
    } else {
      tap(...coords.END); await Bun.sleep(220);
    }
    cur = decodeTurn(snap()) ?? cur;
  }
  return cur;
}

// the deterministic draw offers the joust at TURN 22 (splitmix-scrambled)
const turn = await walkTo(22);
ok(turn === 22, `reached turn 22 for the joust draw (got ${turn})`);
for (let i = 0; i < 40; i++) {
  if (linAt(64, 100)) { tap(10, 10); await Bun.sleep(150); if (linAt(64, 100)) tap(10, 10); }
  else if (linAt(74, 70)) { tap(160, 128); await Bun.sleep(150); }
  else if (i > 14) break;
  else await Bun.sleep(150);
}

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
let posted = false;
let maxRl = 0;
const t0 = Date.now();
while (Date.now() - t0 < 60000 && !home) {
  await Bun.sleep(20);
  const f = snap();
  if (resultsPlate(f)) crownSeen++;
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
    if (rl > maxRl) maxRl = rl;
    if (rl > 200) armed = true; // pass live (opening or return gallop)
    // sweet spot: stripe left edge 92..103 (joust-proven window)
    if (armed && rl >= 92 && rl <= 103 && lanced < 2) {
      key(" ");
      lanced++;
      armed = false;
    }
  }
  if (lanced === 2 && !posted && resultsPlate(f)) {
    posted = true;
    shot("at-results");
  }
  // the plate scrolls in over ~1 s: ride home once the head line is
  // painted gold, or once any banner has sat painted long enough.
  if (crownSeen > 2 && laurelHead(f)) gold = true;
  if (crownSeen > 2 && (gold || crownSeen > 40)) key("Enter"); // results -> map
  if (lanced >= 2 && crownSeen > 2 && decodeTurn(f) === 22) {
    await Bun.sleep(200); // a calm repainted map frame
    home = true;
  }
}

shot("loop-home");
ok(lanced === 2, `both lances thrown off pixel timing (thrown ${lanced})`);
ok(gold, "the results plate head gleamed GOLD LAUREL");
ok(home, "ENTER returned to the campaign map");
console.log(`diag pips=${pipsSeen} maxRl=${maxRl} lanced=${lanced} elapsed=${Date.now() - t0}ms`);
const purse = snap();
ok(regionDiff(purseBefore, purse, 4, 204, 90, 212) > 0, "the purse line moved with the fee and prize");

const hof = localStorage.getItem("doc.hof") ?? "";
ok(hof.startsWith("TOURNAMENT GOLD - SCORE "), `hall of fame chronicles the meet ("${hof}")`);
console.log("LS:", JSON.stringify(Object.keys(localStorage)), "mg.joust=", localStorage.getItem("doc.mg.joust"));
ok(problems.length === 0, `no errors (${problems.slice(0, 3).join(" | ") || "clean"})`);
await finish();
