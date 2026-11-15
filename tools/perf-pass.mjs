// T803: real-browser performance pass. Boots every scene and all 13
// SPORTS in headless Chromium with ?perf=1; the wasm host publishes a
// rolling {mean,p95,max,frames} of rAF frame times on window.__perf
// once a second. Bar (plan §3/T803): p95 <= 12 ms at 320x256.
//   bun tools/perf-pass.mjs          # starts its own static host
// playwright-core is resolved at runtime (PW_CORE may name any path), so a
// static import is impossible. CHROME_EXE names the browser binary; the
// default is the LD_LIBRARY_PATH-wrapped puppeteer chrome (the bare
// headless-shell lacks the Debian libs).
let chromium;
try {
  ({ chromium } = await import("playwright-core"));
} catch {
  ({ chromium } = await import(process.env.PW_CORE ?? "/home/omp/pw-tools/node_modules/playwright-core/index.mjs"));
}

const EXE = process.env.CHROME_EXE ?? Bun.spawnSync(
  ["sh", "-c", "ls /home/omp/.omp/puppeteer/chrome/*/chrome-linux64/chrome | sort | tail -1"],
).stdout.toString().trim();

const PORT = 8137;
const BASE = `http://127.0.0.1:${PORT}/index.html`;
const BAR_MS = 12;

const SPORTS = ["joust", "catapult", "sword", "archery", "rampart", "maze",
  "river", "cloak", "forge", "herbs", "falcon", "tavern", "dragon"];

const SCENES = [
  ["title", ""],
  ["demo", "scene=demo"],
  ["play t1", "scene=play&autoplay=1"],
  ["play t40", "scene=play&autoplay=1&turn=40"],
  ["play champion", "scene=play&autoplay=1&champ=1"],
  ...SPORTS.map((s) => [`mg ${s}`, `scene=mg&mg=${s}&autoplay=1&perf=1`]),
];

const server = Bun.spawn(["bun", `${import.meta.dir}/serve.ts`, String(PORT)], {
  stdout: "ignore", stderr: "ignore",
});
await Bun.sleep(400);

const browser = await chromium.launch({ headless: true, executablePath: EXE, args: ["--no-sandbox"] });
const page = await browser.newPage({ viewport: { width: 960, height: 768 } });

const rows = [];
let fail = 0;
for (const [name, qs] of SCENES) {
  const url = `${BASE}?perf=1${qs ? "&" + qs : ""}`;
  await page.goto(url);
  // two publish windows: the rolling buffer (300 frames) fills and the
  // one-second report has fired at least twice
  await page.waitForTimeout(2600);
  const p = await page.evaluate(() => window.__perf ?? null);
  if (!p || !p.frames) {
    console.log(`${name.padEnd(16)}  NO DATA (probe silent)`);
    fail++;
    continue;
  }
  const bad = p.p95 > BAR_MS;
  if (bad) fail++;
  rows.push({ name, ...p, bad });
  console.log(
    `${name.padEnd(16)}  mean ${String(p.mean).padStart(6)} ms  p95 ${String(p.p95).padStart(6)} ms  max ${String(p.max).padStart(6)} ms  (${p.frames | 0} frames)${bad ? "  <== OVER BAR" : ""}`,
  );
}

await browser.close();
server.kill();
console.log(fail === 0 ? `PERF PASS: all scenes p95 <= ${BAR_MS} ms` : `PERF FAIL: ${fail} scene(s) over bar`);
process.exit(fail === 0 ? 0 : 1);
