# Capability Map: Defender of the Crown 2.0 (Web/Rust/WASM)

One initiative, many independently testable capabilities → this map is the index
for per-module specs (`docs/specs/<id>.md`). Module ids are stable kebab-case.

| Module id | Responsibility | Depends on |
|---|---|---|
| engine-gfx | palette+framebuffer **done (T102/T103)**; font/sprites/blits/transitions pending | — |
| engine-audio | MOD music playback (own modplayer → PCM → WebAudio), chiptune SFX synth, music manager, autoplay unlock **done (T003/T004/T108)** | — |
| engine-input | unified mouse/keyboard/touch event queue with tap/sweep semantics | — |
| scene-core | scene stack + RAF loop + input routing + fade/dissolve transitions **done (T106/T107)** | engine-gfx, engine-input |
| campaign-core | pure game rules: castle graph, kingdom stats (riches/provisions/loyalty/popularity), units, marches, battles, events, spies, win/lose, RNG **done (T201-T209, checkpoint green)** | - |
| campaign-ai | Norman AI (recruit/attack/reinforce/spy, difficulty knobs) **done (T209)** | campaign-core |
| campaign-view | map render, pixel panel UI, battle reports, event banners, coronation/revolt UI **core screen done (map+coast+scenery+banners T301, panel kit + castle panel T302/T304, 4 animated kingdom bars T303, scroll-in battle report T306, omen modal + tourney pennant T307, HUD+one-tap march+end-turn loop, checkpoint T3xx green); pending T308 pacing** | campaign-core, engine-gfx |
| mg-framework | Minigame trait + registry + reward tiers + unlock schedule + HALL OF FAME (localStorage) + shared HUD/results screen **done (T501/T502)** | scene-core, engine-audio |
| mg-joust **done (T502: two-pass tilt, lane aim + lance timing)**, mg-catapult **done (T503: six-stone battery vs wind)**, mg-sword **done (T504: best-of-5 parry+riposte)**, mg-archery **done (T601: pop-window gallery)**, mg-rampart **done (T602: ladders at the curtain, leading shots, rock-fall)**, mg-falcon **done (T702: inertia flight, crowdive intercept)**, mg-tavern **done (T703: three-wave brawl)**, mg-dragon **done (T704: hoard heist - gaze, volley, weight; omen-gated from turn 20, double-take via campaign)**; mg-maze, mg-river, mg-cloak, mg-forge, mg-herbs pending | 13 individual minigames (8 shipped) | mg-framework, engine-* |
| title-flow | boot herald, animated title card, overture music sequencing, menu, difficulty pick **done (T404 pick; herald/menu partial)**, how-to, mute | scene-core, engine-audio, mg-framework |
| web-shell | HTML/CSS page, wasm bootstrap, canvas scaling, audio gesture unlock + persisted mute **done (T107/T108)**, page metadata | scene-core |
| tooling | build.sh, serve.ts, compose.ts (tracker→.MOD), sprite2rs.ts, shot.ts (playwright), fetch-chrome-libs.ts | — |

Dependency direction is acyclic. `campaign-core` and `engine-*` are leaf
modules with no web coupling except `engine-audio`'s WebAudio adapter and
`web-shell` (both thin adapters over pure cores).

**Build order**

```
engine-gfx ─┬─→ scene-core ─→ title-flow ─┐
engine-input┘         │                   │
engine-audio ─────────┼───────────────────┼──→ web-shell polish → CI
campaign-core ─┬─→ campaign-ai            │
               └─→ campaign-view ─────────┤
mg-framework (after scene-core) ─→ mg-*  (13, each independent) ─→ title-flow "HALL OF FAME"
```

Parallelizable: `engine-*` ∥ `campaign-core` ∥ `tooling`. Each minigame is an
independent slice testable alone (its own logic tests + screenshot check).
