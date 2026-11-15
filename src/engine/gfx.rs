//! Palette, indexed framebuffer and drawing primitives (T102/T103).
//!
//! Everything is pure Rust, deterministic, allocation-free after `new`;
//! wasm adapters convert `to_rgba` output into ImageData.

use std::sync::LazyLock;

pub const W: u32 = 320;
pub const H: u32 = 256;

/// 32-color master palette, Sachs-style: five hue families built as shade
/// ramps (sea teal, meadow/forest green, earth sienna, warm stone, gold),
/// one bright accent (EMBER) and paper neutrals. Every channel is a
/// multiple of 17 - representable exactly in the Amiga's 12-bit registers.
/// Dark-atmosphere by design: no pure primaries, off-white highlights.
pub const PALETTE: [[u8; 3]; 32] = [
    [0, 0, 0],       // 0  BLACK (sprite-mask / void)
    [17, 34, 68],    // 1  NAVY
    [51, 102, 153],  // 2  SAXON_BLUE
    [136, 170, 204], // 3  LIGHTBLUE
    [34, 51, 17],    // 4  DARKGREEN
    [68, 102, 34],   // 5  GREEN
    [51, 119, 119],  // 6  TEAL
    [102, 170, 170], // 7  CYAN
    [85, 17, 17],    // 8  DARKRED
    [153, 34, 17],   // 9  RED (Norman)
    [119, 17, 26],   // 10 BLOOD
    [204, 136, 119], // 11 PINK
    [51, 34, 17],    // 12 BROWN
    [153, 85, 34],   // 13 ORANGE (sienna: highlands & embers)
    [204, 170, 102], // 14 TAN (coast sand)
    [221, 170, 119], // 15 PEACH
    [34, 34, 34],    // 16 DARKGRAY
    [85, 77, 68],    // 17 GRAY (dark stone)
    [136, 128, 111], // 18 GRAY2
    [170, 162, 136], // 19 GRAY3 (stone highlight)
    [204, 204, 187], // 20 SILVER
    [238, 238, 221], // 21 WHITE (off-white)
    [187, 153, 51],  // 22 GOLD (ochre)
    [238, 204, 119], // 23 YELLOW (pale gold)
    [136, 119, 34],  // 24 OLIVE (dark gold)
    [85, 153, 204],  // 25 SKY (cerulean)
    [17, 51, 68],    // 26 DEEPWATER
    [102, 136, 51],  // 27 GRASS (lowland)
    [119, 111, 102], // 28 STONE
    [221, 204, 153], // 29 CREAM (parchment)
    [187, 170, 119], // 30 LINEN (dark parchment)
    [238, 136, 34],  // 31 EMBER (single saturated accent)
];

pub const BLACK: u8 = 0;
pub const NAVY: u8 = 1;
pub const SAXON_BLUE: u8 = 2;
pub const LIGHTBLUE: u8 = 3;
pub const DARKGREEN: u8 = 4;
pub const GREEN: u8 = 5;
pub const TEAL: u8 = 6;
pub const CYAN: u8 = 7;
pub const DARKRED: u8 = 8;
pub const NORMAN_RED: u8 = 9;
pub const RED: u8 = 9;
pub const BLOOD: u8 = 10;
pub const PINK: u8 = 11;
pub const BROWN: u8 = 12;
pub const ORANGE: u8 = 13;
pub const TAN: u8 = 14;
pub const PEACH: u8 = 15;
pub const DARKGRAY: u8 = 16;
pub const GRAY: u8 = 17;
pub const GRAY2: u8 = 18;
pub const GRAY3: u8 = 19;
pub const SILVER: u8 = 20;
pub const WHITE: u8 = 21;
pub const GOLD: u8 = 22;
pub const YELLOW: u8 = 23;
pub const OLIVE: u8 = 24;
pub const SKY: u8 = 25;
pub const DEEPWATER: u8 = 26;
pub const GRASS: u8 = 27;
pub const STONE: u8 = 28;
pub const CREAM: u8 = 29;
pub const LINEN: u8 = 30;
pub const EMBER: u8 = 31;

#[inline]
pub fn rgb(idx: u8) -> [u8; 3] {
    PALETTE[idx as usize & 31]
}

/// Nearest palette index (squared euclidean; lowest index wins ties).
pub fn nearest(c: [u8; 3]) -> u8 {
    let mut best = 0usize;
    let mut bd = u32::MAX;
    for (i, p) in PALETTE.iter().enumerate() {
        let dr = p[0] as i32 - c[0] as i32;
        let dg = p[1] as i32 - c[1] as i32;
        let db = p[2] as i32 - c[2] as i32;
        let d = (dr * dr + dg * dg + db * db) as u32;
        if d < bd {
            bd = d;
            best = i;
        }
    }
    best as u8
}

/// Per-color half-brightness mapping (nearest to rgb/2), built once.
static HALF: LazyLock<[u8; 32]> = LazyLock::new(|| {
    let mut t = [0u8; 32];
    for (i, e) in t.iter_mut().enumerate() {
        let p = PALETTE[i];
        *e = nearest([p[0] / 2, p[1] / 2, p[2] / 2]);
    }
    t
});

#[inline]
pub fn half(idx: u8) -> u8 {
    HALF[idx as usize & 31]
}

/// Palette-accurate blend of two colors, `t` in 0..=8 (0 => a, 8 => b).
pub fn mix(a: u8, b: u8, t: u8) -> u8 {
    static MIX: LazyLock<[[u8; 9]; 32 * 32]> = LazyLock::new(|| {
        let mut table = [[0u8; 9]; 32 * 32];
        for (a, row) in table.iter_mut().enumerate() {
            let (ai, bi) = (a / 32, a % 32);
            let (pa, pb) = (rgb(ai as u8), rgb(bi as u8));
            for (k, cell) in row.iter_mut().enumerate() {
                let lerp = |x: u8, y: u8| -> u8 {
                    (x as i32 + (y as i32 - x as i32) * k as i32 / 8).clamp(0, 255) as u8
                };
                *cell = nearest([lerp(pa[0], pb[0]), lerp(pa[1], pb[1]), lerp(pa[2], pb[2])]);
            }
        }
        table
    });
    MIX[(a as usize & 31) * 32 + (b as usize & 31)][t.min(8) as usize]
}

/// 4x4 Bayer matrix, thresholds 0..15 (classic ordered pattern).
pub const BAYER4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

#[inline]
pub fn bayer(x: i32, y: i32) -> u8 {
    BAYER4[(y & 3) as usize][(x & 3) as usize]
}

/// Indexed 320x256 framebuffer; index 0 doubles as "transparent" later.
pub struct Framebuffer {
    pub data: Vec<u8>,
}

impl Default for Framebuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Framebuffer {
    pub fn new() -> Self {
        Self {
            data: vec![0; (W * H) as usize],
        }
    }

    pub fn clear(&mut self, c: u8) {
        self.data.iter_mut().for_each(|p| *p = c);
    }

    #[inline]
    pub fn get(&self, x: u32, y: u32) -> u8 {
        if x < W && y < H {
            self.data[(y * W + x) as usize]
        } else {
            0
        }
    }

    /// Bounds-checked pixel write (i32 coords; out-of-range silently clips).
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, c: u8) {
        if x >= 0 && y >= 0 && (x as u32) < W && (y as u32) < H {
            self.data[(y as u32 * W + x as u32) as usize] = c;
        }
    }
    /// Raster-offset screen shake (T903): shifts the rect by (dx, dy),
    /// filling pixels whose source fell outside the frame with BLACK.
    pub fn shake_viewport(&mut self, x0: u32, y0: u32, w: u32, h: u32, dx: i32, dy: i32) {
        if (dx == 0 && dy == 0) || w == 0 || h == 0 {
            return;
        }
        let x1 = (x0 + w).min(W);
        let y1 = (y0 + h).min(H);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let wv = (x1 - x0) as usize;
        let mut buf = vec![0u8; wv * (y1 - y0) as usize];
        for j in 0..(y1 - y0) as usize {
            let sy = y0 as i32 + j as i32 - dy;
            for i in 0..wv {
                let sx = x0 as i32 + i as i32 - dx;
                buf[j * wv + i] = if sx < 0 || sy < 0 || sx >= W as i32 || sy >= H as i32 {
                    BLACK
                } else {
                    self.data[(sy as u32 * W + sx as u32) as usize]
                };
            }
        }
        for j in 0..(y1 - y0) as usize {
            let d = ((y0 + j as u32) * W + x0) as usize;
            self.data[d..d + wv].copy_from_slice(&buf[j * wv..(j + 1) * wv]);
        }
    }

    pub fn hline(&mut self, x0: i32, x1: i32, y: i32, c: u8) {
        if y < 0 || y >= H as i32 {
            return;
        }
        let (a, b) = (x0.max(0).min(W as i32 - 1), x1.max(0).min(W as i32 - 1));
        if a > b {
            return;
        }
        for x in a..=b {
            self.set(x, y, c);
        }
    }

    pub fn vline(&mut self, x: i32, y0: i32, y1: i32, c: u8) {
        if x < 0 || x >= W as i32 {
            return;
        }
        let (a, b) = (y0.max(0).min(H as i32 - 1), y1.max(0).min(H as i32 - 1));
        if a > b {
            return;
        }
        for y in a..=b {
            self.set(x, y, c);
        }
    }

    /// Filled rect, i32 origin + u32 size, clipped to screen.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, c: u8) {
        for py in y..y + h as i32 {
            self.hline(x, x + w as i32 - 1, py, c);
        }
    }

    /// Rect outline.
    pub fn rect(&mut self, x: i32, y: i32, w: u32, h: u32, c: u8) {
        if w == 0 || h == 0 {
            return;
        }
        self.hline(x, x + w as i32 - 1, y, c);
        self.hline(x, x + w as i32 - 1, y + h as i32 - 1, c);
        self.vline(x, y, y + h as i32 - 1, c);
        self.vline(x + w as i32 - 1, y, y + h as i32 - 1, c);
    }

    /// Dithered fill: of every 16 Bayer levels, `t` land on `a`, rest `b`;
    /// deterministic by absolute coords. `t=0` => all b, `t=16` => all a.
    #[allow(clippy::too_many_arguments)] // rect + 3 dither params; matches rect()/fill() style
    pub fn fill_rect_dither(&mut self, x: i32, y: i32, w: u32, h: u32, a: u8, b: u8, t: u8) {
        for py in y..y + h as i32 {
            for px in x..x + w as i32 {
                let c = if t > bayer(px, py) { a } else { b };
                self.set(px, py, c);
            }
        }
    }

    /// Filled triangle via edge functions (any winding).
    pub fn tri(&mut self, p: [(i32, i32); 3], c: u8) {
        let minx = p.iter().map(|q| q.0).min().unwrap().max(0);
        let maxx = p.iter().map(|q| q.0).max().unwrap().min(W as i32 - 1);
        let miny = p.iter().map(|q| q.1).min().unwrap().max(0);
        let maxy = p.iter().map(|q| q.1).max().unwrap().min(H as i32 - 1);
        let edge = |a: (i32, i32), b: (i32, i32), q: (i32, i32)| -> i64 {
            (b.0 - a.0) as i64 * (q.1 - a.1) as i64 - (b.1 - a.1) as i64 * (q.0 - a.0) as i64
        };
        let s = [
            edge(p[0], p[1], p[2]).signum(),
            edge(p[1], p[2], p[0]).signum(),
            edge(p[2], p[0], p[1]).signum(),
        ];
        for y in miny..=maxy {
            for x in minx..=maxx {
                let e = [
                    edge(p[0], p[1], (x, y)),
                    edge(p[1], p[2], (x, y)),
                    edge(p[2], p[0], (x, y)),
                ];
                if e[0].signum() * s[0] >= 0
                    && e[1].signum() * s[1] >= 0
                    && e[2].signum() * s[2] >= 0
                {
                    self.set(x, y, c);
                }
            }
        }
    }

    /// Filled circle (midpoint algorithm, scanline spans).
    pub fn fill_circle(&mut self, cx: i32, cy: i32, r: u32, c: u8) {
        let r = r as i32;
        let mut x = r;
        let mut y = 0i32;
        let mut err = 1 - r;
        while x >= y {
            self.hline(cx - x, cx + x, cy + y, c);
            self.hline(cx - x, cx + x, cy - y, c);
            self.hline(cx - y, cx + y, cy + x, c);
            self.hline(cx - y, cx + y, cy - x, c);
            y += 1;
            if err < 0 {
                err += 2 * y + 1;
            } else {
                x -= 1;
                err += 2 * (y - x) + 1;
            }
        }
    }

    /// Vertical gradient a->b, 8 mix levels with Bayer smoothing between.
    pub fn gradient_v(&mut self, x: i32, y: i32, w: u32, h: u32, a: u8, b: u8) {
        for py in y..y + h as i32 {
            let lvl = (((py - y) as u32 * 8) / h.max(1)).min(8) as u8;
            let res = (((py - y) as u32 * 16) / h.max(1) % 16) as u8;
            let cur = mix(a, b, lvl);
            let nxt = mix(a, b, (lvl + 1).min(8));
            for px in x..x + w as i32 {
                let c = if res > bayer(px, py) { nxt } else { cur };
                self.set(px, py, c);
            }
        }
    }

    /// Convert to RGBA bytes for ImageData (len must be W*H*4).
    pub fn to_rgba(&self, out: &mut [u8]) {
        debug_assert_eq!(out.len(), (W * H * 4) as usize);
        for (p, px) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.data.iter()) {
            let c = rgb(*px);
            p[0] = c[0];
            p[1] = c[1];
            p[2] = c[2];
            p[3] = 255;
        }
    }

    pub fn count(&self, c: u8) -> u32 {
        self.data.iter().filter(|&&p| p == c).count() as u32
    }
}
