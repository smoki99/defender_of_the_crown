// tools/browser-check.ts — real-headless-Chromium proof of the T107 shell:
// boots the page, verifies fps, canvas pixels, tap/wheel/keys, transitions.
import { chromium, type Browser } from "playwright-core";

const PORT = 8099;
const BASE = `http://127.0.0.1:${PORT}`;
const PW_LIBS = "/home/omp/pw-libs/usr/lib/x86_64-linux-gnu";

const YELLOW = [255, 238, 68];
const GREEN = [34, 136, 34];
const TAN = [187, 153, 102];
const BLOOD = [153, 0, 34];

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
  server.stdout.pipeThrough(dec);
  const reader = dec.readable.getReader();
  let buf = "";
  while (!buf.includes("serving")) {
    const { value, done } = await reader.read();
    if (done) throw new Error("server died");
    buf += value;
  }
  await reader.cancel();
}

let browser: Browser | undefined;
try {
  browser = await chromium.launch({
    env: {
      ...process.env,
      LD_LIBRARY_PATH: `${PW_LIBS}:${process.env.LD_LIBRARY_PATH ?? ""}`,
    },
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
  });
  const page = await (await browser.newContext({ viewport: { width: 1280, height: 800 } })).newPage();
  const errors: string[] = [];
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  page.on("pageerror", (e) => errors.push(String(e)));

  // canvas pixel helpers evaluated inside the page (exact palette match)
  type Box = { x: number; y: number; w: number; h: number };
  const countPix = (rgb: number[], box?: Box) =>
    page.evaluate(
      ([c, b]: [number[], Box | null]) => {
        const cv = document.getElementById("screen") as HTMLCanvasElement;
        const g = cv.getContext("2d")!;
        const x0 = b?.x ?? 0, y0 = b?.y ?? 0;
        const d = g.getImageData(x0, y0, b?.w ?? 320, b?.h ?? 256).data;
        let n = 0;
        for (let i = 0; i < d.length; i += 4)
          if (d[i] === c[0] && d[i + 1] === c[1] && d[i + 2] === c[2]) n++;
        return n;
      },
      [rgb, box ?? null],
    );
  const logical = async (lx: number, ly: number) => {
    const box = await page.locator("#screen").boundingBox();
    const s = (box!.width / 320);
    return { x: box!.x + lx * s, y: box!.y + ly * s };
  };

  // ---- test scene
  await page.goto(`${BASE}/?scene=test`);
  await page.waitForFunction("window.__fps > 0", null, { timeout: 10000 });
  await page.waitForTimeout(1600);
  const fps = await page.evaluate("window.__fps");
  ok(fps >= 45, `fps=${fps} >= 45`);
  ok((await countPix(YELLOW)) > 300, "circle visible (yellow px)");
  ok((await countPix(GREEN)) > 5000, "ground visible (green px)");
  ok((await countPix(TAN)) > 20, "sprites drawn (tan px)");
  await page.screenshot({ path: "/tmp/crown-test.png" });

  // tap moves the aim target; the spring circle converges onto it.
  // Click maps to logical ((1040-160)/3, (512-16)/3) = (293,165);
  // the scene clamps the circle to y<=160.
  await page.mouse.click(1040, 512);
  const centroid = async () =>
    page.evaluate(() => {
      const d = (document.getElementById("screen") as HTMLCanvasElement)
        .getContext("2d")!.getImageData(0, 0, 320, 256).data;
      let n = 0, sx = 0, sy = 0;
      for (let i = 0; i < d.length; i += 4)
        if (d[i] === 255 && d[i + 1] === 238 && d[i + 2] === 68) {
          n++; sx += (i / 4) % 320; sy += Math.floor(i / 4 / 320);
        }
      return n ? { x: sx / n, y: sy / n, n } : { x: -1, y: -1, n: 0 };
    });
  await page.waitForFunction(
    () => {
      const d = (document.getElementById("screen") as HTMLCanvasElement)
        .getContext("2d")!.getImageData(0, 0, 320, 256).data;
      let n = 0, sx = 0, sy = 0;
      for (let i = 0; i < d.length; i += 4)
        if (d[i] === 255 && d[i + 1] === 238 && d[i + 2] === 68) {
          n++; sx += (i / 4) % 320; sy += Math.floor(i / 4 / 320);
        }
      return n > 30 && Math.abs(sx / n - 293) < 12 && Math.abs(sy / n - 160) < 12;
    },
    null,
    { timeout: 5000 },
  ).then(
    () => ok(true, "circle springs to tap point"),
    async () => {
      ok(false, `circle springs to tap point (centroid ${JSON.stringify(await centroid())})`);
    },
  );
  await page.screenshot({ path: "/tmp/after-tap.png" });

  // wheel shrinks the circle
  const yb = await countPix(YELLOW);
  await page.mouse.wheel(0, -200);
  await page.waitForTimeout(200);
  const ya = await countPix(YELLOW);
  ok(ya < yb, `wheel shrank circle (${yb} -> ${ya})`);

  // drag pans the ground, then space restarts via fade transition
  const down = await logical(150, 200);
  const up = await logical(90, 240);
  await page.mouse.move(down.x, down.y);
  await page.mouse.down();
  await page.mouse.move(up.x, up.y, { steps: 5 });
  await page.mouse.up();
  await page.screenshot({ path: "/tmp/crown-drag.png" });
  await page.keyboard.press(" ");
  await page.waitForTimeout(1200);
  ok((await countPix(GREEN)) > 5000, "scene survives fade cycle");
  ok((await page.evaluate("window.__fps")) >= 45, "fps holds after transition");

  // ---- title scene -> tap dissolves into test scene
  await page.goto(BASE + "/");
  await page.waitForFunction("window.__fps > 0", null, { timeout: 10000 });
  ok((await countPix(BLOOD)) > 1000, "title gradient band (blood px)");
  await page.screenshot({ path: "/tmp/crown-title.png" });
  const c = await logical(160, 128);
  await page.mouse.click(c.x, c.y);
  await page.waitForTimeout(1200); // fade + settle
  ok((await countPix(GREEN)) > 5000, "title -> test transition");
  await page.screenshot({ path: "/tmp/crown-after-click.png" });

  ok(errors.length === 0, `no console errors (${errors.slice(0, 3).join(" | ")})`);
} finally {
  await browser?.close();
  server.kill();
}

console.log(fails.length === 0 ? "ALL PASS" : `${fails.length} FAILURES`);
process.exit(fails.length === 0 ? 0 : 1);
