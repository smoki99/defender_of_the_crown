# Changelog

All notable changes to **THE CROWN ENDURES — a Saxon liberation**.
Format roughly follows Keep a Changelog; dates are the commit dates.

## [beta1] — 2026-11-15

First feature-complete beta: full campaign, 13 minigames, music, Amiga
art pass, CI + Pages, playable at the GitHub Pages URL.

### Added
- Campaign core: 12-castle Saxon liberation, fog of war + spies, kingdom
  bars (RICH/FOOD/LOYAL/MOOD glide), events, AI with HARD-beats-EASY
  balance gate (T802 sweep), dawn-march animation, coronation/defeat end
  states (`draw_over`), animated battle-report plates.
- 13 minigames ("SPORTS"): joust, archery, catapult, sword, rampart,
  cloaks, maze, river-canoes, tavern brawl, falcon, forge, herbs-memory,
  hoard-dragon — each with autoplay QA, reward tiers, seasonal meets.
- HALL OF FAME: per-sport record grid, two-step reset, localStorage
  persistence; feat ticker on the title screen.
- Audio: 6 MOD tracks + one-shot Norman turn sting through a ProTracker
  module player; RMS-audible mix gates; mute persistence.
- Feel (T903): fade + Bayer dissolve scene transitions, siege shake
  (raster-offset viewport jolt under fresh battle reports, plate and HUD
  stay still), victory/defeat scenes, turn sting on the AI banner.
- Accessibility (T904): full keyboard-only campaign (Tab/Enter/arrows/E,
  regression test), GRAY3 contrast pass for all dim texts, focus caret
  via hover bevel.
- Tooling/CI: `build.sh` budget entry (≤ 2 MB), jsdom pixel-baseline
  gate (12 states, zero console errors), Playwright-style perf probe
  (all scenes p95 ≤ 6 ms vs 12 ms bar), GitHub Actions test+clippy+budget
  + artifact + Pages deploy on main.

### Fixed (beta polish rounds)
- Battle report now wraps at word boundaries; plate widened (27 cols /
  13 rows) so a full march report never loses its outcome line.
- HUD gauges clear of label ink (WEALTH bar renamed RICH); ticker capped
  at 39 chars; crawl fire-glow dither and flame flicker fixed; dim-text
  contrast pass; menu/options border idiom unified; headings centered.
- Music inaudible in browser (AudioContext resume + mix levels).
- Attack discoverability: adjacent enemy keeps tappable to march, with
  dawn-march replay of every Norman move.

## [alpha2] — 2026-11-13
- 500-campaign AI-vs-AI soak (T801), balance sweep contract (T802),
  perf probe + pass (T803), wasm-opt payload 459 KB (T804), all-13
  autoplay batch on the shipped binary (T805).

## [alpha1] — 2026-11-12
- Engine (gfx/input/scene/audio), title flow with herald boot + crawl,
  campaign vertical slice, first minigame batch, committed screenshot
  baselines + diff gate, 32-color Saxon palette + hand-drafted keeps.
