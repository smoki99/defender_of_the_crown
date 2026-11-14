// soundcheck.ts — T003 proof + reusable WebAudio smoke check (T108/T402 reuse).
// Launches the installed chrome-headless-shell, runs an oscillator->analyser
// graph in the page, and fails if the pipeline produces no real samples.
// Run: LD_LIBRARY_PATH=$HOME/pw-libs bun tools/soundcheck.ts [url]
import { chromium } from "playwright";
import { existsSync } from "node:fs";

process.env.LD_LIBRARY_PATH = `${process.env.HOME}/pw-libs/usr/lib/x86_64-linux-gnu:${process.env.LD_LIBRARY_PATH ?? ""}`;

const EXE = `${process.env.HOME}/.cache/ms-playwright/chromium_headless_shell-1255/chrome-headless-shell-linux64/chrome-headless-shell`;

const browser = await chromium.launch({
  executablePath: existsSync(EXE) ? EXE : undefined,
  args: ["--no-sandbox", "--disable-gpu", "--autoplay-policy=no-user-gesture-required"],
});
const page = await browser.newPage();
const errs: string[] = [];
page.on("console", (m) => m.type() === "error" && errs.push(m.text()));
page.on("pageerror", (e) => errs.push(String(e)));
await page.goto(process.argv[2] ?? "about:blank");

const probe = await page.evaluate(async () => {
  const ctx = new AudioContext();
  await ctx.resume();
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  const an = ctx.createAnalyser();
  osc.frequency.value = 440;
  gain.gain.value = 0.3;
  osc.connect(gain);
  gain.connect(an);
  an.connect(ctx.destination);
  osc.start();
  const { promise: nap, resolve: wake } = Promise.withResolvers<void>();
  setTimeout(wake, 600);
  await nap;
  const buf = new Float32Array(an.fftSize);
  an.getFloatTimeDomainData(buf);
  let acc = 0;
  let peak = 0;
  for (const v of buf) {
    acc += v * v;
    peak = Math.max(peak, Math.abs(v));
  }
  osc.stop();
  await ctx.close();
  return { state: ctx.state, rms: Math.sqrt(acc / buf.length), peak };
});

console.log(JSON.stringify({ ...probe, consoleErrors: errs }));
await browser.close();
if (!(probe.rms > 0.05) || errs.length > 0) {
  console.error("FAIL: inaudible pipeline or console errors");
  process.exit(1);
}
console.log("OK: WebAudio pipeline audible");
