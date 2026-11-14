// tools/alpha1-loop.ts — GATE ALPHA 1 proof: one unbroken play session in a
// real browser. Title -> difficulty -> campaign crawl -> all three shipped
// sports played to their results plate (sword turn 1, catapult turn 2,
// joust turn 22, ?autoplay=1 auto-resolves the run) -> End Turn clicks drive
// the war -> the AI conquests and the chronicle verdict at turn 90 close the
// loop on the victory banner. Screenshots per stage in tmp/crown-a1/.
import { chromium, type Browser, type Page } from "playwright-core";
import { mkdirSync } from "node:fs";

const PORT = 8137;
const BASE = `http://127.0.0.1:${PORT}`;
const PW_LIBS = "/home/omp/pw-libs/usr/lib/x86_64-linux-gnu";
const SHOTS = "tmp/crown-a1";

const GOLD = [204, 170, 0];
const fails: string[] = [];
function ok(cond: boolean, msg: string) {
  console.log(`${cond ? "ok  " : "FAIL"} ${msg}`);
  if (!cond) fails.push(msg);
}

const server = Bun.spawn(["bun", "tools/serve.ts", String(PORT)], {
  stdout: "pipe",
  stderr: "pipe",
});
{
  const dec = new TextDecoderStream();
  const reader = server.stdout.pipeThrough(dec).getReader();
  let buf = "";
  while (!buf.includes("serving")) {
    const { value, done } = await reader.read();
    if (done) throw new Error("server died");
    buf += value;
  }
  await reader.cancel();
}

mkdirSync(SHOTS, { recursive: true });
let page: Page;
let browser: Browser | undefined;
const errors: string[] = [];
try {
  browser = await chromium.launch({
    env: { ...process.env, LD_LIBRARY_PATH: `${PW_LIBS}:${process.env.LD_LIBRARY_PATH ?? ""}` },
    args: ["--no-sandbox", "--disable-dev-shm-usage", "--autoplay-policy=no-user-gesture-required"],
  });
  page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  page.on("pageerror", (e) => errors.push(String(e)));

  // In-canvas pixel reader: samples the game's own 2D surface at logical
  // 320x256 coordinates.
  const pix = (x: number, y: number) =>
    page.evaluate(
      ([x, y]) => {
        const c = document.getElementById("screen") as HTMLCanvasElement;
        const d = c.getContext("2d")!.getImageData(x, y, 1, 1).data;
        return [d[0], d[1], d[2]];
      },
      [x, y],
    );
  const lin = (x: number, y: number) =>
    page.evaluate(([x, y]) => {
      const c = document.getElementById("screen") as HTMLCanvasElement;
      const d = c.getContext("2d")!.getImageData(x, y, 1, 1).data;
      return d[0] === 204 && d[1] === 204 && d[2] === 153;
    }, [x, y]);
  const countIf = (x0: number, y0: number, x1: number, y1: number, pred: "white" | "gold" | "red") =>
    page.evaluate(
      ([x0, y0, x1, y1, pred]) => {
        const c = document.getElementById("screen") as HTMLCanvasElement;
        const d = c.getContext("2d")!.getImageData(x0, y0, x1 - x0, y1 - y0).data;
        let n = 0;
        for (let i = 0; i < d.length; i += 4 * 2) {
          const r = d[i], g = d[i + 1], b = d[i + 2];
          const hit =
            pred === "white" ? r > 200 && g > 200 && b > 200
            : pred === "gold" ? r === 204 && g === 170 && b === 0
            : r > 100 && g < 60 && b < 60;
          if (hit) n++;
        }
        return n;
      },
      [x0, y0, x1, y1, pred],
    );
  const shot = (name: string) => page.screenshot({ path: `${SHOTS}/${name}.png` });
  // logical -> CSS pixel click on the scaled canvas
  const tap = async (x: number, y: number) => {
    const r = await page.evaluate(() => {
      const c = document.getElementById("screen")!.getBoundingClientRect();
      return { l: c.left, t: c.top, w: c.width, h: c.height };
    });
    await page.mouse.click(r.l + (x * r.w) / 320, r.t + (y * r.h) / 256);
  };
  const banner = () => countIf(80, 119, 240, 127, "white");
  // campaign iff the HUD turn counter at (160,204) decodes as digits
  const inCampaign = async () => {
    const plate = await page.evaluate(() => {
      // Results plate = yellow banner border across y=56..58 (40..280).
      const c = document.getElementById("screen") as HTMLCanvasElement;
      const ctx = c.getContext("2d")!;
      let n = 0;
      for (let y = 56; y <= 58; y++)
        for (let x = 40; x <= 280; x++) {
          const p = ctx.getImageData(x, y, 1, 1).data;
          if (p[0] > 150 && p[1] > 150 && p[2] < 90) n++;
        }
      return n;
    });
    if (plate > 50) return false;
    const t = await page.evaluate(() => {
      const DIG = [
        [0x70, 0x88, 0x98, 0xa8, 0xc8, 0x88, 0x70, 0x00],
        [0x20, 0x60, 0x20, 0x20, 0x20, 0x20, 0x70, 0x00],
        [0x70, 0x88, 0x08, 0x10, 0x20, 0x40, 0xf8, 0x00],
        [0xf8, 0x10, 0x20, 0x70, 0x08, 0x88, 0x70, 0x00],
        [0x10, 0x30, 0x50, 0x90, 0xf8, 0x10, 0x10, 0x00],
        [0xf8, 0x80, 0xf0, 0x08, 0x08, 0x88, 0x70, 0x00],
        [0x30, 0x40, 0x80, 0xf0, 0x88, 0x88, 0x70, 0x00],
        [0xf8, 0x08, 0x10, 0x20, 0x40, 0x40, 0x40, 0x00],
        [0x70, 0x88, 0x88, 0x70, 0x88, 0x88, 0x70, 0x00],
        [0x70, 0x88, 0x88, 0x78, 0x08, 0x10, 0x60, 0x00],
      ];
      const c = document.getElementById("screen") as HTMLCanvasElement;
      const ctx = c.getContext("2d")!;
      let s = "";
      for (let k = 0; k < 2; k++) {
        const rows: number[] = [];
        for (let r = 0; r < 8; r++) {
          let m = 0;
          for (let col = 0; col < 8; col++) {
            const p = ctx.getImageData(160 + k * 8 + col, 204 + r, 1, 1).data;
            if (p[0] > 150 && p[1] > 150 && p[2] > 150) m |= 0x80 >> col;
          }
          rows.push(m);
        }
        if (rows.every((v) => v === 0)) break;
        const f = DIG.findIndex((dg) => dg.every((v, i) => v === rows[i]));
        if (f < 0) break;
        s += f;
      }
      return s === "" ? null : parseInt(s, 10);
    });
    return t !== null;
  };

  // Omen/report modals steal the map: tap the dismiss plate (proven coords).
  const dismissModals = async () => {
    if (await lin(64, 100)) {
      await tap(10, 10);
      await page.waitForTimeout(120);
    }
    if (await lin(74, 70)) {
      await tap(160, 128);
      await page.waitForTimeout(120);
    }
  };

  // ---- phase 1: title screen ------------------------------------------------
  await page.goto(`${BASE}/?autoplay=1`, { waitUntil: "load" });
  await page.waitForTimeout(1500); // herald boot + fanfare
  const bootBanner = await pix(160, 60);
  ok(bootBanner[2] > 90 && bootBanner[0] < 60, `boot banner unfurled rgb=${bootBanner}`);
  await shot("00-boot");

  await page.keyboard.press("Space"); // silence herald
  await page.waitForTimeout(1100); // banner rolls up, menu appears
  const titleHi = await pix(160, 124);
  ok(titleHi[0] > 100 && titleHi[1] < 60, `title menu highlights BEGIN rgb=${titleHi}`);
  await shot("00-title");

  await page.keyboard.press("Enter"); // BEGIN CAMPAIGN
  await page.waitForTimeout(500);
    const red = await countIf(60, 150, 260, 215, "red");
    ok(red > 8, `difficulty menu shows a red selection bar (${red}px)`);
  await shot("01-difficulty");

  // ---- phase 2: campaign crawl -> live map ---------------------------------
  await page.keyboard.press("ArrowDown"); // NORMAL
  await page.keyboard.press("Enter");
  for (let i = 0; i < 8; i++) {
    await page.waitForTimeout(600);
    if (await inCampaign()) break;
    await page.keyboard.press("Enter"); // crawl leaves
  }
  await page.waitForTimeout(500);
  ok(await inCampaign(), "campaign map reached (END TURN button on the HUD)");
  await shot("02-campaign");

  // ---- phase 3: play all three shipped sports ------------------------------
  // Draws: sword turn 1 (now), joust turn 5, catapult turn 9. The meet is
  // held on the Buckingham panel; autoplay resolves the run to its plate.
  const played = new Set<string>();
  // the campaign says its payouts as a toast in the bottom HUD band
  const laurel = async () =>
    (await countIf(40, 228, 300, 252, "gold")) + (await countIf(40, 228, 300, 252, "white"));
  const hof = () => page.evaluate(() => localStorage.getItem("doc.hof") ?? "");
  // HUD turn counter: same glyph region inCampaign decodes
  const turnNum = async (): Promise<number | null> =>
    page.evaluate(() => {
      const DIG = [
        [0x70, 0x88, 0x98, 0xa8, 0xc8, 0x88, 0x70, 0x00],
        [0x20, 0x60, 0x20, 0x20, 0x20, 0x20, 0x70, 0x00],
        [0x70, 0x88, 0x08, 0x10, 0x20, 0x40, 0xf8, 0x00],
        [0xf8, 0x10, 0x20, 0x70, 0x08, 0x88, 0x70, 0x00],
        [0x10, 0x30, 0x50, 0x90, 0xf8, 0x10, 0x10, 0x00],
        [0xf8, 0x80, 0xf0, 0x08, 0x08, 0x88, 0x70, 0x00],
        [0x30, 0x40, 0x80, 0xf0, 0x88, 0x88, 0x70, 0x00],
        [0xf8, 0x08, 0x10, 0x20, 0x40, 0x40, 0x40, 0x00],
        [0x70, 0x88, 0x88, 0x70, 0x88, 0x88, 0x70, 0x00],
        [0x70, 0x88, 0x88, 0x78, 0x08, 0x10, 0x60, 0x00],
      ];
      const c = document.getElementById("screen") as HTMLCanvasElement;
      const ctx = c.getContext("2d")!;
      let s = "";
      for (let k = 0; k < 2; k++) {
        const rows: number[] = [];
        for (let r = 0; r < 8; r++) {
          let m = 0;
          for (let col = 0; col < 8; col++) {
            const p = ctx.getImageData(160 + k * 8 + col, 204 + r, 1, 1).data;
            if (p[0] > 150 && p[1] > 150 && p[2] > 150) m |= 0x80 >> col;
          }
          rows.push(m);
        }
        if (rows.every((v) => v === 0)) break;
        const f = DIG.findIndex((dg) => dg.every((v, i) => v === rows[i]));
        if (f < 0) break;
        s += f;
      }
      return s === "" ? null : parseInt(s, 10);
    });
  // END TURN with proof: a modal can swallow the tap, so demand the
  // counter move, clearing modals between tries.
  const advance = async (): Promise<boolean> => {
    for (let i = 0; i < 4; i++) {
      const t0 = await turnNum();
      await dismissModals();
      await tap(264, 217);
      await page.waitForTimeout(950);
      if ((await turnNum()) !== t0) return true;
    }
    return false;
  };
  const playSport = async (label: string) => {
    let head = 0;
    for (let attempt = 0; attempt < 3 && head <= 8; attempt++) {
      await dismissModals();
      await tap(125, 100); // open Buckingham panel
      await page.waitForTimeout(250);
      await tap(184, 165); // SPORTS
      // intro animation, autoplay run, plate scroll-in: poll for the gold head
      for (let i = 0; i < 30 && head <= 8; i++) {
        await page.waitForTimeout(200);
        head = await countIf(104, 72, 204, 79, "gold");
      }
      if (head <= 8) await shot(`sport-${label}-miss${attempt}`);
    }
    ok(head > 8, `${label}: results plate head gleams gold (${head}px)`);
    await shot(`sport-${label}`);
    // the plate answers ENTER; a stray omen modal answers the OK tap
    let home = await inCampaign();
    for (let i = 0; i < 8 && !home; i++) {
      await page.keyboard.press("Enter");
      await page.waitForTimeout(250);
      home = await inCampaign();
      if (!home) await dismissModals();
    }
    ok(home, `${label}: returned to the campaign map`);
    const lz = await laurel();
    if (label === "sword") {
      const strip = await page.evaluate(() => {
        const c = document.getElementById("screen") as HTMLCanvasElement;
        const t = document.createElement("canvas");
        t.width = 1920; t.height = 72;
        const g = t.getContext("2d")!;
        g.imageSmoothingEnabled = false;
        g.drawImage(c, 0, 244, 320, 12, 0, 0, 1920, 72);
        return t.toDataURL();
      });
      const b64 = strip.split(",")[1];
      await Bun.write(`${SHOTS}/toast-sword.png`, Buffer.from(b64, "base64"));
      console.log("  ls keys:", await page.evaluate(() => {
        const a = JSON.stringify(Object.keys(localStorage));
        try {
          localStorage.setItem("probe", "x");
          return a + " jsSet=" + localStorage.getItem("probe") + " len=" + localStorage.length;
        } catch (e) {
          return a + " jsErr=" + String(e);
        }
      }));
      await page.keyboard.press("KeyM");
      await page.waitForTimeout(300);
      const muteOn = await page.evaluate(() => localStorage.getItem("doc.mute"));
      await page.keyboard.press("KeyM");
      await page.waitForTimeout(300);
      const muteOff = await page.evaluate(() => localStorage.getItem("doc.mute"));
      ok(muteOn === "1" && muteOff === "0", `M key toggles mute and persiteth it (${muteOn}->${muteOff})`);
      await shot("after-sword-home");
    }
    console.log(`  ${label}: hof="${(await hof()).slice(0, 46)}"`);
    played.add(label);
  };
  await playSport("sword"); // turn 1
  await advance(); // -> turn 2: the catapult draw
  await playSport("catapult");
  for (let t = 3; t <= 22; t++) await advance();
  await playSport("joust"); // turn 22 (splitmix-scrambled draw)

  const hofFinal = await hof();
  ok(/(GOLD|SILVER|BRONZE) LAUREL|GOLD - SCORE|CROWNED/.test(hofFinal), `hall of fame chronicles the run ("${hofFinal.slice(0, 46)}")`);

  // ---- phase 4: end turn drives the AI war to its verdict ------------------
  let endTurns = 0;
  let verdictAt = 0;
  for (let t = 10; t <= 130; t++) {
    const before = await banner();
    if (!(await advance())) continue;
    endTurns++;
    if ((await banner()) > before + 40) {
      verdictAt = endTurns;
      break;
    }
  }
  ok(verdictAt > 0, `chronicle verdict painted on the victory banner (End Turn #${verdictAt})`);
  await page.waitForTimeout(1200);
  const stable = await banner();
  ok(stable > 40, `banner still painted (end screen lingers: ${stable}px white)`);
  await shot("03-victory");

  ok(errors.length === 0, `no console errors (${errors.slice(0, 3).join(" | ") || "clean"})`);
  ok(played.size === 3, `all three sports played (${[...played].join(", ")})`);
} finally {
  await browser?.close();
  server.kill();
}
if (fails.length) {
  console.log(`FAILED ${fails.length}`);
  process.exit(1);
}
console.log("ALPHA1 LOOP GREEN");
process.exit(0);
