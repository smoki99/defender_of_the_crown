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
- T501 — minigame framework: `Minigame` trait + SportSpec registry (12
  sports, DRAGON unlock_turn=u32::MAX = rare-event only), deterministic
  per-season `pick(turn, seed)` (xorshift64* — joust/catapult/sword/archery
  from turn 1, rampart..cloak +turn5, forge..tavern +turn10), Cheer 60 s
  wall-clock threshold, tier+loot results screen, MgHost scene bridged
  through the campaign (SPORTS, 10 g fee, herald variant), ?autoplay= hook.
- T502 — mg-joust: two passes, lane aim + release timing, lance-snap fail,
  crowd cheer ramp; gold-tier perfect script proven in tests + browser
  (crown-shots/00-joust-present, 01-joust-end); sports-check ticker smoke.
- T503 — mg-catapult: 6 stones, angle+power vs per-volley wind, towers and
  river ships as targets, distance-scored; full battery incl. sail-hit
  gold path (tests/mg_catapult.rs 6/6), host arm wired, whole suite green,
  wasm rebuilt, campaign browser smoke end to end (menu->crawl->map->levy->
  END TURN->omen modal OK dismissal, Chrome 150 on the debian .so sysroot).
- T504 — mg-sword: THE DUEL on the tilt band; foe telegraphs a lane via its
  blade tip (high/mid/low arrow keys), late feints switch the tell before the
  strike (last guard press wins), parry opens a 12-tick stagger meter whose
  BLACK gap (ticks 14..26, `riposte_ok`) takes one SPACE thrust per round;
  3 parries decide the bout, tier via resolve on the 0..=100 scale.
  tests/mg_sword.rs scripted bout + feint determinism, mg_host_tests sword
  arm, panel_kit probes; whole cargo suite green 2026-11-14, clippy clean.
  tools/sword-play-check.mjs pixel-solver (tip read at x178..188, guard on
  lane change, SPACE 250 ms into stagger) wins GOLD LAUREL off the real
  framebuffer twice frame-deterministic, purse 70->146 + "GOLD LAUREL: 86
  GOLD" log line on the map (crown-shots/00-sword-present, 01-sword-end,
  02-sword-home). Pushed ec84967.
- T505 — reward loop welded: a GOLD laurel now writes `doc.hof` via
  AppCtx::record_feat in sports_flow (title HALL OF FAME reads the same
  key), and tools/reward-loop-check.mjs drives the whole chain in jsdom:
  title->NORMAL->END TURN x4->turn 5 joust draw->pixel-solver two perfect
  lances->GOLD LAUREL plate (head band >8 GOLD px while ENTER waits)->map
  purse line (4,204..212) repainted->`doc.hof = "TOURNAMENT GOLD - SCORE
  97"`. 8/8 asserts, 0 errors, frame-deterministic; full cargo suite +
  clippy 0 warnings (2026-11-14).
- T601 — mg-archery (commit 9523056): THE GALLERY — pop-up gallery with
  rising targets and a crossing jester bonus; scripted point-and-shoot run
  takes GOLD (tests/mg_archery.rs 5/5) and plays GOLD in the browser SPORTS
  meet; host arm + mg_host_tests wired.
- T602 — mg-rampart (commit 99b2b78): the DEFEND wall siege — ladders scale
  per wave, rock-fall special, breach counter; survived-every-wave script
  takes GOLD (tests/mg_rampart.rs 4/4). Same commit welds the SPORTS draw:
  splitmix64 scramble of (turn,seed) before xorshift — raw xorshift left
  low bits a pure fn of the turn-only seed, making some sports unpickable;
  determinism + reachability tests added.
- ALPHA 1 loop (commit 4ba0c50): real Chromium plays the whole war end to
  end — title→difficulty→campaign→SPORTS meet→minigame→purse→END TURN→AI
  march→omen→conquest→crowned end screen, 0 console errors.
- T603 — mg-maze (commit 56eab72): RESCUE the baron — corridors clamp the
  runner, four gaolers ping-pong the streets, two lit torches freeze the
  watch, the freed baron walks the runner's footprints (90-tick trail) to
  the postern. The first layout dead-locked the east side behind the
  bottom-lane patrol — the y13 street is the only link in a 1-wide maze and
  crossing it under strict prediction is capture — so the dungeon got a y11
  bridge street at (8,11), torch I moved onto the right column (17,7), and
  the column gaoler re-clocked to the upper middle street. The scripted
  runner predicts every gaoler's folded beat position per tick (11 px
  clearance) and rides one fright dash up the column: 20 + 60 escape + 2×5
  torches = 90 = GOLD (tests/mg_maze.rs 5/5 incl. catch, torch-route,
  clamp). Whole cargo suite + clippy -D warnings green; wasm rebuilt; real
  Chromium smoke: turn-30 SPORTS draw lands on RESCUE, the dungeon paints
  bridge + new torch spots, runner answers arrows, torch II grabs (score
  0→5, flame gone, watch frozen), 0 console errors. (2026-11-14)
- T604 — mg-river (commit 88a9357): CANOE on the RIVER — pole-and-vault to
  the falls: charge/release vaults over seeded rocks, four pike boats sweep
  the lanes, wipeout sinks the paddler and swims them back, 2600 px ends the
  run. The scripted-gold test exposed unclearable seed pairs (boat@1900 +
  rock@2000), so two fairness rules welded in: WAKE ROWS — a boat with a
  rock within 130 px behind sweeps only that rock's lane (mid±20) instead of
  the full 228 px stream, which makes both marks hittable in sequence under
  the lane-bias guarantee; LANDING SKIM — 14 ticks of touch-down grace where
  an in-lane mark is credited as the hop itself, never a capsize (grounded
  contact without skim still wipes out). tests/mg_river.rs 6/6: seed-7 bot
  vaults all 8 rocks, sinks all 4 boats, 0 wipeouts → GOLD; drift-reaches-
  falls, banks, wipeout-score + sweep-invariant tests green. QA deep link
  ?scene=mg&mg=<id>[&seed=][&autoplay=1] boots any shipped sport standalone.
  Real Chromium smoke: tableau renders, keyboard steer + charge bar, WIPEOUT
  banner + swim-back live, autoplay resolves through results → campaign pop,
  0 console errors. Full cargo suite green, clippy 0 warnings (2 own hits
  fixed). (2026-11-14)

- T605 — mg-cloak (commit b15e366): STEALTH in the KEEP — douse all five
  lanterns and escape through the postern: WASD one-cell steps on a 20×16
  keep plan, four gaolers pace 20-23-tick beats with 4-deep side cones, a
  sighting burns a cloak and re-sends the thief to the postern behind a
  90-tick freeze (irons at zero), stepping an alarm bell quickens every
  beat forever (-4 ticks, floor 12), and the north nook torch waits behind
  a portcullis the lever draws; 900 ticks end the night. Scoring 5×15 + 10
  escape + 15 clean sweep, tiers 45/75/95. predicted_sight(ahead) publishes
  the safety contract (cones + gaoler cells at t+ahead): the gold-sweep
  test is a wait-for-clear bot, honest on every seed - seed 42's beat-4
  cone covers the fourth torch at boot and punished the naive script
  before the loop existed. tests/mg_cloak.rs 10/10: geometry floodcheck,
  WASD + portcullis gating, bell escalation, prediction tracks the beats,
  catch→freeze→irons chain, time-up, determinism, needs-all-torches
  escape, 100-point GOLD sweep. QA: cloak added to baseline-sports.sh -
  play frame 0 px, plate frame GOLD LAUREL, all 16 sport frames green.
  Real Chromium smoke via deep link: scene boots, W-walk renders, honest
  cone misses (no false catches), HUD cloaks + bell pips live, 0 console
  errors; title regression unharmed. Full cargo suite green, clippy 0
  warnings (6 own hits fixed), wasm32 check green, 507 KB bundle rebuilt.
  (2026-11-14)

- T606 - mg-forge (commit ed38270): RHYTHM in the SMITHY - hammer three
  tempos against the sweep line: eighteen beats at 40/28/18 ticks, space
  judged +/-3 PERFECT (5), +/-6 GOOD (2), anything else wastes the steel,
  and an unstruck beat walks away as a miss. The clean-chain bounty (+5)
  makes all-perfect exactly 95 - the gold blade line; bronze 40, silver 70,
  the sword quality IS the reward tier. The blank's heat colour cools on a
  miss and re-sparks on a perfect blow. Zero seed variance: the run is a
  pure function of the taps. Ten scripted tests (gold chain 95, all-GOOD
  bronze 41, one GOOD denies gold at 92, wasted swing + combo break, tempo
  ramp seam 40->28->18, idle-all-lost, space-only input, determinism).
  Real Chromium smoke via ?scene=mg&mg=forge: PRESENT banner, TEMPO I
  tableau (hammer, anvil, sweep bar, pips), an honest early Space judged
  MISSED with blood flash and cooled blank, autoplay plate GOLD LAUREL
  SCORE 95 PURSE GOLD, 0 console errors. Full cargo suite green, all 18/18
  baselines diff-clean (PLAY frames 0 px, plates within blink tol), clippy
  0 warnings, wasm32 check green, 511 KB bundle rebuilt; title regression
  1010 frames green. (2026-11-14)

- P6 CHECKPOINT (2026-11-14): all nine phase-6 sports live in the registry
  (sword, joust, catapult, archery, rampart, maze, river, cloak, forge) -
  each with its module, make_game arm, mg_<id> tests and two committed
  baselines. BATCH SOAK: three full baseline passes (18 captures each:
  PLAY spawn 0 px, autoplay PLATE within blink tol) - 54 captures, 0 FAIL;
  every sport's deep link boots, resolves and plates deterministically.
  Full cargo suite green, clippy 0 warnings, wasm32 check green, title
  regression 1010 frames clean. (2026-11-14)
- T4xx CHECKPOINT (2026-11-14): entry sequence = THREE inputs from load -
  boot key (audio unlock), BEGIN CAMPAIGN, difficulty; the intro crawl now
  AUTO-ROLLS (CRAWL_DWELL 300 ticks = 5 s per leaf, dwell resets on NEXT /
  BACK / entry) so a hands-off player rides straight into the war. Proof:
  title-flow-check rewritten around the budget - dwells (leaf-1 lion pixel),
  NEXT skips early, leaf 3 arrives hands-free (240 gold px), auto-commit
  reaches a live campaign (decoded HUD turn 1), 19/19 in BOTH the default
  and doc.crt=0 seed runs, 0 errors; headless-check campaign click loop
  25/25 green on the rebuilt bundle. Music: audio-check all green - silent
  pre-gesture, worklet sink, RMS 0.432-0.464 fanfare window, title loop
  0.433-0.438 at +6 s, M + reload mute persistence. Screenshots: title/menu/
  how-to/fame/options/crawl set /tmp/crown-title + shots=10 per run,
  approved at T403-T405; committed baselines land with A3.
- ALPHA 1 GATE (2026-11-14), all five green: A1 - the real-Chromium loop
  (tools/alpha1-loop.ts, commit 4ba0c50) plays title -> difficulty ->
  campaign -> sword+catapult+joust with gold rewards -> end turn -> AI dawn
  -> conquest -> victory screen, 0 console errors, shots archived.
  A2 - ./test.sh all green on a *freshly wiped* source volume (35 s cached):
  full cargo test --release, clippy -D warnings 0 hits, wasm32 + release
  build + wasm-bindgen web/pkg; test.sh ensure_stage() now RESETS the crown-src
  volume before docker cp (cp is additive - a deleted source kept building,
  and the proxy 403s `docker exec` [attach hijack], so the wipe rides on
  `docker run --rm`). A3 - committed pixel baselines under shots/ (.rgba.zz
  + human PNGs in shots/png/): tools/baseline-check.mjs 12 states (boot,
  menu, options, howto x3, fame, difficulty, crawl, CASTLE PANEL, BATTLE
  REPORT - forced by marching Buckingham on London, plate marker = LINEN at
  (64,88), omen-safe -, turn-1 campaign) and tools/baseline-sports.sh 14
  frames (PLAY spawn + result PLATE per shipped sport via ?scene=mg deep
  link, one cold-boot process per capture); every state diffs 0 px except
  report (144 - ticker/march text, tol 400) and plates (<=72 - blink ring,
  tol 1300) across two consecutive runs. A4 - audio RMS audible in headless
  (title loop 0.433-0.438, fanfare window 0.432-0.464, pre-gesture silent).
  A5 - progress.md complete T0xx-T6xx. Tag `alpha1` cut on this commit.

- T701 - mg-herbs (commit 701a252): MEMORY in the herb garden - a 4x4
  salve grid of eight herb kinds: one tile stays revealed until its
  partner lands (+10, blanked and locked) or any other tile is chosen;
  the mismatch shows both, shakes in blood-red and re-hides after 90
  ticks - seen stays seen. The cauldron liquid climbs olive -> gold ->
  blood over 40 s (BOWL 2400) and the boil-over ends the brew with pairs
  alone; a clean fast match banks up to +20 speed, so the flawless
  fastest brew is exactly 100 and gold 90 needs all eight inside 20 s.
  Silver 70 (seven pairs or a late full match), bronze 40 (four pairs).
  Layout is a seed-true pair-bag shuffle - no other randomness. Nine
  scripted tests (banner gate, mismatch shake + re-hide + memory, match
  lock, gold >=90, dawdle->silver, boil-over loss, partial->bronze 40,
  pair-bag + seed variance, gutter/hover never flip). Closed-meet gate
  tests (mg_host_tests, panel_kit) now SCAN for a turn whose posted meet
  stays unshipped instead of pinning turn 11 - the meet pool shifts as
  sports land, and herbs landing exposed the pin. Real Chromium smoke
  via ?scene=mg&mg=herbs: THE HERB SALVE banner, live grid (pouch tiles,
  climbing pot, MEMORY HUD), trusted mouse flips a real mismatch pair
  through show and re-hide, autoplay plate GOLD LAUREL MEMORY SCORE 90
  PURSE, 0 console errors. Full cargo suite green (258 tests), all 20/20
  sport baselines diff-clean twice over (PLAY 0 px, plates in blink
  tol), clippy 0 warnings, wasm32 release rebuilt (514 KB bundle);
  title regression 1014 frames green. (2026-11-15)
- **T702 mg-falcon - THE FALCON'S CAST (GOLD).** Tenth sport (SPORTS index
  10, label FALCONER), inertia flight over the ripe field. Ten crows
  launch every 150 ticks (rng x 20..300), each claiming the earliest
  standing ear of corn; they descend wobbling at 0.9/tick with a 0.45
  lateral pull, hover 40 ticks on the ear, then haul the corn off at
  0.55/tick - the ear counts lost only once the crow clears the top of
  the field, so a rescue during the drag gives it back. Any hawk touch
  within 14 px turns any crow in any stage (carrying included). The hawk
  is the toy: velocity blends toward the pointer (vx = 0.9*vx +
  0.1*clamp((mx-x)*0.14, +-3.2)), so a pointer warp yields a gliding
  overshoot, never a teleport. Score = intercept rate (10 per crow); 9/10
  = 90 beats the 88 gold line; all ten escaping is a loss. The game ends
  when all ten resolve, the host cheer (60 s) is the wall clock. Kite
  hawk (tan wingtips, hood glint, ground shadow), beating black crows
  with orange beaks, ten corn ears on a dusk field. Ten scripted tests
  (banner gate, distinct ear claims, touch-turn + ear stays, unhawked
  crow carries away, drag rescue returns the ear, full-wave rescue = 100
  GOLD, all-escape = Lost, seed-only determinism, hawk inertia +
  overshoot, dead crows stay dead). Host: deep-link alias falcon via
  SPORTS; jsdom baselines PLAY s0/s15 (banner->live flight, 33020 px)
  and PLATE s48 (63 px blink-only) recorded byte-identical twice; real
  Chromium smoke shows the FALCONER band + banner, the hawk gliding to a
  live mouse over the field (shadow, descending crow, ten ears) and the
  autoplay run freezing GOLD at the 88 line under the navy laurel plate,
  0 console errors. Full cargo suite green (268 tests, +10), all 22/22
  sport baselines diff-clean twice over, clippy 0 warnings, wasm32
  release rebuilt; title-flow regression 1022 frames green. Stale-bundle
  trap recorded: the first falcon captures ran against a pre-falcon
  web/pkg (unknown sport -> the driver fell back to title/campaign and
  the "plate" frame was a boot strip); the harness serves whatever
  web/pkg was built last - always rebuild before capturing. (2026-11-15)
- **T703 mg-tavern - THE TAVERN BRAWL (GOLD).** Eleventh sport (SPORTS
  index 11, label BRAWL): three waves of 4/5/6 bruisers (15 rogues)
  march in through alternating doors at FLOOR_Y; the belt rule makes the
  nearest marcher the one who swings, so a pile can never chain-stun the
  brawler, and W_SPEED 0.5/0.55/0.6 with 26/23 hit rings keeps wave 3
  mean but fair. Grab (G, 10-tick stun), hurl (G at a stunned rogue: 6
  ring points, 10 px knockback) and the three keg barrels (6/tick throw,
  3-hit break) give the scoring; hitting the barmaid costs a foul and
  her 15-tick kick-lock, and every foul shaves 15 off the 12-point-per-
  rogue plus 20-clean-floor total - the win line is 92 = six rogues plus
  the clean floor. The tavern itself draws the rule: two mottled ale
  barrels (30/250/146, hoops, bunghole, bung) sit inside the table legs
  at x 100/202, the pit boss guards the middle door (x 158, arms-out
  8-wide stance so no rogue can cross the counter line at x 165), and
  the maid patrols 25..275 at 0.6/tick, walking with swinging dress,
  arms and hair. Eight scripted tests (banner gate, march-in + swing,
  punch-stun-grab-hurl, stun drops the grip, barrel flattens the line,
  barmaid out-of-bounds foul + kick-lock, clean sweep earns the gold at
  92, the seed alone picks the maid's return). Host: deep-link alias
  tavern, 88 ms gold; jsdom baselines PLAY s0 (banner spawn frame per
  the SETTLE=250 contract, 0 px diff) and PLATE (frozen GOLD LAUREL,
  blink-only 138/0 px) across two full 24-row sweeps, zero FAILs; real
  Chromium smoke over the deep link proved the BRAWL band, WAVE/PUNCH
  HUD, both door bruisers, animation (785 px moved/s), arrow-key
  control (1164 px), the frozen GOLD LAUREL plate and 0 console errors.
  Full cargo suite green (275 tests, +7), clippy 0
  warnings, wasm32 release rebuilt (513 KB), title-flow regression 1022
  frames green. Two traps recorded: SETTLE=3000 on the PLAY row looks
  livelier but rAF tick jitter across the banner boundary made the
  frame nondeterministic (227044 px swings) - the 250 ms static-spawn
  contract exists for a reason; and a jsdom 320x256 target decoded at
  320x200 crops the HUD row and fakes a frozen room. (2026-11-15)

## T704 mg-dragon THE HOARD - done 2026-11-15

The thirteenth sport: creep past the sleeping wyrm in twelve 40 px
columns, lift the three bags (6/8/16) and the cursed crown (10, jangles
the beast awake) at 15 px a step, and carry the lot out west - every
burden slows the boots (1.6 - 0.15 px per weight, heavy crown 1.15 flat)
while the sleep/doze/gaze cycle (90/30/30 ticks) sweeps a 14 px/t beam
that is faster than any loaded thief, so standing still or retracing a
marked column is death. The doze adds a noise gate (two jingled drops
wake him), the wake phase telegraphs 24 ticks of roar and then fires
flame marks at every still target on a 45-tick volley, and escape
through the west door banks 90 + 10 points per carried item plus a
100-tick fame timer. Balance is arithmetic, not feel: at full weight the
lag per volley is 16 x (1.6 - 6 x SLOW) and SLOW 0.15 puts 11.2 above
the 9-point FLAME_W - a walking thief always outwalks the fire, a
standing one always burns; at the pre-fix 0.18 that margin flipped
(8.3 < 9) and the whole 100-point line went unreachable, so the constant
carries the proof in its comment. A *silent* flawless heist is
provably impossible - the beam overtakes any open-field return from
bag2 and the 28x32 cover boxes cannot be crossed between gaze windows -
so `the_perfect_heist` encodes the intended rule instead: east-first,
crown last, never stand, never retrace, full-load escape, Won(Gold)
alive. Thirteen scripted tests (banner gate, spawn in beam shadow,
weight steps, crown awake-on-jingle, doze noise gate, volley stillness
rule, door bank, fame timer) plus eight campaign tests: the hoard
omen fires once per campaign at turn>=20 (never twice, never at 19),
gates the HOARD panel row, and hoard_taken lore keeps it out forever
after the run. Registry sync got a guard of its own: `every_registered_
sport_boots_by_id` walks all 13 SPORTS through make_game so the table
and the factory can never drift apart again. Host alias dragon (86 ms
deep link, 100 GOLD), baselines for the new sport captured, then both
full 13-sport PLAY/PLATE sweeps green 52/52 with zero FAILs; the live
Chromium bundle (built fresh from this tree via docker build-web) booted
straight into the hoard, took arrow input, banked an 18-point bag, and
showed the open-eye HE WAKES volley with 0 console errors. Full cargo
suite green (323 tests, +48), clippy 0 warnings, title-flow regression
green. Three traps recorded: the jsdom harness overwrites
globalThis.setTimeout at import, so a driver's own sleep() must use
Bun.sleep or jsdom timer init recurses to a stack overflow (the crash
looks like the harness, it is not); the idle animation pair for the
smoke must be sampled before the scripted death plate (a static plate
fakes a frozen scene); and weight/gaze tests must spawn x<42, inside
the beam's shadow - mid-room spawns get swept mid-assert. (2026-11-15)

## T705 HALL OF FAME - done 2026-11-15

The Fame panel now reads every persisted record: a two-column grid of
all thirteen sports (left seven, right six, SPORTS-table order, so the
header row is TOURNAMENT - the joust sits at index 0) with each best
pulled live from `doc.mg.<id>`, tier-coloured by the sport's own
bronze/silver/gold thresholds (white label, yellow under silver, silver,
gold, gray dash while unset) and a right-aligned score computed with the
engine's real 8 px glyph advance - the first cut used 6 px and silently
drifted multi-digit scores left of their column. Records are written by
the host on every finish (`mg_best` keeps the max, `doc.mg.` namespace),
so the panel needs no game-side cooperation. Reset is two-step and
visible: ENTER arms (panel frame flips to blood 153,0,34 with the
ERASES prompt), ENTER again wipes all `doc.mg.*` keys plus `doc.hof`, a
third press only re-arms - no confirm dialog, no way to nuke the hall
with a stray keypress. Verification: cargo suite green (323) with
clippy 0; title-flow-check drives the whole path in both modes - clean
boot stays green and, seeded (`doc.mg.joust=57;doc.hof=...`), the
TOURNAMENT row scans 28 px of tier-yellow at (116,82), the armed frame
scans the blood border, and the second ENTER leaves every key null with
a clean console. Persistence across reload ran in real Chromium, not a
mock: the wasm write path banked `doc.mg.joust=95` via autoplay, a fresh
document on the same origin still read 95, and the menu walk to HALL OF
FAME rendered it gold (23 warm px in the score cell;
`10-reload-fame.png`). Three traps for whoever extends this: the
jsdom harness seeds localStorage *before* init and that part works -
what burned a full hour here was scanning SPORTS row i=1 for the joust
(SPORTS[0] is TOURNAMENT, row y=82, not y=93) and then chasing a ghost;
every CRT-darkened scanline sits at 2/3 intensity, so colour filters
must accept both (170,170,0) and (255,255,0) for yellow; and
`tab.screenshot({path})` from the harness writes nowhere - pull the
base64 from `page.screenshot` inside `tab.run` and write the buffer.

## T706 Unlock pacing integration - done 2026-11-15

The seasonal meet now breathes with the campaign: `season_sport` (ui)
draws the turn's sport from the same deterministic splitmix+Xor draw the
launcher uses, and the castle-panel SPORTS button prints its label
(DUEL on turn 3, CANOE on 7, MEMORY on 11) instead of the static word -
the era pacing is legible before the 10-gold fee is spent. Hoard stays
rare-event-only via its `unlock_turn == u32::MAX` sentinel, so the pool
grows exactly 4 (turns 1-4) / 9 (5-9) / 12 (10-12) and NO MEET is
unreachable in play. Two panel-button rects widened (sports 68->84px for
TOURNAMENT-length labels, neighbour 68->54). Verification is two-layer:
`tests/era_pacing.rs` pins the invariants on the real table (tier sizes,
the 5-5-2 turn partition, every turn offers a meet, every drawn sport
fits its era), and `tools/era-shots.mjs` boots the *actual* campaign at
turn=3/7/11 (harness honors `turn=` in CROWN_QS via `with_turn`), taps
Buckingham and compares the button's 8x8 glyph bitmap pixel-exact
against a host-side oracle - `examples/sports.rs` now emits the exact
rows `glyph()` paints, so the check can't be fooled by color or scaling
drift. All three eras pass and shot the panel (`era-t3/7/11`). Traps:
id vs label bit twice (SPORTS[0] is id joust with label TOURNAMENT; id
sword draws label DUEL) and a driver sentinel made of `#` chars that ate
its own grid rows - the oracle header is `@` now. Found and fixed a
pre-existing bug on the way: panel labels centered with a 6px advance
while the font is FONT_W=8 - every label sat 1px per glyph left of
center, long labels overran the button edge. Baselines: fame/panel/report
re-recorded (UPDATE=1 rewrote all twelve - restored the nine that still
passed their interior compare, only genuine drift was committed); suite
green (~39 targets, 0 fail) and clippy clean.

## T707 Champion triple-gold flow - done 2026-11-15

The triple crown now proclaims itself. The game-side recruit (third
event-tourney GOLD pushes a Champion into the hosting garrison, or pays
+40 when the banner already carries three) landed with rewards_v2; T707
is its voice: `champ_open` rides the omen-plate kit - LINEN field, NAVY
plaque with a hand-set three-crown crest (gold bands, cream tips, one
DARKRED jewel), "THE TRIPLE CROWN / A CHAMPION RIDETH TO / THY GARRISON
- GLORY ALONE", same OK rect as the dawn omen - and the map is deaf
until OK. The scene hook threads the flag straight out of the meet
result tuple (`(msg, crowned)`), no new mailbox. Two findings worth the
ink: (1) modal stacking is order-sensitive twice over - the champ gate
had to move *above* the omen gate so tap precedence matches paint
order, exactly the bug the crown-over-omen test caught; (2) the panel
needed nothing - the unit rows are data-driven off ALL_KINDS, so
CHAMP X1 rides in automatically (confirmed pixel-exact, row 3 under
LEVY/BOW/KNIGHT, header counts 13 men). Harness reachability got a
`?champ=1` boot hook (same builder shape as `?turn=`) that banks a
champion and opens the proclamation, since winning three tourneys is
not a fixture. `tools/champ-shot.mjs` verifies the whole flow in the
real campaign - plate/crown/jewel pixels, deaf castle tap, OK clears,
then the CHAMP X1 row matched glyph-for-glyph against the new generic
`examples/labelgrid` oracle (same 8x8 MSB-first grid trick as the T706
season labels; the jewel scans exactly (102,0,0) on a full-bright
scanline). shots: champ-modal, champ-panel. Tests 3/3 in
`tests/champion_flow.rs`; full suite 45 targets green, clippy 0.
