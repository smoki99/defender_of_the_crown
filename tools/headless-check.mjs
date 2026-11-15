// Headless end-to-end (T1xx-T3xx campaign): keyboard-driven title flow
// (boot -> menu -> difficulty -> crawl -> campaign), then real click play:
// castle select, levy, march, five turn cycles, report + omen dismissal.
import { coords, tap, key, px, snap, regionDiff, shot, ok, finish, getFrame, getFrameCount, problems, decodeTurn } from "./jsdom-harness.mjs";

await Bun.sleep(700);
ok(getFrameCount() > 30, `rAF ran (${getFrameCount()} frames)`);
shot("boot");
ok(px(160, 60)[2] > 90 && px(160, 60)[0] < 60, `boot banner unfurled rgb=${px(160, 60)}`);

// keyboard: one gesture lifts the boot herald, Enter starts the campaign
key(" ");
await Bun.sleep(120);
ok(px(102, 124)[0] > 100 && px(102, 124)[1] < 60, `menu highlights BEGIN CAMPAIGN rgb=${px(102, 124)}`);
key("Enter");
await Bun.sleep(120);
ok(px(82, 181)[0] > 120 && px(82, 181)[1] > 90 && px(82, 181)[2] < 40 && px(162, 181)[0] > 90 && px(162, 181)[1] < 60, `difficulty highlights EASY (gold fill) rgb=${px(82, 181)} vs rgb=${px(162, 181)}`);
key("ArrowDown"); // NORMAL
key("Enter");
await Bun.sleep(120);
shot("crawl");
const crawl = snap();
const cA = snap();
await Bun.sleep(350);
const cB = snap();
await Bun.sleep(350);
const cC = snap();
const flameD = Math.max(regionDiff(cA, cB, 0, 0, 320, 16), regionDiff(cB, cC, 0, 0, 320, 16), regionDiff(cA, cC, 0, 0, 320, 16));
ok(flameD > 40, `crawl flame border animates (${flameD}px changed)`);
key("Enter"); key("Enter"); key("Enter"); // advance x2 + commit
await Bun.sleep(900);
ok(getFrameCount() > 60, `frames keep flowing after transition (${getFrameCount()})`);
const campaign = snap();
shot("campaign");
ok(regionDiff(crawl, campaign, 0, 0, 320, 256) > 2000, "crawl gave way to the campaign map");

// Buckingham selection opens the panel
const [bx, by] = coords.BUCK;
tap(bx, by);
await Bun.sleep(150);
const selected = snap();
const panelDiff = regionDiff(campaign, selected, 0, 88, 216, 132);
ok(panelDiff > 400, `panel region changed on Buckingham tap (${panelDiff}px)`);
shot("selected");

// LEVY charge mutates the info text
tap(...coords.LEVY);
await Bun.sleep(150);
const levyFrame = snap();
shot("levy");
tap(bx, by); // re-tap keeps panel; then check HUD gold band changed
await Bun.sleep(120);
const goldBandChanged = regionDiff(campaign, snap(), 4, 214, 120, 252);
ok(goldBandChanged > 10, `HUD gold band changed after levy (${goldBandChanged}px)`);
// March on London: (re-)select Buckingham, then tap the adjacent enemy keep.
tap(bx, by);
await Bun.sleep(100);
tap(...coords.LON);
await Bun.sleep(200);
// March shows "THE HOST MARCHES ON London" in the ticker (drawn y246);
// the enemy keep's pixels only change once combat resolves at dawn.
const marchDiff = regionDiff(levyFrame, snap(), 0, 244, 320, 256);
ok(marchDiff > 60, `click castle->march printed ticker message (${marchDiff}px of text)`);
shot("march");

// END TURN x5: five player+AI cycles purely by clicking, reading the HUD
// turn counter straight out of the framebuffer (8x8 glyph decode at (160,204)).

async function waitTurn(target, budgetMs = 20000) {
  const t0 = Date.now();
  while (Date.now() - t0 < budgetMs) {
    if (getFrame() && decodeTurn(getFrame()) === target) return true;
    await Bun.sleep(100);
  }
  return false;
}
// Overlay plates (LINEN) at text-free probe points: battle report (T306)
// centre-left, omen modal (T307) top-left corner.
const linAt = (x, y) => { const f = getFrame(); if (!f) return false; const i = (y * 320 + x) * 4; return f[i] === 204 && f[i + 1] === 204 && f[i + 2] === 153; };
let reportSeen = false;
let modalSeen = false;
async function dismissReport() {
  for (let i = 0; i < 30 && !linAt(64, 100) && !linAt(74, 70); i++) await Bun.sleep(100);
  const rep = linAt(64, 100);
  const mod = linAt(74, 70);
  if (rep && !reportSeen) { reportSeen = true; shot("report"); }
  if (rep) {
    tap(10, 10);
    await Bun.sleep(150);
    tap(10, 10);
    await Bun.sleep(150);
    ok(!linAt(64, 100), "report dismissed by two taps");
  }
  if (mod && !modalSeen) { modalSeen = true; shot("omen"); }
  if (mod) {
    tap(160, 128); // the modal's OK button
    await Bun.sleep(150);
    ok(!linAt(74, 70), "OK button dismissed the omen modal");
  }
  return rep || mod;
}
const before = getFrameCount();
const turn0 = decodeTurn(getFrame());
ok(turn0 !== null, `HUD turn counter decodes from pixels (turn ${turn0})`);
for (let cyc = 1; cyc <= 5; cyc++) {
  await dismissReport(); // battle report blocks the map until skipped+closed
  tap(...coords.END);
  ok(await waitTurn(turn0 + cyc), `click ${cyc}/5: HUD reaches TURN ${turn0 + cyc}`);
  if (cyc === 1 || cyc === 5) shot(`turn-${turn0 + cyc}`);
}
ok(reportSeen, "a battle report appeared and was skipped+closed by clicks");
ok(modalSeen, "an omen modal appeared and OK dismissed it");
// jsdom's rAF throttles under polling load (~6-60fps observed); the decoded
// TURN digits are the real liveness proof, this only catches a dead loop.
ok(getFrameCount() - before > 10, `frame loop alive through five cycles (${getFrameCount() - before} frames)`);
ok(problems.length === 0, `no errors (${problems.slice(0, 3).join(" | ") || "clean"})`);
finish();
