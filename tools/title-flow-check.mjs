// T404/T405 acceptance: pure-keyboard title flow (boot->menu->difficulty->
// crawl->campaign), illustrated how-to pages, Options keyboard + CRT filter
// with localStorage persistence. LS_SEED=doc.crt=0 run asserts the OFF
// preference survives a fresh boot.
import { window, coords, tap, key, px, snap, regionDiff, shot, ok, finish, getFrame, getFrameCount, problems, decodeTurn } from "./jsdom-harness.mjs";

const seededOff = window.localStorage.getItem("doc.crt") === "0";
const GOLD_ON = [136, 113, 0]; // 204,170,0 darkened 2/3 on a scanline row
const GOLD_OFF = [204, 170, 0];
const near = (p, c) => Math.abs(p[0] - c[0]) <= 8 && Math.abs(p[1] - c[1]) <= 8 && Math.abs(p[2] - c[2]) <= 8;

await Bun.sleep(700);
ok(getFrameCount() > 30, `rAF ran (${getFrameCount()} frames)`);
shot("boot");
ok(px(160, 60)[2] > 90 && px(160, 60)[0] < 60, `boot banner unfurled rgb=${px(160, 60)}`);

// ENTER lifts the herald and the menu appears with a keyboard selection
key("Enter");
await Bun.sleep(120);
ok(px(160, 124)[0] > 100 && px(160, 124)[1] < 60, `menu highlight on BEGIN rgb=${px(160, 124)}`);

// --- Options by keyboard only (index 3)
key("ArrowDown"); key("ArrowDown"); key("ArrowDown");
key("Enter");
await Bun.sleep(120);
shot("options");
const crtPx = px(100, 120); // gold frame pixel on a darkened scanline row
ok(near(crtPx, seededOff ? GOLD_OFF : GOLD_ON), `CRT filter ${seededOff ? "off" : "on"} at boot rgb=${crtPx}`);
key("ArrowDown"); // CRT row
key("Enter");
await Bun.sleep(120);
const flipped = px(100, 120);
ok(near(flipped, seededOff ? GOLD_ON : GOLD_OFF), `CRT toggles via keyboard rgb=${flipped}`);
ok(window.localStorage.getItem("doc.crt") === (seededOff ? "1" : "0"), `doc.crt persisted (${window.localStorage.getItem("doc.crt")})`);
shot("options-flip");
key("Escape");
await Bun.sleep(120);

// --- How-to: illustrated pages, arrow-key paging (back() resets menu item to 0)
key("ArrowDown"); // HOW TO PLAY
key("Enter");
await Bun.sleep(120);
shot("howto-1");
ok(near(px(64, 130), [136, 119, 102]), `p1 keep drawn rgb=${px(64, 130)}`);
ok(near(px(86, 132), seededOff ? [34, 34, 34] : [51, 51, 51]), `p1 road dash rgb=${px(86, 132)}`); // CRT flipped mid-run
const p1 = snap();
key("ArrowRight");
await Bun.sleep(200);
shot("howto-2");
ok(near(px(72, 115), [153, 0, 34]), `p2 shield rgb=${px(72, 115)}`);
ok(regionDiff(p1, snap(), 16, 96, 304, 216) > 150, `p2 art differs (${regionDiff(p1, snap(), 16, 96, 304, 216)}px)`);
key("ArrowRight");
await Bun.sleep(200);
shot("howto-3");
ok(near(px(160, 129), GOLD_ON) || near(px(160, 129), GOLD_OFF), `p3 crown gold rgb=${px(160, 129)}`);
key("Escape"); // back to the menu (item reset to 0)
await Bun.sleep(120);
// --- Hall of Fame reachable (index 2 from menu top)
key("ArrowDown"); key("ArrowDown");
key("Enter");
await Bun.sleep(150);
shot("fame");
const fameBorder = px(16, 120);
ok(near(fameBorder, GOLD_ON) || near(fameBorder, GOLD_OFF), `fame panel frame rgb=${fameBorder}`);
key("Escape");
await Bun.sleep(120);
// Escape landed back on the menu top (BEGIN CAMPAIGN)

// --- crawl by keyboard: 3 illustrated screens, then the campaign map
key("Enter"); // BEGIN CAMPAIGN
await Bun.sleep(120);
key("Enter"); // EASY by default
await Bun.sleep(150);
shot("crawl-1");
const c1 = snap();
ok(px(2, 0)[0] > 90, `crawl flames rgb=${px(2, 0)}`);
key("Enter");
await Bun.sleep(200);
shot("crawl-2");
ok(regionDiff(c1, snap(), 40, 60, 280, 210) > 120, "crawl advances to next leaf");
key("Enter");
await Bun.sleep(200);
let crown = 0; for (let y = 145; y < 172; y++) for (let x = 138; x < 184; x++) { const p = px(x, y); if (p[0] > 90 && p[1] > 70 && p[2] < 60) crown++; }
ok(crown > 40, `final crawl shows the crown (${crown} gold px)`);

key("Enter"); // commit: the war begins
await Bun.sleep(1100);
ok(getFrameCount() > 80, `frames keep flowing (${getFrameCount()})`);
const campaign = snap();
ok(regionDiff(campaign, c1, 0, 0, 320, 256) > 2000, "crawl gave way to the campaign map");

const turn = decodeTurn(getFrame());
ok(turn !== null, `keyboard flow reached a live campaign (HUD turn ${turn})`);
ok(problems.length === 0, `no errors (${problems.slice(0, 3).join(" | ") || "clean"})`);
finish();
