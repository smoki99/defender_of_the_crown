# The Crown Endures — a Saxon liberation

A web strategy game in the spirit of the Amiga classic *Defender of the
Crown*: reclaim England castle by castle — levy armies, ride tournaments,
win the duels — rendered by a Rust/WASM engine under a self-imposed Amiga
OCS budget: 320×256, one 32-colour palette, one frame per vsync.

The game is fully original: own map, art, music (tracker MODs compiled into
the binary) and code — no assets from the original game (see
`docs/SPEC.md` §1.1).

**Play:** the latest `main` builds deploy to GitHub Pages (CI job `pages`).
Locally: `bun run build && bun run serve` → http://127.0.0.1:8123

## How to play

Any key or click wakes the herald; the title menu is fully keyboard-driven
(arrows + Enter). One click enters the campaign: click a Saxon castle,
press the action buttons (LEVY / BOW / KNIGHT / SIEGE / END TURN),
manage the four kingdom bars (RICHES, PROVISIONS, LOYALTY, POPULARITY —
see SPEC §5.2), and march to reclaim every castle from the Normans —
tapping an adjacent Norman keep leads your host out. The campaign plays
mouseless too: Tab cycles buttons, arrows move the caret, Enter
activates, E ends the round, Esc closes a plate.
Captured castles host sports meets — the 13 minigames pay gold and fame,
and your best scores live in the hall of fame (browser storage).
Per-minigame controls (SPEC §6):

| sport | controls |
|---|---|
| TOURNAMENT (joust) | ←/→ lane + SPACE release |
| SIEGE (catapult) | mouse drag (angle + power) |
| DUEL (sword) | A/S/D guard + SPACE riposte |
| GALLERY (archery) / DEFEND (rampart) | mouse aim + click/SPACE |
| RESCUE (maze) / STEALTH (cloak) / HOARD (dragon) | arrows or WASD |
| CANOE (river) | ↑/↓ + SPACE |
| RHYTHM (forge) | SPACE on the beat |
| MEMORY (herbs) | mouse |
| FALCONER (falcon) | mouse steer |
| BRAWL (tavern) | keys (punch / grab / barrel) |

Escape dismisses panels; the chronicle (report screen) tracks the war.

## URL knobs (deep links for testing)

`?scene=play` jump straight into the campaign · `&seed=N&turn=N`
deterministic world · `&mg=<sport>` open one meet · `&mute=1` silence ·
`&perf=1` frame-time probe overlay (see `docs/TOOLS.md`).

## Building

Toolchain: stable Rust + `wasm32-unknown-unknown`, Bun ≥ 1.2.
The wasm-bindgen CLI and binaryen `wasm-opt` self-download into
`tools/bin/` on first build.

```sh
bun install
CROWN_LOCAL=1 bun run build   # host cargo; also enforces the 2 MB payload budget
bun run serve                 # http://127.0.0.1:8123
```

On the development host, compilation is dockerised instead (remote daemon,
see `test.sh`): `./test.sh all` runs tests + clippy + wasm check, and plain
`bun run build` (without `CROWN_LOCAL`) builds the wasm bundle inside the
`crown-rs:1` image.

## Tests

- `cargo test --release` — engine, campaign rules (property/seeded tests),
  combat determinism, minigame logic, 500-game soak.
- `bun tools/*-check.mjs` — headless browser gates in jsdom: full title→
  campaign flow, real playthroughs of joust/sword/archery (pixel matchers
  read the framebuffer), reward loop into localStorage, console-error gate,
  RGBA baseline diffing (`UPDATE=1` recaptures). See `docs/TOOLS.md`.
- CI (`.github/workflows/ci.yml`): native tests, clippy `-D warnings`,
  wasm build + payload budget, headless play gate; deploys Pages from `main`.

## Layout & docs

```
src/engine/     framebuffer, palette (gfx.rs), font, audio (MOD player), input
src/campaign/   map graph, kingdom rules, AI, events, save/hall-of-fame
src/minigames/  the 13 sports, shared framework (rewards, results plate)
src/web.rs      scene router, deep links, storage glue
docs/           SPEC.md (design bible), CAPABILITY-MAP.md, TOOLS.md
tasks/          plan.md / todo.md / progress.md (execution ledger)
tools/          build-web.ts, serve.ts, jsdom harness + pixel QA suite
shots/          committed RGBA baselines (+ PNG copies) for the diff gate
```

License: MIT — see `LICENSE`. Music/art are original, generated in-repo
(`tools/compose.ts`, `tools/mkkeep.ts`; provenance notes in `assets/`).
