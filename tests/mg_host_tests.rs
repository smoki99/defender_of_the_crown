//! T501: `MgHost` semantics (pause, resolve-on-end, cheer timer, exit) and
//! the campaign-side flow (outcome mailbox -> purse).

use defender_of_the_crown::campaign::game::{Difficulty, Stronghold, Tier, PLAYER, SPORTS_FEE};
use defender_of_the_crown::engine::gfx::Framebuffer;
use defender_of_the_crown::engine::input::{Input, KEY_ENTER};
use defender_of_the_crown::engine::scene::{AppCtx, Scene, SceneEvent};
use defender_of_the_crown::minigames::framework::{
    MgOutcome, MgState, Minigame, SportSpec, SPORTS,
};
use defender_of_the_crown::ui::host::{MgHost, OutcomeCell, SportMode};
use defender_of_the_crown::ui::{CampaignUi, PlayScene};
use std::cell::RefCell;
use std::rc::Rc;

const KEY_P: u32 = b'p' as u32;

/// Dummy sport: +1 point per advancing tick; optionally ends (Gold) at N.
struct Dmg {
    t: u32,
    end_at: Option<u32>,
}

impl Dmg {
    fn boxed(end_at: Option<u32>) -> Box<dyn Minigame> {
        Box::new(Self { t: 0, end_at })
    }
}

impl Minigame for Dmg {
    fn id(&self) -> &'static str {
        SPORTS[0].id
    }
    fn tick(&mut self, dt: f32, _i: &Input, _fb: &mut Framebuffer) {
        if dt > 0.0 {
            self.t += 1;
        }
    }
    fn score(&self) -> u32 {
        self.t
    }
    fn state(&self) -> MgState {
        match self.end_at {
            Some(n) if self.t >= n => MgState::Won(Tier::Gold),
            _ => MgState::Running,
        }
    }
}

struct NullApp {
    pushed: Rc<RefCell<Vec<usize>>>,
    best: Rc<RefCell<Vec<(String, u32)>>>,
}

impl NullApp {
    fn new() -> Self {
        Self {
            pushed: Rc::default(),
            best: Rc::default(),
        }
    }
}

impl AppCtx for NullApp {
    fn mute(&self) -> bool {
        true
    }
    fn play_music(&mut self, _t: Option<&'static str>) {}
    fn sfx(&mut self, _k: defender_of_the_crown::engine::audio::Sfx) {}
    fn push(&mut self, _s: Box<dyn Scene>) {
        self.pushed.borrow_mut().push(1);
    }
    fn mg_best(&mut self, id: &str, score: u32) {
        self.best.borrow_mut().push((id.to_string(), score));
    }
}

/// A spec whose `gold_at` a 10-tick dummy can clear (or the easiest there is).
fn spec_win() -> &'static SportSpec {
    SPORTS
        .iter()
        .find(|s| s.gold_at <= 10)
        .unwrap_or_else(|| SPORTS.iter().min_by_key(|s| s.gold_at).unwrap())
}

fn pump(h: &mut MgHost, fb: &mut Framebuffer, n: u32) {
    for _ in 0..n {
        h.draw(fb);
    }
}

fn key(h: &mut MgHost, code: u32) {
    // a real keyboard gives both edges; the input model needs the release
    // before the next press registers.
    h.event(&SceneEvent::Key {
        code,
        pressed: true,
    });
    h.event(&SceneEvent::Key {
        code,
        pressed: false,
    });
}

#[test]
fn host_resolves_on_engine_end_and_leaves_only_after_enter() {
    let cell: OutcomeCell = Rc::default();
    let mut h = MgHost::new(
        spec_win(),
        SportMode::Sports { sid: 0 },
        Dmg::boxed(Some(10)),
        cell.clone(),
        false,
    );
    let mut fb = Framebuffer::new();
    pump(&mut h, &mut fb, 30);
    let o = cell.borrow().expect("outcome written at end state");
    assert_eq!(o.score, 10);
    assert_eq!(o.tier, Some(Tier::Gold));
    assert!(
        !h.wants_leave(),
        "results screen first, leave only after ENTER"
    );
    key(&mut h, KEY_ENTER);
    pump(&mut h, &mut fb, 1);
    assert!(h.wants_leave());
}

#[test]
fn cheer_timeout_resolves_and_saves_best() {
    let spec = SPORTS.iter().min_by_key(|s| s.silver_at).unwrap();
    let cell: OutcomeCell = Rc::default();
    let mut h = MgHost::new(
        spec,
        SportMode::Sports { sid: 1 },
        Dmg::boxed(None),
        cell.clone(),
        false,
    );
    let mut fb = Framebuffer::new();
    let mut app = NullApp::new();
    pump(&mut h, &mut fb, 61 * 60); // past the 60 s cheer
    let o = cell.borrow().expect("timer resolves a Running game");
    assert!(o.score > 0);
    h.tick(&mut app);
    assert_eq!(app.best.borrow().len(), 1, "one best-score write");
    assert_eq!(app.best.borrow()[0].1, o.score);
}

#[test]
fn pause_freezes_the_core_until_resumed() {
    let cell: OutcomeCell = Rc::default();
    let mut h = MgHost::new(
        spec_win(),
        SportMode::Sports { sid: 0 },
        Dmg::boxed(None),
        cell.clone(),
        false,
    );
    let mut fb = Framebuffer::new();
    pump(&mut h, &mut fb, 5);
    let before = h.game_score();
    key(&mut h, KEY_P); // pause
    pump(&mut h, &mut fb, 20);
    assert_eq!(
        h.game_score(),
        before,
        "paused ticks must not advance the core"
    );
    key(&mut h, KEY_P); // resume
    pump(&mut h, &mut fb, 3);
    assert!(h.game_score() > before, "resumed ticks advance the core");
    pump(&mut h, &mut fb, 61 * 60); // let the cheer resolve it
    assert!(
        cell.borrow().is_some(),
        "cheer timer must resolve the round"
    );
    assert!(!h.wants_leave(), "results screen first");
    key(&mut h, KEY_ENTER);
    pump(&mut h, &mut fb, 1);
    assert!(h.wants_leave());
}

#[test]
fn autoplay_resolves_immediately_and_exits() {
    let cell: OutcomeCell = Rc::default();
    let mut h = MgHost::new(
        spec_win(),
        SportMode::Herald { sid: 2 },
        Dmg::boxed(None),
        cell.clone(),
        true,
    );
    let mut fb = Framebuffer::new();
    pump(&mut h, &mut fb, 1);
    let b = cell.borrow();
    let o = b.as_ref().expect("autoplay resolves at once");
    assert_eq!(o.score, spec_win().gold_at, "outcome must carry the gold line score, not the unplayed core's 0");
    assert_eq!(o.tier, Some(Tier::Gold), "resolving at the gold line is a gold tier");
    pump(&mut h, &mut fb, 95);
    assert!(h.wants_leave(), "autoplay leaves the results screen itself");
}

fn owned_castle(ui: &CampaignUi) -> usize {
    ui.g.strongholds
        .iter()
        .position(|s: &Stronghold| s.owner == PLAYER)
        .unwrap()
}

#[test]
fn sports_flow_applies_the_purse_after_the_host_returns() {
    let mut ui = CampaignUi::new(7, Difficulty::Easy);
    let sid = owned_castle(&ui);
    let before = ui.g.gi(PLAYER);
    assert!(before >= SPORTS_FEE, "fixture purse covers the fee");
    let spec = spec_win();
    ui.sport_ctx = Some((SportMode::Sports { sid }, spec));
    ui.sport_out.borrow_mut().replace(MgOutcome {
        id: spec.id,
        score: spec.gold_at,
        tier: Some(Tier::Gold),
    });
    let mut ps = PlayScene::new(7, Difficulty::Easy);
    ps.ui = ui;
    let mut app = NullApp::new();
    ps.tick(&mut app);
    assert!(app.pushed.borrow().is_empty(), "no launch without a game");
    let after = ps.ui.g.gi(PLAYER);
    assert!(after > before, "gold purse landed: {before} -> {after}");
    assert!(ps.ui.sport_ctx.is_none(), "context consumed");
    assert!(ps.ui.sport_out.borrow().is_none(), "mailbox drained");
}

#[test]
fn failed_meet_charges_nothing() {
    let mut ui = CampaignUi::new(9, Difficulty::Easy);
    let sid = owned_castle(&ui);
    let before = ui.g.gi(PLAYER);
    let spec = &SPORTS[0];
    ui.sport_ctx = Some((SportMode::Sports { sid }, spec));
    ui.sport_out.borrow_mut().replace(MgOutcome {
        id: spec.id,
        score: 0,
        tier: None,
    });
    let mut ps = PlayScene::new(9, Difficulty::Easy);
    ps.ui = ui;
    let mut app = NullApp::new();
    ps.tick(&mut app);
    assert_eq!(ps.ui.g.gi(PLAYER), before, "a lost meet is free");
}

#[test]
fn sports_button_click_without_a_shipped_game_stays_honest() {
    let mut ui = CampaignUi::new(11, Difficulty::Easy);
    let sid = owned_castle(&ui);
    ui.sel = Some(sid);
    // season whose posted meet (falconry) still has no shipped game
    ui.g.turn = 11;
    ui.act("sports", sid);
    assert!(ui.launch_req.is_none(), "nothing to launch yet");
    assert!(
        ui.msg.contains("TOURNEY") || ui.msg.contains("HERALD") || ui.msg.contains("SEASON"),
        "message explains the closed gate: {}",
        ui.msg
    );
}

/// Regression: the web driver dispatches `event()` before `draw()`.
/// MgHost used to `begin_frame` AFTER draining its stash, wiping the fresh
/// key_pressed the same frame - the plate never saw ENTER and the host
/// stayed open forever over the campaign.
#[test]
fn driver_enter_key_closes_results_plate() {
    use defender_of_the_crown::ui::host::make_game;
    let cell: OutcomeCell = Rc::new(RefCell::new(None));
    let spec = SPORTS.iter().find(|s| s.id == "sword").unwrap();
    let mut host = MgHost::new(
        spec,
        SportMode::Sports { sid: 2 },
        make_game("sword", 7).unwrap(),
        cell.clone(),
        true,
    );
    let mut fb = Framebuffer::new();
    for _ in 0..90 {
        host.draw(&mut fb);
    }
    assert!(!host.wants_leave(), "plate waits for an explicit ack");
    host.event(&SceneEvent::Key {
        code: KEY_ENTER,
        pressed: true,
    });
    host.draw(&mut fb);
    assert!(host.wants_leave(), "ENTER on the plate must close the host");
}
