# Implementation Plan: The Crown Endures (Defender of the Crown 2.0)

Contract: `docs/SPEC.md` (approved) + `docs/CAPABILITY-MAP.md`. Task execution
list: `tasks/todo.md` (checkboxes, same task ids). Progress log:
`tasks/progress.md` (append per task: what ran, what verified).

## Overview

Rust→WASM retro strategy game: campaign + 13 minigames + tracker music. Built
as three gated stages so I can run unattended for hours and hand over only a
testable build:

| Stage | Tag | Exit definition |
|---|---|---|
| **ALPHA 1** | `alpha1` | Playable vertical slice: title+overture → difficulty → campaign → minigames **joust/catapult/sword** with rewards → AI turns → victory/defeat. All automated checks green. Pushed to `main`. |
| **ALPHA 2** | `alpha2` | **All 13 minigames** pass MG-QA (§3), soak/balance/perf passes, HALL OF FAME + unlock pacing live. Pushed + tagged. |
| **BETA 1** | `beta1` | SPEC §13 acceptance all green, art/audio pass, README, CI + GitHub Pages URL. **Stop → human testing.** |

## Architecture decisions (recap of approved spec)

- Own software renderer: 320×256 indexed framebuffer → ImageData; 32-color
  palette; Bayer 4×4 dither; 8×8 bitmap font; no game engine crates.
- `xmrs` (MOD playback) + wasm-bindgen/web-sys/js-sys only; hand-rolled
  WebAudio SFX; `tools/compose.ts` synthesizes samples + emits real `.mod`.
- Pure deterministic cores (`campaign`, minigame logic, engine math) testable
  with plain `cargo test` in docker `rust:1.90-slim-bookworm` (host lacks cc);
  thin wasm adapters; `Xor` seeded RNG everywhere, no wall-clock in sim.
- QA = Playwright (installed): screenshots + pixel-diff vs baselines, scripted
  input playtests via `?autoplay=<id>`, console-error gate, audio-RMS gate,
  frame-time probe. Chromium runtime libs via `tools/fetch-chrome-libs.ts`.
- Minigames implement `Minigame` trait + registry; each game is one module,
  independently specced/tested/screenshot-ed (MG-QA below).

## §2 Dependency graph & build order

```
T0xx toolchain ──► (everything below needs tests + browser proof)
T1xx engine(gfx→input→scene→audio) ─┬► T3xx title+music ─► (alpha1 assembly)
T2xx campaign(core→combat/ai→view) ─┤
T4xx mg-framework + mg1..3 ──────────┴► ALPHA1 → T5xx mg4..9 → T6xx mg10..13
                                       → T7xx soak/balance/perf → ALPHA2
                                       → T8xx polish/art/audio/docs → BETA1
```

Parallelization: minigames 4–13 are independent once `mg-framework` exists —
each gets its own subagent implementing + unit-testing against the shared
trait, while I integrate serially (one integrator avoids shared-file
conflicts). Everything else sequential.

## §3 MG-QA protocol (runs for EVERY minigame — "check each one itself")

| # | Check | How |
|---|---|---|
| Q1 | module spec | `docs/specs/mg-<id>.md`: rules, scoring formula, tier thresholds, art list, SFX list, fail/win conditions |
| Q2 | logic tests | pure-logic `cargo test` ≥6 cases: timers, spawn patterns, scoring math, win/lose, boundary |
| Q3 | renders | `?shot=mg-<id>` screenshot: idle + mid-play; reviewed against presentation bible (palette count, outlines, readability) — iterate until pass (max 5 loops, then note + move on) |
| Q4 | playable | `?autoplay=<id>` scripted inputs reach WON; casual script reaches LOST/BRONZE; zero console errors both |
| Q5 | wired | campaign integration test: SPORTS button → game → result → reward applied (gold/troops/etc.), persistence of high score |
| Q6 | balance | thresholds monotone test: perfect-script ≥ GOLD, casual ∈ [BRONZE, SILVER), GOLD < perfect − slack (skill still matters) |

A minigame task is Done only when Q1–Q6 green; recorded in `tasks/progress.md`.

## §4 Performance & size budgets (checked at ALPHA2, rechecked BETA1)

- Frame: logic+draw ≤ 12 ms p95 @320×256 (probe: `?perf=1` rolling stats).
- Payload: wasm+mods ≤ 2 MB (6 tracks embedded). `wasm-opt -Oz` (binaryen,
  from github releases — reachable) in build.sh.
- Startup: interactive ≤ 2 s on local serve.
- Soak: AI-vs-AI 500 games, 0 panics, 100 % termination ≤ 90 turns, invariant
  checks (bars in range, no negative armies, graph rules).
- Balance: HARD AI beats EASY-policy player ≥ 8/10 sim games; minigame reward
  economy cannot trivially break income (cap test).

## §5 Autonomous run rules (user instruction: run until beta1)

- Loop per task: implement → run its Verification block → append result line
  to `tasks/progress.md` → flip checkbox in `tasks/todo.md` → git commit
  (small, conventional messages). Never leave a failing check behind.
- A checkpoint task failing = fix before any later phase task. If a check is
  blocked by environment (not code), write `BLOCKED:` line with evidence and
  pick a fallback, never silently skip the gate's purpose.
- No questions to the human between alpha1 and beta1 (pre-authorized). Stop
  only at BETA1 handover, or on: a SPEC contradiction, a new dependency
  outside the approved list, or a destructive need.
- Every screenshot baseline regenerated only by me with a reason logged;
  baselines commit with the change.

## Risks & mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| `xmrs` won't build/run on wasm32 | High (music) | Fallback: hand-rolled 4-ch MOD player (~500 lines, format public) — T012 proves the choice early |
| Chromium can't run (missing libs) | High (all browser QA) | T001 `fetch-chrome-libs.ts` (deb mirror verified); fallback: docker-side chrome + CDP attach |
| Autoplay policy blocks audio proof | Med | launch `--autoplay-policy=no-user-gesture-required` (verified flag) + AnalyserNode RMS gate in T012 |
| Hand-crafted pixel art quality | Med | MG-QA Q3 loop + presentation-bible checklist + a fresh-context judge subagent pass per screen at BETA1 |
| Scope drift / 13 games fatigue | Med | strict per-game protocol + soak/alpha gates; games are 45–90 s designs, not simulators |
| Docker test cycle slow | Low | test --release caching, run focused test names during dev, full suite at checkpoints |

## Definition of Done (every task)

Verification block executed & pasted to `progress.md`; clippy-clean code (no
`.unwrap()` outside init, no `unsafe`); no web-sys imports in pure cores; docs
touched if behavior changed; commit made.
