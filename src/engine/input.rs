//! T106: unified input — raw event queue + per-frame edge queries +
//! tap/hold/swipe gesture recognition. Pure logic: the web adapter feeds
//! raw DOM events in, scenes poll gestures per frame. Frame numbers come
//! from the caller so tests stay deterministic.

use std::collections::VecDeque;

/// One raw DOM-level event; coordinates already mapped to internal 320x256.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Raw {
    Down {
        x: i32,
        y: i32,
    },
    Move {
        x: i32,
        y: i32,
    },
    Up {
        x: i32,
        y: i32,
    },
    /// Key state change; `code` = codepoint when printable, else KEY_*.
    Key {
        code: u32,
        pressed: bool,
    },
    Wheel {
        dy: i32,
    },
}

// Synthetic codes for non-printable keys (extend as needed).
pub const KEY_ENTER: u32 = 0x1100;
pub const KEY_ESC: u32 = 0x1101;
pub const KEY_SPACE: u32 = 0x1102;
pub const KEY_LEFT: u32 = 0x1103;
pub const KEY_UP: u32 = 0x1104;
pub const KEY_RIGHT: u32 = 0x1105;
pub const KEY_DOWN: u32 = 0x1106;
pub const KEY_M: u32 = 0x1107;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

/// Recognized pointer gestures, produced once on release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    /// Quick press/release within `TAP_SLOP` px; origin coords.
    Tap { x: i32, y: i32, frame: u64 },
    /// Released after moving beyond slop (drag ended); start -> end.
    DragEnd { x0: i32, y0: i32, x1: i32, y1: i32 },
    /// Fast directional release; 8-way bucket of the delta.
    Swipe { dir: Dir },
}

/// Max stroke drift still counted as a tap (Chebyshev px).
pub const TAP_SLOP: i32 = 6;
/// Max Down->Up frames for a Tap (~0.5 s at 60 fps).
pub const TAP_FRAMES: u64 = 30;
/// Min travel for a swipe.
pub const SWIPE_MIN: i32 = 40;
/// Min average px/frame travel for a swipe.
pub const SWIPE_SPEED: i32 = 12;

#[derive(Clone, Copy, Default)]
struct Keys {
    down: bool,
    pressed: bool,  // down edge this frame
    released: bool, // up edge this frame
}

const KEY_SLOTS: usize = 512;

pub struct Input {
    queue: VecDeque<Raw>,
    keys: Vec<Keys>,
    key_code: Vec<u32>, // stored as code+1, 0 = empty slot
    mx: i32,
    my: i32,
    pmx: i32,
    pmy: i32,
    down: bool,
    down_edge: bool,
    up_edge: bool,
    stroke: Option<(i32, i32, u64)>, // origin x, y, Down frame
    stroke_max: i32,
    wheel: i32,
    gestures: Vec<Gesture>,
    key_ev: Vec<(u32, bool)>,
    text: String,
    frame: u64,
    /// First pointer-down/key this session — the audio-unlock flag.
    pub user_gesture: bool,
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Input {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            keys: vec![Keys::default(); KEY_SLOTS],
            key_code: vec![0; KEY_SLOTS],
            mx: -1,
            my: -1,
            pmx: -1,
            pmy: -1,
            down: false,
            down_edge: false,
            up_edge: false,
            stroke: None,
            stroke_max: 0,
            wheel: 0,
            gestures: Vec::new(),
            key_ev: Vec::new(),
            text: String::new(),
            frame: 0,
            user_gesture: false,
        }
    }

    /// Adapter feed point.
    pub fn push(&mut self, ev: Raw) {
        self.queue.push_back(ev);
    }

    /// Called once per frame by the scene driver *before* scenes poll:
    /// drains all queued raw events as "this frame", resets edges/wheel/text.
    pub fn begin_frame(&mut self, frame: u64) {
        self.frame = frame;
        self.pmx = self.mx;
        self.pmy = self.my;
        for k in self.keys.iter_mut() {
            k.pressed = false;
            k.released = false;
        }
        self.down_edge = false;
        self.up_edge = false;
        self.wheel = 0;
        self.gestures.clear();
        self.key_ev.clear();
        while let Some(ev) = self.queue.pop_front() {
            self.apply(ev, frame);
        }
    }

    fn apply(&mut self, ev: Raw, frame: u64) {
        match ev {
            Raw::Down { x, y } => {
                self.user_gesture = true;
                self.mx = x;
                self.my = y;
                if !self.down {
                    self.down_edge = true;
                }
                self.down = true;
                self.stroke = Some((x, y, frame));
                self.stroke_max = 0;
            }
            Raw::Move { x, y } => {
                self.mx = x;
                self.my = y;
                if let Some((ox, oy, _)) = self.stroke {
                    let d = (x - ox).abs().max((y - oy).abs());
                    self.stroke_max = self.stroke_max.max(d);
                }
            }
            Raw::Up { x, y } => {
                self.mx = x;
                self.my = y;
                if self.down {
                    self.up_edge = true;
                }
                self.down = false;
                if let Some((ox, oy, f0)) = self.stroke.take() {
                    let dx = x - ox;
                    let dy = y - oy;
                    let travel = dx.abs().max(dy.abs());
                    let frames = frame.saturating_sub(f0).max(1);
                    let speed = (travel as u64 / frames).max(1) as i32;
                    if travel <= TAP_SLOP && frames <= TAP_FRAMES {
                        self.gestures.push(Gesture::Tap {
                            x: ox,
                            y: oy,
                            frame: f0,
                        });
                    } else if travel >= SWIPE_MIN && speed >= SWIPE_SPEED {
                        self.gestures.push(Gesture::Swipe {
                            dir: bucket(dx, dy),
                        });
                    } else {
                        self.gestures.push(Gesture::DragEnd {
                            x0: ox,
                            y0: oy,
                            x1: x,
                            y1: y,
                        });
                    }
                }
            }
            Raw::Key { code, pressed } => {
                if pressed {
                    self.user_gesture = true;
                }
                let slot = self.slot(code);
                let k = &mut self.keys[slot];
                if pressed && !k.down {
                    k.pressed = true;
                }
                if !pressed && k.down {
                    k.released = true;
                }
                k.down = pressed;
                self.key_ev.push((code, pressed));
                if pressed {
                    if let Some(ch) = char_for(code) {
                        self.text.push(ch);
                    }
                }
            }
            Raw::Wheel { dy } => self.wheel += dy,
        }
    }

    fn slot(&mut self, code: u32) -> usize {
        let s = (code as usize) % KEY_SLOTS;
        for i in 0..8 {
            let t = (s + i) % KEY_SLOTS;
            if self.key_code[t] == 0 {
                self.key_code[t] = code + 1;
                return t;
            }
            if self.key_code[t] == code + 1 {
                return t;
            }
        }
        s
    }

    fn lookup(&self, code: u32) -> usize {
        let s = (code as usize) % KEY_SLOTS;
        for i in 0..8 {
            let t = (s + i) % KEY_SLOTS;
            if self.key_code[t] == 0 {
                return s;
            }
            if self.key_code[t] == code + 1 {
                return t;
            }
        }
        s
    }

    // ---- per-frame queries ----

    /// Key edges this frame (down/up), for scene routing.
    pub fn key_events(&self) -> &[(u32, bool)] {
        &self.key_ev
    }
    pub fn mouse(&self) -> (i32, i32, bool) {
        (self.mx, self.my, self.down)
    }
    /// Mouse position at the start of this frame (hover deltas).
    pub fn prev_mouse(&self) -> (i32, i32) {
        (self.pmx, self.pmy)
    }
    /// Button went down during this frame's drain.
    pub fn pressed(&self) -> bool {
        self.down_edge
    }
    /// Button came up during this frame's drain.
    pub fn released(&self) -> bool {
        self.up_edge
    }
    /// Active stroke origin while held.
    pub fn dragging(&self) -> Option<(i32, i32)> {
        self.stroke.map(|(x, y, _)| (x, y))
    }
    /// Max stroke drift so far (<= TAP_SLOP while still a tap candidate).
    pub fn drag_travel(&self) -> i32 {
        self.stroke_max
    }
    pub fn gestures(&self) -> &[Gesture] {
        &self.gestures
    }
    pub fn wheel(&self) -> i32 {
        self.wheel
    }
    pub fn key_down(&self, code: u32) -> bool {
        self.keys[self.lookup(code)].down
    }
    pub fn key_pressed(&self, code: u32) -> bool {
        self.keys[self.lookup(code)].pressed
    }
    pub fn key_released(&self, code: u32) -> bool {
        self.keys[self.lookup(code)].released
    }
    /// Printable characters typed since the last `begin_frame`.
    pub fn take_text(&mut self) -> String {
        std::mem::take(&mut self.text)
    }
    pub fn frame(&self) -> u64 {
        self.frame
    }
}

/// 8-way bucket of a delta; screen y grows downward, so N = negative dy.
pub fn bucket(dx: i32, dy: i32) -> Dir {
    let adx = dx.abs();
    let ady = dy.abs();
    let (hi, lo) = if adx >= ady { (adx, ady) } else { (ady, adx) };
    // diagonal if off-axis ratio > tan(22.5 deg) ~ 99/239
    let diag = (lo as u32 * 239) > (hi as u32 * 99);
    let h = adx.cmp(&ady);
    if h == std::cmp::Ordering::Equal {
        // pure diagonal (or zero -> treat as E)
        return if dx >= 0 && dy > 0 {
            Dir::SE
        } else if dx > 0 {
            Dir::NE
        } else if dy != 0 {
            if dy > 0 {
                Dir::SW
            } else {
                Dir::NW
            }
        } else {
            Dir::E
        };
    }
    if h == std::cmp::Ordering::Greater {
        // horizontal dominant
        if diag {
            match (dx > 0, dy > 0) {
                (true, true) => Dir::SE,
                (true, false) => Dir::NE,
                (false, true) => Dir::SW,
                (false, false) => Dir::NW,
            }
        } else if dx > 0 {
            Dir::E
        } else {
            Dir::W
        }
    } else if diag {
        match (dx > 0, dy > 0) {
            (true, true) => Dir::SE,
            (true, false) => Dir::NE,
            (false, true) => Dir::SW,
            (false, false) => Dir::NW,
        }
    } else if dy > 0 {
        Dir::S
    } else {
        Dir::N
    }
}

fn char_for(code: u32) -> Option<char> {
    if (0x21..0x7f).contains(&code) {
        char::from_u32(code)
    } else if code == KEY_SPACE {
        Some(' ')
    } else {
        None
    }
}
