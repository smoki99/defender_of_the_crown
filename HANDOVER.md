# Handover — THE CROWN ENDURES (defender_of_the_crown)

Web strategy game in Rust/WASM, Amiga-style homage to *Defender of the
Crown* (1989). Playable: https://smoki99.github.io/defender_of_the_crown/
(pushes to `main` redeploy via CI). Design source of truth:
`docs/SPEC.md`; decision/feature history: `tasks/progress.md`,
`CHANGELOG.md`.

## Play

- Open the Pages URL (or `bun tools/serve.ts` → http://127.0.0.1:8123).
- Mouse or keyboard: Tab/arrows move focus, Enter activates, E ends the
  round, Esc closes plates. First click unlocks audio (browser policy);
  crawl auto-rolls, war starts 3 inputs from load.
- Campaign: click your keep to recruit (BUY in panel), click an adjacent
  keep to march; battle report plate summarizes; 12 Saxon castles to
  liberate before the coronation timer. Sports minigames earn gold +
  HALL OF FAME records (localStorage, resettable from HALL OF FAME).

## Build / test / ship

| What | Command |
| --- | --- |
| Native suite (48 targets) | `cargo test` |
| Lint gate | `cargo clippy --all-targets -- -D warnings` |
| Web bundle + 2 MB budget | `./build.sh` (wraps `tools/build-web.ts`) |
| Pixel/console baseline gate | `bun tools/baseline-check.mjs` (`UPDATE=1` recaptures) |
| Sports autoplay batch | `bun tools/baseline-sports.sh` via its runner |
| Perf p95 probe | `bun tools/perf-pass.mjs` (bar: 12 ms) |
| Serve locally | `bun tools/serve.ts` |

CI (`.github/workflows/ci.yml`): release tests + clippy + wasm build +
budget + jsdom headless gate on every push/PR; on `main` additionally
uploads `web/` artifact and deploys GitHub Pages (workflow build type).

## Architecture (src/)

- `engine/` — gfx (32-color indexed framebuffer, Bayer dithers,
  `shake_viewport`), fonts (8×8 + caps3 titles), input, scene stack
  (Fade/Dissolve transitions), audio (ProTracker `.mod` player + synth
  SFX), art (hand-drafted keep sprites).
- `campaign/` — deterministic rules: 12-castle graph, fog/spies,
  kingdom bars, events (incl. dragon), combat, AI (3 difficulties),
  coronation/defeat checks. `game.over` freezes play; `draw_over` banners.
- `ui/` — campaign scene: map render, panel (recruit/spy/duel), battle
  report plate (word-wrapped, siege shake behind it), HUD bars,
  event/champion modals, keyboard focus.
- `minigames/` — framework + 13 sports (joust…dragon); seasonal meets,
  reward tiers, autoplay hooks for QA (`?scene=mg&mg=<id>`).
- `web.rs` — scenes (boot/title/crawl/menu/fame/options/difficulty),
  fixed-step loop, localStorage (records, mute, CRT), CRT filter.

## QA artifacts

- `shots/` — committed RGBA baselines; `shots/png/` human viewable.
- `tests/` — one file per feature group (see `tasks/progress.md` B1 for
  the SPEC §13 acceptance-to-test mapping).
- `tools/` — harnesses listed above + font/sound/baseline utilities.

## Known gaps / future

- No `cargo doc` pass yet; module docs exist but rustdoc is not gated.
- Mobile/portrait out of scope (SPEC §14.5 decision); English-only.
- Art is procedural pixel art, provenance tooling in `tools/`
  (`genfont.ts`, `mkkeep.ts`, `compose.ts`).
- Next candidates: scenario/victory epilogue crawl, difficulty-specific
  AI personality, sport season rotation UI.
