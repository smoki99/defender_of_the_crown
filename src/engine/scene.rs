//! T107: scene stack + retro transitions. Pure logic — the web adapter
//! calls `step()` once per fixed logic tick and `paint()` for the final
//! frame; scenes implement init/event/tick/draw.
//!
//! Transition model (Bayer-threshold composites, no floats):
//! - `Fade`: 16 frames old->black, swap at midpoint, 16 frames black->new.
//! - `Dissolve`: swap immediately, 16 frames old pixels -> new pixels.
//!   During a transition the incoming scene gets no events/ticks; it receives
//!   `SceneEvent::Revealed` (+ `init`) when fully visible.

use crate::engine::gfx::{bayer, Framebuffer, BLACK, W};
use crate::engine::input::{Dir, Gesture, Input};

/// Minimal shared services handed to a scene on `init`.
pub trait AppCtx {
    fn mute(&self) -> bool;
    fn play_music(&mut self, track: Option<&'static str>);
    fn sfx(&mut self, kind: crate::engine::audio::Sfx);
    /// One-shot sting from a .MOD payload (T308); default: silence.
    fn sting(&mut self, track: &'static [u8]) {
        let _ = track;
    }
}

/// Semantic events scenes receive (raw input already transformed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneEvent {
    Move {
        x: i32,
        y: i32,
    },
    DragStart {
        x: i32,
        y: i32,
    },
    DragMove {
        x0: i32,
        y0: i32,
        x: i32,
        y: i32,
    },
    Tap {
        x: i32,
        y: i32,
    },
    DragEnd {
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
    },
    Swipe {
        dir: Dir,
    },
    Key {
        code: u32,
        pressed: bool,
    },
    Wheel {
        dy: i32,
    },
    /// Transition finished; scene is now fully visible and interactive.
    Revealed,
}

pub trait Scene {
    /// Once, after the scene became fully visible.
    fn init(&mut self, _app: &mut dyn AppCtx) {}
    fn event(&mut self, _ev: &SceneEvent) {}
    /// Fixed-step logic tick (60 Hz), only when no transition runs.
    /// `app` lets a tick fire audio (e.g. the Norman turn-call, T308).
    fn tick(&mut self, _app: &mut dyn AppCtx) {}
    fn draw(&mut self, fb: &mut Framebuffer);
    /// Driver asks each frame; the app decides what a leave means.
    fn wants_leave(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    None,
    Fade,
    Dissolve,
}

pub const TRANS_FRAMES: u8 = 16;

enum Change {
    Push(Box<dyn Scene>),
    Replace(Box<dyn Scene>),
    Pop,
    Nop,
}

struct Running {
    change: Change,
    kind: Transition,
    /// Captured render of the outgoing frame (the base layer).
    old: Framebuffer,
    t: u8,
    /// Fade is running its second (black->new) half.
    in_half: bool,
}

pub struct SceneStack {
    stack: Vec<Box<dyn Scene>>,
    running: Option<Running>,
    /// Top scene awaits `init` on the next step (after reveal).
    pending_init: bool,
}

impl SceneStack {
    pub fn new(first: Box<dyn Scene>) -> Self {
        Self {
            stack: vec![first],
            running: None,
            pending_init: true,
        }
    }

    pub fn depth(&self) -> usize {
        self.stack.len()
    }
    pub fn in_transition(&self) -> bool {
        self.running.is_some()
    }
    pub fn top_wants_leave(&self) -> bool {
        self.stack.last().is_some_and(|s| s.wants_leave())
    }

    pub fn replace(&mut self, s: Box<dyn Scene>, tr: Transition) {
        self.start(Change::Replace(s), tr);
    }
    pub fn push(&mut self, s: Box<dyn Scene>, tr: Transition) {
        self.start(Change::Push(s), tr);
    }
    pub fn pop(&mut self, tr: Transition) -> bool {
        if self.stack.len() < 2 {
            return false;
        }
        self.start(Change::Pop, tr);
        true
    }

    /// Ignored while a transition runs (a scene cannot leave twice).
    fn start(&mut self, change: Change, tr: Transition) {
        if self.in_transition() {
            return;
        }
        match tr {
            Transition::None => {
                apply(&mut self.stack, change);
                self.pending_init = true;
            }
            kind => {
                let mut old = Framebuffer::new();
                self.stack
                    .last_mut()
                    .expect("scene stack empty")
                    .draw(&mut old);
                // Dissolve swaps immediately and cross-fades forward;
                // Fade keeps the old scene until the black midpoint.
                let change = if kind == Transition::Dissolve {
                    apply(&mut self.stack, change);
                    Change::Nop
                } else {
                    change
                };
                self.running = Some(Running {
                    change,
                    kind,
                    old,
                    t: 0,
                    in_half: false,
                });
            }
        }
    }

    fn finish(&mut self) {
        self.running = None;
        self.pending_init = true;
    }

    /// Drain input, route to top scene, tick, advance any transition.
    /// `frame` numbers come from the driver (deterministic in tests).
    pub fn step(&mut self, app: &mut dyn AppCtx, input: &mut Input, frame: u64) {
        input.begin_frame(frame);
        if self.in_transition() {
            self.advance();
            return;
        }
        if self.pending_init {
            self.pending_init = false;
            if let Some(top) = self.stack.last_mut() {
                top.event(&SceneEvent::Revealed);
                top.init(app);
            }
        }
        let evs = collect_events(input);
        if let Some(top) = self.stack.last_mut() {
            for e in &evs {
                top.event(e);
            }
            top.tick(app);
        }
    }

    fn advance(&mut self) {
        let r = self.running.as_mut().expect("advance without transition");
        r.t += 1;
        match r.kind {
            Transition::Fade => {
                if !r.in_half && r.t >= TRANS_FRAMES {
                    let change = std::mem::replace(&mut r.change, Change::Nop);
                    apply(&mut self.stack, change);
                    r.in_half = true;
                    r.t = 0;
                } else if r.in_half && r.t >= TRANS_FRAMES {
                    self.finish();
                }
            }
            Transition::Dissolve => {
                if r.t >= TRANS_FRAMES {
                    self.finish();
                }
            }
            Transition::None => unreachable!(),
        }
    }

    /// Composite into `fb`; `scratch` is per-frame scratch (kept by caller).
    pub fn paint(&mut self, fb: &mut Framebuffer, scratch: &mut Framebuffer) {
        let Self { stack, running, .. } = self;
        let live = stack.last_mut().expect("scene stack empty");
        let r = match running.as_mut() {
            None => {
                live.draw(fb);
                return;
            }
            Some(r) => r,
        };
        let Running {
            kind,
            old,
            t,
            in_half,
            ..
        } = r;
        let t = *t;
        let in_half = *in_half;
        live.draw(scratch);
        match kind {
            Transition::Fade => {
                if in_half {
                    // black -> new: new shows through as bayer rises under t
                    for (i, p) in fb.data.iter_mut().enumerate() {
                        let (x, y) = ((i as u32 % W) as i32, (i as u32 / W) as i32);
                        *p = if bayer(x, y) < t {
                            scratch.data[i]
                        } else {
                            BLACK
                        };
                    }
                } else {
                    // old -> black
                    for (i, p) in fb.data.iter_mut().enumerate() {
                        let (x, y) = ((i as u32 % W) as i32, (i as u32 / W) as i32);
                        *p = if bayer(x, y) < t { BLACK } else { old.data[i] };
                    }
                }
            }
            Transition::Dissolve => {
                // old -> new
                for (i, p) in fb.data.iter_mut().enumerate() {
                    let (x, y) = ((i as u32 % W) as i32, (i as u32 / W) as i32);
                    *p = if bayer(x, y) < t {
                        scratch.data[i]
                    } else {
                        old.data[i]
                    };
                }
            }
            Transition::None => unreachable!(),
        }
    }
}

fn apply(stack: &mut Vec<Box<dyn Scene>>, ch: Change) {
    match ch {
        Change::Push(s) => stack.push(s),
        Change::Replace(s) => *stack.last_mut().expect("scene stack empty") = s,
        Change::Pop => {
            stack.pop();
        }
        Change::Nop => {}
    }
}

/// Transform per-frame input queries into scene events.
pub fn collect_events(input: &mut Input) -> Vec<SceneEvent> {
    let mut evs = Vec::new();
    let (mx, my, _) = input.mouse();
    if let Some((x0, y0)) = input.dragging() {
        if input.pressed() {
            evs.push(SceneEvent::DragStart { x: mx, y: my });
        } else {
            evs.push(SceneEvent::DragMove {
                x0,
                y0,
                x: mx,
                y: my,
            });
        }
    } else if input.prev_mouse() != (mx, my) && mx >= 0 {
        evs.push(SceneEvent::Move { x: mx, y: my });
    }
    for g in input.gestures() {
        evs.push(match *g {
            Gesture::Tap { x, y, .. } => SceneEvent::Tap { x, y },
            Gesture::DragEnd { x0, y0, x1, y1 } => SceneEvent::DragEnd { x0, y0, x1, y1 },
            Gesture::Swipe { dir } => SceneEvent::Swipe { dir },
        });
    }
    for &(code, pressed) in input.key_events() {
        evs.push(SceneEvent::Key { code, pressed });
    }
    let w = input.wheel();
    if w != 0 {
        evs.push(SceneEvent::Wheel { dy: w });
    }
    evs
}
