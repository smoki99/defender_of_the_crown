// tools/build-web.ts — build the wasm bundle + JS glue into web/pkg.
// Rust compiles inside the crown-rs:1 docker image (see test.sh); the
// wasm-bindgen CLI is a static musl binary staged into the source volume.
import { existsSync } from "node:fs";

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

async function run(...args: string[]) {
  const p = Bun.spawn(args, { stdout: "inherit", stderr: "inherit" });
  if ((await p.exited) !== 0) throw new Error(`failed: ${args.join(" ")}`);
}

await run("./test.sh", "wasm-build");
await run("./test.sh", "bindgen");
await Bun.write("web/pkg/.keep", "");
await run("./test.sh", "pull-pkg");
const files = Array.from(
  new Bun.Glob("web/pkg/*").scanSync(),
).map((f) => f.split("/").pop());
console.log("web/pkg:", files.join(" "));
