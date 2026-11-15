// T601 browser proof: the GALLERY drawn at TURN 14 shoots GOLD from pixels
// only. Boot deep-links scene=play&turn=14 (season pick = archery under the
// splitmix-scrambled draw),
// the script steers the shaft with the arrow keys reading the GOLD bullseye
// and RED sight pixel off the framebuffer, looses with SPACE, and finally
// clips the apple. Score plate + purse delta close the proof.
process.env.CROWN_QS ??= "scene=play&seed=7&turn=14&mute=1";
const { coords, tap, key, keyDown, keyUp, px, snap, decodeTurn, regionDiff, shot, ok, finish, problems } = await import("./jsdom-harness.mjs");
// T706 gold family: bright (187,153,51), dimmed (124,102,34)
const goldP = (p) => p[0] >= 110 && p[0] - p[1] >= 20 && p[0] - p[1] <= 55 && p[1] - p[2] >= 30;
const RED = [153, 34, 17]; // T706 Norman red
const WHITE = [255, 255, 255];
const eq = (p, q) => p[0] === q[0] && p[1] === q[1] && p[2] === q[2];

// --- pixel senses -----------------------------------------------------------
// bullseye centre: the 4x4 GOLD patch inside the black booth hole. The
// results plate text is GOLD too but lives at y<=104, above the field.
function targets(f) {
  const out = [];
  for (let y = 108; y < 186; y++) {
    for (let x = 56; x < 264; x++) {
      const i = (y * 320 + x) * 4;
      if (goldP([f[i], f[i + 1], f[i + 2]])) {
        // cluster top-left of a 4x4 run
        if (!out.some((t) => Math.abs(t[0] - x) < 6 && Math.abs(t[1] - y) < 6)) out.push([x + 2, y + 2]);
      }
    }
  }
  return out;
}
// sight: the lone RED pixel ringed by the BLACK+WHITE cross arms (the silk
// can cover one arm, so each side only has to read dark or white)
function cross(f) {
  for (let y = 54; y < 214; y++) {
    for (let x = 8; x < 316; x++) {
      const i = (y * 320 + x) * 4;
      if (f[i] === RED[0] && f[i + 1] === RED[1] && f[i + 2] === RED[2]) {
        const arm = (dx, dy) => {
          const j = ((y + dy) * 320 + (x + dx)) * 4;
          if (f[j] < 60 && f[j + 1] < 60 && f[j + 2] < 60) return true; // BLACK arm
          // WHITE arm, incl. its dither-fade grays (neutral mid-tones)
          return f[j] > 140 && Math.abs(f[j] - f[j + 1]) < 24 && Math.abs(f[j + 1] - f[j + 2]) < 24;
        };
        if (arm(3, 0) && arm(-3, 0) && arm(0, 3) && arm(0, -3)) return [x, y];
      }
    }
  }
  return null;
}
// apple: 6x6 RED blob high on the silk (no white ring, y < 92)
function apple(f) {
  for (let y = 40; y < 94; y++) {
    let run = 0;
    for (let x = 90; x < 232; x++) {
      const i = (y * 320 + x) * 4;
      run = f[i] === RED[0] && f[i + 1] === RED[1] && f[i + 2] === RED[2] ? run + 1 : 0;
      if (run >= 4) return [x - 2, y + 3];
    }
  }
  return null;
}

// the harness drives rAF at ~60 Hz; sleep(ms) advances ~ms/16 frames
const adv = async (n) => { await Bun.sleep(Math.max(16, Math.round((n * 1000) / 60))); };

// NO space here: on the campaign screen it means END TURN, and the fresh
// world already satisfies the coronation win (8 shires + London) - one
// stray turn ends the war before the meet. FakeAudioCtx needs no gesture.
// Boot is NOT a wall-clock bet: the wasm only starts after the coords
// helper's cargo build resolves, so a cold compile would outrun any fixed
// sleep. Poll the deep-linked HUD (TURN 14) instead.
// Campaign tell: the map's cyan sky at (160,44) - the title intro crown
// and the yellow menu lettering never paint pure cyan there. decodeTurn
// reads only the first two glyphs, so the deep-linked TURN 14 arrives
// as 14 (the meet may already have turned it to 15 or so).
let booted = false;
for (let i = 0; i < 600 && !booted; i++) {
  await Bun.sleep(200);
  const sky = px(160, 44);
  if (sky[0] >= 140 && sky[0] - sky[2] >= 40 && sky[1] - sky[2] >= 25) { // T706 parchment map top (bright or dim row)
    const t = decodeTurn(snap());
    if (t !== null && t >= 14 && t < 26) booted = true;
  } else if (i > 150) {
    problems.push("no campaign map within 30s - the war ended before we arrived");
    break;
  }
}
ok(booted, "the deep-linked campaign answered at turn 14");
await Bun.sleep(120);
const linAt = (x, y) => { const f = snap(); const i = (y * 320 + x) * 4; const r = f[i], g = f[i + 1], b = f[i + 2]; return (Math.abs(r - 187) <= 14 && Math.abs(g - 170) <= 14 && Math.abs(b - 119) <= 14) || (Math.abs(r - 124) <= 12 && Math.abs(g - 113) <= 12 && Math.abs(b - 79) <= 12); };
// dismiss any campaign modal that covers the plate area (linen title at the
// centre), but never tap on a bare map: a stray tap there selects a castle,
// whose linen plate the loop would then mistake for yet another modal.
for (let i = 0; i < 40 && linAt(64, 100); i++) { tap(160, 128); await Bun.sleep(140); }
for (let i = 0; i < 40 && linAt(74, 70); i++) { tap(160, 128); await Bun.sleep(140); }
ok(!linAt(64, 100), "the map is bare before the meet");

// the campaign runs a REAL-TIME turn clock: idling lets the AI finish the
// whole war while we watch. Do not wait on frame equality (marching armies
// and flags never stop) - the FF is synchronous, a short nap suffices.
await Bun.sleep(450);
const purseBefore = snap();
shot("pre-round");
let opened = false;
for (let round = 0; round < 3 && !opened; round++) {
  tap(...coords.BUCK);
  await Bun.sleep(250);
  tap(...coords.SPORTS);
  // the meet launches through a fade: poll for the sight crosshair or the
  // blinking "NOCK ARROWS" banner (WHITE glyphs at y128) as launch proof
  for (let i = 0; i < 120 && !opened; i++) {
    const f = snap();
    if (cross(f)) { opened = true; break; } // the sight: red ringed by arms
    for (let x = 96; x < 190 && !opened; x++) {
      const i2 = (128 * 320 + x) * 4;
      if (f[i2] === 255 && f[i2 + 1] === 255 && f[i2 + 2] === 255) opened = true;
    }
    // linen at (64,100) is BOTH the gallery silk and a campaign modal: only
    // tap it away while the campaign footer (black band at (5,225)) is up.
    if (!opened && linAt(64, 100)) {
      const i3 = (225 * 320 + 5) * 4;
      if (f[i3] < 40 && f[i3 + 1] < 40 && f[i3 + 2] < 40) { tap(160, 128); await adv(9); continue; }
    }
    if (!opened) await adv(3);
  }
  if (!opened) { // a campaign modal probably ate the taps: clear and retry
    for (let i = 0; i < 8 && (linAt(64, 100) || linAt(74, 70)); i++) { tap(160, 128); await Bun.sleep(120); }
    await Bun.sleep(200);
  }
}
shot("launch-state");
ok(opened, "SPORTS launched the GALLERY at the turn-14 season meet");
if (!opened) { await finish(); process.exit(1); }
shot("archery-present");
await adv(45); // let the "NOCK ARROWS" banner run down
for (let i = 0; i < 200 && !cross(snap()); i++) await adv(3);

// --- shoot the poppers --------------------------------------------------------
// one shaft per bullseye: walk the sight onto the gold patch, loose, confirm.
async function looseOn(gx, gy, appleMode = false) {
  // hold-and-release: engine steps 9 px per held frame
  const hold = async (k, d) => {
    const n = Math.max(1, Math.round(Math.abs(d) / 9));
    keyDown(k);
    await adv(n);
    keyUp(k);
    await adv(1);
  };
  for (let tries = 0; tries < 6; tries++) {
    if (appleMode) [gx, gy] = apple(snap()) ?? [gx, gy]; // it swings; re-read
    const c = cross(snap());
    if (!c) return false;
    let [cx, cy] = c;
    for (let adjust = 0; adjust < 4; adjust++) {
      [cx, cy] = cross(snap()) ?? [cx, cy];
      const dx = gx - cx;
      const dy = gy - cy;
      if (Math.abs(dx) <= 5 && Math.abs(dy) <= 5) break;
      if (Math.abs(dx) > 5) await hold(dx > 0 ? "ArrowRight" : "ArrowLeft", dx);
      if (Math.abs(dy) > 5) await hold(dy > 0 ? "ArrowDown" : "ArrowUp", dy);
    }
    [cx, cy] = cross(snap()) ?? [cx, cy];
    if (Math.abs(gx - cx) > 9 || Math.abs(gy - cy) > 9) continue;
    key(" "); // loose
    await adv(9);
    const f = snap();
    if (appleMode) { if (!apple(f)) return true; continue; } // gone = clipped
    // hit = the GOLD bullseye no longer burns at the mark
    const near = (o) => Math.abs(o[0] - gx) < 14 && Math.abs(o[1] - gy) < 14;
    if (!targets(f).some(near)) return true;
  }
  return false;
}

let hits = 0;
let misses = 0;
let appleHit = false;
let idle = 0;
const t0 = Date.now();
for (let it = 0; it < 100000; it++) {
  if (Date.now() - t0 >= 18000) break;
  const f = snap();
  const tg = targets(f);
  if (!tg.length) {
    // the apple is live from volley 3 on: clip it inline whenever it swings in
    const a = apple(f);
    if (a && !appleHit) { appleHit = await looseOn(a[0], a[1], true); continue; }
    if (appleHit && ++idle > 8) break; // done here: let the board come up
    await Bun.sleep(60);
    continue;
  }
  idle = 0;
  // nearest bullseye first: cheapest walk
  const c = cross(f);
  if (!c) { await Bun.sleep(60); continue; } // mid-fade frame: wait, no shot
  tg.sort((a, b) => Math.abs(a[0] - c[0]) + Math.abs(a[1] - c[1]) - Math.abs(b[0] - c[0]) - Math.abs(b[1] - c[1]));
  const [gx, gy] = tg[0];
  if (await looseOn(gx, gy)) hits++;
  else misses++;
}
shot("archery-run");

// --- clip the apple ------------------------------------------------------------
// late fallback: keep swinging at the apple until the board takes over
const tA = Date.now();
while (!appleHit && Date.now() - tA < 12000) {
  const a = apple(snap());
  if (!a) { await Bun.sleep(60); continue; }
  appleHit = await looseOn(a[0], a[1], true);
}
shot("archery-apple");

// --- results plate --------------------------------------------------------------
let scoreSeen = false;
for (let i = 0; i < 150 && !scoreSeen; i++) {
  const f = snap();
  // the scanline filter dims alternate rows: T706 GOLD (bright or dim)
  // anywhere in the SCORE text band (y56..72, x100..220) is the results plate.
  scan: for (let y = 56; y < 72; y++) for (let x = 100; x < 220; x++) { const i2 = (y * 320 + x) * 4; const r = f[i2], g = f[i2 + 1], b = f[i2 + 2]; if (r >= 115 && r - g >= 20 && r - g <= 60 && g - b >= 60) { scoreSeen = true; break scan; } }
  if (!scoreSeen) await Bun.sleep(100);
}
ok(scoreSeen, "the GALLERY board showed the score");
await Bun.sleep(400);
// board -> cheer -> home: Enter ONLY until the campaign footer is back.
// One Enter too many means END TURN, and the deep-linked fresh world is
// one turn from the chronicle verdict (the Saxons start with 8 + London).
let home = false;
for (let i = 0; i < 12 && !home; i++) {
  key("Enter");
  await Bun.sleep(120);
  const f = snap();
  const i3 = (225 * 320 + 5) * 4;
  home = f[i3] < 40 && f[i3 + 1] < 40 && f[i3 + 2] < 40;
}
ok(home, "the campaign map received the hero back");
for (let i = 0; i < 8 && linAt(64, 100); i++) { tap(160, 128); await Bun.sleep(120); }

const purse = snap();
ok(regionDiff(purseBefore, purse, 4, 204, 90, 212) > 0, "the purse line moved with the fee and prize");
ok(hits >= 9, `script loosed at ${hits} targets${appleHit ? " and the apple" : ""} (misses ${misses})`);
ok(appleHit, "the apple was clipped for the Gold route");
ok(problems.length === 0, `no errors (${problems.slice(0, 3).join(" | ") || "clean"})`);
await finish();
