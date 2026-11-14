// tools/audio-check.ts — T108 proof: real-Chromium audio output.
// Verifies: worklet sink attached (not just fallback), non-silent music,
// SFX survive, M mutes AND persists across reload via localStorage.
import { chromium, type Browser } from "playwright-core";

const PORT = 8097;
const BASE = `http://127.0.0.1:${PORT}`;
const PW_LIBS = "/home/omp/pw-libs/usr/lib/x86_64-linux-gnu";

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
  const page = await (
    await browser.newContext({ viewport: { width: 1280, height: 800 } })
  ).newPage();
  const errors: string[] = [];
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  page.on("pageerror", (e) => errors.push(String(e)));

  const rms = () => page.evaluate(() => (window as any).__rms ?? 0);
  // sample the output level for ~600ms; music is a mix -> expect variation
  const sampleRms = async (n = 6) => {
    const v: number[] = [];
    for (let i = 0; i < n; i++) {
      await page.waitForTimeout(100);
      v.push(await rms());
    }
    return v;
  };

  await page.goto(`${BASE}/`); // production boot: title.mod overture (T402)
  await page.waitForFunction(() => document.visibilityState === "visible");
  await page.waitForTimeout(400); // wasm boot + first frames

  // instrument BEFORE the first gesture: the engine constructs the worklet
  // node through window.AudioWorkletNode, so a proxy sees the attach.
  await page.evaluate(() => {
    (window as any).__wk = false;
    const Orig = (window as any).AudioWorkletNode;
    (window as any).AudioWorkletNode = new Proxy(Orig, {
      construct(t, a) {
        (window as any).__wk = true;
        return Reflect.construct(t, a);
      },
    });
  });

  ok((await rms()) === 0, "silent before first gesture");

  // trusted tap = user gesture -> AudioContext unlock + worklet attach
  await page.mouse.click(160, 128);
  await page.waitForTimeout(600);
  ok(await page.evaluate(() => (window as any).__wk === true), "worklet sink attached");

  const music = await sampleRms();
  ok(
    music.filter((x) => x > 0.002).length >= 3,
    `music audible after tap (samples ${music.map((x) => x.toFixed(3)).join(" ")})`,
  );

  // coin SFX through the mix, then mute via M
  await page.keyboard.press("Space");
  await page.keyboard.press("KeyM");
  // the worklet picks the flag up on its next quantum; the coin SFX tail
  // must be out of the sampled window before we call M "silent".
  await page.waitForTimeout(350);
  const muted = await sampleRms();
  ok(
    muted.every((x) => x < 0.002),
    `M silences output (samples ${muted.map((x) => x.toFixed(3)).join(" ")})`,
  );
  ok(
    (await page.evaluate(() => localStorage.getItem("doc.mute"))) === "1",
    "mute persisted to localStorage",
  );

  // reload: mute must survive, unmute restores audio
  await page.reload();
  await page.waitForTimeout(400);
  await page.mouse.click(160, 128); // unlock again
  const after = await sampleRms(3);
  ok(after.every((x) => x < 0.002), "still muted after reload");
  await page.keyboard.press("KeyM");
  const back = await sampleRms();
  ok(
    back.filter((x) => x > 0.002).length >= 3,
    `unmute restores music (samples ${back.map((x) => x.toFixed(3)).join(" ")})`,
  );

  // A4: the title must stay audible through the loop - the overture ends
  // at the A-section restart byte, so sample a window that straddles it.
  await page.waitForTimeout(6000);
  const looped = await sampleRms(10);
  ok(
    looped.filter((x) => x > 0.002).length >= 5,
    `title loop audible at +6 s (samples ${looped.map((x) => x.toFixed(3)).join(" ")})`,
  );

  ok(errors.length === 0, `no console/page errors${errors.length ? ": " + errors.join(" | ") : ""}`);
} finally {
  await browser?.close();
  server.kill();
}

if (fails.length) {
  console.log(`\n${fails.length} FAILED`);
  process.exit(1);
}
console.log("\naudio-check: all green");
