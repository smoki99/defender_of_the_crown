// T504 browser playtest: TURN 1's SPORTS meet hands out the SWORD duel.
// The harness reads the foe's blade tip and feint flash off the framebuffer,
// answers with the matching guard lane (the foe's last tell before the
// strike wins - guard presses overwrite), strikes SPACE inside the
// stagger-meter gap, and rides the laurel plate home to the campaign map.
import { coords, tap, key, px, snap, regionDiff, shot, ok, finish, problems, decodeTurn } from "./jsdom-harness.mjs";

const GOLD = [204, 170, 0];
const BLACK = [0, 0, 0];
const WHITE = [255, 255, 255];
const eq = (p, q) => p[0] === q[0] && p[1] === q[1] && p[2] === q[2];
const POSE_Y = [118, 152, 188];
const LANE_KEY = ["ArrowUp", "ArrowRight", "ArrowDown"];

// foe's blade tip: WHITE block at x180..186 on the tell-lane row
function tipLane() {
  for (let l = 0; l < 3; l++) {
    const y = POSE_Y[l];
    for (let x = 178; x < 188; x += 2) if (eq(px(x, y), WHITE)) return l;
  }
  return null;
}
const meterOn = () => eq(px(160, 232), BLACK);
const duelPlate = () => eq(px(36, 92), GOLD); // THE DUEL plate border
const resPlate = () => eq(px(40, 56), GOLD) && eq(px(279, 199), GOLD); // host results
function laurel() {
  // the results head line is GOLD on a win, BLOOD on a loss
  let n = 0;
  for (let y = 72; y < 79; y++) for (let x = 104; x < 204; x += 2) if (eq(px(x, y), GOLD)) n++;
  return n > 8;
}
// the map HUD purse line: "GOLD nnn" in GOLD glyphs at (4,204).
function purseText() {
  let n = 0;
  for (let y = 204; y < 212; y++)
    for (let x = 4; x < 90; x++) if (eq(px(x, y), GOLD)) n++;
  return n;
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
await dismissQuick();
const mapA = snap();
ok(decodeTurn(mapA) === 1, "campaign opens at turn 1 (the sword's drawn meet)");

tap(...coords.BUCK);
await Bun.sleep(150);
tap(...coords.SPORTS);
await Bun.sleep(400);
ok(duelPlate(), "THE DUEL plate presents the tilt");
shot("sword-present");

let guards = 0;      // lane keys thrown at the foe's tell
let ripostes = 0;    // SPACE thrown inside the gap
let won = false;     // laurel head on the results plate
let resSeen = false; // results plate already judged once
let home = false;
let spaceAt = 0;     // riposte beat: the gap opens ~250 ms into the stagger
let seen = null;     // lane the tip has been showing, with sample count
let armed = false;   // inside an exchange round
const t1 = Date.now();
while (Date.now() - t1 < 60000 && !home) {
  await Bun.sleep(33);
  if (decodeTurn(snap()) === 1) home = true;
  if (home) break;
  if (spaceAt !== 0 && Date.now() >= spaceAt) { key(" "); ripostes++; spaceAt = 0; }
  if (resPlate()) {
    if (!resSeen) {
      resSeen = true;
      won = laurel();
      shot("sword-result");
    }
    key("Enter");
    await Bun.sleep(250);
    continue;
  }
  if (duelPlate()) continue;
  const tip = tipLane();
  if (meterOn()) {
    if (spaceAt === 0) spaceAt = Date.now() + 250; // stagger gap opens at tick ~14
    continue;
  }
  if (tip === null) { armed = true; seen = null; continue; }
  if (!armed) armed = true;
  if (tip !== seen) {
    // the tell appeared or the feint switched: the newest lane wins,
    // since a later guard press simply overwrites the earlier one.
    seen = tip;
    key(LANE_KEY[tip]); guards++;
  }
}
// the foe reeled: the stagger meter must have been seen while armed
ok(guards >= 3, `guard lanes read and answered off pixels (thrown ${guards})`);
ok(ripostes >= 1, `riposte strikes thrown in the stagger gap (${ripostes})`);
ok(won, "a laurel closed the duel (results plate head in gold)");
ok(home, "ENTER leaves the duel back on the campaign map");
await Bun.sleep(200); // let the map repaint a calm frame before comparing
const mapB = snap();
shot("sword-home");
ok(regionDiff(mapA, mapB, 4, 204, 90, 212) > 0, "the purse line moved with the fee and prize");
ok(purseText() > 0, "the purse still reads on the HUD");
ok(problems.length === 0, `no errors (${problems.slice(0, 3).join(" | ") || "clean"})`);
await finish();
