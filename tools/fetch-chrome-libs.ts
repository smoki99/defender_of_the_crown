// Downloads + extracts Debian runtime libraries so cached Chromium builds
// (playwright / puppeteer) can run headless in this root-less slim container.
// Extract-only: never touches system files. Usage: bun tools/fetch-chrome-libs.ts
import { mkdir, exists } from "node:fs/promises";

const PREFIX = "/home/omp/pw-libs";
const MIRROR = "http://deb.debian.org/debian";

const SEEDS = [
  // chromium runtime deps (Debian 13 names)
  "libglib2.0-0t64", "libnss3", "libnspr4", "libdbus-1-3", "libatk1.0-0t64",
  "libatk-bridge2.0-0t64", "libcups2t64", "libdrm2", "libxkbcommon0",
  "libxcomposite1", "libxdamage1", "libxfixes3", "libxrandr2", "libgbm1",
  "libasound2t64", "libpango-1.0-0", "libcairo2", "libatspi2.0-0t64",
  "libx11-6", "libxcb1", "libxext6", "libxi6", "libxtst6", "libxshmfence1",
  "libx11-xcb1", "libxrender1", "libxcb-render0", "libxcb-shm0",
  // fonts so text renders in screenshots
  "fontconfig", "fonts-dejavu-core", "fonts-liberation",
];
// System already provides these; extracting copies can shadow/incompat them.
const EXCLUDE = new Set(["libc6"]);

async function main() {
  await mkdir("/tmp/debs", { recursive: true });
  await mkdir(PREFIX, { recursive: true });

  if (!(await exists("/tmp/Packages"))) {
    // reuse the full trixie index from a prior run if present (56 MB, uncompressed)
    if (await exists("/tmp/Packages-trixie")) {
      console.log("reusing /tmp/Packages-trixie");
      const c = Bun.spawn(["cp", "/tmp/Packages-trixie", "/tmp/Packages"]);
      if ((await c.exited) !== 0) throw new Error("cp index failed");
    } else {
      console.log("fetching Packages index...");
      const r = await fetch(`${MIRROR}/dists/trixie/main/binary-amd64/Packages.gz`);
      if (!r.ok) throw new Error("index " + r.status);
      await Bun.write("/tmp/Packages.gz", await r.arrayBuffer());
      const g = Bun.spawn(["gunzip", "-f", "/tmp/Packages.gz"]);
      if ((await g.exited) !== 0) throw new Error("gunzip failed");
    }
  }

  // parse stanzas
  const txt = await Bun.file("/tmp/Packages").text();
  const pkgs = new Map<string, { deps: string[]; file: string }>();
  for (const st of txt.split("\n\n")) {
    let name = "", deps = "", file = "";
    for (const line of st.split("\n")) {
      if (line.startsWith("Package: ")) name = line.slice(9).trim();
      else if (line.startsWith("Depends: ")) deps = line.slice(9);
      else if (line.startsWith("Filename: ")) file = line.slice(10).trim();
    }
    if (name && file) {
      const list: string[] = [];
      for (const grp of deps.split(",")) {
        const first = grp.split("|")[0].trim().split(" ")[0];
        if (first) list.push(first.split(":")[0]); // strip arch qualifiers
      }
      pkgs.set(name, { deps: list, file });
    }
  }
  console.log("index parsed:", pkgs.size, "packages");

  // dependency closure
  const need = new Set<string>();
  const queue = [...SEEDS];
  while (queue.length) {
    const n = queue.pop()!;
    if (need.has(n) || EXCLUDE.has(n)) continue;
    const p = pkgs.get(n);
    if (!p) continue; // virtual package
    need.add(n);
    queue.push(...p.deps);
  }

  console.log("to fetch:", need.size);
  let done = 0, bytes = 0;
  for (const n of [...need].sort()) {
    const p = pkgs.get(n)!;
    const dest = `/tmp/debs/${n}.deb`;
    if (!(await exists(dest))) {
      const r = await fetch(`${MIRROR}/${p.file}`);
      if (!r.ok) { console.log("SKIP fetch", n, r.status); continue; }
      const buf = await r.arrayBuffer();
      await Bun.write(dest, buf);
      bytes += buf.byteLength;
    }
    const x = Bun.spawn(["dpkg-deb", "-x", dest, PREFIX]);
    if ((await x.exited) !== 0) { console.log("EXTRACT FAIL", n); continue; }
    done++;
    if (done % 10 === 0) console.log(`extracted ${done}/${need.size}`);
  }
  console.log(`done. ${done} packages, ${(bytes / 1e6).toFixed(1)} MB downloaded → ${PREFIX}`);
}
main().catch((e) => { console.error("FATAL", e); process.exit(1); });
