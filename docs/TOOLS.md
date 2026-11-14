# Toolchain (T006) — everything verified working on this host, 2026-11-14

## Rust build/test: `./test.sh` (docker)

Host has no C linker → all cargo work runs against the remote docker daemon
(`DOCKER_HOST=tcp://tecnativa-proxy:2375`; `info` is 403-filtered, `ps`/`exec`
patterns below account for that).

- Image `crown-rs:1` = `rust:1.90-slim-bookworm` + `clippy` + `wasm32-unknown-unknown`
  (rebake: `printf 'FROM rust:1.90-slim\nRUN rustup component add clippy && rustup target add wasm32-unknown-unknown\n' | docker build -t crown-rs:1 -`).
- Remote daemon supports **no bind mounts and no `docker exec` upgrade** →
  `test.sh` stages sources into named volume `crown-src` via a detached
  staging container (`crown-stage`) + `docker cp`, target cache lives in
  named volume `crown-target`.

```sh
./test.sh check    # cargo check (native)
./test.sh test     # cargo test --release (all suites incl. tests/*.rs)
./test.sh clippy   # clippy --all-targets -- -D warnings
./test.sh wasm     # cargo check --target wasm32-unknown-unknown
./test.sh all      # test + clippy + wasm  ← the gate
```

## Browser QA: headless Chromium

- System libs (glib, libnss3, fontconfig …) unpacked by
  `bun tools/fetch-chrome-libs.ts` → `~/pw-libs` (deb.debian.org, no root).
- Binary: `~/.cache/ms-playwright/chromium_headless_shell-1255/…/chrome-headless-shell`
  (Chrome for Testing 159.0.8154.6); playwright 1.65.1 from bun's global cache.
- Required env at spawn time: `LD_LIBRARY_PATH=$HOME/pw-libs/usr/lib/x86_64-linux-gnu`
  (tools/soundcheck.ts sets this itself). Launch args:
  `--no-sandbox --disable-gpu --autoplay-policy=no-user-gesture-required`.

**Status 2026-11-14:** container recreate wiped `~/pw-libs` + the cached
playwright shell; until they are restored, browser QA runs through
`bun run check:headless` — `tools/headless-check.mjs` boots the real wasm
bundle in **jsdom** (patched perf/canvas/audio globals, synthetic pointer
events, framebuffer readback via `putImageData`, PNGs to `SHOTS_DIR`,
default `/tmp/crown-shots`). Scripted loop: title -> difficulty -> campaign
-> levy -> march -> 5x END TURN, HUD turn counter decoded from the 8x8 font
glyphs (`cargo run --example coords` emits tap coords + digit bitmaps).
Green = 17 assertions + 0 console errors. Caveats: jsdom rAF throttles
(liveness = decoded pixels, not frame counts); no pixel-diff baselines yet
— deferred until native Chromium returns.

## Music: `bun tools/compose.ts <out.mod> [tone]`

ProTracker writer: sample synthesis (single-cycle pulse/square/tri + kick/noise
envelopes), pattern DSL, M.K. encoder, and a reader for self-check. Assets in
`assets/music/` (`test.mod` demo, `tone.mod` pitch proof) are generated, never
hand-edited; regenerate after changing compose.ts.

Verified MOD facts (from xmrs source `patternslot.rs`): cell =
`IIII PPPPPPPPPPPP IIIF DDDDDDDD`; **no volume column** (volume via `C`
effect, 0-64); `M.K.` tag at offset 1080; pitch = `3 546 896 / period`;
timing = `bpm/5` ticks/s.

## Audio proof: `bun tools/soundcheck.ts [url]`

Headless Chromium + WebAudio oscillator→AnalyserNode; exits non-zero unless
RMS > 0.05 and console-error count is 0. Reused by in-browser game checks.

## Game audio proof: `bun tools/audio-check.ts`

T108 gate: boots `?scene=test`, proxies `window.AudioWorkletNode` before the
first gesture, then asserts — silent pre-gesture, worklet sink actually
attached (not the ScriptProcessor fallback), music RMS > 0.002 after the tap,
`M` silences + persists `doc.mute` to localStorage across reload, unmute
restores. `window.__rms` (set by the render callbacks) is the probe channel.
Needs a login shell for docker/chromium env: `sh -lc 'bun tools/audio-check.ts'`.

## IDE

`​.omp/lsp.json` → standalone `rust-analyzer` (`~/.local/bin/rust-analyzer`,
0.3.3113), root marker `Cargo.toml`.

## Lint/format policy

`cargo clippy -D warnings` and `./test.sh all` are the CI gate; the wasm
release payload budget is enforced at P8 (wasm-opt, SPEC §7).
