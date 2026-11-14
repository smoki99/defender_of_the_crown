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
- T205 combat v2: champion +25 % garrison aura + duel moment, walls/breach,
  repel, defender-favor; tests/combat_v2.rs 5 green (same seed -> same result).
- T206 events v2: 7 kinds applied at dawn incl. Danish raiders >=t12 on coast
  keeps, tournament banners, plague/caravan/hunger log lines; events.rs seeded
  roll (event_rng separate stream); tests/events_v2.rs 7 green.
- T207 win/lose: coronation (London + 8 keeps), capital-fall -30 loyalty (not
  instant), revolt chain at loyalty 0, turn-90 popularity judgment;
  tests/victory_v2.rs 7 green.
- T208 champions + rewards API: triple-gold recruit flag, garrison bonus,
  apply_reward fns (gold/troops/provisions/bars) for mg framework;
  tests/rewards_v2.rs 10 green.
- T209 AI v2: attack-ratio/recruit-budget/spy-freq knobs x3 difficulties;
  beats idle player <=100 turns; HARD >= EASY 8/10 fixed seeds; monotonicity;
  tests/ai_v2.rs 10 green.
- T2xx CHECKPOINT: full docker suite green (re-verified 2026-11-14: every
  binary "ok", 0 failures, 148 tests) + determinism soak: 100 games, same
  seed -> identical end state.
- T3xx STATUS (partial, honest split): src/ui/mod.rs CampaignUi renders the
  campaign screen: roads, keeps w/ ownership colors, garrison + army counters,
  fog "??" markers, selection panel; HUD = GOLD + 3 bars + MOOD, turn counter,
  message ticker, END TURN; castle panel = garrison w/ View::Rough estimates,
  5 recruit buttons w/ costs (10/20/45/60), spy 15 g; one-tap march to
  adjacent; win/lose overlay w/ new-war restart. Title scene: EASY/NORMAL/HARD
  pick -> difficulty + seed (deep-link query too). tests/ui_play.rs 7
  click-contract green. NOT met: T303 animated bar tweens + norman.mod
  end-turn sting; T306 animated battle report (battle result = ticker line);
  T307 event modals (ticker line only). Those boxes stay open. T301 shipped
  (see entry below); T302 kit + T304 castle panel shipped 2026-11-14, sports
  button greyed w/ "HELD AT THE TOURNEY" msg until T501.
- T305 march verified end-to-end in the browser: click Buckingham -> click
  London -> panel reads "Buckingham (Saxon) 0 men" (host moved out) + ticker
  "THE HOST MARCHES ON London" (screenshot /tmp/crown-shots/04-march.png).
- T3xx CHECKPOINT — scripted loop GREEN, 17/17 assertions, 0 console errors
  (2026-11-14): tools/headless-check.mjs boots the REAL wasm bundle + glue
  (web/pkg) inside jsdom with synthetic pointer events + putImageData pixel
  readback (jsdom globals patched: perf shim, canvas-free ImageData/AudioContext
  stubs). Loop: title -> NORMAL tap -> campaign; Buckingham tap -> panel
  (9186 px region change); LEVY -> gold band change (charge = units.rs cost
  10 minus tick income); LON tap -> march ticker (630 px text); then FIVE
  END TURN clicks -> HUD turn counter 1->3->5->7->9->11 decoded straight from
  the framebuffer using the 8x8 glyph table (examples/coords.rs emits digit
  bitmaps). jsdom rAF throttles under load (~6-60 fps) so liveness = decoded
  digits, not frame count. Re-run: bun run check:headless (~4 min: cargo
  example build + polls). PNG baselines in /tmp only for now; commit vs-diff
  baselines when native Chromium returns (T001 libs lost in container
  recreate; restore via setup-gitea-env.sh is out of scope for the game).

## T301 map renderer - done 2026-11-14
- Coastline: piecewise-linear `coast_edge(y)` (Norfolk bulge -> London
  bight -> Dover -> Channel corner), DEEPWATER fill, 2px checkerboard SKY
  hatch at the shore, TEAL wave dashes drifting one row per 9 anim ticks.
- Scenery: 10 tree canopies (Sherwood/New Forest/Essex/SW) + 5 hills with
  SILVER highlights, static tables in src/ui/mod.rs.
- Keeps: two-frame fluttering banner from the left crenel (owner color,
  skipped on fog-hidden keeps), gold port marker on Dover, selection ring
  breathes yellow->gold and expands 1px every 8 ticks (CampaignUi.anim,
  advanced in tick()).
- Verification: tests/map_render.rs (4 pixel-pinned tests: sea/shore/wave
  drift, scenery pixels, banner frame A/B, pulse ring) - two initial
  failures were real collisions (Dover crenel over the shore pixel,
  Lincoln label over the hill highlight), retargeted. `./test.sh all`
  green (suite + clippy -D warnings + wasm32 check; two clippy lints
  fixed: is_multiple_of -> explicit-paren & 1). check:headless 17/17 on
  the rebuilt bundle, 0 console errors; screenshots /tmp/crown-shots/
  01-campaign.png (full map: coast, trees, hills, banners) and
  03-levy.png (panel open over new terrain). Baselines commit with the
  T3xx checkpoint follow-up once native Chromium libs are restored.

- T302+T304 PANEL UI KIT + CASTLE PANEL GREEN (2026-11-14): new src/ui/kit.rs
  pixel kit: 3-tone bevel buttons (normal/hot/disabled), TAN-rimmed panel
  w/ GRAY-lit BLACK-shadow inner bevel, LINEN tooltip plate w/ BLACK rim,
  list scrollbar (WHITE arrows, TAN thumb, DARKGRAY track) + hit-testing,
  ALL-CAPS 8px text throughout. Castle panel grown to y86..172: info line
  (name/faction/men/walls), scrollable garrison roll (unit rows + WALLS
  row), recruit row LEVY/BOW/KNIGHT/SIEGE w/ costs, CLOSE/SPY/SPORTS row;
  SPORTS greyed, click says "THE SPORTS ARE HELD AT THE TOURNEY" (T501);
  hover tooltips on every control (ui.hover + tip()); scrollbar arrows
  + track drag set ui.list_top. Verify: tests/panel_kit.rs 6 pixel tests
  (button tones, panel bevels, tooltip plate, garrison scroll, sports
  grey+msg, end-turn hit order) + ./test.sh all green + check:headless
  17/17 exit 0 on rebuilt bundle, shots /tmp/crown-shots/02-selected.png
  + 03-levy.png (panel open, LEVY X7 recruited, tooltip visible).
  TOOLING (root cause, was silently shipping stale wasm): test.sh staged
  the host tree BEFORE every stage incl. pull-pkg, so bindgen output in
  crown-src:/work/web/pkg got clobbered by the stale host web/pkg copy
  before it was copied back. Fixed: test.sh pull-pkg skips ensure_stage;
  new stage pull-pkg (docker cp out of crown-stage); build-web.ts calls
  ./test.sh pull-pkg instead of raw docker cp (docker env lives only in
  test.sh); headless-check.mjs now process.exit()s with the verdict
  instead of hanging in jsdom rAF until killed.
```

## T303 kingdom bars HUD - done 2026-11-14

WEALTH/FOOD/LOYAL/MOOD in a 2x2 grid under the GOLD readout (gold bar
scaled 200=full, spec-adjacent: DoC shows a gold number plus three bars;
the fourth makes the gold visible as a meter too). Fills tween: bars_disp
[u32;4] in CampaignUi glides toward the true meters each tick with
step=diff.div_ceil(8) (~8 ticks, deterministic), and while a bar moves a
WHITE tick marks the unreached target column (Amiga change-marker feel).
Exact GOLD n text + TURN/ARMY + ticker row kept untouched (ui_play pixel
tests still pass).
- src/ui/mod.rs: bars_disp field + bar_targets_of + tick glide; bar()
takes (v,tgt); draw_hud 2x2 at (4|84,214|224).
- tests/bars_hud.rs 3 tests: settled bar has no marker; drained gold
shows old-long fill + white tick mid-glide, converges, marker gone
(GRAY border returns); provisions bar same on the FOOD track.
- Deviations: end-turn button, turn counter and event banner area
(ticker + panel_kit modal frame) were already shipped in earlier tasks;
norman.mod AI-turn sting NOT done - module audio needs an asset pipeline
that does not exist (browser has no mod decoder), deferred.
Verified: cargo test host green; docker gate green (22 suites, exit 0);
headless 17/17 clean; shot 01-campaign pixels confirmed for all four bar
colors + ticker text.

## T306 battle report - done 2026-11-14

A fight (march combat or an AI dawn raid) opens a TAN-rim LINEN-plate
window over the map; the battle's log lines scroll in one per 4 ticks,
the result line paints GREEN ("WON") or NORMAN_RED ("REPULSED"/champion
turn), breach and duel lines ride along, and the whole map is deaf until
the report dies: first tap skips the scroll, second closes it.
- combat: log_battle adds "THE WALLS BREACHED IN ROUND n." whenever
  walls_broken_round is Some (was silently dropped).
- ui: CampaignUi{report,report_shown,report_open}; capture_report(before
  log len) fires from BOTH march and end_turn; tap intercept sits above
  every other hit-test in event(); tick reveals at anim.is_multiple_of(4);
  draw_report is the topmost layer.
- tests/battle_report.rs 3 tests: seeded Buckingham-vs-levy fight opens
  the overlay, line 1 waits for its 4-tick slot, WON row is GREEN ink,
  blank rows stay LINEN; skip-then-close and a stray tap after close
  burns no turn; a battleless tap opens nothing.
- headless: march combat now opens the report mid-run, so the click loop
  calls dismissReport() before each END tap (plate probe at text-free
  (64,100)); shot 05/06-report; run asserts a report DID appear.
  19/19 pass, clean console.
- Deviation: "animated round lines" -> one scroll-in block per battle;
  the core resolves combat in a single step, per-round simulation fx are
  not in the data. Duel + breach lines cover the named content.

## T307 event modals - done 2026-11-14

Every dawn omen now announces itself instead of hiding in the ticker:
a TAN-rim LINEN-plate modal (70,64 180x74) with a hand-set 16x16
pixel icon on a NAVY plaque, NAVY title, two story rows and a raised
DARKRED OK button with CREAM label. While it stands the whole map is
deaf - castle taps, action buttons AND end-turn are swallowed; only a
tap inside event_ok_rect() dismisses. Tourney year: the hosting keep
flies a YELLOW pennant from its right crenel (fluttered with the same
anim/8 phase as the house banner) - drawn even through fog because a
herald's cry is public knowledge.
- src/ui/mod.rs: event_kind/event_open/event_turn fields; hook in
  end_turn() (one announce per turn, event_turn guard); tap intercept
  sits BELOW the report (report first, then modal, then map);
  event_body() story rows; draw_event_icon() 7 pictographs (bugle,
  pouch, blighted stalks, death's head, crown, slinking man, longship);
  tournament.contains(&Some(s.id)) pennant in the castle loop.
- tests/event_modals.rs 3 tests (seed 13 forced plague via the spy-tip
  override - two end_player_turn calls per END press means the tip
  seeds roll 1 and the FINAL g.event is roll 2, so the seed was probed):
  modal+icon+button pixels + OK close + plate gone; deaf map (keep tap
  no-select, END swallowed, OK frees, next dawn re-announces); tourney
  pennant YELLOW on the pick, none on a dark keep.
- headless: dismissReport() now clears BOTH overlays (report at (64,100),
  modal at (74,70)); OK tap at (160,128); asserts an omen DID appear;
  shot 07-omen palette-checked (title NAVY 166px, plaque 224+32 icon
  px, OK face 234px + label 30px). 21/21 pass, clean console.
- Deviation: "plague/caravan/raider modals" per event -> one modal
  template, per-omen icon+text (all 7 kinds); "screenshot each" -> one
  representative omen shot + host pixel tests per kind stay in the
  shared painter.

## T308 end-turn march + war-horn - done 2026-11-14
- Enemy dawn replay: `game.ai_moves` records every Norman march during the
  AI turn (Vec<(usize,usize)>, cleared at end_turn start, begin/end
  recording brackets). CampaignUi freezes on `ai_anim = AI_ANIM_TICKS (60)`;
  draw interpolates each move to `t = (60-ai_anim)/60` and stamps a 3x3
  NORMAN_RED banner dot at the marching position, after the castle loop so
  hosts ride above the map. While `ai_anim > 0` the whole HUD is deaf
  (castle taps, action buttons, END, report AND omen open all deferred);
  tick() counts down and `open_dawn()` then releases report -> omen in
  order.
- `norman.mod`: 1-bar 4-step war-horn (D4-A4-F4-D5 saw-ish duty, per-pat
  volume fall 2A->40) + trailing echo pat, title "NORMAN STING". Added to
  compose.ts (`norman` target, WAV -> `bun tools/compose.ts norman`),
  committed as assets/music/norman.mod.
- Horn plumbing: AppCtx::sting(track) -> web Ctx drives Horn (once-player
  over the bed via ModPlayer::set_sting; silence when idle). Sting fires
  exactly once per dawn from PlayScene::tick while `sting_armed` (armed in
  end_turn, never re-armed while the host marches). Native tests render the
  sting channel (tone pitch/PCM, once-player silence, demo mixture).
- tests/dawn_march.rs (4): march recorded + map deaf mid-march + reachable
  after; banner dots proven by NORMAN_RED pixel delta (drawn frame vs
  ai_anim=0 frame); omen/report stay closed until the last dot lands; horn
  bytes ("NORMAN STING" title) bled exactly once per dawn.
- **BUGFIX (found live in Chromium, one-click audit):** ui.end_turn() had
  called g.end_player_turn() twice + a stray take_turn ever since T301 -
  every END click skipped a whole round (turn +2, Normans acted 3x, double
  payday & double hunger). One click is now one round; the "+2" pins in
  tests/event_modals.rs + tests/ui_play.rs encoded the bug and were
  corrected. T307's seed-13 probe note in the log above is moot (spy tip
  now seeds the only roll).
- Live proof (Chromium via CDP): TURN 1 -> 2, gold +26 once, hunger once;
  mid-march frame shows marching dots, no modal; BAD HARVEST plate opens
  only after the last banner lands; window.__rms 0.400 bed -> 0.424 peak
  with the horn over the campaign track. 25/25 test groups, clippy clean.

## T401-T403 tracks, boot herald, title card - done 2026-11-14
- T401 — compose.ts 7 tracks (title/saxon/norman/battle/minigame/victory/
  defeat), all decode + budget via tests/tracks.rs (title restart byte =
  A-section, 7×~17 KiB < 128 KiB). Player render() is the only clock, so a
  queued pre-gesture track parks at row 0: the overture resumes bar one on
  unlock (2026-11-14)
- T402 — TitleScene::Boot: unfurling herald banner + blinking PRESS ANY KEY;
  first gesture swallowed as unlock (fanfare from tick 0). audio-check.ts
  retargeted from ?scene=test to `/`: silent before gesture, worklet attached,
  RMS 0.43 in first 600 ms window after tap, M + reload mute persistence,
  0 errors (2026-11-14)
- T403 — title card: flame-licked border, bobbing bejewelled crown, flanking
  banners (black-outlined), dithered Saxon field, 4-row menu with keyboard
  (arrows/Enter, deferred pending->tick for AppCtx access) + mouse; HOW TO
  PLAY 3 pages (prev/next/back, Enter pages), HALL OF FAME reads doc.hof
  (empty + rendered entries proven), OPTIONS SOUND ON/OFF round-trips
  doc.mute with the M key and reload (playwright: 1 -> 0 -> 1). PlayScene:
  saxon bed on init, battle bed brackets the report overlay, victory/defeat
  sting + record_feat exactly once (tests/music_flow.rs 4/4). Screenshots
  /tmp/crown-title k1-k9 + 01-06 (2026-11-14)
- T404/T405 — difficulty pick (GOLD fill on chosen row, y181 interior probes),
  3-leaf intro crawl (flame border 3-sample diff, crown vignette 240 gold px,
  NEXT -> PlayScene), illustrated HowTo x3, Options CRT row wired to the
  whole-frame scanline darkening + doc.crt. Harness split: jsdom-harness.mjs
  (boot+helpers+decodeTurn, CRT-agnostic >150 glyph threshold) + scenarios
  headless-check.mjs (click campaign: turn +1/cycle proven 1..6) and
  title-flow-check.mjs (T404/T405 keyboard flow, LS_SEED CRT-off variant).
  3 scenarios 0 fails; 27 cargo suites green; clippy -D warnings clean (2026-11-14)
