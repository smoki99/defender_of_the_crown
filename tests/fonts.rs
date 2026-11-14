//! T114 proofs: glyph-table coverage, '?' fallback, determinism, draw_text
//! pixel placement / gap transparency, shadow offset, width, edge clipping.

use defender_of_the_crown::engine::fonts::*;
use defender_of_the_crown::engine::gfx::*;

fn lit(bits: &[u8; 8]) -> u32 {
    bits.iter().map(|b| b.count_ones()).sum()
}

fn is_lit(bits: &[u8; 8], col: i32, row: i32) -> bool {
    col >= 0 && row >= 0 && col < 8 && row < 8 && bits[row as usize] & (0x80u8 >> col as u32) != 0
}

#[test]
fn space_blank_and_every_other_printable_glyph_has_ink() {
    assert_eq!(glyph(' '), [0; 8], "space must be all-zero");
    for c in 33u8..=126 {
        assert_ne!(glyph(c as char), [0; 8], "glyph {c} is blank");
    }
}

#[test]
fn question_mark_fallback_covers_non_printable() {
    assert_ne!(glyph('?'), [0; 8], "'?' itself must have ink");
    assert_eq!(glyph('λ'), glyph('?'), "non-ASCII falls back to '?'");
    assert_eq!(glyph('\n'), glyph('?'), "control char falls back to '?'");
    assert_eq!(glyph('\0'), glyph('?'), "NUL falls back to '?'");
    assert_eq!(glyph('\u{7F}'), glyph('?'), "DEL falls back to '?'");
    assert_eq!(glyph('🚀'), glyph('?'), "astral char falls back to '?'");
}

#[test]
fn glyph_lookup_is_deterministic_and_table_indexed() {
    for c in 32u8..=126 {
        let ch = c as char;
        assert_eq!(glyph(ch), glyph(ch), "glyph({ch}) not reproducible");
        assert_eq!(GLYPHS[c as usize - 32], glyph(ch), "GLYPHS index off");
    }
}

#[test]
fn glyphs_stay_inside_5x7_of_8x8_cell() {
    for (i, g) in GLYPHS.iter().enumerate() {
        for (row, bits) in g.iter().enumerate() {
            assert_eq!(bits & 0x07, 0, "glyph {i} leaks into gap column");
            if row == 7 {
                assert_eq!(*bits, 0, "glyph {i} leaks into gap row");
            }
        }
    }
}

#[test]
fn letter_a_top_row_and_corner_sanity() {
    let a = glyph('A');
    assert!(a[0].count_ones() >= 2, "A top row must have >=2 lit px");
    // mirror of the five used bits (7..3) — A's apex is left-right symmetric.
    let rev5 = ((a[0] & 0x80) >> 4)
        | ((a[0] & 0x40) >> 2)
        | (a[0] & 0x20)
        | ((a[0] & 0x10) << 2)
        | ((a[0] & 0x08) << 4);
    assert_eq!(a[0], rev5, "A top row not symmetric");
    assert_ne!(a[6] & 0x80, 0, "A bottom-left corner unlit");
    assert_ne!(a[6] & 0x08, 0, "A bottom-right corner unlit");
}

#[test]
fn draw_text_at_origin_writes_exact_pixel_count_and_keeps_gaps() {
    let s = "A?9x";
    let mut fb = Framebuffer::new();
    draw_text(&mut fb, 0, 0, s, WHITE);
    let want: u32 = s.chars().map(|c| lit(&glyph(c))).sum();
    assert!(want > 0);
    assert_eq!(
        fb.count(WHITE),
        want,
        "changed px must equal summed lit bits"
    );
    // nothing outside the 8px-cell band
    for y in 0..H {
        for x in 0..W {
            if fb.get(x, y) != 0 {
                assert!(x < text_width(s) && y < FONT_H, "px at ({x},{y}) outside");
            }
        }
    }
    // every cell's gap column and the gap row stay transparent
    for i in 0..s.chars().count() as u32 {
        for y in 0..FONT_H {
            assert_eq!(fb.get(i * 8 + 7, y), 0, "cell {i} gap column touched");
        }
    }
    for x in 0..text_width(s) {
        assert_eq!(fb.get(x, 7), 0, "gap row touched");
    }
}

#[test]
fn draw_text_pixels_match_glyph_bits_at_offset() {
    let mut fb = Framebuffer::new();
    let (x, y) = (5i32, 4i32);
    draw_text(&mut fb, x, y, "A", GOLD);
    let a = glyph('A');
    for (r, row) in a.iter().enumerate() {
        for c in 0..8u32 {
            let want = row & (0x80u8 >> c) != 0;
            let got = fb.get((x + c as i32) as u32, (y + r as i32) as u32);
            assert_eq!(got == GOLD, want, "px ({x}+{c},{y}+{r}) mismatch");
        }
    }
    // specific documented geometry: row 6 of 'A' = cols 0 and 4 (bits 7 and 3).
    assert_eq!(a[6], 0x88);
    assert_eq!(fb.get(x as u32, (y + 6) as u32), GOLD);
    assert_eq!(fb.get((x + 4) as u32, (y + 6) as u32), GOLD);
}

#[test]
fn draw_text_shadow_offsets_copy_by_one_and_fg_wins_on_overlap() {
    let mut fb = Framebuffer::new();
    let (x, y) = (10i32, 10i32);
    draw_text_shadow(&mut fb, x, y, "A", WHITE, GRAY);
    // shadow px of A's bottom corners land on the primary gap row -> visible.
    assert_eq!(fb.get((x + 1) as u32, (y + 7) as u32), GRAY);
    assert_eq!(fb.get((x + 5) as u32, (y + 7) as u32), GRAY);
    // both layers light (x+4, y+4): fg drawn last must win.
    assert_eq!(fb.get((x + 4) as u32, (y + 4) as u32), WHITE);
    // exact bookkeeping: fg everywhere the glyph is lit, shadow only where
    // the primary glyph is transparent.
    assert_eq!(fb.count(WHITE), lit(&glyph('A')));
    let overlap = (0..8i32)
        .map(|c| {
            (0..8i32)
                .filter(|r| is_lit(&glyph('A'), c, *r) && is_lit(&glyph('A'), c - 1, r - 1))
                .count()
        })
        .sum::<usize>();
    assert_eq!(fb.count(GRAY) as usize, lit(&glyph('A')) as usize - overlap);
}

#[test]
fn text_width_is_eight_px_per_char() {
    assert_eq!(text_width("abc"), 24);
    assert_eq!(text_width(""), 0);
    assert_eq!(text_width("λ"), 8, "width counts chars, not bytes");
}

#[test]
fn draw_text_clips_at_edges_without_panic() {
    let a = glyph('A');
    // hanging off the top-left: only rows 2..=6, cols 2..=4 stay on screen.
    let mut fb = Framebuffer::new();
    draw_text(&mut fb, -2, -2, "A", WHITE);
    let mut want = 0;
    for row in a.iter().skip(2).take(5) {
        for c in 2..5usize {
            if row & (0x80u8 >> c) != 0 {
                want += 1;
            }
        }
    }
    assert!(want > 0);
    assert_eq!(fb.count(WHITE), want);
    // hanging off the bottom-right: only rows 0..=2, cols 0..=1 land.
    let mut fb2 = Framebuffer::new();
    draw_text(&mut fb2, W as i32 - 2, H as i32 - 3, "A", WHITE);
    let mut want2 = 0;
    for row in a.iter().take(3) {
        for c in 0..2usize {
            if row & (0x80u8 >> c) != 0 {
                want2 += 1;
            }
        }
    }
    assert!(want2 > 0);
    assert_eq!(fb2.count(WHITE), want2);
    // fully off-screen: silently draws nothing, never panics.
    let mut fb3 = Framebuffer::new();
    draw_text(&mut fb3, -8, -8, "hello", WHITE);
    draw_text(&mut fb3, W as i32 + 1, 4, "hello", WHITE);
    assert_eq!(fb3.count(WHITE), 0);
    // string walking past the right edge: clipped tail, no wrap-around writes.
    let mut fb4 = Framebuffer::new();
    draw_text(&mut fb4, W as i32 - 12, H as i32 - 4, "WW@~", WHITE);
    assert!(fb4.count(WHITE) > 0);
    for y in 0..H {
        for x in 0..(W - 12) {
            assert_eq!(fb4.get(x, y), 0, "wrapped write at ({x},{y})");
        }
    }
}
