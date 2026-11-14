# Progress Log

Format: `T<id> — <what ran> — <result> (<date>)`. Append only.
- T001 — fetch-chrome-libs.ts (fixed un-awaited exists bugs) → 111 pkgs → ~/pw-libs; chrome-headless-shell 159.0.8154.6 launches; playwright screenshot /tmp/pw-test.png 41 KB, 0 console errors (2026-11-14)
- T002 — .omp/lsp.json → /home/omp/.local/bin/rust-analyzer --stdio, rootMarker Cargo.toml (2026-11-14)
- T005 — test.sh: docker cp staging (remote daemon: no bind mounts, exec 403-filtered) + named vols crown-src/crown-target; image crown-rs:1 = rust:1.90-slim + wasm32 target (docker build works). ./test.sh wasm → Finished dev profile (2026-11-14)
- T101 — lib crate reset: src/lib.rs + campaign/* (draft reparented, 3 compile fixes: Xor derives, march() move-before-len, _atk) + engine stub + web stub; cargo check --target wasm32 GREEN (2026-11-14)

- plan+todo created; execution started (2026-11-14)
- T003 — audio proof BOTH legs: (a) Rust decode: tests/mod_roundtrip.rs renders assets/music/tone.mod, pitch 259 Hz ±4 % from period 107, audible RMS; (b) browser: tools/soundcheck.ts headless-chromium WebAudio oscillator → RMS 0.212, peak 0.30, 0 console errors. DECISION: xmrs 0.15 dropped (source-verified: module *model*/import library, manual mixer plumbing — not turnkey render); SPEC §3 fallback taken = own dep-free `engine/audio/modplayer.rs` (M.K. 4-ch, C/B/D/F/9 effects, A-loops, vibrato/arp/porta/setvol). SPEC/CAPABILITY-MAP/plan updated (2026-11-14)
- T004 — compose.ts MOD cell encoding FIXED against xmrs patternslot.rs ground truth: `IIII PPPPPPPPPPPP IIIF DDDDDDDD`, NO volume column (volume only via C effect 0-64). Old test.mod was valid M.K. but garbage musically (fx nibble landed in instrument byte). Assets regenerated; title NUL-trim in parser; ./test.sh test 5/5 green (2026-11-14)
- T006 — docs/TOOLS.md written (docker pipeline, browser env incl. LD_LIBRARY_PATH deep path, compose.ts verified MOD facts, soundcheck.ts, LSP, gate). Image crown-rs:1 rebaked with clippy. ./test.sh all green: 5 tests + clippy -D warnings + wasm32 check (2026-11-14)
- T102/T103 — src/engine/gfx.rs: 32-color palette + named consts + nearest/half/mix (LazyLock tables; fixed `a as u8` wrap bug aliasing MIX rows mod 256 - regression test added), BAYER4 ordered dither, Framebuffer 320x256 (clipped rect/line/tri(edge fn)/circle/gradient/dither-fill), to_rgba. tests/gfx.rs 11 tests green; gate: 16 tests + clippy -D + wasm32 + doc green. docs/specs/engine-gfx.md written (2026-11-14)
- T107(web) — src/web.rs shell: wasm entry (console panic hook), canvas 320x256 backing + integer-scale pixelated fit, RAF driver (fixed 16.6ms logic step, dt clamp 250ms), pointer/wheel/key → Input via web-sys closures, per-frame ImageData put, DPR/resize, localStorage mute, ?scene=test/?shot/?mute query; tools/build-web.ts (docker wasm-build + wasm-bindgen 0.2.130 via npm cache + pkg extract), tools/serve.ts, tools/browser-check.ts (10-assertion headless proof). ENGINE FIX: SceneStack::start ignored while a transition runs — a leave-happy scene re-called replace() every tick, resetting Fade (title froze; found via __raw/__leave probes). Regression test replace_while_transition_running_does_not_restart_it. Gate: 48 tests + clippy -D + wasm32 + browser ALL PASS (61 fps) (2026-11-14)
- T105 — src/engine/sprite.rs: indexed 1-byte/px sprites (index 32 = TRANS), blit w/ hit-flash, nearest blit_scaled, blit_anim 2-frame helper; clipped via fb.set/fill_rect. tests/sprite.rs 5 green (transparency, 3-way edge clipping math, x3 scaling footprint, flash-only-opaque, frame parity). web.rs TestScene demos: 2 marching 8x8 soldiers (anim + periodic flash), x2 soldier, x2 banner on tower; browser-check gains "sprites drawn (tan px)" assertion -> 11/11 ALL PASS. Lint note: rust 1.90 clippy denies manual `% 2 == 0` (use is_multiple_of). (2026-11-14)
- T108 — engine-audio full stack live in browser: `synth.rs` SFX voices (blip/coin/hit/crowd/sword… square+noise, deterministic), `music.rs` Music manager (track bed + voice mix, clamp, voice reap), web.rs `AudioState` rewired to a PULL sink: inline AudioWorklet processor (`doc-sink`, ring + port-request protocol, data-URL module — CSP-safe) with ScriptProcessor fallback, main thread renders on demand; gesture unlock + `ctx.resume()`; SFX wired (tap→Blip, Space→Coin); mute now persisted (`localStorage doc.mute`); master headroom ×0.5 in `mix_frame` (was slamming the clamp, RMS 0.73 → 0.40). API landmines (js-sys 0.3.107 / wasm-bindgen 0.2.130): `Promise::then` now demands `&ScopedClosure`, `Reflect::apply` is `JsFunction`-generic, and web-sys has NO `create_audio_worklet_node*` binding → node built via `Reflect::construct` on `window.AudioWorkletNode`, promise hooked via 4-line shim `src/promise_hook.js`. tools/audio-check.ts = permanent 8-assertion headless proof (worklet-attach proxy + `window.__rms` sampling). Gate: 58 tests + clippy -D + wasm32 + audio-check 8/8 + browser-check 10/10, 0 console errors (2026-11-14)
- T1xx CHECKPOINT — `?scene=demo` DemoScene: 3-row multicolor font sheet on LINEN band, full 32-color palette strip (index==position), 8-step Bayer dither swatches GOLD->NAVY (t=i*2 over bayer 0..15), dual dithered gradient bands, marching anim soldier pair + periodic flash, static x3 soldier + x4 banner, rotating demo SFX (Coin/Sword/Hoof /45 frames). RAF publishes `window.__ms` (mean step+paint+put). browser-check.ts +7 demo assertions (fps, __ms<=12, band, palette slots 31/22 exact 64px, dither blend, scaled sprite) -> 17/17 ALL PASS, 1.6 ms/frame, 0 console errors; baseline engine-demo.png committed, visually approved (2026-11-14)

## Phase 2 - campaign core
- CONTAINER RECREATE HIT 2026-11-14: /usr lost gcc/binutils ("linker cc not found").
  Restored native linking without root: libc6-dev+libgcc-14-dev+libgcc-s1 debs ->
  ~/.local/opt/link-sysroot + bun shim ~/.local/bin/rust-cc-lld driving rust-lld
  (cc-arg translation, crt injection, --dynamic-linker, rewritten libc.so script).
  ~/.cargo/config.toml points cargo at it. Idempotent restore: ~/.omp/agent/restore-rust-linker.sh.
- T201 data: UnitKind::Champion (5th kind, special - not buyable, +icon/hp/atk),
  KindCounts [u32;5] everywhere (combat losses/counts), map coast/port flags
  (Wallingford/London/Dover coast, Dover port) + roads() edge list, events.rs
  EventKind table (7 types, weights, Danish gate >= turn 12, deterministic roll).
  tests/campaign_core.rs 8 green; full suite 66 green.
- T202 bars: Bars{provisions,loyalty,popularity} per faction (Neutral slot 2),
  income x(50+loy)/100, recruit price x(200-pop)/200, provisions upkeep 1/troop
  with cap 10/castle + market (2g/u) + hunger (biggest army -20%, levies first,
  loyalty -3), loyalty==0 -> lowest non-capital castle revolts to Neutral/turn,
  battle mood swings. tests/kingdom_bars.rs 10 green; suite 76 green.
- T203 fog: Intel{counts,walls,seen} per [castle][faction]; can_see = own or
  adjacent-to-own gives live truth, else last spy report, else None;
  spy(f,sid) SPY_COST=8 keeps stamped report; battle snapshot persists even
  after the staging ring is lost. tests/fog_spies.rs 7 green; suite 83.
- T204 marches: whole-garrison moves spend the day for BOTH endpoints (no
  two-leg days), captured castles count as marched, dawn resets; gates tested.
  Fog realigned to SPEC 5.3: View::{Own,Rough(±50% stable "SEEN: ~N"),Spied,
  Dark}; spy 15g/70% reveals castle+ring, caught = gold only (repeatable),
  10% leaks next-turn event into GameState.tip (event_rng, battle stream
  untouched). Browser deps moved under cfg(wasm32); shim now bakes -L paths
  so host build-scripts link when cross-compiling. clippy -D warnings clean.
