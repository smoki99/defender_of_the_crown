//! T401: the campaign scene drives the music bed and the endgame sting.
use defender_of_the_crown::campaign::game::{Difficulty, PLAYER};
use defender_of_the_crown::engine::audio::Sfx;
use defender_of_the_crown::engine::scene::{AppCtx, Scene};
use defender_of_the_crown::ui::{PlayScene, VICTORY_MOD};
use std::cell::RefCell;

struct Ctx {
    log: RefCell<Vec<String>>,
    feats: RefCell<Vec<String>>,
}

impl Ctx {
    fn new() -> Self {
        Self {
            log: RefCell::new(vec![]),
            feats: RefCell::new(vec![]),
        }
    }
    fn drain(&self) -> Vec<String> {
        let l = self.log.borrow_mut().drain(..).collect();
        l
    }
}

impl AppCtx for Ctx {
    fn mute(&self) -> bool {
        false
    }
    fn play_music(&mut self, track: Option<&'static str>) {
        self.log
            .borrow_mut()
            .push(format!("music:{}", track.unwrap_or("-")));
    }
    fn sfx(&mut self, _kind: Sfx) {}
    fn sting(&mut self, track: &'static [u8]) {
        self.log.borrow_mut().push(format!("sting:{}", track.len()));
    }
    fn record_feat(&mut self, feat: &str) {
        self.feats.borrow_mut().push(feat.to_string());
    }
}

#[test]
fn the_campaign_opens_with_the_hall_theme() {
    let mut ps = PlayScene::new(7, Difficulty::Easy);
    let mut ctx = Ctx::new();
    ps.init(&mut ctx);
    assert_eq!(ctx.drain(), vec!["music:saxon"]);
    ps.tick(&mut ctx);
    assert!(
        ctx.drain().is_empty(),
        "steady state must not restart the bed"
    );
}

#[test]
fn the_battle_bed_frames_the_report_and_yields_again() {
    let mut ps = PlayScene::new(7, Difficulty::Easy);
    let mut ctx = Ctx::new();
    ps.ui.report_open = true;
    ps.tick(&mut ctx);
    assert_eq!(ctx.drain(), vec!["music:battle"]);
    ps.ui.report_open = false;
    ps.tick(&mut ctx);
    assert_eq!(ctx.drain(), vec!["music:saxon"]);
}

#[test]
fn victory_stingeth_once_and_recordeth_the_feat() {
    let mut ps = PlayScene::new(7, Difficulty::Easy);
    let mut ctx = Ctx::new();
    ps.ui.g.over = Some(PLAYER);
    ps.tick(&mut ctx);
    assert_eq!(
        ctx.drain(),
        vec!["music:-", &format!("sting:{}", VICTORY_MOD.len())]
    );
    let feats = ctx.feats.borrow().clone();
    assert_eq!(feats.len(), 1);
    assert!(feats[0].starts_with("CROWNED - TURN"), "got {:?}", feats[0]);
    ps.tick(&mut ctx);
    assert!(ctx.drain().is_empty(), "the sting must fire exactly once");
    assert_eq!(ctx.feats.borrow().len(), 1);
}

#[test]
fn defeat_playeth_the_dirge() {
    let mut ps = PlayScene::new(7, Difficulty::Easy);
    let mut ctx = Ctx::new();
    ps.ui.g.over = Some(defender_of_the_crown::campaign::game::Faction::Norman);
    ps.tick(&mut ctx);
    let log = ctx.drain();
    assert_eq!(log[0], "music:-");
    assert!(log[1].starts_with("sting:"), "got {:?}", log);
    assert_ne!(
        log[1],
        format!("sting:{}", VICTORY_MOD.len()),
        "defeat uses a different tune"
    );
    assert!(ctx.feats.borrow()[0].starts_with("CROWN LOST"));
}
