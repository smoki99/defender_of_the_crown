//! T107 proofs: transition state machine, event routing, composites.
use std::cell::RefCell;
use std::rc::Rc;

use defender_of_the_crown::engine::audio::Sfx;
use defender_of_the_crown::engine::gfx::{bayer, Framebuffer, BLACK, BLOOD, WHITE, W};
use defender_of_the_crown::engine::input::{Input, Raw, KEY_ENTER};
use defender_of_the_crown::engine::scene::{
    collect_events, AppCtx, Scene, SceneEvent, SceneStack, Transition, TRANS_FRAMES,
};

const PX: u32 = W * 256;

struct Ctx;
impl AppCtx for Ctx {
    fn mute(&self) -> bool {
        true
    }
    fn play_music(&mut self, _t: Option<&'static str>) {}
    fn sfx(&mut self, _kind: Sfx) {}
}

#[derive(Default)]
struct Log {
    events: Vec<SceneEvent>,
    ticks: u32,
    inits: u32,
}

/// Solid-color scene reporting into a shared log.
struct S {
    color: u8,
    log: Rc<RefCell<Log>>,
    leave: bool,
}
impl S {
    fn new(color: u8) -> (Self, Rc<RefCell<Log>>) {
        let log = Rc::new(RefCell::new(Log::default()));
        (
            Self {
                color,
                log: log.clone(),
                leave: false,
            },
            log,
        )
    }
}
impl Scene for S {
    fn init(&mut self, _app: &mut dyn AppCtx) {
        self.log.borrow_mut().inits += 1;
    }
    fn event(&mut self, ev: &SceneEvent) {
        self.log.borrow_mut().events.push(*ev);
    }
    fn tick(&mut self) {
        self.log.borrow_mut().ticks += 1;
    }
    fn draw(&mut self, fb: &mut Framebuffer) {
        fb.clear(self.color);
    }
    fn wants_leave(&self) -> bool {
        self.leave
    }
}

#[test]
fn first_scene_revealed_and_inited_on_first_step() {
    let (s, log) = S::new(WHITE);
    let mut st = SceneStack::new(Box::new(s));
    let mut ctx = Ctx;
    let mut inp = Input::new();
    st.step(&mut ctx, &mut inp, 1);
    let l = log.borrow();
    assert!(l.events.contains(&SceneEvent::Revealed));
    assert_eq!(l.inits, 1);
    assert_eq!(l.ticks, 1);
}

#[test]
fn replace_none_reveals_on_next_step() {
    let (_, l0) = S::new(WHITE);
    let mut st = SceneStack::new(Box::new(S {
        color: WHITE,
        log: l0,
        leave: false,
    }));
    let mut ctx = Ctx;
    let mut inp = Input::new();
    let (b, lb) = S::new(BLACK);
    st.replace(Box::new(b), Transition::None);
    assert!(!st.in_transition());
    st.step(&mut ctx, &mut inp, 1);
    let l = lb.borrow();
    assert!(l.events.contains(&SceneEvent::Revealed));
    assert_eq!(l.inits, 1);
}

#[test]
fn fade_holds_events_swaps_at_midpoint_and_reveals() {
    let (a, la) = S::new(WHITE);
    let mut st = SceneStack::new(Box::new(a));
    let mut ctx = Ctx;
    let mut inp = Input::new();
    let (b, lb) = S::new(BLACK);
    st.replace(Box::new(b), Transition::Fade);
    assert!(st.in_transition());
    // tap while fading - must reach nobody
    inp.push(Raw::Down { x: 5, y: 5 });
    inp.push(Raw::Up { x: 5, y: 5 });
    let total = 2 * TRANS_FRAMES as u64; // midpoint + in-half
    for f in 0..total {
        st.step(&mut ctx, &mut inp, f);
    }
    assert!(!st.in_transition(), "fade must end after 2*16 steps");
    st.step(&mut ctx, &mut inp, total);
    {
        let l = la.borrow();
        assert!(
            !l.events.iter().any(|e| matches!(e, SceneEvent::Tap { .. })),
            "old scene got tap during fade"
        );
        assert_eq!(l.ticks, 0, "old scene never ticks during fade");
    }
    {
        let l = lb.borrow();
        assert!(l.events.contains(&SceneEvent::Revealed));
        assert_eq!(l.inits, 1);
        assert!(
            !l.events.iter().any(|e| matches!(e, SceneEvent::Tap { .. })),
            "new scene got tap during fade: {:?}",
            l.events
        );
        assert_eq!(l.ticks, 1, "one tick after reveal only");
    }
}

#[test]
fn fade_paint_old_black_new() {
    let (a, _la) = S::new(WHITE);
    let mut st = SceneStack::new(Box::new(a));
    let mut ctx = Ctx;
    let mut inp = Input::new();
    let (b, _lb) = S::new(BLOOD);
    let mut fb = Framebuffer::new();
    let mut sc = Framebuffer::new();
    st.step(&mut ctx, &mut inp, 0); // settle first scene
    st.replace(Box::new(b), Transition::Fade);
    st.paint(&mut fb, &mut sc);
    assert_eq!(fb.count(WHITE), PX, "t=0 full old");
    for f in 1..=8 {
        st.step(&mut ctx, &mut inp, f);
    }
    st.paint(&mut fb, &mut sc);
    let black = fb.count(BLACK);
    assert!(black > 0 && black < PX, "partial out-fade {black}");
    for f in 9..=16 {
        st.step(&mut ctx, &mut inp, f);
    }
    st.paint(&mut fb, &mut sc);
    assert_eq!(fb.count(WHITE), 0, "old gone at midpoint");
    assert_eq!(fb.count(BLACK), PX, "black midpoint frame");
    for f in 17..=24 {
        st.step(&mut ctx, &mut inp, f);
    }
    st.paint(&mut fb, &mut sc);
    let blood = fb.count(BLOOD);
    assert!(blood > 0 && blood < PX, "partial in-fade {blood}");
    for f in 25..=32 {
        st.step(&mut ctx, &mut inp, f);
    }
    assert!(!st.in_transition());
    st.paint(&mut fb, &mut sc);
    assert_eq!(fb.count(BLOOD), PX, "fully new scene");
}

#[test]
fn dissolve_swaps_immediately_completes_in_16() {
    let (a, _la) = S::new(WHITE);
    let mut st = SceneStack::new(Box::new(a));
    let mut ctx = Ctx;
    let mut inp = Input::new();
    let (b, lb) = S::new(BLACK);
    let mut fb = Framebuffer::new();
    let mut sc = Framebuffer::new();
    st.step(&mut ctx, &mut inp, 0);
    st.replace(Box::new(b), Transition::Dissolve);
    st.paint(&mut fb, &mut sc);
    assert_eq!(fb.count(WHITE), PX, "starts from old pixels");
    for f in 1..=TRANS_FRAMES as u64 {
        st.step(&mut ctx, &mut inp, f);
    }
    assert!(!st.in_transition(), "16 steps complete dissolve");
    st.step(&mut ctx, &mut inp, 98);
    {
        let l = lb.borrow();
        assert!(l.events.contains(&SceneEvent::Revealed));
    }
    st.step(&mut ctx, &mut inp, 99);
    st.paint(&mut fb, &mut sc);
    assert_eq!(fb.count(BLACK), PX);
}

#[test]
fn dissolve_threshold_matches_bayer() {
    let (a, _la) = S::new(WHITE);
    let mut st = SceneStack::new(Box::new(a));
    let mut ctx = Ctx;
    let mut inp = Input::new();
    let (b, _lb) = S::new(BLACK);
    let mut fb = Framebuffer::new();
    let mut sc = Framebuffer::new();
    st.step(&mut ctx, &mut inp, 0);
    st.replace(Box::new(b), Transition::Dissolve);
    for f in 1..9 {
        st.step(&mut ctx, &mut inp, f);
    }
    st.paint(&mut fb, &mut sc);
    for (x, y) in [(0u32, 0u32), (1, 2), (3, 3), (17, 91), (319, 255)] {
        let want = if bayer(x as i32, y as i32) < 8 {
            BLACK
        } else {
            WHITE
        };
        assert_eq!(fb.get(x, y), want, "px {x},{y}");
    }
}

#[test]
fn push_pop_depth() {
    let (a, _la) = S::new(WHITE);
    let mut st = SceneStack::new(Box::new(a));
    let mut ctx = Ctx;
    let mut inp = Input::new();
    let (b, _lb) = S::new(BLACK);
    st.push(Box::new(b), Transition::Dissolve);
    assert_eq!(st.depth(), 2, "push applied at transition start");
    for f in 0..TRANS_FRAMES as u64 {
        st.step(&mut ctx, &mut inp, f);
    }
    assert_eq!(st.depth(), 2);
    assert!(st.pop(Transition::None));
    assert_eq!(st.depth(), 1);
    assert!(!st.pop(Transition::None), "cannot pop last");
}

#[test]
fn collect_events_maps_all_channels() {
    let mut inp = Input::new();
    inp.push(Raw::Down { x: 10, y: 12 });
    inp.begin_frame(1);
    let evs = collect_events(&mut inp);
    assert!(
        evs.contains(&SceneEvent::DragStart { x: 10, y: 12 }),
        "{evs:?}"
    );
    inp.push(Raw::Move { x: 25, y: 35 });
    inp.begin_frame(2);
    let evs = collect_events(&mut inp);
    assert!(
        evs.contains(&SceneEvent::DragMove {
            x0: 10,
            y0: 12,
            x: 25,
            y: 35
        }),
        "{evs:?}"
    );
    inp.push(Raw::Up { x: 25, y: 35 });
    inp.begin_frame(3);
    let evs = collect_events(&mut inp);
    assert!(
        evs.contains(&SceneEvent::DragEnd {
            x0: 10,
            y0: 12,
            x1: 25,
            y1: 35
        }),
        "{evs:?}"
    );
    inp.push(Raw::Key {
        code: KEY_ENTER,
        pressed: true,
    });
    inp.push(Raw::Key {
        code: KEY_ENTER,
        pressed: false,
    });
    inp.push(Raw::Wheel { dy: -3 });
    inp.begin_frame(4);
    let evs = collect_events(&mut inp);
    assert!(evs.contains(&SceneEvent::Key {
        code: KEY_ENTER,
        pressed: true
    }));
    assert!(evs.contains(&SceneEvent::Key {
        code: KEY_ENTER,
        pressed: false
    }));
    assert!(evs.contains(&SceneEvent::Wheel { dy: -3 }));
}

#[test]
fn wants_leave_exposed() {
    let (a, _la) = S::new(WHITE);
    let mut st = SceneStack::new(Box::new(a));
    let mut ctx = Ctx;
    let mut inp = Input::new();
    let (mut b, _lb) = S::new(BLACK);
    b.leave = true;
    st.replace(Box::new(b), Transition::None);
    st.step(&mut ctx, &mut inp, 1);
    assert!(st.top_wants_leave());
}

#[test]
fn replace_while_transition_running_does_not_restart_it() {
    // A scene that never stops wanting leave makes a naive driver call
    // replace() every frame; that must not stall the Fade forever.
    let (mut a, _la) = S::new(WHITE);
    a.leave = true;
    let mut st = SceneStack::new(Box::new(a));
    let mut ctx = Ctx;
    let mut inp = Input::new();
    st.step(&mut ctx, &mut inp, 1);
    for f in 2..=2 * TRANS_FRAMES as u64 + 1 {
        assert!(st.top_wants_leave());
        let (mut spam, _l) = S::new(BLACK);
        spam.leave = true;
        st.replace(Box::new(spam), Transition::Fade);
        st.step(&mut ctx, &mut inp, f);
    }
    assert!(
        !st.in_transition(),
        "fade must complete despite repeated replace calls"
    );

    // and a fresh replace after completion still runs
    let (b, lb) = S::new(BLACK);
    st.replace(Box::new(b), Transition::Fade);
    assert!(st.in_transition());
    for f in 0..=2 * TRANS_FRAMES as u64 {
        st.step(&mut ctx, &mut inp, 100 + f);
    }
    assert!(!st.in_transition());
    let l = lb.borrow();
    assert_eq!(l.inits, 1);
    assert_eq!(l.events.as_slice(), &[SceneEvent::Revealed]);
}
