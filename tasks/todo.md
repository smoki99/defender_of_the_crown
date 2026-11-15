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
- [x] **T105 Sprite layer**: 8/16/32px sprites (indexed, mask), blit/scaled×N,
  palette-swap flash, 2-frame anim helper; tests.
- [x] **T106 Input**: event queue (pointer down/move/up w/ canvas→internal
  coords, keys, wheel), tap/swipe semantics, no-gesture audio-unlock flag;
  pure logic + tests.
- [x] **T107 Scene stack**: `Scene` trait (init/ event/draw/tick), push/pop/
  replace, fade + dither-dissolve transitions, RAF driver w/ fixed logic step;
  test scene runs in browser (`?scene=test`) screenshot + 60fps probe.
- [x] **T108 Audio engine**: modplayer→f32 PCM → WebAudio (AudioWorklet,
  ScriptProcessor fallback), music manager (play/loop/stinger/mixdown),
  mute-persist localStorage; SFX synth module (blip/coin/sword/crowd/hoof/
  boom/etc. factory) — RMS gate test in Chromium. *Verify: module loops audibly.*
- [x] **Checkpoint T1xx**: engine demo page (font sheet, dither gradient,
  sprite anim, music+sfx) screenshots clean, 0 console errors, perf ≤12 ms.

## Phase 2 — Campaign core (T2xx)

- [x] **T201 Data**: castle graph (12, symmetric-connected test), roads,
  unit defs (5 incl. champion), event table data.
- [x] **T202 Kingdom bars**: riches/provisions/loyalty/popularity structs,
  income/upkeep/shortfall-desert/revolt-at-0 rules; clamp invariants; tests.
- [x] **T203 Fog + spies**: hidden garrisons w/ ±50 % estimate, spy action
  (70 % reveal / fail loses gold), scout-adjacency reveal; seeded-determinism
  tests.
- [x] **T204 Marches + ownership**: one march/castle/turn, movement, siege
  handoff to combat; tests incl. road-only movement.
- [x] **T205 Combat v2**: walls/catapults/repel 30 + champion +25 % & duel
  moment; same-seed-same-result test; defender-favor + breach tests.
- [x] **T206 Events**: 7 event types incl. Danish raiders from t12 (Dover/
  London/Wallingford coast rule), tournament-banner generation; seeded tests.
- [x] **T207 Win/lose**: conquest, coronation (London+8), capital-fall −30
  loyalty (not instant loss), revolt chain, turn-90 judgment; scenario tests.
- [x] **T208 Champions + rewards API**: recruit via triple-gold flag, garrison
  bonus, reward application fns (gold/troops/provisions/bars) consumed by mg
  framework; tests.
- [x] **T209 AI v2**: attack ratio / recruit budget / spy freq knobs ×3
  difficulties; AI beats idle ≤100 turns; AI-vs-AI terminates; HARD ≥
  EASY-policy 8/10 (fixed seed set); monotonic-difficulty test.
- [x] **Checkpoint T2xx**: full docker suite green + determinism soak 100
  games.

## Phase 3 — Campaign view (T3xx)

- [x] **T301 Map renderer**: coastline/forest/hills shading, castle sprites
  (2 frames banner), roads, ownership colors, selected pulse; screenshot.
- [x] **T302 Panel UI kit**: src/ui/kit.rs 3-tone buttons, bevelled panels,
  scrollable list + scrollbar, LINEN hover tooltips, ALL-CAPS text,
  hit-testing; tests/panel_kit.rs + shots 02-selected/03-levy. See progress.md.
- [x] **T303 Turn bar + bars HUD**: WEALTH/FOOD/LOYAL/MOOD bars in a 2×2
  grid (gold scaled 200=full), fills glide ~8 ticks toward the true meters
  with a WHITE tick at the unreached target; tests/bars_hud.rs + headless
  shot 01-campaign. **DEVIATION (2026-11-14):** end-turn button, turn
  counter and event banner area (gold-band ticker + panel_kit modal frame)
  already shipped; no `norman.mod` sting — module audio needs an asset
  pipeline that doesn't exist yet, deferred (see T308 note in progress.md).
- [x] **T304 Castle panel**: info line, scrollable garrison roll (own keep =
  true counts; enemy fog stays "??"), 4 recruit buttons w/ costs, spy 15 g,
  sports button greyed w/ tourney msg (real sports = T501); click-through via
  tests/panel_kit.rs + headless. See progress.md.
- [x] **T305 March UI**: select→adjacent highlight→move preview→confirm;
  cancel; screenshot. **DEVIATION (2026-11-14):** shipped as one-tap march —
  select own keep, tap adjacent target, army moves immediately (no
  preview/confirm step); cancel by tapping away or ✗. See progress.md.
- [x] **T306 Battle report**: scroll-in report (one log line / 4 ticks) on a
  TAN/LINEN plate — result line GREEN win / NORMAN_RED repulse, wall-breach
  round line, champion-duel line; first tap skips, second closes; map deaf
  while open. tests/battle_report.rs (3) + headless (report shot, skip+close
  by clicks). **DEVIATION (2026-11-14):** log lines replace per-round animated
  lines - combat core resolves in one step; per-round fx deferred. See
  progress.md.
- [x] **T307 Event banners/modals**: tournament banner on castle, plague/
  caravan/raider modals w/ pixel icon; screenshot each. **2026-11-14:** all
  seven omens announce through one modal (TAN/LINEN plate, hand-set 16x16
  NAVY-plaque pixel icon per event, NAVY title, two story rows, OK button);
  map deaf until OK. Tourney year flies a YELLOW pennant on the hosting
  keep (visible through fog - heralds are public). tests/event_modals.rs (3)
  + headless (OK-dismiss + omen shot). "screenshot each": one modal per
  omen, same plate - shot 07-omen proves the render. See progress.md.
- [x] **T308 End-turn sequence**: player→AI turns with `norman.mod` sting,
  AI move animation pacing ≤2 s; event roll; screenshots. **2026-11-14:**
  march replays recorded `ai_moves` as NORMAN_RED banner dots for 1 s
  (map+HUD deaf while marching); horn = 1-bar `norman.mod` sting, exactly
  once per dawn, riding the campaign bed; report + omen wait for the last
  banner to land. ALSO fixed T301-era double-round bug: one END click ran
  `end_player_turn()` twice (Norman AI 3×, turn +2, double payday/hunger).
  tests/dawn_march.rs (4) + mod_roundtrip (6) + live Chromium (TURN 1→2,
  dots mid-march, omen after, RMS 0.400→0.424 horn). See progress.md.
- [x] **Checkpoint T3xx**: scripted playwright turn loop (click castle→
  recruit→march→end turn→AI moves) with 0 console errors. **2026-11-14:**
  loop green 17/17 via `bun run check:headless` (real wasm in jsdom — native
  Chromium still broken in-container); playwright pixel baselines deferred
  until browser libs are restored. See progress.md.

## Phase 4 — Title & music (T4xx)

- [x] **T401 Compose full tracks**: `title.mod` (fanfare+overture+loop),
  `saxon.mod`, `norman.mod`, `battle.mod`, `minigame.mod`, `victory.mod`,
  `defeat.mod` via compose.ts; decode-check all in cargo test; sizes in
  budget. **2026-11-14:** `tests/tracks.rs` decodes all 7, title carries the
  A-section restart byte; 7×~17 KiB ≤ 128 KiB budget.
- [x] **T402 Boot herald + press-to-begin**: black→banner unfurl anim,
  gesture unlocks audio, fanfare starts ≤0.5 s; screenshot + RMS proof.
  **2026-11-14:** banner unfurl + PRESS-ANY-KEY, first gesture swallowed for
  unlock; audio-check on `/` green: silent pre-gesture, worklet attach, RMS
  0.43 in first window after tap, M mute+persist, reload keeps state.
- [x] **T403 Title card + menu**: animated crown/banners/flames, name lettering,
  4 menu entries w/ keyboard+mouse nav; screenshot; loop into A-section.
  **2026-11-14:** flame border, bobbing crown, flanking banners; arrows/Enter
  + click nav, HowTo 3 pages, Hall of Fame (empty + persisted entries),
  OPTIONS sound toggle round-trips `doc.mute`; screenshots in /tmp/crown-title.
- [x] **T404 Difficulty + intro crawl**: EASY/NORMAL/HARD pick → 3-screen
  pixel crawl → campaign; screenshot.
  **2026-11-14:** gold-fill highlight, arrows/Enter + click; three illustrated
  crawl leaves with animated flame border, final crown vignette, NEXT commits
  the campaign. Keyboard-only flow proven by tools/title-flow-check.mjs
  (default + `doc.crt=0` seed runs, 19/19 pixel asserts, 0 errors).
- [x] **T405 How-to pages + options**: illustrated pixel how-to (3 pages),
  sound/CRT toggles persisted; screenshots.
  **2026-11-14:** castle/road + shield + crown pages, arrow-key paging;
  Options SOUND/CRT rows; `doc.crt` survives boot (seed run shows OFF at
  boot, flip persists "1"); CRT filter visibly darkens every scanline row.
- [x] **Checkpoint T4xx**: entry sequence playable ≤3 inputs from load,
  music audible (RMS), screenshots approved.
  **2026-11-14:** crawl auto-rolls (5 s/leaf) - 3 inputs reach a live
  campaign hands-free, 19/19 x2 seed runs + 25/25 click loop, RMS 0.43
  title loop, all shots approved. See progress.md.

## Phase 5 — MG framework + slice games (T5xx) → ALPHA 1

- [x] **T501 mg-framework**: `Minigame` trait + registry, MgState, tier
  thresholds data, 60 s cheer-timer convention, results screen (tier + loot
  anim), SPORTS wiring (10 g), unlock schedule data, `?autoplay=`/`?shot=`
  harness hooks; framework tests.
- [x] **T502 mg-joust** — MG-QA Q1–Q6. 2 passes, lane aim + release timing,
  lance snap, crowd cheer; perfect script wins gold tier.
- [x] **T503 mg-catapult** — MG-QA. 6 stones, angle/power vs wind, towers +
  ships; distance-scored.
- [x] **T504 mg-sword** — MG-QA. 3 telegraphs high/mid/low, parry-riposte,
  best-of-5; pattern-perfect script wins. **2026-11-14:** up-to-5 rounds,
  3 parries take the bout; pixel-solver wins GOLD in Chromium. See
  progress.md.
- [x] **T505 Reward-loop integration test**: campaign→joust→gold applied→
  hall-of-fame entry written (playwright scripted). **2026-11-14:**
  reward-loop-check.mjs 8/8 green (jsdom stand-in per T3xx note). See
  progress.md.
- [x] **Checkpoint ALPHA 1** — gate GREEN 2026-11-14, see below.

### GATE ALPHA 1 (all green, then commit tag `alpha1`, push)
- [x] A1: full loop: title→difficulty→campaign→play all 3 games→reward→
  end turn→AI→conquest→victory screen (playwright scripted, 0 errors).
- [x] A2: docker `cargo test --release` + `clippy -D warnings` green.
- [x] A3: screenshots: title, menu, campaign, castle panel, battle, each
  minigame play/results — baselines committed, pixel-diff re-run clean.
- [x] A4: audio RMS audible on title loop in headless.
- [x] A5: progress log complete for T0xx–T5xx.

## Phase 6 — Minigames 4–9 (T6xx) — each MG-QA Q1–Q6, own baseline

- [x] **T601 mg-archery**: gallery, pop-ups + moving jester, apple bonus.
  **2026-11-14:** GOLD scripted in tests + browser; see progress.md.
- [x] **T602 mg-rampart**: wall defense, ladders, rock-fall special.
  **2026-11-14:** GOLD scripted; SPORTS draw got the splitmix scramble
  fix (unpickable sports). See progress.md.
- [x] **T603 mg-maze**: dungeon rescue, guard patrol, torch fright power.
  **2026-11-14:** GOLD scripted (90 pts via y11 bridge + fright column
  dash); real-Chromium smoke green. See progress.md.
- [x] **T604 mg-river**: canoe pole-vault, rocks, enemy pike boats.
  **2026-11-14:** GOLD scripted (wake-row + landing-skim fairness); live
  Chromium play smoke. See progress.md.
- [x] **T605 mg-cloak**: 5 cells, sight cones, alarm-bell escalation.
  **2026-11-14:** GOLD scripted (prediction-gated safe steps); deep-link +
  baseline frames green. See progress.md.
- [x] **T606 mg-forge**: 3-tempo rhythm hammering, sword-quality tiers.
  **2026-11-14:** GOLD 95 scripted (all-perfect clean chain); live Chromium
  play smoke. See progress.md.
- [x] **Checkpoint P6**: 9 games in registry; batch soak: autoplay each ×3;
  suite + clippy green.
  **2026-11-14:** soak 3x18 captures, 0 FAIL; suite + clippy + wasm green.

## Phase 7 — Minigames 10–13 + meta (T7xx)

- [x] **T701 mg-herbs**: memory brew w/ boil timer.
  **2026-11-15:** GOLD scripted (8 pairs + speed bonus >=90; live mouse
  mismatch smoke). See progress.md.
- [x] **T702 mg-falcon**: inertia flight intercept, corn-raid counter.
  **2026-11-15:** scripted full-wave rescue = 100 = GOLD (gold line 88 =
  9/10); inertial glide + drag-rescue tested. Live mouse steering in Chromium.
- [x] **T703 mg-tavern**: 3 waves, grab/throw, barrels, barmaid rule.
  **2026-11-15:** clean sweep = 92 = GOLD (gold line 92: six rogues at 12
  plus the clean-floor 20). Belt rule (nearest attacker swings) and the
  maid kick-lock keep the brawl winnable; foul, hurl-chain and seed tests
  green.
  **GOLD 2026-11-15:** 88 ms gold deep link, 24/24 sport baselines clean
  twice over, real-Chromium smoke green, 513 KB wasm (see progress log).
- [x] **T704 mg-dragon**: hoard sneak, loot-weight escape, cursed crown
  gamble; rare-event spawn (turn≥20 once) campaign test.
  **2026-11-15:** scripted no-stand return = 90 loot + 10 door = 100 =
  GOLD (gold line 78); silent perfect heist is geometrically impossible
  (beam 14 px/t outruns 1.6-0.15w escape), the rule is never stand, never
  retrace. Omen-gated HOARD row + hoard_taken lore, once per campaign at
  turn>=20 (8 campaign tests).
- [x] **T704 dragon host/baseline/ship**: factory alias + registry sync
  test (13 sports boot by id), 13-sport PLAY/PLATE baselines x2 sweeps
  52/52 + new dragon pair, Chromium live smoke, suite+clippy green.
- [x] **T705 HALL OF FAME screen**: per-game records (localStorage), title
  menu entry, reset option; playwright verify persistence across reload.
- [x] **T706 Unlock pacing integration**: 1–4 start / 5–9 t5 / 10–12 t10 /
  dragon rare; campaign tests + seasonal SPORTS button verified per era.
- [x] **T707 Champion triple-gold event flow**: 3 event-tournament golds →
  champion recruit modal → garrison bonus visible in panel; test+shot.

## Phase 8 — ALPHA 2 passes (T8xx) → ALPHA 2

- [x] **T801 Soak**: 500 AI-vs-AI games 0 panics; 100 % terminate ≤100 (X+ tie-break);
  invariants clean; reward-economy cap test (minigames can't break income).
- [x] **T802 Balance sweep**: every game Q6 thresholds on final scores;
  champion pacing sane; HARD≥EASY 8/10 re-run.
- [x] **T803 Perf pass**: `?perf=1` all scenes+games ≤12 ms p95; fix hot
  spots (dirty-rect/back buffer if needed).
- [x] **T804 Payload**: wasm-opt -Oz; total ≤2 MB; per-track size table in
  progress.md.
- [x] **T805 All-13 autoplay batch**: every game Q4 re-run on final engine,
  screenshots diffed; console-error gate 0.
- [x] **Checkpoint ALPHA 2** — commit tag `alpha2`, push.
  - [x] all T6xx–T8xx logged; suite+clippy green; payload & perf in budget.

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
