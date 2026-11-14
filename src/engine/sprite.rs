//! T105: indexed sprite layer. One byte per pixel, palette indices 0..=31,
//! index 32 (`TRANS`) = transparent. Blit / scaled blit / hit-flash /
//! two-frame animation helpers — all clipped, allocation-free.

use crate::engine::gfx::{Framebuffer, WHITE};

/// Transparent color-index sentinel.
pub const TRANS: u8 = 32;

#[derive(Clone, Copy)]
pub struct Sprite {
    pub w: u32,
    pub h: u32,
    pub px: &'static [u8],
}

impl Sprite {
    pub const fn sized(w: u32, h: u32, px: &'static [u8]) -> Self {
        debug_assert!(px.len() == (w * h) as usize);
        Self { w, h, px }
    }
    /// Square sprite (the 8/16/32 unit sizes).
    pub const fn square(size: u32, px: &'static [u8]) -> Self {
        Self::sized(size, size, px)
    }

    /// Top-left anchored blit. `flash` renders every opaque pixel WHITE.
    pub fn blit(&self, fb: &mut Framebuffer, x: i32, y: i32, flash: bool) {
        for (row, p) in self.px.chunks(self.w as usize).enumerate() {
            for (col, &c) in p.iter().enumerate() {
                if c != TRANS {
                    fb.set(
                        x + col as i32,
                        y + row as i32,
                        if flash { WHITE } else { c },
                    );
                }
            }
        }
    }

    /// Nearest-neighbour blit at integer zoom (`scale >= 1`).
    pub fn blit_scaled(&self, fb: &mut Framebuffer, x: i32, y: i32, scale: u32, flash: bool) {
        if scale <= 1 {
            return self.blit(fb, x, y, flash);
        }
        for (row, p) in self.px.chunks(self.w as usize).enumerate() {
            for (col, &c) in p.iter().enumerate() {
                if c != TRANS {
                    fb.fill_rect(
                        x + col as i32 * scale as i32,
                        y + row as i32 * scale as i32,
                        scale,
                        scale,
                        if flash { WHITE } else { c },
                    );
                }
            }
        }
    }
}

/// Two-frame animation helper: even frames draw `a`, odd draw `b`.
pub fn blit_anim(a: &Sprite, b: &Sprite, fb: &mut Framebuffer, x: i32, y: i32, frame: u32) {
    (if frame.is_multiple_of(2) { a } else { b }).blit(fb, x, y, false);
}
