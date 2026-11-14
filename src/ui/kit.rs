//! T302 pixel UI kit: three-tone bevel panels and buttons, a scrollbar
//! widget and a cursor tooltip. Pure palette-index painting so the whole
//! kit is pixel-testable without a browser.

use super::Btn;
use crate::engine::gfx::{
    Framebuffer, BLACK, DARKGRAY, DARKRED, EMBER, GRAY, H, LINEN, ORANGE, SILVER, TAN, W,
};

/// Bevelled panel: TAN rim, DARKGRAY face, light inner top/left edge and
/// dark inner bottom/right edge (Amiga three-tone).
pub fn panel(fb: &mut Framebuffer, x: i32, y: i32, w: u32, h: u32) {
    fb.fill_rect(x, y, w, h, DARKGRAY);
    fb.rect(x, y, w, h, TAN);
    let x1 = x + w as i32 - 1;
    let y1 = y + h as i32 - 1;
    fb.hline(x + 1, x1 - 1, y + 1, GRAY);
    fb.vline(x + 1, y + 1, y1 - 1, GRAY);
    fb.hline(x + 1, x1 - 1, y1 - 1, BLACK);
    fb.vline(x1 - 1, y + 1, y1 - 1, BLACK);
}

/// Raised button: three tones (face/light/shadow) per state - enabled,
/// hover (hot) or disabled. Callers paint the label.
pub fn button(fb: &mut Framebuffer, b: &Btn, hot: bool, enabled: bool) {
    let face = if !enabled {
        GRAY
    } else if hot {
        ORANGE
    } else {
        DARKRED
    };
    let light = if enabled { EMBER } else { SILVER };
    fb.fill_rect(b.x, b.y, b.w, b.h, face);
    fb.rect(b.x, b.y, b.w, b.h, BLACK);
    let x1 = b.x + b.w as i32 - 1;
    let y1 = b.y + b.h as i32 - 1;
    fb.hline(b.x + 1, x1 - 1, b.y + 1, light);
    fb.vline(b.x + 1, b.y + 1, y1 - 1, light);
    fb.hline(b.x + 1, x1 - 1, y1 - 1, BLACK);
    fb.vline(x1 - 1, b.y + 1, y1 - 1, BLACK);
}

/// Vertical scrollbar: arrow buttons plus a proportional thumb.
/// `hit` returns the row delta a click would apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scrollbar {
    pub x: i32,
    pub y: i32,
    pub h: u32,
    pub top: usize,
    pub total: usize,
    pub rows: usize,
}

impl Scrollbar {
    pub fn up(&self) -> Btn {
        Btn {
            id: "scr:up",
            x: self.x,
            y: self.y,
            w: 8,
            h: 8,
        }
    }

    pub fn down(&self) -> Btn {
        Btn {
            id: "scr:dn",
            x: self.x,
            y: self.y + self.h as i32 - 8,
            w: 8,
            h: 8,
        }
    }

    pub fn visible(&self) -> bool {
        self.total > self.rows
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        if !self.visible() {
            return None;
        }
        if self.up().hits(x, y) {
            Some(-1)
        } else if self.down().hits(x, y) {
            Some(1)
        } else if x >= self.x && x < self.x + 8 && y > self.y + 8 && y < self.y + self.h as i32 - 8
        {
            // Track click pages toward the click.
            let mid = self.y + self.h as i32 / 2;
            Some(if y < mid {
                -(self.rows as i32)
            } else {
                self.rows as i32
            })
        } else {
            None
        }
    }

    pub fn draw(&self, fb: &mut Framebuffer, hot: Option<&str>) {
        button(fb, &self.up(), hot == Some("scr:up"), self.top > 0);
        button(
            fb,
            &self.down(),
            hot == Some("scr:dn"),
            self.top + self.rows < self.total,
        );
        // Arrow glyphs as pixels (the 8x8 font has no triangle glyphs).
        let u = self.up();
        fb.tri(
            [(u.x + 2, u.y + 5), (u.x + 5, u.y + 5), (u.x + 3, u.y + 2)],
            WHITEISH,
        );
        let d = self.down();
        fb.tri(
            [(d.x + 2, d.y + 2), (d.x + 5, d.y + 2), (d.x + 3, d.y + 5)],
            WHITEISH,
        );
        // Track + thumb.
        let ty = self.y + 8;
        let th = self.h as i32 - 16;
        fb.fill_rect(self.x, ty, 8, th as u32, DARKGRAY);
        let span = (self.total - self.rows).max(1) as i32;
        let thumb = ((th * self.rows as i32) / self.total as i32).max(4);
        let off = (th - thumb) * self.top as i32 / span;
        fb.fill_rect(self.x, ty + off, 8, thumb as u32, TAN);
    }
}

const WHITEISH: u8 = crate::engine::gfx::WHITE;

/// Single-line tooltip: LINEN plate, black text, clamped on-screen.
pub fn tooltip(fb: &mut Framebuffer, x: i32, y: i32, text: &str) {
    let w = text.len() as u32 * 6 + 4;
    let tx = x.max(0).min(W as i32 - w as i32 - 1);
    let ty = y.max(0).min(H as i32 - 11);
    fb.fill_rect(tx, ty, w, 10, LINEN);
    fb.rect(tx, ty, w, 10, BLACK);
    crate::engine::fonts::draw_text(fb, tx + 2, ty + 1, text, BLACK);
}
