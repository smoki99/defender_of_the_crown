# Execution Task List — The Crown Endures

Run order = file order. Protocol references: `tasks/plan.md` §3 (MG-QA), §5
(run rules). Flip `[x]` only after Verification logged to `tasks/progress.md`.

## Phase 0 — Toolchain (T0xx)

- [x] **T001 Chromium runtime libs**: run `bun tools/fetch-chrome-libs.ts` →
  `~/pw-libs`; `LD_LIBRARY_PATH=~/pw-libs/... chrome-headless-shell --version`
  prints version; playwright screenshot of `example.com` lands `/tmp/pw-test.png`.
  *Verify: PNG exists, >10 KB, no console errors.*
- [x] **T002 LSP wiring**: `.omp/lsp.json` → rust-analyzer with absolute
  command + `--stdio`, rootMarker `Cargo.toml`; server starts. *Verify:
  hover/references work on a src symbol.*
- [x] **T003 audio pipeline proof**: host decode of one generated test `.mod`
  (own modplayer in cargo test via docker) + WebAudio sine-RMS>0 in headless
  Chromium (`--autoplay-policy=no-user-gesture-required`). *Verify both logs.*
- [x] **T004 compose.ts skeleton**: DSL synthesizes 4 samples (square, pulse,
  noise, kick) + 4-row pattern → valid `.MOD` bytes; decoder round-trip test
  (cargo test parses header/sample-count). *Verify: file + test green.*
- [x] **T005 test harness**: docker alias `./test.sh [filter]` (rust:1.90-slim-
  bookworm, cargo test --release, mounted). *Verify: trivial test passes.*
- [x] **T006 docs/TOOLS.md**: installed inventory + commands + debugging
  workflow (DevTools wasm, panic hook, playwright, docker, fallbacks).

## Phase 1 — Engine (T1xx)

- [x] **T101 Cargo project reset**: lib crate `defender`, feature-free deps
  (wasm-bindgen, js-sys, web-sys exact features, console_error_panic_hook);
  old drafted src/ modules moved under `src/campaign/` for absorption;
  `cargo check` on wasm32 target green (docker). *Verify: check + test green.*
- [x] **T102 Palette + framebuffer**: 32-color palette (Amiga-like, table in
  code w/ names), `Framebuffer` u8-indexed 320×256, blit-to-RGBA (ImageData)
  pure fn; unit tests (bounds, palette→RGB).
- [x] **T103 Drawing primitives**: px/hline/vline/rect/fill/triangle/circle +
  Bayer 4×4 dithered fill/gradient + half-bright mixing; tests incl. dither
  determinism.
- [x] **T104 Bitmap font**: original 8×8 glyph set (A–Z 0–9 punctuation,
  lowercase) drawn in code; `text()` + `text_center()`; renders in tests
  (pixel fingerprints); screenshot of glyph sheet.
- [ ] **T105 Sprite layer**: 8/16/32px sprites (indexed, mask), blit/scaled×N,
  palette-swap flash, 2-frame anim helper; tests.
- [x] **T106 Input**: event queue (pointer down/move/up w/ canvas→internal
  coords, keys, wheel), tap/swipe semantics, no-gesture audio-unlock flag;
  pure logic + tests.
- [x] **T107 Scene stack**: `Scene` trait (init/ event/draw/tick), push/pop/
  replace, fade + dither-dissolve transitions, RAF driver w/ fixed logic step;
  test scene runs in browser (`?scene=test`) screenshot + 60fps probe.
- [ ] **T108 Audio engine**: modplayer→f32 PCM → WebAudio (AudioWorklet,
  ScriptProcessor fallback), music manager (play/loop/stinger/mixdown),
  mute-persist localStorage; SFX synth module (blip/coin/sword/crowd/hoof/
  boom/etc. factory) — RMS gate test in Chromium. *Verify: module loops audibly.*
- [ ] **Checkpoint T1xx**: engine demo page (font sheet, dither gradient,
  sprite anim, music+sfx) screenshots clean, 0 console errors, perf ≤12 ms.

## Phase 2 — Campaign core (T2xx)

- [ ] **T201 Data**: castle graph (12, symmetric-connected test), roads,
  unit defs (5 incl. champion), event table data.
- [ ] **T202 Kingdom bars**: riches/provisions/loyalty/popularity structs,
  income/upkeep/shortfall-desert/revolt-at-0 rules; clamp invariants; tests.
- [ ] **T203 Fog + spies**: hidden garrisons w/ ±50 % estimate, spy action
  (70 % reveal / fail loses gold), scout-adjacency reveal; seeded-determinism
  tests.
- [ ] **T204 Marches + ownership**: one march/castle/turn, movement, siege
  handoff to combat; tests incl. road-only movement.
- [ ] **T205 Combat v2**: walls/catapults/repel 30 + champion +25 % & duel
  moment; same-seed-same-result test; defender-favor + breach tests.
- [ ] **T206 Events**: 7 event types incl. Danish raiders from t12 (Dover/
  London/Wallingford coast rule), tournament-banner generation; seeded tests.
- [ ] **T207 Win/lose**: conquest, coronation (London+8), capital-fall −30
  loyalty (not instant loss), revolt chain, turn-90 judgment; scenario tests.
- [ ] **T208 Champions + rewards API**: recruit via triple-gold flag, garrison
  bonus, reward application fns (gold/troops/provisions/bars) consumed by mg
  framework; tests.
- [ ] **T209 AI v2**: attack ratio / recruit budget / spy freq knobs ×3
  difficulties; AI beats idle ≤100 turns; AI-vs-AI terminates; HARD ≥
  EASY-policy 8/10 (fixed seed set); monotonic-difficulty test.
- [ ] **Checkpoint T2xx**: full docker suite green + determinism soak 100
  games.

## Phase 3 — Campaign view (T3xx)

- [ ] **T301 Map renderer**: coastline/forest/hills shading, castle sprites
  (2 frames banner), roads, ownership colors, selected pulse; screenshot.
- [ ] **T302 Panel UI kit**: pixel buttons/lists/scrollbar/tooltip, 3-tone
  shading, ALL-CAPS text; hit-testing; screenshot sheet.
- [ ] **T303 Turn bar + bars HUD**: 4 kingdom bars w/ animations, end-turn
  button, turn counter, event banner area; screenshot.
- [ ] **T304 Castle panel**: garrison list w/ fog estimate, recruit buttons
  (costs), spy button (cost/odds text), sports button; click-through test.
- [ ] **T305 March UI**: select→adjacent highlight→move preview→confirm;
  cancel; screenshot.
- [ ] **T306 Battle report**: animated round lines, wall breach fx, result
  modal, champion duel line; screenshot + skip-click.
- [ ] **T307 Event banners/modals**: tournament banner on castle, plague/
  caravan/raider modals w/ pixel icon; screenshot each.
- [ ] **T308 End-turn sequence**: player→AI turns with `norman.mod` sting,
  AI move animation pacing ≤2 s; event roll; screenshots.
- [ ] **Checkpoint T3xx**: scripted playwright turn loop (click castle→
  recruit→march→end turn→AI moves) with 0 console errors.

## Phase 4 — Title & music (T4xx)

- [ ] **T401 Compose full tracks**: `title.mod` (fanfare+overture+loop),
  `saxon.mod`, `norman.mod`, `battle.mod`, `minigame.mod`, `victory.mod`,
  `defeat.mod` via compose.ts; decode-check all in cargo test; sizes in
  budget.
- [ ] **T402 Boot herald + press-to-begin**: black→banner unfurl anim,
  gesture unlocks audio, fanfare starts ≤0.5 s; screenshot + RMS proof.
- [ ] **T403 Title card + menu**: animated crown/banners/flames, name lettering,
  4 menu entries w/ keyboard+mouse nav; screenshot; loop into A-section.
- [ ] **T404 Difficulty + intro crawl**: EASY/NORMAL/HARD pick → 3-screen
  pixel crawl → campaign; screenshot.
- [ ] **T405 How-to pages + options**: illustrated pixel how-to (3 pages),
  sound/CRT toggles persisted; screenshots.
- [ ] **Checkpoint T4xx**: entry sequence playable ≤3 inputs from load,
  music audible (RMS), screenshots approved.

## Phase 5 — MG framework + slice games (T5xx) → ALPHA 1

- [ ] **T501 mg-framework**: `Minigame` trait + registry, MgState, tier
  thresholds data, 60 s cheer-timer convention, results screen (tier + loot
  anim), SPORTS wiring (10 g), unlock schedule data, `?autoplay=`/`?shot=`
  harness hooks; framework tests.
- [ ] **T502 mg-joust** — MG-QA Q1–Q6. 2 passes, lane aim + release timing,
  lance snap, crowd cheer; perfect script wins gold tier.
- [ ] **T503 mg-catapult** — MG-QA. 6 stones, angle/power vs wind, towers +
  ships; distance-scored.
- [ ] **T504 mg-sword** — MG-QA. 3 telegraphs high/mid/low, parry-riposte,
  best-of-5; pattern-perfect script wins.
- [ ] **T505 Reward-loop integration test**: campaign→joust→gold applied→
  hall-of-fame entry written (playwright scripted).
- [ ] **Checkpoint ALPHA 1** — see gate below.

### GATE ALPHA 1 (all green, then commit tag `alpha1`, push)
- [ ] A1: full loop: title→difficulty→campaign→play all 3 games→reward→
  end turn→AI→conquest→victory screen (playwright scripted, 0 errors).
- [ ] A2: docker `cargo test --release` + `clippy -D warnings` green.
- [ ] A3: screenshots: title, menu, campaign, castle panel, battle, each
  minigame play/results — baselines committed, pixel-diff re-run clean.
- [ ] A4: audio RMS audible on title loop in headless.
- [ ] A5: progress log complete for T0xx–T5xx.

## Phase 6 — Minigames 4–9 (T6xx) — each MG-QA Q1–Q6, own baseline

- [ ] **T601 mg-archery**: gallery, pop-ups + moving jester, apple bonus.
- [ ] **T602 mg-rampart**: wall defense, ladders, rock-fall special.
- [ ] **T603 mg-maze**: dungeon rescue, guard patrol, torch fright power.
- [ ] **T604 mg-river**: canoe pole-vault, rocks, enemy pike boats.
- [ ] **T605 mg-cloak**: 5 cells, sight cones, alarm-bell escalation.
- [ ] **T606 mg-forge**: 3-tempo rhythm hammering, sword-quality tiers.
- [ ] **Checkpoint P6**: 9 games in registry; batch soak: autoplay each ×3;
  suite + clippy green.

## Phase 7 — Minigames 10–13 + meta (T7xx)

- [ ] **T701 mg-herbs**: memory brew w/ boil timer.
- [ ] **T702 mg-falcon**: inertia flight intercept, corn-raid counter.
- [ ] **T703 mg-tavern**: 3 waves, grab/throw, barrels, barmaid rule.
- [ ] **T704 mg-dragon**: hoard sneak, loot-weight escape, cursed crown
  gamble; rare-event spawn (turn≥20 once) campaign test.
- [ ] **T705 HALL OF FAME screen**: per-game records (localStorage), title
  menu entry, reset option; playwright verify persistence across reload.
- [ ] **T706 Unlock pacing integration**: 1–4 start / 5–9 t5 / 10–12 t10 /
  dragon rare; campaign tests + SPORTS list screenshot per era.
- [ ] **T707 Champion triple-gold event flow**: 3 event-tournament golds →
  champion recruit modal → garrison bonus visible in panel; test+shot.

## Phase 8 — ALPHA 2 passes (T8xx) → ALPHA 2

- [ ] **T801 Soak**: 500 AI-vs-AI games 0 panics; 100 % terminate ≤90;
  invariants clean; reward-economy cap test (minigames can't break income).
- [ ] **T802 Balance sweep**: every game Q6 thresholds on final scores;
  champion pacing sane; HARD≥EASY 8/10 re-run.
- [ ] **T803 Perf pass**: `?perf=1` all scenes+games ≤12 ms p95; fix hot
  spots (dirty-rect/back buffer if needed).
- [ ] **T804 Payload**: wasm-opt -Oz; total ≤2 MB; per-track size table in
  progress.md.
- [ ] **T805 All-13 autoplay batch**: every game Q4 re-run on final engine,
  screenshots diffed; console-error gate 0.
- [ ] **Checkpoint ALPHA 2** — commit tag `alpha2`, push.
  - [ ] all T6xx–T8xx logged; suite+clippy green; payload & perf in budget.

## Phase 9 — BETA polish (T9xx)

- [ ] **T901 Art pass**: fresh-context judge subagent reviews every baseline
  vs presentation bible (palette/outline/readability); fix list → iterate;
  re-approve baselines.
- [ ] **T902 Audio mix pass**: track transitions (no clicks), SFX volume
  balance vs music, mute persistence; RMS table logged.
- [ ] **T903 Feel pass**: transitions everywhere (dither dissolve), screen
  shake on siege, victory/defeat sequences (coronation scene, siege-collapse
  scene), turn-start sting.
- [ ] **T904 Accessibility/readability**: contrast check on all text,
  keyboard-only campaign path, visible focus on menu.
- [ ] **T905 Docs**: README (controls, build, play), CHANGELOG, LICENSE
  (MIT — approved), module docs pointers.
- [ ] **T906 CI**: `.github/workflows/ci.yml`: native cargo test + clippy +
  wasm build + payload budget + artifact; Pages deploy from main (approved);
  green on branch push.
- [ ] **T907 Final soak+smoke**: full scripted playthrough incl. dragon
  event, hall-of-fame, coronation victory.

### GATE BETA 1 (stop here → human testing)
- [ ] B1: SPEC §13 all criteria verified with evidence in progress.md.
- [ ] B2: suite + clippy + CI green on `main`; Pages URL live (or artifact).
- [ ] B3: tag `beta1` + GitHub release w/ notes; final commit pushed.
- [ ] B4: handover note: how to play, what to test, known limits.

## Open follow-ups (post-beta, human-prioritized — do NOT start)
- Mobile portrait layout; high-score sharing; more tracks; scenario editor.
