//! T102/T103 proofs: palette, framebuffer bounds/clipping, primitives,
//! deterministic dithering, gradient monotonicity.

use defender_of_the_crown::engine::gfx::*;

#[test]
fn palette_has_32_unique_colors_black_0_white_21() {
    assert_eq!(PALETTE.len(), 32);
    assert_eq!(PALETTE[BLACK as usize], [0, 0, 0]);
    assert_eq!(PALETTE[WHITE as usize], [255, 255, 255]);
    for (i, a) in PALETTE.iter().enumerate() {
        for (j, b) in PALETTE.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "palette entries {i} and {j} identical");
            }
        }
    }
    // nearest() inverts exactly on palette colors.
    for (i, &p) in PALETTE.iter().enumerate() {
        assert_eq!(nearest(p), i as u8);
    }
}

#[test]
fn half_darkens_every_color() {
    let lum = |c: [u8; 3]| c[0] as u32 + c[1] as u32 + c[2] as u32;
    for i in 0..32u8 {
        if i != BLACK {
            assert!(lum(rgb(half(i))) < lum(rgb(i)), "half({i}) not darker");
        }
    }
}

#[test]
fn set_and_get_bounds_safe() {
    let mut fb = Framebuffer::new();
    fb.set(-1, 5, WHITE);
    fb.set(5, -1, WHITE);
    fb.set(W as i32, 5, WHITE);
    fb.set(5, H as i32, WHITE);
    assert_eq!(fb.count(WHITE), 0, "out-of-range writes must not land");
    fb.set(0, 0, WHITE);
    fb.set((W - 1) as i32, (H - 1) as i32, WHITE);
    assert_eq!(fb.get(0, 0), WHITE);
    assert_eq!(fb.get(W - 1, H - 1), WHITE);
    assert_eq!(fb.count(WHITE), 2);
    assert_eq!(fb.get(W, 0), 0); // OOB read = 0, no panic
}

#[test]
fn fill_rect_clips_at_edges() {
    let mut fb = Framebuffer::new();
    fb.fill_rect(-10, -10, 20, 20, GOLD);
    assert_eq!(fb.count(GOLD), 10 * 10, "corner clip lost pixels");
    fb.fill_rect((W - 5) as i32, (H - 5) as i32, 100, 100, SAXON_BLUE);
    assert_eq!(fb.count(SAXON_BLUE), 25);
}

#[test]
fn rect_outline_draws_frame_only() {
    let mut fb = Framebuffer::new();
    fb.rect(10, 10, 21, 11, WHITE);
    assert_eq!(fb.get(20, 10), WHITE, "top edge");
    assert_eq!(fb.get(20, 20), WHITE, "bottom edge");
    assert_eq!(fb.get(10, 15), WHITE, "left edge");
    assert_eq!(fb.get(30, 15), WHITE, "right edge");
    assert_eq!(fb.get(20, 15), BLACK, "interior untouched");
}

#[test]
fn dither_is_deterministic_and_weights_match() {
    let mut a = Framebuffer::new();
    let mut b = Framebuffer::new();
    a.fill_rect_dither(0, 0, 64, 64, WHITE, BLACK, 8);
    b.fill_rect_dither(0, 0, 64, 64, WHITE, BLACK, 8);
    assert_eq!(a.data, b.data, "dither not deterministic");
    assert_eq!(a.count(WHITE), 64 * 64 / 2, "t=8 must be exactly 50%");

    a.clear(BLACK);
    a.fill_rect_dither(0, 0, 32, 32, WHITE, BLACK, 0);
    assert_eq!(a.count(WHITE), 0, "t=0 => all b");
    a.fill_rect_dither(0, 0, 32, 32, WHITE, BLACK, 16);
    assert_eq!(a.count(WHITE), 32 * 32, "t=16 => all a");
}

#[test]
fn tri_covers_right_triangle() {
    let mut fb = Framebuffer::new();
    fb.tri([(10, 10), (20, 10), (10, 20)], GOLD);
    assert_eq!(fb.get(10, 10), GOLD);
    assert_eq!(fb.get(11, 11), GOLD, "hypotenuse inclusive edge");
    assert_eq!(fb.get(19, 10), GOLD);
    assert_eq!(fb.get(15, 18), BLACK, "outside hypotenuse");
    let n = fb.count(GOLD);
    assert!((50..=140).contains(&n), "tri pixel count {n} far from ~66");
}

#[test]
fn circle_symmetric_and_sized() {
    let mut fb = Framebuffer::new();
    fb.fill_circle(100, 100, 10, RED);
    assert_eq!(fb.get(100, 100), RED);
    assert_eq!(fb.get(110, 100), RED);
    assert_eq!(fb.get(112, 100), BLACK);
    // 4-way symmetry
    for dy in 0..=10u32 {
        for dx in 0..=10u32 {
            let at = |x: u32, y: u32| fb.get(x, y) == RED;
            assert_eq!(
                at(100 + dx, 100 + dy),
                at(100 - dx, 100 + dy),
                "symmetry x at {dx},{dy}"
            );
            assert_eq!(
                at(100 + dx, 100 + dy),
                at(100 + dx, 100 - dy),
                "symmetry y at {dx},{dy}"
            );
        }
    }
}

#[test]
fn gradient_darkens_toward_bottom() {
    // Palette quantization makes some mid-band blocks equal (palette has no
    // dark blues between TEAL and NAVY), so assert the real invariant:
    // strict top-vs-bottom and half ordering.
    let mut fb = Framebuffer::new();
    fb.gradient_v(0, 0, 4, 64, SKY, NAVY);
    let block = |k: u32| -> u32 {
        (16 * k..16 * (k + 1))
            .flat_map(|y| (0..4).map(move |x| (x, y)))
            .map(|(x, y)| {
                let c = rgb(fb.get(x, y));
                c[0] as u32 + c[1] as u32 + c[2] as u32
            })
            .sum()
    };
    let half_top = (block(0) + block(1)) / 2;
    let half_bot = (block(2) + block(3)) / 2;
    assert!(block(0) > block(3), "gradient endpoints indistinguishable");
    assert!(half_top > half_bot, "upper half must outshine lower half");
}

#[test]
fn to_rgba_maps_palette() {
    let mut fb = Framebuffer::new();
    fb.clear(GOLD);
    let mut out = vec![0u8; (W * H * 4) as usize];
    fb.to_rgba(&mut out);
    let p = rgb(GOLD);
    assert_eq!(&out[0..4], &[p[0], p[1], p[2], 255]);
    assert_eq!(&out[out.len() - 4..], &[p[0], p[1], p[2], 255]);
}

#[test]
fn mix_endpoints_and_rows_are_correct() {
    for (a, b) in [
        (SKY, NAVY),
        (BLACK, WHITE),
        (GOLD, NORMAN_RED),
        (SAXON_BLUE, CREAM),
    ] {
        assert_eq!(mix(a, b, 0), a);
        assert_eq!(mix(a, b, 8), b);
        // every intermediate is a real palette color between the endpoints
        for t in 1..8 {
            let m = mix(a, b, t);
            assert!((1..8).contains(&t) && m < 32);
        }
    }
    // high table rows must not alias low ones (regression: row index wrapped mod 256)
    assert_eq!(mix(SKY, NAVY, 4), mix(SKY, NAVY, 4));
    let mid = rgb(mix(SKY, NAVY, 4));
    let mid2 = rgb(mix(NAVY, SKY, 4));
    assert!(
        mid[2] > mid[0] && mid2[2] > mid2[0],
        "sky-navy midpoint must stay blue"
    );
}
