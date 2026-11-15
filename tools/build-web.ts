// tools/build-web.ts — build the wasm bundle + JS glue into web/pkg.
// Default: Rust compiles inside the crown-rs:1 docker image (see test.sh).
// CROWN_LOCAL=1 (CI): host cargo + host wasm-bindgen, no docker.
import { existsSync, renameSync } from "node:fs";
import { gzipSync } from "node:zlib";

const CRATE_VER = "0.2.130"; // keep in sync with Cargo.toml pin
const CLI = "tools/bin/wasm-bindgen";

if (!existsSync(CLI)) {
  const url = `https://github.com/rustwasm/wasm-bindgen/releases/download/${CRATE_VER}/wasm-bindgen-${CRATE_VER}-x86_64-unknown-linux-musl.tar.gz`;
  console.log("downloading wasm-bindgen CLI", CRATE_VER);
  const r = await fetch(url);
  if (!r.ok) throw new Error(`cli download ${r.status}`);
  await Bun.write("/tmp/wbg.tar.gz", r);
  await Bun.$`mkdir -p tools/bin`;
  // archive root: wasm-bindgen-cli-<ver>-x86_64-.../wasm-bindgen
  await Bun.$`tar xzf /tmp/wbg.tar.gz -C tools/bin --strip-components=1 wasm-bindgen-${CRATE_VER}-x86_64-unknown-linux-musl/wasm-bindgen`;
  await Bun.$`chmod +x ${CLI}`;
}

// binaryen wasm-opt -Oz, vendored beside wasm-bindgen (both gitignored).
const OPT = "tools/bin/wasm-opt";
if (!existsSync(OPT)) {
  const BV = "version_122"; // matches the crown-rs:1 image
  console.log("downloading binaryen", BV);
  const r = await fetch(`https://github.com/WebAssembly/binaryen/releases/download/${BV}/binaryen-${BV}-x86_64-linux.tar.gz`);
  if (!r.ok) throw new Error(`binaryen download ${r.status}`);
  await Bun.write("/tmp/binaryen.tar.gz", r);
  await Bun.$`mkdir -p tools/bin`;
  await Bun.$`tar xzf /tmp/binaryen.tar.gz -C tools/bin --strip-components=2 binaryen-${BV}/bin/wasm-opt`;
  await Bun.$`chmod +x ${OPT}`;
}

async function run(...args: string[]) {
  const p = Bun.spawn(args, { stdout: "inherit", stderr: "inherit" });
  if ((await p.exited) !== 0) throw new Error(`failed: ${args.join(" ")}`);
}

const LOCAL = process.env.CROWN_LOCAL === "1";
await Bun.write("web/pkg/.keep", "");
if (LOCAL) {
  await run("cargo", "build", "--release", "--target", "wasm32-unknown-unknown");
  await run(CLI, "--target", "web", "--out-dir", "web/pkg",
    "target/wasm32-unknown-unknown/release/defender_of_the_crown.wasm");
} else {
  await run("./test.sh", "wasm-build");
  await run("./test.sh", "bindgen");
  await run("./test.sh", "pull-pkg");
}

// T804: wasm-opt -Oz (-40 % here); keep the CLI in sync with test.sh's image.
const WASM = "web/pkg/defender_of_the_crown_bg.wasm";
await run(OPT, "-Oz", "--enable-mutable-globals", WASM, "-o", "web/pkg/.opt.wasm");
renameSync("web/pkg/.opt.wasm", WASM);

// T804 payload budget: wasm + glue + embedded MODs must stay <= 2 MB.
const EMBEDDED = ["title", "saxon", "battle", "minigame", "victory", "defeat"];
let mods = 0;
for (const t of EMBEDDED) {
  const s = Bun.file(`assets/music/${t}.mod`).size;
  mods += s;
  console.log(`track ${t.padEnd(9)} ${String(s).padStart(7)} B`);
}
const wasmS = Bun.file(WASM).size;
const jsS = Bun.file("web/pkg/defender_of_the_crown.js").size;
const total = wasmS + jsS + mods;
const gz = gzipSync(await Bun.file(WASM).bytes()).length;
console.log(`payload wasm ${wasmS} + js ${jsS} + mods ${mods} = ${total} B (wasm gz ${gz})`);
if (total > 2 * 1024 * 1024) throw new Error(`payload ${total} B over 2 MB budget`);

const files = Array.from(
  new Bun.Glob("web/pkg/*").scanSync(),
).map((f) => f.split("/").pop());
console.log("web/pkg:", files.join(" "));
