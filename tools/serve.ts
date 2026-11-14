// Minimal static host for the wasm build (web/). `bun tools/serve.ts [port]`.
// application/wasm is required or browsers refuse to compile the module.
import { serve, file } from "bun";

const port = Number(process.argv[2] ?? 8123);
const MIME: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".wasm": "application/wasm",
  ".png": "image/png",
  ".css": "text/css",
  ".json": "application/json",
};
serve({
  port,
  async fetch(req) {
    const path = new URL(req.url).pathname;
    const rel = path === "/" ? "index.html" : path.slice(1);
    if (rel.includes("..")) return new Response("no", { status: 400 });
    const f = file(new URL(`../web/${rel}`, import.meta.url).pathname);
    if (!(await f.exists())) return new Response("not found", { status: 404 });
    const ext = rel.slice(rel.lastIndexOf("."));
    return new Response(await f.bytes(), { headers: { "content-type": MIME[ext] ?? "application/octet-stream", "cache-control": "no-cache" } });
  },
});
console.log(`serving web/ on http://127.0.0.1:${port}`);
