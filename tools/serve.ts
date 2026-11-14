// tools/serve.ts — tiny static file server for web/ (dev testing).
const port = Number(process.argv[2] ?? Bun.env.PORT ?? 8080);
const root = new URL("../web/", import.meta.url).pathname;
const MIME: Record<string, string> = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".wasm": "application/wasm",
  ".css": "text/css",
  ".png": "image/png",
  ".mod": "application/octet-stream",
};
Bun.serve({
  port,
  async fetch(req) {
    const url = new URL(req.url);
    let p = decodeURIComponent(url.pathname);
    if (p === "/") p = "/index.html";
    const file = Bun.file(root + p.replace(/^\/+/, ""));
    if (!(await file.exists())) return new Response("404", { status: 404 });
    const ext = p.slice(p.lastIndexOf("."));
    return new Response(file, {
      headers: { "Content-Type": MIME[ext] ?? "application/octet-stream" },
    });
  },
});
console.log(`serving ${root} on http://127.0.0.1:${port}`);
