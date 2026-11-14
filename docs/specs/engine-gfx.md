# Spec: engine-gfx (palette + framebuffer core)

Status: **implemented** (T102 palette/dither, T103 framebuffer/primitives).
Depends on: nothing (leaf). Consumers: scene-core, campaign-view, mg-framework, web adapter (T104).

## Public surface (`src/engine/gfx.rs`)

- `W:320 H:256 INTERNAL:u32=320*256`
- `PALETTE: [[u8;3];32]` + `rgb(idx)` + named consts (`BLACK=0`, `NAVY`, `SAXON_BLUE`, `NORMAN_RED=RED=9`, `GOLD=22`, `WHITE=21`, `SKY=25`, `NAVY=1`, … 32 names).
- `nearest([r,g,b]) -> idx` — squared euclidean, lowest index wins ties.
- `half(idx) -> idx` — nearest to rgb/2 (shading), built once (LazyLock).
- `mix(a, b, t) -> idx`, `t in 0..=8` — palette-snapped blend, 1024-entry LazyLock table.
- `BAYER4` + `bayer(x,y)` — classic 4×4 ordered pattern, thresholds 0..15.
- `Framebuffer { data: Vec<u8> }` — `new/clear/get(in-bounds panics)/try_get/in_bounds/fill_rect/fill/hline/vline/rect/fill_rect_dither/tri/fill_circle/gradient_v/to_rgba`.
  - All draws clip to screen; negative or oversized rects clip, never wrap.
  - `fill_rect_dither(x,y,w,h,a,b,t)`: `t` of 16 Bayer cells -> `a`, rest `b`; deterministic by absolute pixel coords.
  - `gradient_v`: 8 mix levels + 16-level Bayer residual; `t=0` top color, bottom `b`.
  - `to_rgba(buf4)` — palette expand for the canvas.

## Decisions

- 32 fixed colors chosen from the EGA-style Amiga-ish set; dithered gradients/fills recover the feel of more colors.
- All state explicit (`Framebuffer.data`); no globals except read-only lookup tables (HALF/MIX LazyLock) -> pure + testable + wasm-friendly.
- Integer-only math in table construction (no float, no libm) for byte-identical behavior on wasm and host.

## Tests (`tests/gfx.rs`, 11)

palette uniqueness/black0/white21 · nearest maps exact+approx · half halves luminance · fill_rect clipping (negative + oversized) · bounds helpers · fill_circle symmetry+size · tri edge-function exclusion · dither 50% exact + determinism (two framebuffers byte-equal) · gradient endpoint+half ordering · to_rgba mapping · mix endpoints + no row aliasing (regression: 1024-row table index was once cast through `as u8` and wrapped mod 256).
