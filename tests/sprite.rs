//! T105 proofs: indexed blit with transparency, clipping, scaled blit,
//! hit-flash, two-frame animation helper.
use defender_of_the_crown::engine::gfx::{Framebuffer, BLACK, BLOOD, GREEN, NAVY, WHITE};
use defender_of_the_crown::engine::sprite::{blit_anim, Sprite, TRANS};

const S: u8 = TRANS;

/// 4x4 checker-ish pattern with transparent corners.
static PAT: [u8; 16] = [
    S, BLOOD, BLOOD, S, BLOOD, GREEN, GREEN, BLOOD, BLOOD, GREEN, GREEN, BLOOD, S, BLOOD, BLOOD, S,
];

#[test]
fn blit_honours_transparency_and_placement() {
    let sp = Sprite::square(4, &PAT);
    let mut fb = Framebuffer::new();
    fb.clear(NAVY);
    sp.blit(&mut fb, 2, 3, false);
    // opaque body landed
    assert_eq!(fb.get(3, 3), BLOOD);
    assert_eq!(fb.get(4, 4), GREEN);
    // transparent corners keep the background
    assert_eq!(fb.get(2, 3), NAVY);
    assert_eq!(fb.get(5, 6), NAVY);
    // untouched elsewhere
    assert_eq!(fb.get(6, 3), NAVY);
    assert_eq!(fb.count(BLACK), 0);
}

#[test]
fn blit_clips_at_all_edges() {
    let sp = Sprite::square(4, &PAT);
    let mut fb = Framebuffer::new();
    fb.clear(NAVY);
    sp.blit(&mut fb, -2, -2, false); // top-left offscreen
    sp.blit(&mut fb, 318, 254, false); // bottom-right offscreen
    sp.blit(&mut fb, -99, 40, false); // fully offscreen
    assert_eq!(fb.get(0, 0), GREEN); // pattern (2,2) visible at edge
    assert_eq!(fb.get(319, 255), GREEN); // pattern (1,1)
    assert_eq!(fb.get(319, 254), BLOOD); // pattern (1,0)
    assert_eq!(fb.count(GREEN), 2); // one px per clipped copy
}

#[test]
fn blit_scaled_multiplies_each_pixel() {
    let sp = Sprite::square(4, &PAT);
    let mut fb = Framebuffer::new();
    fb.clear(NAVY);
    sp.blit_scaled(&mut fb, 10, 10, 3, false);
    // pixel (1,1)=GREEN -> 3x3 block at (13,13)
    for dy in 0..3 {
        for dx in 0..3 {
            assert_eq!(fb.get(13 + dx, 13 + dy), GREEN);
        }
    }
    // transparent corner stays transparent at scale
    assert_eq!(fb.get(10, 10), NAVY);
    // scaled body count: 12 opaque px * 9
    assert_eq!(fb.count(BLOOD) + fb.count(GREEN), 12 * 9);
    // scale 1 == plain blit footprint
    let mut fb2 = Framebuffer::new();
    fb2.clear(NAVY);
    sp.blit_scaled(&mut fb2, 0, 0, 1, false);
    assert_eq!(fb2.count(GREEN), 4);
}

#[test]
fn flash_paints_only_opaque_pixels_white() {
    let sp = Sprite::square(4, &PAT);
    let mut fb = Framebuffer::new();
    fb.clear(NAVY);
    sp.blit(&mut fb, 5, 5, true);
    assert_eq!(fb.get(6, 5), WHITE);
    assert_eq!(fb.get(7, 6), WHITE);
    assert_eq!(fb.get(5, 5), NAVY); // corner untouched
    assert_eq!(fb.count(WHITE), 12);
}

#[test]
fn anim_helper_alternates_frames() {
    static A: [u8; 4] = [BLOOD, BLOOD, BLOOD, BLOOD];
    static B: [u8; 4] = [GREEN, GREEN, GREEN, GREEN];
    let a = Sprite::square(2, &A);
    let b = Sprite::square(2, &B);
    let mut fb = Framebuffer::new();
    fb.clear(BLACK);
    blit_anim(&a, &b, &mut fb, 0, 0, 0);
    assert_eq!(fb.count(BLOOD), 4);
    fb.clear(BLACK);
    blit_anim(&a, &b, &mut fb, 0, 0, 1);
    assert_eq!(fb.count(GREEN), 4);
}
