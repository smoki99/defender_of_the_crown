//! T107: browser adapter — canvas blit, RAF fixed-step driver, DOM input,
//! WebAudio output. `wasm32` only; host builds skip this file entirely.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::Clamped;

use crate::engine::audio::{Mod, Player};
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{
    Framebuffer, BLACK, BLOOD, DARKGRAY, GOLD, GRAY, GREEN, LIGHTBLUE, NAVY, SAXON_BLUE, TAN,
    WHITE, YELLOW, H, W,
};
use crate::engine::input::{Input, Raw, KEY_DOWN, KEY_ENTER, KEY_ESC, KEY_LEFT, KEY_RIGHT, KEY_SPACE, KEY_UP};
use crate::engine::scene::{AppCtx, Scene, SceneEvent, SceneStack, Transition};
use crate::engine::sprite::{blit_anim, Sprite, TRANS};

fn w() -> web_sys::Window {
    web_sys::window().expect("no window")
}

fn query_has(kv: &str) -> bool {
    w().location().search().unwrap_or_default().contains(kv)
}

fn query_get(key: &str) -> Option<String> {
    w().location()
        .search()
        .unwrap_or_default()
        .trim_start_matches('?')
        .split('&')
        .find_map(|p| p.strip_prefix(key).and_then(|v| v.strip_prefix('=')))
        .map(str::to_string)
}

// ---------------------------------------------------------------- driver state

enum Mode {
    Title,
    Test,
}

struct App {
    fb: Framebuffer,
    scratch: Framebuffer,
    input: Input,
    stack: SceneStack,
    mode: Mode,
    frame: u64,
    rgba: Vec<u8>,
    canvas: web_sys::HtmlCanvasElement,
    ctx2d: web_sys::CanvasRenderingContext2d,
    last: f64,
    acc: f64,
    fps_seen: u32,
    fps_at: f64,
    shot_at: u64,
    shot_done: bool,
}

struct Ctx {
    muted: Rc<Cell<bool>>,
    audio: AudioState,
}
impl AppCtx for Ctx {
    fn mute(&self) -> bool {
        self.muted.get()
    }
    fn play_music(&mut self, _track: Option<&'static str>) {}
}

struct AudioState {
    ctx: Option<web_sys::AudioContext>,
    player: Rc<RefCell<Option<Player>>>,
}

impl AudioState {
    /// First user gesture: build AudioContext + ScriptProcessor sink.
    fn unlock(&mut self, muted: &Rc<Cell<bool>>) {
        if self.ctx.is_some() {
            return;
        }
        let Ok(ctx) = web_sys::AudioContext::new() else {
            return;
        };
        let player = self.player.clone();
        let muted = muted.clone();
        if let Ok(sp) = ctx
            .create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(
                4096, 0, 2,
            )
        {
            let cb = Closure::wrap(Box::new(move |ev: web_sys::AudioProcessingEvent| {
                let Ok(buf) = ev.output_buffer() else { return };
                let n = buf.length() as usize;
                let mut out = vec![0.0f32; n * 2];
                if !muted.get() {
                    if let Some(p) = player.borrow_mut().as_mut() {
                        p.render(&mut out, buf.sample_rate() as u32);
                    }
                }
                let mut l = vec![0.0f32; n];
                let mut r = vec![0.0f32; n];
                for i in 0..n {
                    l[i] = out[2 * i];
                    r[i] = out[2 * i + 1];
                }
                let _ = buf.copy_to_channel(&l, 0);
                let _ = buf.copy_to_channel(&r, 1);
            }) as Box<dyn FnMut(web_sys::AudioProcessingEvent)>);
            sp.set_onaudioprocess(Some(cb.as_ref().unchecked_ref()));
            cb.forget();
            let node: web_sys::AudioNode = sp.unchecked_into();
            let dest: web_sys::AudioNode = ctx.destination().unchecked_into();
            let _ = node.connect_with_audio_node(&dest);
        }
        self.ctx = Some(ctx);
    }
}

// ---------------------------------------------------------------- scenes

/// T105 demo troops: two-frame marching, 8x8, palette-indexed.
const T: u8 = TRANS;
static FOOT_A: [u8; 64] = [
    T, T, GRAY, GRAY, GRAY, GRAY, T, T,
    T, GRAY, GRAY, GRAY, GRAY, GRAY, GRAY, T,
    T, T, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, T, T,
    T, SAXON_BLUE, SAXON_BLUE, WHITE, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, T,
    T, T, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, T, T,
    T, T, TAN, T, T, TAN, T, T,
    T, TAN, TAN, T, T, TAN, TAN, T,
    T, T, T, T, T, T, T, T,
];
static FOOT_B: [u8; 64] = [
    T, T, GRAY, GRAY, GRAY, GRAY, T, T,
    T, GRAY, GRAY, GRAY, GRAY, GRAY, GRAY, T,
    T, T, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, T, T,
    T, SAXON_BLUE, SAXON_BLUE, WHITE, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, T,
    T, T, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, T, T,
    T, T, TAN, TAN, TAN, TAN, T, T,
    T, T, TAN, T, T, TAN, T, T,
    T, T, T, T, T, T, T, T,
];
static FOOT_SOLDIER: Sprite = Sprite::square(8, &FOOT_A);
static FOOT_SOLDIER_B: Sprite = Sprite::square(8, &FOOT_B);
static BANNER: [u8; 64] = [
    TAN, BLOOD, BLOOD, BLOOD, BLOOD, T, T, T,
    TAN, BLOOD, GOLD, GOLD, BLOOD, T, T, T,
    TAN, BLOOD, GOLD, GOLD, BLOOD, T, T, T,
    TAN, BLOOD, BLOOD, BLOOD, BLOOD, T, T, T,
    TAN, T, T, T, T, T, T, T,
    TAN, T, T, T, T, T, T, T,
    TAN, T, T, T, T, T, T, T,
    TAN, T, T, T, T, T, T, T,
];
static BANNER_SPRITE: Sprite = Sprite::square(8, &BANNER);

/// T107 proof scene: gfx primitives, fonts, all input channels, transitions.
pub struct TestScene {
    pan: (i32, i32),
    circle: (i32, i32, i32, i32), // x, y, vx, vy in 1/256 px
    target: (i32, i32),
    radius: u32,
    scroll: i32,
    want_leave: bool,
    msg: &'static str,
}
impl TestScene {
    fn new(msg: &'static str) -> Self {
        Self {
            pan: (0, 0),
            circle: (W as i32 * 128 * 256 / W as i32, 60 * 256, 2 * 256, 3 * 256),
            target: (160, 100),
            radius: 14,
            scroll: 0,
            want_leave: false,
            msg,
        }
    }
}
impl Scene for TestScene {
    fn event(&mut self, ev: &SceneEvent) {
        match *ev {
            SceneEvent::Tap { x, y } => self.target = (x, y),
            SceneEvent::DragMove { x0, y0, x, y } => {
                self.pan = (x - x0, y - y0);
            }
            SceneEvent::Wheel { dy } => {
                self.radius = (self.radius as i32 + dy / 40).clamp(4, 60) as u32;
            }
            SceneEvent::Key {
                code: KEY_SPACE,
                pressed: true,
            }
            | SceneEvent::Key {
                code: KEY_ESC,
                pressed: true,
            } => self.want_leave = true,
            _ => {}
        }
    }
    fn tick(&mut self) {
        let (x, y, vx, vy) = self.circle;
        let (tx, ty) = self.target;
        let vx = (vx + (tx * 256 - x) / 120) * 60 / 64;
        let vy = (vy + (ty * 256 - y) / 120) * 60 / 64;
        let mut x = x + vx;
        let mut y = y + vy;
        let (wmax, hmax) = (W as i32 * 256, 160 * 256);
        if x < 0 {
            x = 0;
        }
        if x > wmax {
            x = wmax;
        }
        if y < 0 {
            y = 0;
        }
        if y > hmax {
            y = hmax;
        }
        self.circle = (x, y, vx, vy);
        self.scroll += 1;
    }
    fn draw(&mut self, fb: &mut Framebuffer) {
        fb.gradient_v(0, 0, W, 128, LIGHTBLUE, NAVY);
        fb.fill_rect(0, 128, W, 128, GREEN);
        // pannable scrolling checker ground
        let ox = ((self.scroll + self.pan.0) % 32 + 32) % 32;
        let oy = (self.pan.1 % 32 + 32) % 32;
        let mut y = 128 - oy;
        while y < 256 {
            let mut x = -ox;
            while x < W as i32 {
                if ((x + ox) / 32 + (y + oy) / 32) % 2 == 0 {
                    fb.fill_rect(x, y, 16, 16, NAVY);
                }
                x += 32;
            }
            y += 32;
        }
        for i in 0..8 {
            let cx = 8 + i * 40;
            fb.fill_rect(cx, 96, 24, 32, BLOOD);
            fb.fill_rect(cx + 4, 88, 6, 8, BLOOD);
            fb.fill_rect(cx + 14, 88, 6, 8, BLOOD);
        }
        // T105: two marching soldiers (anim + periodic hit-flash), x2 banner
        let march = (self.scroll / 3) % (W as i32 + 24) - 12;
        let frame = (self.scroll.max(0) / 12) as u32;
        if self.scroll % 90 < 4 {
            FOOT_SOLDIER.blit(fb, march, 148, true);
        } else {
            blit_anim(&FOOT_SOLDIER, &FOOT_SOLDIER_B, fb, march, 148, frame);
        }
        blit_anim(&FOOT_SOLDIER_B, &FOOT_SOLDIER, fb, march - 20, 150, frame + 1);
        FOOT_SOLDIER.blit_scaled(fb, 4, 232, 2, false);
        BANNER_SPRITE.blit_scaled(fb, 302, 72, 2, false);
        let (cx, cy) = (self.circle.0 / 256, self.circle.1 / 256);
        fb.fill_circle(cx, cy, self.radius, YELLOW);
        draw_text_shadow(fb, 4, 4, self.msg, WHITE, DARKGRAY);
        draw_text_shadow(
            fb,
            4,
            14,
            "TAP=AIM DRAG=PAN WHEEL=SIZE SPACE=RESTART M=MUTE",
            WHITE,
            DARKGRAY,
        );
    }
    fn wants_leave(&self) -> bool {
        self.want_leave
    }
}

/// Entry scene: title card; any tap/key dissolves into the test scene.
pub struct TitleScene {
    ticks: u32,
    touched: bool,
}
impl Scene for TitleScene {
    fn event(&mut self, ev: &SceneEvent) {
        match ev {
            SceneEvent::Tap { .. } | SceneEvent::Key { pressed: true, .. } => {
                self.touched = true;
            }
            _ => {}
        }
    }
    fn tick(&mut self) {
        self.ticks += 1;
    }
    fn draw(&mut self, fb: &mut Framebuffer) {
        fb.clear(NAVY);
        fb.gradient_v(0, 90, W, 40, SAXON_BLUE, BLOOD);
        fb.fill_rect(0, 128, W, 2, GOLD);
        draw_text_shadow(fb, 60, 64, "THE CROWN ENDURES", YELLOW, DARKGRAY);
        draw_text_shadow(fb, 84, 140, "A SAXON LIBERATION", WHITE, DARKGRAY);
        if self.ticks / 20 % 2 == 0 {
            draw_text_shadow(fb, 92, 190, "CLICK OR PRESS A KEY", WHITE, DARKGRAY);
        }
        draw_text_shadow(fb, 4, 246, "RUST + WASM + 320X256", DARKGRAY, BLACK);
    }
    fn wants_leave(&self) -> bool {
        self.touched
    }
}

// ---------------------------------------------------------------- main

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let document = w().document().expect("no document");
    let canvas = document
        .get_element_by_id("screen")
        .ok_or_else(|| JsValue::from_str("missing #screen canvas"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    canvas.set_width(W);
    canvas.set_height(H);
    let style = canvas.style();
    style.set_property("image-rendering", "pixelated")?;

    let ctx2d = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("no 2d context"))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;
    let rgba = vec![0u8; (W * H * 4) as usize];

    let muted = Rc::new(Cell::new(query_has("mute=1")));
    let audio = AudioState {
        ctx: None,
        player: Rc::new(RefCell::new(None)),
    };
    if let Ok(m) = Mod::parse(include_bytes!("../assets/music/test.mod")) {
        *audio.player.borrow_mut() = Some(Player::new(m));
    }
    let want_test = query_has("scene=test");
    let shot_at: u64 = query_get("shot").and_then(|v| v.parse().ok()).unwrap_or(0);

    let first: Box<dyn Scene> = if want_test {
        Box::new(TestScene::new("TEST SCENE"))
    } else {
        Box::new(TitleScene {
            ticks: 0,
            touched: false,
        })
    };
    let app = App {
        fb: Framebuffer::new(),
        scratch: Framebuffer::new(),
        input: Input::new(),
        stack: SceneStack::new(first),
        mode: if want_test { Mode::Test } else { Mode::Title },
        frame: 0,
        rgba,
        canvas,
        ctx2d,
        last: 0.0,
        acc: 0.0,
        fps_seen: 0,
        fps_at: 0.0,
        shot_at,
        shot_done: false,
    };
    let state = Rc::new(RefCell::new(app));
    let ctxrc = Rc::new(RefCell::new(Ctx { muted, audio }));

    wire_input(&state, &ctxrc);
    {
        let s = state.clone();
        let on_resize = Closure::wrap(Box::new(move || fit(&s)) as Box<dyn FnMut()>);
        w().add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref())?;
        on_resize.forget();
    }
    fit(&state);

    let closure: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::default();
    {
        let state = state.clone();
        let ctxrc = ctxrc.clone();
        let closure2 = closure.clone();
        let c: Closure<dyn FnMut(f64)> = Closure::wrap(Box::new(move |now: f64| {
            let mut guard = state.borrow_mut();
            let st = &mut *guard;
            let mut ctx = ctxrc.borrow_mut();
            if st.last == 0.0 {
                st.last = now;
                st.fps_at = now;
            }
            st.acc += (now - st.last).min(250.0);
            st.last = now;
            const STEP: f64 = 1000.0 / 60.0;
            while st.acc >= STEP {
                st.acc -= STEP;
                st.frame += 1;
                let frame = st.frame;
                st.stack.step(&mut *ctx, &mut st.input, frame);
                if st.input.user_gesture {
                    let m = ctx.muted.clone();
                    ctx.audio.unlock(&m);
                }
                // mute toggle
                if st.input.key_pressed(crate::engine::input::KEY_M) {
                    let v = !ctx.muted.get();
                    ctx.muted.set(v);
                }
                if st.stack.top_wants_leave() {
                    let next = match st.mode {
                        Mode::Title => {
                            st.mode = Mode::Test;
                            Box::new(TestScene::new("TEST SCENE")) as Box<dyn Scene>
                        }
                        Mode::Test => Box::new(TestScene::new("FADE CYCLE COMPLETE")),
                    };
                    st.stack.replace(next, Transition::Fade);
                }
                st.fps_seen += 1;
            }
            st.stack.paint(&mut st.fb, &mut st.scratch);
            st.fb.to_rgba(&mut st.rgba);
            if let Ok(img) =
                web_sys::ImageData::new_with_u8_clamped_array(Clamped(&st.rgba), W)
            {
                let _ = st.ctx2d.put_image_data(&img, 0.0, 0.0);
            }
            if now - st.fps_at >= 1000.0 {
                st.fps_at = now;
                let fps = st.fps_seen;
                st.fps_seen = 0;
                let _ = js_sys::Reflect::set(
                    &w().into(),
                    &JsValue::from_str("__fps"),
                    &JsValue::from(fps),
                );
            }
            if st.shot_at > 0 && st.frame >= st.shot_at && !st.shot_done {
                st.shot_done = true;
                let _ = js_sys::Reflect::set(
                    &w().into(),
                    &JsValue::from_str("__shot"),
                    &JsValue::from(true),
                );
            }
            w().request_animation_frame(
                closure2
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unchecked_ref::<js_sys::Function>(),
            )
            .ok();
        }));
        *closure.borrow_mut() = Some(c);
        w().request_animation_frame(
            closure
                .borrow()
                .as_ref()
                .unwrap()
                .as_ref()
                .unchecked_ref::<js_sys::Function>(),
        )
        .ok();
    }
    Ok(())
}

/// Largest integer scale fitting the viewport.
fn fit(state: &Rc<RefCell<App>>) {
    let iw = w().inner_width().unwrap().as_f64().unwrap_or(1280.0) as i32;
    let ih = w().inner_height().unwrap().as_f64().unwrap_or(720.0) as i32;
    let s = (iw / W as i32).min(ih / H as i32).max(1);
    let st = state.borrow();
    let style = st.canvas.style();
    let _ = style.set_property("width", &format!("{}px", W * s as u32));
    let _ = style.set_property("height", &format!("{}px", H * s as u32));
}

/// Map `KeyboardEvent.key` to our raw codes.
fn key_code(key: &str) -> Option<u32> {
    let c = match key {
        "Enter" => KEY_ENTER,
        "Escape" => KEY_ESC,
        " " => KEY_SPACE,
        "ArrowLeft" => KEY_LEFT,
        "ArrowUp" => KEY_UP,
        "ArrowRight" => KEY_RIGHT,
        "ArrowDown" => KEY_DOWN,
        "m" | "M" => crate::engine::input::KEY_M,
        other => {
            let mut chars = other.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) if (32..127).contains(&(ch as u32)) => ch as u32,
                _ => return None,
            }
        }
    };
    Some(c)
}

fn wire_input(state: &Rc<RefCell<App>>, ctx: &Rc<RefCell<Ctx>>) {
    let canvas = state.borrow().canvas.clone();
    let logical = {
        let s = state.clone();
        move |cx: f64, cy: f64| -> (i32, i32) {
            let st = s.borrow();
            let rect = st.canvas.get_bounding_client_rect();
            let scale = (rect.width() / W as f64).max(1.0);
            (
                ((cx - rect.left()) / scale).floor() as i32,
                ((cy - rect.top()) / scale).floor() as i32,
            )
        }
    };
    macro_rules! mevent {
        ($name:expr, $mk:expr) => {{
            let l = logical.clone();
            let s = state.clone();
            let cb = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                let (x, y) = l(e.client_x() as f64, e.client_y() as f64);
                s.borrow_mut().input.push($mk(x, y));
            }) as Box<dyn FnMut(web_sys::MouseEvent)>);
            canvas
                .add_event_listener_with_callback($name, cb.as_ref().unchecked_ref())
                .unwrap();
            cb.forget();
        }};
    }
    // pointer events arrive through the MouseEvent bindings (PointerEvent
    // extends MouseEvent; web-sys casts share clientX/clientY).
    mevent!("pointerdown", |x, y| Raw::Down { x, y });
    mevent!("pointermove", |x, y| Raw::Move { x, y });
    mevent!("pointerup", |x, y| Raw::Up { x, y });
    mevent!("pointercancel", |x, y| Raw::Up { x, y });
    {
        let s = state.clone();
        let cb = Closure::wrap(Box::new(move |e: web_sys::WheelEvent| {
            e.prevent_default();
            s.borrow_mut().input.push(Raw::Wheel { dy: e.delta_y() as i32 });
        }) as Box<dyn FnMut(web_sys::WheelEvent)>);
        canvas
            .add_event_listener_with_callback("wheel", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }
    {
        let s = state.clone();
        let cb = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
            if let Some(code) = key_code(&e.key()) {
                s.borrow_mut()
                    .input
                    .push(Raw::Key { code, pressed: true });
                if e.key().starts_with("Arrow") || e.key() == " " {
                    e.prevent_default();
                }
            }
        }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
        w().add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }
    {
        let s = state.clone();
        let cb = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
            if let Some(code) = key_code(&e.key()) {
                s.borrow_mut()
                    .input
                    .push(Raw::Key { code, pressed: false });
            }
        }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
        w().add_event_listener_with_callback("keyup", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }
    let _ = ctx; // ctx reached only through the RAF driver
}
