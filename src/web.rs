//! T107: browser adapter — canvas blit, RAF fixed-step driver, DOM input,
//! WebAudio output. `wasm32` only; host builds skip this file entirely.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use wasm_bindgen::JsCast;

#[wasm_bindgen(module = "/src/promise_hook.js")]
extern "C" {
    fn crown_hook(promise: &JsValue, on_ok: &JsValue, on_err: &JsValue);
}

use crate::engine::audio::{Mod, Music, Player, Sfx};
use crate::engine::fonts::{draw_text, draw_text_shadow};
use crate::engine::gfx::{
    Framebuffer, BLACK, BLOOD, BROWN, DARKGRAY, DARKGREEN, EMBER, GOLD, GRASS, GRAY, GREEN, H,
    LIGHTBLUE, LINEN, NAVY, OLIVE, ORANGE, SAXON_BLUE, SILVER, SKY, STONE, TAN, W, WHITE, YELLOW,
};
use crate::engine::input::{
    Gesture, Input, Raw, KEY_DOWN, KEY_ENTER, KEY_ESC, KEY_LEFT, KEY_RIGHT, KEY_SPACE, KEY_UP,
};

use crate::campaign::game::Difficulty;
use crate::engine::scene::{AppCtx, Scene, SceneEvent, SceneStack, Transition};
use crate::engine::sprite::{blit_anim, Sprite, TRANS};
use crate::ui::PlayScene;

/// Difficulty picked on the title screen (single-threaded wasm; 0 easy, 1
/// normal, 2 hard).
static PICK: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);

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
    Demo,
    Play { diff: Difficulty, seed: u64 },
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
    perf_acc: f64,
    perf_n: u32,
    crt: Rc<Cell<bool>>,
}

struct Ctx {
    muted: Rc<Cell<bool>>,
    crt: Rc<Cell<bool>>,
    music: Rc<RefCell<Music>>,
    audio: AudioState,
    /// T501: scenes queued via AppCtx::push, drained by the driver.
    pending_push: Rc<RefCell<Vec<Box<dyn Scene>>>>,
}

/// T401: named music beds. "title" loops back to its march section
/// (restart byte), so the fanfare only plays on first entry.
const TRACKS: &[(&str, &[u8])] = &[
    ("title", include_bytes!("../assets/music/title.mod")),
    ("saxon", include_bytes!("../assets/music/saxon.mod")),
    ("battle", include_bytes!("../assets/music/battle.mod")),
    ("minigame", include_bytes!("../assets/music/minigame.mod")),
    ("victory", include_bytes!("../assets/music/victory.mod")),
    ("defeat", include_bytes!("../assets/music/defeat.mod")),
];

impl AppCtx for Ctx {
    fn mute(&self) -> bool {
        self.muted.get()
    }
    fn set_mute(&mut self, muted: bool) {
        self.muted.set(muted);
        ls_set("doc.mute", if muted { "1" } else { "0" });
    }
    fn crt(&self) -> bool {
        self.crt.get()
    }
    fn set_crt(&mut self, on: bool) {
        self.crt.set(on);
        ls_set("doc.crt", if on { "1" } else { "0" });
    }
    fn play_music(&mut self, track: Option<&'static str>) {
        let p = track
            .and_then(|t| TRACKS.iter().find(|(n, _)| *n == t))
            .and_then(|(_, bytes)| Mod::parse(bytes).ok())
            .map(Player::new);
        self.music.borrow_mut().set_track(p);
    }
    fn sfx(&mut self, kind: Sfx) {
        self.music.borrow_mut().sfx(kind);
    }
    fn sting(&mut self, track: &'static [u8]) {
        if let Ok(m) = Mod::parse(track) {
            self.music.borrow_mut().sting(Player::new(m).once());
        }
    }
    fn record_feat(&mut self, feat: &str) {
        let old = ls_get("doc.hof").unwrap_or_default();
        let mut entries: Vec<String> = old
            .split(';')
            .filter(|e| !e.is_empty())
            .map(String::from)
            .collect();
        entries.insert(0, feat.to_string());
        entries.truncate(5);
        ls_set("doc.hof", &entries.join(";"));
    }
    fn push(&mut self, s: Box<dyn Scene>) {
        self.pending_push.borrow_mut().push(s);
    }
    fn mg_best(&mut self, id: &str, score: u32) {
        let key = format!("doc.mg.{id}");
        let prev = ls_get(&key).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        if score > prev {
            ls_set(&key, &score.to_string());
        }
    }
}

/// T108: pull-based audio sink. The worklet asks for N samples whenever its
/// ring runs low; the main thread renders the full mix (track + SFX voices)
/// and posts it back. ScriptProcessor fallback for engines without
/// AudioWorklet. `window.__rms` exposes output level for the browser gate.
const WORKLET_JS: &str = r#"
class DocSink extends AudioWorkletProcessor {
  constructor() {
    super();
    this.ring = new Float32Array(16384);
    this.w = 0; this.r = 0;
    this.port.onmessage = (e) => {
      const c = e.data;
      for (let i = 0; i < c.length; i++) { this.ring[this.w] = c[i]; this.w = (this.w + 1) & 16383; }
    };
    this.port.postMessage(8192);
  }
  process(_ins, outs) {
    const o = outs[0]; const n = o[0].length;
    for (let i = 0; i < n; i += 2) {
      if (((this.w - this.r) & 16383) >= 2) {
        o[0][i] = this.ring[this.r]; o[1][i] = this.ring[(this.r + 1) & 16383];
        this.r = (this.r + 2) & 16383;
      } else { o[0][i] = 0; o[1][i] = 0; }
    }
    if (((this.w - this.r) & 16383) < 4096) this.port.postMessage(8192);
    return true;
  }
}
registerProcessor('doc-sink', DocSink);
"#;

fn set_rms(buf: &[f32]) {
    if buf.is_empty() {
        return;
    }
    let ssq: f64 = buf.iter().map(|s| (*s as f64) * (*s as f64)).sum();
    let rms = (ssq / buf.len() as f64).sqrt();
    let _ = js_sys::Reflect::set(
        &w().into(),
        &JsValue::from_str("__rms"),
        &JsValue::from(rms),
    );
}

fn attach_script(ctx: &web_sys::AudioContext, music: Rc<RefCell<Music>>, muted: Rc<Cell<bool>>) {
    let Ok(sp) = ctx
        .create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(
            4096, 0, 2,
        )
    else {
        return;
    };
    let cb = Closure::wrap(Box::new(move |ev: web_sys::AudioProcessingEvent| {
        let Ok(buf) = ev.output_buffer() else { return };
        let n = buf.length() as usize;
        let mut out = vec![0.0f32; n * 2];
        if !muted.get() {
            music
                .borrow_mut()
                .render(&mut out, buf.sample_rate() as u32);
        }
        let mut l = vec![0.0f32; n];
        let mut r = vec![0.0f32; n];
        for i in 0..n {
            l[i] = out[2 * i];
            r[i] = out[2 * i + 1];
        }
        let _ = buf.copy_to_channel(&l, 0);
        let _ = buf.copy_to_channel(&r, 1);
        set_rms(&out);
    }) as Box<dyn FnMut(web_sys::AudioProcessingEvent)>);
    sp.set_onaudioprocess(Some(cb.as_ref().unchecked_ref()));
    cb.forget();
    let node: web_sys::AudioNode = sp.unchecked_into();
    let dest: web_sys::AudioNode = ctx.destination().unchecked_into();
    let _ = node.connect_with_audio_node(&dest);
}

struct AudioState {
    ctx: Option<web_sys::AudioContext>,
    music: Rc<RefCell<Music>>,
}

impl AudioState {
    /// First user gesture: AudioContext + worklet sink (fallback: ScriptProcessor).
    fn unlock(&mut self, muted: &Rc<Cell<bool>>) {
        if self.ctx.is_some() {
            return;
        }
        let Ok(ctx) = web_sys::AudioContext::new() else {
            return;
        };
        let _ = ctx.resume();
        let music = self.music.clone();
        let muted = muted.clone();
        let url = format!(
            "data:text/javascript,{}",
            WORKLET_JS.replace('%', "%25").replace('#', "%23")
        );
        let attached = match ctx.audio_worklet().ok() {
            Some(aw) => {
                let c2 = ctx.clone();
                let m2 = music.clone();
                let mu2 = muted.clone();
                let mk = Closure::once(move |_v: JsValue| {
                    let made = (|| -> Option<web_sys::MessagePort> {
                        let ctor = js_sys::Reflect::get(
                            &w().into(),
                            &JsValue::from_str("AudioWorkletNode"),
                        )
                        .ok()?;
                        let opts = js_sys::Object::new();
                        js_sys::Reflect::set(
                            &opts,
                            &JsValue::from_str("outputChannelCount"),
                            &JsValue::from(js_sys::Array::of1(&JsValue::from(2))),
                        )
                        .ok()?;
                        let args = js_sys::Array::of3(
                            &c2.clone().into(),
                            &JsValue::from_str("doc-sink"),
                            &opts.into(),
                        );
                        let node = js_sys::Reflect::construct(
                            ctor.unchecked_ref::<js_sys::Function>(),
                            &args,
                        )
                        .ok()?;
                        let an: web_sys::AudioNode = node.clone().unchecked_into();
                        let _ = an.connect_with_audio_node(&c2.destination().unchecked_into());
                        js_sys::Reflect::get(&node, &JsValue::from_str("port"))
                            .ok()
                            .map(|p| p.unchecked_into::<web_sys::MessagePort>())
                    })();
                    let Some(port) = made else {
                        attach_script(&c2, m2, mu2);
                        return;
                    };
                    let sr = c2.sample_rate() as u32;
                    let feed_port = port.clone();
                    let feed = Closure::wrap(Box::new(move |ev: web_sys::MessageEvent| {
                        let samples = (ev.data().as_f64().unwrap_or(0.0) as usize).min(16384);
                        if samples == 0 {
                            return;
                        }
                        let mut buf = vec![0.0f32; samples];
                        if !mu2.get() {
                            m2.borrow_mut().render(&mut buf, sr);
                        }
                        let arr = js_sys::Float32Array::new_with_length(samples as u32);
                        arr.copy_from(&buf);
                        let _ = feed_port.post_message(&arr);
                        set_rms(&buf);
                    })
                        as Box<dyn FnMut(web_sys::MessageEvent)>);
                    port.set_onmessage(Some(feed.as_ref().unchecked_ref()));
                    feed.forget();
                });
                let c3 = ctx.clone();
                let m3 = music.clone();
                let mu3 = muted.clone();
                let fb = Closure::once(move |_e: JsValue| attach_script(&c3, m3, mu3));
                let res = match aw.add_module(&url) {
                    Ok(p) => {
                        crown_hook(&p, mk.as_ref(), fb.as_ref());
                        true
                    }
                    Err(_) => false,
                };
                mk.forget();
                fb.forget();
                res
            }
            None => false,
        };
        if !attached {
            let c4 = ctx.clone();
            attach_script(&c4, music, muted);
        }
        self.ctx = Some(ctx);
    }
}

// ---------------------------------------------------------------- scenes

/// T105 demo troops: two-frame marching, 8x8, palette-indexed.
const T: u8 = TRANS;
static FOOT_A: [u8; 64] = [
    T, T, GRAY, GRAY, GRAY, GRAY, T, T, T, GRAY, GRAY, GRAY, GRAY, GRAY, GRAY, T, T, T, SAXON_BLUE,
    SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, T, T, T, SAXON_BLUE, SAXON_BLUE, WHITE, SAXON_BLUE,
    SAXON_BLUE, SAXON_BLUE, T, T, T, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, T, T, T, T,
    TAN, T, T, TAN, T, T, T, TAN, TAN, T, T, TAN, TAN, T, T, T, T, T, T, T, T, T,
];
static FOOT_B: [u8; 64] = [
    T, T, GRAY, GRAY, GRAY, GRAY, T, T, T, GRAY, GRAY, GRAY, GRAY, GRAY, GRAY, T, T, T, SAXON_BLUE,
    SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, T, T, T, SAXON_BLUE, SAXON_BLUE, WHITE, SAXON_BLUE,
    SAXON_BLUE, SAXON_BLUE, T, T, T, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, SAXON_BLUE, T, T, T, T,
    TAN, TAN, TAN, TAN, T, T, T, T, TAN, T, T, TAN, T, T, T, T, T, T, T, T, T, T,
];
static FOOT_SOLDIER: Sprite = Sprite::square(8, &FOOT_A);
static FOOT_SOLDIER_B: Sprite = Sprite::square(8, &FOOT_B);
static BANNER: [u8; 64] = [
    TAN, BLOOD, BLOOD, BLOOD, BLOOD, T, T, T, TAN, BLOOD, GOLD, GOLD, BLOOD, T, T, T, TAN, BLOOD,
    GOLD, GOLD, BLOOD, T, T, T, TAN, BLOOD, BLOOD, BLOOD, BLOOD, T, T, T, TAN, T, T, T, T, T, T, T,
    TAN, T, T, T, T, T, T, T, TAN, T, T, T, T, T, T, T, TAN, T, T, T, T, T, T, T,
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
    fn tick(&mut self, _app: &mut dyn AppCtx) {
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
        blit_anim(
            &FOOT_SOLDIER_B,
            &FOOT_SOLDIER,
            fb,
            march - 20,
            150,
            frame + 1,
        );
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

/// T1xx checkpoint scene: font sheet, full palette strip, dither swatches,
/// gradient bands, sprite anim/scaled/flash. Music loops from boot; the loop
/// fires rotating SFX while in Demo mode.
pub struct DemoScene {
    ticks: u32,
    want_leave: bool,
}
impl Scene for DemoScene {
    fn event(&mut self, ev: &SceneEvent) {
        match *ev {
            SceneEvent::Key {
                code: KEY_ESC,
                pressed: true,
            }
            | SceneEvent::Key {
                code: KEY_SPACE,
                pressed: true,
            }
            | SceneEvent::Tap { .. } => self.want_leave = true,
            _ => {}
        }
    }
    fn tick(&mut self, _app: &mut dyn AppCtx) {
        self.ticks += 1;
    }
    fn draw(&mut self, fb: &mut Framebuffer) {
        fb.gradient_v(0, 0, W, 72, NAVY, BLACK);
        fb.fill_rect(0, 72, W, 88, BLACK);
        fb.gradient_v(0, 160, W, 96, DARKGREEN, BLACK);
        draw_text_shadow(fb, 8, 4, "T1xx ENGINE CHECKPOINT", GOLD, DARKGRAY);
        fb.fill_rect(0, 13, W, 36, LINEN);
        // font sheet: every glyph in a distinct palette color
        let rows = [
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "0123456789 .,:;!?+-*/=#%&'",
            "abcdefghijklmnopqrstuvwxyz",
        ];
        for (r, row) in rows.iter().enumerate() {
            let y = 16 + r as i32 * 11;
            for (i, ch) in row.chars().enumerate() {
                let mut b = [0u8; 4];
                let s = ch.encode_utf8(&mut b);
                draw_text(fb, 8 + i as i32 * 8, y, s, 1 + ((i + r * 10) % 31) as u8);
            }
        }
        // full 32-color palette strip (index == position)
        for i in 0..32u8 {
            fb.fill_rect(i as i32 * 10, 50, 8, 8, i);
        }
        fb.rect(0, 49, 320, 10, DARKGRAY);
        // dither blend swatches GOLD->NAVY
        for i in 0..8u8 {
            fb.fill_rect_dither(8 + i as i32 * 20, 76, 16, 16, GOLD, NAVY, i * 2);
        }
        draw_text_shadow(fb, 176, 80, "DITHER+", WHITE, BLACK);
        draw_text_shadow(fb, 176, 90, "GRADIENTS", WHITE, BLACK);
        // sprite row: marching anim pair, x3 scaled w/ periodic flash, x4 banner
        let march = (self.ticks as i32 / 2) % (W as i32 + 32) - 16;
        let frame = (self.ticks / 10) as u32;
        if self.ticks % 70 < 4 {
            FOOT_SOLDIER.blit(fb, march, 110, true);
        } else {
            blit_anim(&FOOT_SOLDIER, &FOOT_SOLDIER_B, fb, march, 110, frame);
        }
        blit_anim(
            &FOOT_SOLDIER_B,
            &FOOT_SOLDIER,
            fb,
            march + 14,
            112,
            frame + 1,
        );
        FOOT_SOLDIER.blit_scaled(fb, 200, 108, 3, false);
        BANNER_SPRITE.blit_scaled(fb, 288, 100, 4, false);
        draw_text_shadow(
            fb,
            8,
            148,
            "MODPLAYER+SFX LIVE (AUDIO-CHECK PROVES)",
            WHITE,
            BLACK,
        );
        let msg = format!("TICK {}", self.ticks);
        draw_text_shadow(fb, 248, 246, &msg, YELLOW, BLACK);
        if self.ticks % 40 < 20 {
            fb.fill_rect(240, 246, 6, 8, YELLOW);
        }
    }
    fn wants_leave(&self) -> bool {
        self.want_leave
    }
}

/// T404 intro crawl: heading + two story lines per screen.
const CRAWL: [(&str, &str, &str); 3] = [
    (
        "1066 - THE CROWN IS LOST",
        "HASTINGS HAS CLAIMED KING HAROLD.",
        "NORMAN BANNERS BLACKEN EVERY SHIRE.",
    ),
    (
        "BERIC OF CHELTENHAM",
        "A SAXON EARL REMEMBERS THE OATH.",
        "HE LEVIES THE FIRST KEEP FROM THE YOKE.",
    ),
    (
        "REIGN - OR RUIN",
        "WIN THE EARLS. STARVE THE GARRISONS.",
        "TAKE YOUR CROWN IN LONDON.",
    ),
];
/// T4xx: ticks each crawl screen stands before it rolls on (5 s at the
/// fixed 60 Hz step). The intro is flavor, not a toll gate: with the crawl
/// auto-rolling, the campaign is three inputs from load (boot key + BEGIN +
/// difficulty); NEXT still skips ahead.
const CRAWL_DWELL: u32 = 300;

/// Entry sequence (T402/T403): black screen + unfurling heraldic banner,
/// then an animated title card with a real menu: campaign start (difficulty
/// pick), how-to pages, hall of fame, sound options. Mouse/keyboard equal.

#[derive(PartialEq, Eq, Clone, Copy)]
enum Stage {
    Boot,
    Menu,
    Difficulty,
    Crawl,
    HowTo,
    Fame,
    Options,
}

pub struct TitleScene {
    ticks: u32,
    touched: bool,
    stage: Stage,
    item: usize, // menu/difficulty highlight (keyboard)
    page: usize, // how-to page
    crawl: usize, // intro crawl screen (T404)
    dwell: u32,   // ticks the current crawl leaf has stood (T4xx auto-roll)
    pending: Option<usize>, // activation deferred to tick (needs AppCtx)
    reset_armed: bool, // HOF erase needs a second ENTER (T705)
    sound: bool, // OPTIONS mirror of the persisted mute flag
    crt: bool,   // OPTIONS mirror of the persisted scanline flag (T405)
}

impl TitleScene {
    pub fn new() -> Self {
        Self {
            ticks: 0,
            touched: false,
            stage: Stage::Boot,
            item: 0,
            page: 0,
            crawl: 0,
            dwell: 0,
            pending: None,
            reset_armed: false,
            sound: initial_sound(),
            crt: ls_get("doc.crt").as_deref() != Some("0"),
        }
    }
}

/// Persisted mute flag inverted: OPTIONS opens showing the stored truth.
fn initial_sound() -> bool {
    match ls_get("doc.mute").as_deref() {
        Some("1") => false,
        Some("0") => true,
        _ => !query_has("mute=1"),
        }
    }

const MENU: [&str; 4] = ["BEGIN CAMPAIGN", "HOW TO PLAY", "HALL OF FAME", "OPTIONS"];
const HOWTO: [(&str, [&str; 5]); 3] = [
    (
        "THE LIBERATION",
        [
            "CLICK A SAXON KEEP TO SELECT IT",
            "GOLD BUYS LEVIES IN THE PANEL",
            "DRAG AN ARMY TO AN ADJACENT KEEP",
            "END TURN SUMMONS THE NEXT MARCH",
            "OMENS WARN OF NORMAN PLANS",
        ],
    ),
    (
        "WAR AND WIT",
        [
            "WALLS FAVOR THE DEFENDER",
            "A LOST KEEP MEANS LOST TROOPS",
            "SPIES PIERCE THE FOG OF WAR",
            "TOURNAMENTS BRING GOLD AND FAME",
            "HUNGER WASTES IDLE ARMIES",
        ],
    ),
    (
        "VICTORY AND DEFEAT",
        [
            "RAZE EVERY NORMAN GARRISON",
            "HOLD EVERY SAXON HEARTH",
            "THE CHRONICLE ENDS AT TOTAL",
            "VICTORY OR TOTAL DEFEAT -",
            "NEVER AT A DRAW",
        ],
    ),
];

fn diff_rect(i: usize) -> (i32, i32, u32, u32) {
    (46 + i as i32 * 80, 178, 72, 14)
}
const DIFF_NAMES: [&str; 3] = ["EASY", "NORMAL", "HARD"];
fn menu_rect(i: usize) -> (i32, i32, u32, u32) {
    (100, 118 + i as i32 * 17, 120, 13)
}
fn in_rect(r: (i32, i32, u32, u32), x: i32, y: i32) -> bool {
    x >= r.0 && x < r.0 + r.2 as i32 && y >= r.1 && y < r.1 + r.3 as i32
}
fn back_rect() -> (i32, i32, u32, u32) {
    (134, 226, 52, 14)
}

/// `s` in `sc`-scaled 8x8 bitmap caps with offset shadow.
fn draw_caps(fb: &mut Framebuffer, x: i32, y: i32, s: &str, fg: u8, sh: u8, sc: u32) {
    for (col, ox) in [(sh, sc as i32), (fg, 0)].into_iter() {
        let mut cx = x + ox;
        for ch in s.chars() {
            for (r, bits) in crate::engine::fonts::glyph(ch).iter().enumerate() {
                for c in 0..8u32 {
                    if bits & (0x80 >> c) != 0 {
                        fb.fill_rect(
                            cx + c as i32 * sc as i32,
                            y + ox + r as i32 * sc as i32,
                            sc,
                            sc,
                            col,
                        );
                    }
                }
            }
            cx += 8 * sc as i32;
        }
    }
}

impl TitleScene {
    fn activate(&mut self, i: usize, app: &mut dyn AppCtx) {
        match self.stage {
            Stage::Menu => {
                self.stage = match i {
                    0 => Stage::Difficulty,
                    1 => {
                        self.page = 0;
                        Stage::HowTo
                    }
                    2 => {
                        self.reset_armed = false;
                        Stage::Fame
                    }
                    _ => Stage::Options,
                };
                self.item = 0;
            }
            Stage::Difficulty => {
                PICK.store(i as u8, std::sync::atomic::Ordering::Relaxed);
                self.crawl = 0;
                self.dwell = 0;
                self.stage = Stage::Crawl;
            }
            Stage::Options => {
                if i == 0 {
                    app.set_mute(!app.mute());
                    self.sound = !app.mute();
                } else {
                    app.set_crt(!app.crt());
                    self.crt = app.crt();
                }
            }
            Stage::Fame => {
                // two-step erase: first ENTER arms, second wipes the vault
                if self.reset_armed {
                    Self::erase_records();
                    self.reset_armed = false;
                } else {
                    self.reset_armed = true;
                }
            }
            _ => {}
        }
    }
    fn back(&mut self) {
        self.dwell = 0;
        if self.stage == Stage::Crawl {
            self.stage = if self.crawl > 0 {
                self.crawl -= 1;
                Stage::Crawl
            } else {
                Stage::Difficulty
            };
            return;
        }
        self.stage = Stage::Menu;
        self.item = 0;
    }
    /// NEXT on the crawl: last screen commits the campaign start.
    fn crawl_forward(&mut self) {
        self.dwell = 0;
        if self.crawl + 1 < CRAWL.len() {
            self.crawl += 1;
        } else {
            self.touched = true;
        }
    }
    /// Flame-licked border: 2px cycling fire on all four edges.
    fn draw_flames(&self, fb: &mut Framebuffer) {
        let pal = [BLOOD, GOLD, YELLOW];
        for x in (0..W as i32).step_by(2) {
            fb.fill_rect(x, 0, 2, 2, pal[((x / 3 + self.ticks as i32 / 2) as usize) % 3]);
            fb.fill_rect(
                x,
                H as i32 - 2,
                2,
                2,
                pal[((x / 5 + self.ticks as i32 / 3) as usize) % 3],
            );
        }
        for y in (0..H as i32).step_by(2) {
            fb.fill_rect(0, y, 2, 2, pal[((y / 3 + self.ticks as i32 / 2) as usize) % 3]);
            fb.fill_rect(
                W as i32 - 2,
                y,
                2,
                2,
                pal[((y / 5 + self.ticks as i32 / 3) as usize) % 3],
            );
        }
    }
    fn draw_back(&self, fb: &mut Framebuffer) {
        let r = back_rect();
        fb.fill_rect(r.0, r.1, r.2, r.3, BLOOD);
        fb.rect(r.0, r.1, r.2, r.3, GOLD);
        draw_text_shadow(fb, r.0 + 13, r.1 + 4, "BACK", WHITE, BLACK);
    }
    fn draw_nav(&self, fb: &mut Framebuffer) {
        for (x, t) in [(12, "<"), (264, ">")] {
            fb.fill_rect(x, 226, 44, 14, BLOOD);
            fb.rect(x, 226, 44, 14, GOLD);
            draw_text_shadow(fb, x + 19, 230, t, WHITE, BLACK);
        }
        self.draw_back(fb);
    }
    /// T405: a pixel vignette for each how-to page.
    fn howto_art(&self, fb: &mut Framebuffer) {
        match self.page {
            0 => {
                // three keeps on a road, the picked one pulsing
                for (cx, cy) in [(56, 126), (152, 118), (248, 128)] {
                    fb.fill_rect(cx, cy, 16, 12, STONE);
                    for b in (cx..cx + 16).step_by(4) {
                        fb.fill_rect(b, cy - 3, 3, 3, STONE);
                    }
                    fb.fill_rect(cx + 6, cy + 5, 4, 7, DARKGRAY);
                }
                for rx in [84, 96, 108, 120, 176, 188, 200, 212] {
                    fb.fill_rect(rx, 132, 4, 2, DARKGRAY);
                }
                if self.ticks / 16 % 2 == 0 {
                    fb.rect(52, 113, 24, 28, GOLD);
                }
            }
            1 => {
                // shield + sword, a coin and a spy-eye
                fb.fill_rect(70, 112, 18, 16, BLOOD);
                fb.tri([(70, 128), (88, 128), (79, 138)], BLOOD);
                fb.fill_rect(77, 112, 4, 20, GOLD);
                fb.fill_rect(70, 118, 18, 4, GOLD);
                fb.tri([(104, 138), (108, 138), (122, 110)], SILVER);
                fb.fill_rect(106, 126, 10, 3, GOLD);
                fb.fill_circle(170, 124, 7, GOLD);
                fb.fill_circle(170, 124, 4, YELLOW);
                fb.fill_rect(169, 120, 3, 8, BLOOD);
                fb.fill_rect(216, 118, 26, 12, LINEN);
                fb.fill_circle(229, 124, 4, NAVY);
                fb.fill_circle(229, 124, 2, BLACK);
            }
            _ => {
                // crown between two waving banners
                let wob = (self.ticks / 12 % 2) as i32;
                for (bx, col) in [(96, BLOOD), (216, SAXON_BLUE)] {
                    fb.fill_rect(bx, 108, 2, 34, SILVER);
                    fb.fill_rect(bx + 2, 108 + wob, 14, 10, col);
                    fb.fill_rect(bx + 7, 108 + wob, 2, 10, GOLD);
                }
                fb.fill_rect(146, 126, 28, 8, GOLD);
                for px in [146, 158, 170] {
                    fb.fill_rect(px, 120, 4, 6, GOLD);
                }
                fb.fill_rect(152, 128, 3, 3, BLOOD);
                fb.fill_rect(163, 128, 3, 3, SAXON_BLUE);
            }
        }
    }
    /// T705: the hall shows one row per sport (best score, tier by the
    /// spec's own cut lines), the feat ticker underneath, and an eraser.
    fn fame_rect() -> (i32, i32, u32, u32) {
        (98, 196, 124, 12)
    }
    fn draw_fame(&self, fb: &mut Framebuffer) {
        fb.fill_rect(16, 60, 288, 150, BLACK);
        fb.rect(16, 60, 288, 150, GOLD);
        draw_text_shadow(fb, 100, 66, "HALL OF FAME", GOLD, BLACK);
        for (i, spec) in crate::minigames::framework::SPORTS.iter().enumerate() {
            let col = i / 7; // 7 left, rest right
            let x = 26 + col as i32 * 146;
            let y = 82 + (i % 7) as i32 * 11;
            let best = ls_get(&format!("doc.mg.{}", spec.id))
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);
            let tc = if best >= spec.gold_at {
                GOLD
            } else if best >= spec.silver_at {
                SILVER
            } else if best > 0 {
                YELLOW
            } else {
                GRAY
            };
            draw_text_shadow(fb, x, y, spec.label, if best > 0 { WHITE } else { GRAY }, BLACK);
            let s = if best > 0 { best.to_string() } else { "-".into() };
            draw_text_shadow(fb, x + 108 - s.len() as i32 * 8, y, &s, tc, BLACK);
        }
        match ls_get("doc.hof") {
            Some(s) if !s.is_empty() => {
                draw_text_shadow(fb, 26, 162, "FEATS", GOLD, BLACK);
                for (i, e) in s.split(';').take(3).enumerate() {
                    draw_text_shadow(fb, 90, 162 + i as i32 * 10, e, WHITE, BLACK);
                }
            }
            _ => {
                draw_text_shadow(fb, 26, 162, "NO FEATS RECORDED - WIN A WAR", GRAY, BLACK);
            }
        }
        let msg = if self.reset_armed {
            "SURE? ENTER ERASES"
        } else {
            "ENTER ERASES RECORDS"
        };
        let r = Self::fame_rect();
        fb.rect(r.0, r.1, r.2, r.3, if self.reset_armed { BLOOD } else { STONE });
        draw_text_shadow(fb, r.0 + 6, r.1 + 3, msg, if self.reset_armed { BLOOD } else { GRAY }, BLACK);
        self.draw_back(fb);
    }
    /// Wipe every persisted record: all sport bests and the feat ticker.
    fn erase_records() {
        for spec in crate::minigames::framework::SPORTS.iter() {
            ls_del(&format!("doc.mg.{}", spec.id));
        }
        ls_del("doc.hof");
    }
    /// T404: one crawl screen - heading, pixel vignette, two story lines.
    fn draw_crawl(&mut self, fb: &mut Framebuffer) {
        let (head, l1, l2) = CRAWL[self.crawl];
        fb.fill_rect(16, 96, 288, 140, BLACK);
        fb.rect(16, 96, 288, 140, GOLD);
        let hx = (W as i32 - head.len() as i32 * 6) / 2;
        draw_text_shadow(fb, hx, 102, head, GOLD, BLACK);
        match self.crawl {
            0 => self.crawl_fire(fb),
            1 => self.crawl_arms(fb),
            _ => self.crawl_keep(fb),
        }
        for (i, l) in [l1, l2].into_iter().enumerate() {
            let lx = (W as i32 - l.len() as i32 * 6) / 2;
            draw_text_shadow(fb, lx, 178 + i as i32 * 13, l, WHITE, BLACK);
        }
        let pg = format!("{}/{}", self.crawl + 1, CRAWL.len());
        draw_text_shadow(fb, 152, 204, &pg, GRAY, BLACK);
        if self.crawl + 1 == CRAWL.len() {
            draw_text_shadow(fb, 88, 214, "ENTER MARCHES TO WAR", GOLD, BLACK);
        }
        self.draw_nav(fb);
    }
    /// Screen 1: a shire ablaze under the Norman banner.
    fn crawl_fire(&self, fb: &mut Framebuffer) {
        fb.fill_rect_dither(20, 112, 276, 46, BLACK, BLOOD, 20);
        fb.fill_rect(20, 158, 276, 8, BLACK);
        for (hx, hw) in [(56i32, 30i32), (100, 22)] {
            fb.fill_rect(hx, 142, hw as u32, 16, BROWN);
            fb.tri([(hx - 4, 144), (hx + hw + 4, 144), (hx + hw / 2, 128)], OLIVE);
            fb.fill_rect(hx + hw / 2 - 3, 148, 6, 10, DARKGRAY);
        }
        // flame flicker riding the rooftops
        for k in 0..14i32 {
            let fx = 54 + k * 6 + ((self.ticks as i32 / 3 + k) % 3 - 1);
            let fy = 128 - (self.ticks as i32 + k * 7) % 6;
            fb.fill_rect(
                fx,
                fy,
                3,
                14,
                [EMBER, ORANGE, YELLOW][((k + (self.ticks / 4) as i32) % 3) as usize],
            );
        }
        for k in 0..5i32 {
            let sy = 122 - k * 9 - (self.ticks as i32 % 8);
            fb.fill_circle(70 + k * 12, sy, 3 + k as u32 / 2, DARKGRAY);
        }
        // Norman war banner planted right of the ruin
        fb.fill_rect(226, 114, 2, 46, SILVER);
        fb.fill_rect(230, 116, 34, 26, BLOOD);
        fb.rect(230, 116, 34, 26, GOLD);
        fb.fill_rect(236, 126, 16, 8, GOLD); // lion
        fb.fill_circle(254, 128, 3, GOLD);
        fb.fill_rect(238, 134, 3, 6, GOLD);
        fb.fill_rect(247, 134, 3, 6, GOLD);
    }
    /// Screen 2: axe and sword crossed over the forgotten crown.
    fn crawl_arms(&self, fb: &mut Framebuffer) {
        fb.fill_rect_dither(20, 112, 276, 56, NAVY, BLACK, 40);
        // axe: upright haft, silver crescent head left
        fb.fill_rect(158, 114, 4, 52, BROWN);
        fb.tri([(158, 116), (158, 134), (140, 125)], SILVER);
        fb.tri([(158, 120), (158, 130), (148, 125)], NAVY);
        // sword crossed right, gold guard
        fb.tri([(166, 164), (171, 164), (190, 114)], SILVER);
        fb.fill_rect(178, 128, 14, 4, GOLD);
        fb.fill_circle(190, 116, 2, GOLD);
        // the Saxon crown, buried but gold
        fb.fill_rect(146, 156, 28, 8, GOLD);
        for px in [146, 158, 170] {
            fb.fill_rect(px, 150, 4, 6, GOLD);
        }
        fb.fill_rect(152, 158, 3, 3, BLOOD);
        fb.fill_rect(163, 158, 3, 3, SAXON_BLUE);
        for k in 0..6i32 {
            if (self.ticks as usize / 8 + k as usize) % 4 == 0 {
                fb.fill_rect(60 + k * 42, 120 + (k * 13) % 40, 2, 2, WHITE);
            }
        }
    }
    /// Screen 3: the keep awaiteth its rightful lord.
    fn crawl_keep(&self, fb: &mut Framebuffer) {
        fb.gradient_v(20, 112, 276, 48, SKY, NAVY);
        fb.fill_rect(20, 160, 276, 8, GRASS);
        // keep + towers with crenels and blood roofs
        fb.fill_rect(120, 128, 80, 32, STONE);
        fb.fill_rect(110, 120, 18, 40, STONE);
        fb.fill_rect(192, 120, 18, 40, STONE);
        for cx in (120..200).step_by(8) {
            fb.fill_rect(cx, 124, 4, 4, NAVY);
        }
        fb.tri([(108, 120), (130, 120), (119, 106)], BLOOD);
        fb.tri([(190, 120), (212, 120), (201, 106)], BLOOD);
        fb.fill_rect(152, 142, 16, 18, DARKGRAY); // gate
        fb.rect(152, 142, 16, 18, GOLD);
        // Saxon banner over the gate, waving
        let wob = (self.ticks / 10 % 2) as i32;
        fb.fill_rect(159, 100, 2, 12, SILVER);
        fb.fill_rect(161, 100 + wob, 12, 8, SAXON_BLUE);
        fb.fill_rect(165, 100 + wob, 2, 8, GOLD);
        fb.fill_rect(161, 103 + wob, 12, 2, GOLD);
        // crown before the gate, dawn rays blinking
        fb.fill_rect(148, 164, 24, 6, GOLD);
        for px in [148, 158, 168] {
            fb.fill_rect(px, 159, 4, 5, GOLD);
        }
        if self.ticks / 12 % 2 == 0 {
            for k in 1..4 {
                fb.hline(150 - 8 * k, 156, 158 - 6 * k, YELLOW);
                fb.hline(164, 170 + 8 * k, 158 - 6 * k, YELLOW);
            }
        }
    }
}

impl Scene for TitleScene {
    fn event(&mut self, ev: &SceneEvent) {
        // The boot herald swallows the very first gesture (it also unlocks
        // audio one tick later in the app loop).
        if self.stage == Stage::Boot {
            if matches!(ev, SceneEvent::Tap { .. } | SceneEvent::Key { pressed: true, .. }) {
                self.stage = Stage::Menu;
            }
            return;
        }
        match ev {
            SceneEvent::Tap { x, y } => {
                let (x, y) = (*x, *y);
                match self.stage {
                    Stage::Menu => {
                        for i in 0..MENU.len() {
                            if in_rect(menu_rect(i), x, y) {
                                self.item = i;
                                self.pending = Some(i);
                            }
                        }
                    }
                    Stage::Difficulty => {
                        for i in 0..3 {
                            if in_rect(diff_rect(i), x, y) {
                                self.item = i;
                                self.pending = Some(i);
                            }
                        }
                        if in_rect(back_rect(), x, y) {
                            self.back();
                        }
                    }
                    Stage::Crawl => {
                        if in_rect((264, 226, 44, 14), x, y) {
                            self.crawl_forward();
                        }
                        if in_rect((12, 226, 44, 14), x, y) || in_rect(back_rect(), x, y) {
                            self.back();
                        }
                    }
                    Stage::HowTo => {
                        if in_rect((12, 226, 44, 14), x, y) {
                            self.page = self.page.saturating_sub(1);
                        }
                        if in_rect((264, 226, 44, 14), x, y) {
                            self.page = (self.page + 1).min(HOWTO.len() - 1);
                        }
                        if in_rect(back_rect(), x, y) {
                            self.back();
                        }
                    }
                    Stage::Fame => {
                        if in_rect(Self::fame_rect(), x, y) {
                            self.pending = Some(0);
                        }
                        if in_rect(back_rect(), x, y) {
                            self.back();
                        }
                    }
                    Stage::Options => {
                        if in_rect((100, 118, 120, 14), x, y) {
                            self.pending = Some(0);
                        }
                        if in_rect((100, 138, 120, 14), x, y) {
                            self.pending = Some(1);
                        }
                        if in_rect(back_rect(), x, y) {
                            self.back();
                        }
                    }
                    Stage::Boot => {}
                }
            }
            SceneEvent::Key {
                code,
                pressed: true,
            } => {
                let n = match self.stage {
                    Stage::Menu => MENU.len(),
                    Stage::Difficulty => 3,
                    Stage::Options => 2,
                    Stage::Fame => 1, // the eraser row
                    _ => 0,
                };
                match *code {
                    KEY_UP if n > 0 => self.item = (self.item + n - 1) % n,
                    KEY_DOWN if n > 0 => self.item = (self.item + 1) % n,
                    KEY_LEFT if self.stage == Stage::Crawl => self.back(),
                    KEY_LEFT if self.stage == Stage::HowTo => {
                        self.page = self.page.saturating_sub(1)
                    }
                    KEY_RIGHT if self.stage == Stage::HowTo => {
                        self.page = (self.page + 1).min(HOWTO.len() - 1)
                    }
                    KEY_RIGHT | KEY_ENTER | KEY_SPACE if self.stage == Stage::Crawl => {
                        self.crawl_forward()
                    }
                    KEY_ENTER | KEY_SPACE if self.stage == Stage::HowTo => {
                        if self.page + 1 >= HOWTO.len() {
                            self.back();
                        } else {
                            self.page += 1;
                        }
                    }
                    KEY_ENTER | KEY_SPACE if n > 0 => {
                        // deferred: tick() picks it up (needs &mut AppCtx)
                        self.pending = Some(self.item);
                    }
                    KEY_ESC if self.stage != Stage::Menu => self.back(),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    fn tick(&mut self, app: &mut dyn AppCtx) {
        self.ticks += 1;
        if let Some(i) = self.pending.take() {
            self.activate(i, app);
        }
        // T4xx: the crawl rolls by itself - a hands-off player rides the
        // intro straight into the war.
        if self.stage == Stage::Crawl {
            self.dwell += 1;
            if self.dwell >= CRAWL_DWELL {
                self.crawl_forward();
            }
        }
    }
    fn draw(&mut self, fb: &mut Framebuffer) {
        if self.stage == Stage::Boot {
            // heraldic banner unfurls from the top over 30 frames (T402)
            fb.clear(BLACK);
            let h = (self.ticks * 4).min(120);
            fb.fill_rect(132, 12, 56, h, BLOOD);
            fb.fill_rect(134, 12, 52, h, SAXON_BLUE);
            if h >= 24 {
                fb.fill_rect(138, 116, 44, 8, GOLD);
            }
            if h >= 90 {
                fb.fill_rect(146, 44, 28, 10, GOLD);
                for px in [146, 158, 170] {
                    fb.fill_rect(px, 38, 4, 6, GOLD);
                }
                fb.fill_rect(152, 46, 4, 4, BLOOD);
                fb.fill_rect(164, 46, 4, 4, BLOOD);
            }
            if self.ticks > 44 && self.ticks % 40 < 26 {
                draw_text_shadow(fb, 68, 190, "PRESS ANY KEY - SOUND ON", WHITE, BLACK);
            }
            return;
        }
        fb.clear(NAVY);
        fb.gradient_v(0, 84, W, 126, SAXON_BLUE, NAVY);
        self.draw_flames(fb);
        // side banners flanking the menu panel, clear of the lettering
        for (bx, col) in [(40, BLOOD), (268, SAXON_BLUE)] {
            fb.rect(bx - 1, 99, 14, 78, BLACK);
            fb.fill_rect(bx, 100, 12, 76, col);
            fb.fill_rect(bx + 5, 100, 2, 76, GOLD);
            fb.fill_rect(bx, 126, 12, 2, GOLD);
        }
        // bobbing crown above the name
        let bob = (self.ticks / 20 % 2) as i32;
        fb.fill_rect(144, 30 + bob, 32, 6, GOLD);
        for px in [144, 158, 172] {
            fb.fill_rect(px, 24 + bob, 4, 6, GOLD);
        }
        fb.fill_rect(150, 32 + bob, 3, 3, BLOOD);
        fb.fill_rect(159, 32 + bob, 3, 3, SAXON_BLUE);
        fb.fill_rect(168, 32 + bob, 3, 3, BLOOD);
        draw_caps(fb, 24, 44, "THE CROWN ENDURES", YELLOW, BLOOD, 2);
        draw_text_shadow(fb, 63, 66, "A SAXON LIBERATION", WHITE, DARKGRAY);
        match self.stage {
            Stage::Menu => {
                fb.fill_rect(96, 112, 128, 84, NAVY);
                for i in 0..MENU.len() {
                    let r = menu_rect(i);
                    let hi = self.item == i;
                    fb.fill_rect(r.0, r.1, r.2, r.3, if hi { BLOOD } else { NAVY });
                    fb.rect(r.0, r.1, r.2, r.3, if hi { GOLD } else { DARKGRAY });
                    let t = MENU[i];
                    draw_text_shadow(
                        fb,
                        r.0 + (r.2 as i32 - t.len() as i32 * 6) / 2,
                        r.1 + 3,
                        t,
                        if hi { WHITE } else { GRAY },
                        BLACK,
                    );
                }
            }
            Stage::Difficulty => {
                draw_text_shadow(fb, 108, 162, "CHOOSE YOUR FOE", GOLD, BLACK);
                for i in 0..3 {
                    let r = diff_rect(i);
                    let hi = self.item == i;
                    fb.fill_rect(r.0, r.1, r.2, r.3, if hi { GOLD } else { BLOOD });
                    fb.rect(r.0, r.1, r.2, r.3, GOLD);
                    let n = DIFF_NAMES[i];
                    draw_text_shadow(
                        fb,
                        r.0 + (r.2 as i32 - n.len() as i32 * 6) / 2,
                        r.1 + 4,
                        n,
                        if hi { BLACK } else { WHITE },
                        BLACK,
                    );
                }
                draw_text_shadow(fb, 66, 200, "ENTER CHOOSES - ESC GOES BACK", GRAY, BLACK);
                self.draw_back(fb);
            }
            Stage::HowTo => {
                let (head, lines) = HOWTO[self.page];
                fb.fill_rect(16, 96, 288, 120, BLACK);
                fb.rect(16, 96, 288, 120, GOLD);
                let hx = (W as i32 - head.len() as i32 * 6) / 2;
                draw_text_shadow(fb, hx, 101, head, GOLD, BLACK);
                self.howto_art(fb);
                for (i, l) in lines.iter().enumerate() {
                    let lx = (W as i32 - l.len() as i32 * 6) / 2;
                    draw_text_shadow(fb, lx, 146 + i as i32 * 12, l, WHITE, BLACK);
                }
                let pg = format!("{}/{}", self.page + 1, HOWTO.len());
                draw_text_shadow(fb, 292 - pg.len() as i32 * 6, 204, &pg, GRAY, BLACK);
                self.draw_nav(fb);
            }
            Stage::Crawl => self.draw_crawl(fb),
            Stage::Fame => self.draw_fame(fb),
            Stage::Options => {
                let t = format!("SOUND: {}", if self.sound { "ON" } else { "OFF" });
                fb.fill_rect(100, 118, 120, 14, if self.item == 0 { BLOOD } else { NAVY });
                fb.rect(100, 118, 120, 14, GOLD);
                draw_text_shadow(fb, 118, 121, &t, WHITE, BLACK);
                let c = format!("CRT FILTER: {}", if self.crt { "ON" } else { "OFF" });
                fb.fill_rect(100, 138, 120, 14, if self.item == 1 { BLOOD } else { NAVY });
                fb.rect(100, 138, 120, 14, GOLD);
                draw_text_shadow(fb, 106, 141, &c, WHITE, BLACK);
                draw_text_shadow(fb, 78, 166, "M TOGGLES THE SOUND ANYTIME", GRAY, BLACK);
                self.draw_back(fb);
            }
            _ => {}
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

    let persisted = ls_get("doc.mute");
    let muted = Rc::new(Cell::new(match persisted.as_deref() {
        Some("1") => true,
        Some("0") => false,
        _ => query_has("mute=1"),
    }));
    // T405: scanline filter, on unless the player turned it off.
    let crt = Rc::new(Cell::new(ls_get("doc.crt").as_deref() != Some("0")));
    let music = Rc::new(RefCell::new(Music::new()));
    // T402: the overture is queued pre-gesture but silent; the first click or
    // key unlocks the AudioContext and the fanfare starts from bar one.
    if let Ok(m) = Mod::parse(TRACKS[0].1) {
        music.borrow_mut().set_track(Some(Player::new(m)));
    }
    let audio = AudioState {
        ctx: None,
        music: music.clone(),
    };
    let want_test = query_has("scene=test");
    let shot_at: u64 = query_get("shot").and_then(|v| v.parse().ok()).unwrap_or(0);
    let want_demo = query_has("scene=demo");
    let want_play = query_has("scene=play");
    let play_seed: u64 = query_get("seed").and_then(|v| v.parse().ok()).unwrap_or(7);
    let play_diff = if query_has("easy=1") {
        Difficulty::Easy
    } else if query_has("hard=1") {
        Difficulty::Hard
    } else {
        Difficulty::Normal
    };
    // dev deep link: ?scene=mg&mg=<id> boots straight into one sport
    let mg_sport = query_get("scene")
        .filter(|s| s == "mg")
        .and_then(|_| query_get("mg"))
        .and_then(|id| {
            crate::minigames::framework::SPORTS
                .iter()
                .position(|s| s.id == id)
                .and_then(|sid| {
                    crate::ui::host::make_game(crate::minigames::framework::SPORTS[sid].id, play_seed)
                        .map(|g| (sid, g))
                })
        });
    let first: Box<dyn Scene> = if want_test {
        Box::new(TestScene::new("TEST SCENE"))
    } else if want_demo {
        Box::new(DemoScene {
            ticks: 0,
            want_leave: false,
        })
    } else if want_play {
        Box::new(
            PlayScene::new(play_seed, play_diff)
                .with_autoplay(query_has("autoplay=1"))
                .with_turn(query_get("turn").and_then(|v| v.parse().ok()).unwrap_or(0)),
        )
    } else if let Some((sid, game)) = mg_sport {
        Box::new(crate::ui::host::MgHost::new(
            &crate::minigames::framework::SPORTS[sid],
            crate::ui::host::SportMode::Sports { sid },
            game,
            Rc::new(RefCell::new(None)),
            query_has("autoplay=1"),
        ))
    } else {
        Box::new(TitleScene::new())
    };
    let app = App {
        fb: Framebuffer::new(),
        scratch: Framebuffer::new(),
        input: Input::new(),
        stack: SceneStack::new(first),
        mode: if want_test {
            Mode::Test
        } else if want_demo {
            Mode::Demo
        } else if want_play {
            Mode::Play {
                diff: play_diff,
                seed: play_seed,
            }
        } else {
            Mode::Title
        },
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
        perf_acc: 0.0,
        perf_n: 0,
        crt: crt.clone(),
    };
    let state = Rc::new(RefCell::new(app));
    let ctxrc = Rc::new(RefCell::new(Ctx {
        muted,
        crt,
        music,
        audio,
        pending_push: Rc::default(),
    }));

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
            let t0 = js_sys::Date::now();
            while st.acc >= STEP {
                st.acc -= STEP;
                st.frame += 1;
                let frame = st.frame;
                st.stack.step(&mut *ctx, &mut st.input, frame);
                // T501: layered scenes (SPORTS minigames) queue through Ctx.
                let queued = ctx.pending_push.borrow_mut().pop();
                if let Some(s) = queued {
                    st.stack.push(s, Transition::Fade);
                }
                if st.input.user_gesture {
                    let m = ctx.muted.clone();
                    ctx.audio.unlock(&m);
                }
                // T108: gestures fire SFX through the mix
                if st
                    .input
                    .gestures()
                    .iter()
                    .any(|g| matches!(g, Gesture::Tap { .. }))
                {
                    ctx.music.borrow_mut().sfx(Sfx::Blip);
                }
                if st.input.key_pressed(crate::engine::input::KEY_SPACE) {
                    ctx.music.borrow_mut().sfx(Sfx::Coin);
                }
                if matches!(st.mode, Mode::Demo) && frame % 45 == 0 {
                    let s = match (frame / 45) % 3 {
                        0 => Sfx::Coin,
                        1 => Sfx::Sword,
                        _ => Sfx::Hoof,
                    };
                    ctx.music.borrow_mut().sfx(s);
                }
                // mute toggle (persisted, like the OPTIONS toggle)
                if st.input.key_pressed(crate::engine::input::KEY_M) {
                    let v = !ctx.muted.get();
                    ctx.muted.set(v);
                    ls_set("doc.mute", if v { "1" } else { "0" });
                }
                if st.stack.top_wants_leave() && st.stack.depth() > 1 {
                    // a layered scene (minigame) finished: fall back to the
                    // campaign underneath, no mode change.
                    st.stack.pop(Transition::Fade);
                } else if st.stack.top_wants_leave() {

                    let next = match st.mode {
                        Mode::Title => {
                            let diff = match PICK.load(std::sync::atomic::Ordering::Relaxed) {
                                0 => Difficulty::Easy,
                                2 => Difficulty::Hard,
                                _ => Difficulty::Normal,
                            };
                            st.mode = Mode::Play { diff, seed: 7 };
                            Box::new(PlayScene::new(7, diff).with_autoplay(query_has("autoplay=1")))
                                as Box<dyn Scene>
                        }
                        Mode::Test => Box::new(TestScene::new("FADE CYCLE COMPLETE")),
                        Mode::Demo => Box::new(DemoScene {
                            ticks: 0,
                            want_leave: false,
                        }),
                        Mode::Play { diff, seed } => {
                            let seed = seed + 1;
                            st.mode = Mode::Play { diff, seed };
                            Box::new(PlayScene::new(seed, diff).with_autoplay(query_has("autoplay=1")))
                                as Box<dyn Scene>
                        }
                    };
                    st.stack.replace(next, Transition::Fade);
                }
                st.fps_seen += 1;
            }
            st.stack.paint(&mut st.fb, &mut st.scratch);
            st.fb.to_rgba(&mut st.rgba);
            if st.crt.get() {
                // every third scanline darkened: the tube breathes
                for y in (0..H).step_by(3) {
                    let base = (y * W * 4) as usize;
                    for row in st.rgba[base..base + (W * 4) as usize].chunks_exact_mut(4) {
                        for c in row[..3].iter_mut() {
                            *c = (*c as u16 * 2 / 3) as u8;
                        }
                    }
                }
            }
            if let Ok(img) = web_sys::ImageData::new_with_u8_clamped_array(Clamped(&st.rgba), W) {
                let _ = st.ctx2d.put_image_data(&img, 0.0, 0.0);
            }
            st.perf_acc += js_sys::Date::now() - t0;
            st.perf_n += 1;
            if now - st.fps_at >= 1000.0 {
                st.fps_at = now;
                let fps = st.fps_seen;
                st.fps_seen = 0;
                let _ = js_sys::Reflect::set(
                    &w().into(),
                    &JsValue::from_str("__fps"),
                    &JsValue::from(fps),
                );
                if st.perf_n > 0 {
                    let ms = (st.perf_acc * 100.0 / st.perf_n as f64).round() / 100.0;
                    st.perf_acc = 0.0;
                    st.perf_n = 0;
                    let _ = js_sys::Reflect::set(
                        &w().into(),
                        &JsValue::from_str("__ms"),
                        &JsValue::from(ms),
                    );
                }
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
            s.borrow_mut().input.push(Raw::Wheel {
                dy: e.delta_y() as i32,
            });
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
                s.borrow_mut().input.push(Raw::Key {
                    code,
                    pressed: true,
                });
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
                s.borrow_mut().input.push(Raw::Key {
                    code,
                    pressed: false,
                });
            }
        }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
        w().add_event_listener_with_callback("keyup", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }
    let _ = ctx; // ctx reached only through the RAF driver
}

fn ls_get(k: &str) -> Option<String> {
    w().local_storage()
        .ok()
        .flatten()?
        .get_item(k)
        .ok()
        .flatten()
}

fn ls_set(k: &str, v: &str) {
    if let Some(s) = w().local_storage().ok().flatten() {
        let _ = s.set_item(k, v);
    }
}

fn ls_del(k: &str) {
    if let Some(s) = w().local_storage().ok().flatten() {
        let _ = s.remove_item(k);
    }
}
