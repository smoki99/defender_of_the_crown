//! T308 enemy dawn march: Norman moves are recorded during the AI turn,
//! replayed for one second as marching dots, the horn sounds at the start,
//! and the battle report / omen overlays wait until the replay has ended.
use defender_of_the_crown::campaign::events::EventKind;
use defender_of_the_crown::campaign::game::{Difficulty, Faction, PLAYER};
use defender_of_the_crown::engine::audio::Sfx;
use defender_of_the_crown::engine::gfx::{Framebuffer, H, NORMAN_RED, W};
use defender_of_the_crown::engine::scene::{AppCtx, Scene};
use defender_of_the_crown::ui::event_ok_rect;
use defender_of_the_crown::ui::{
    castle_screen, end_turn_rect, CampaignUi, PlayScene, SceneEvent, AI_ANIM_TICKS,
};

fn tap(x: i32, y: i32) -> SceneEvent {
    SceneEvent::Tap { x, y }
}

fn end() -> SceneEvent {
    let b = end_turn_rect();
    tap(b.x + b.w as i32 / 2, b.y + b.h as i32 / 2)
}

/// Run the replay out, so whatever the dawn unlocked may open.
fn run_out(ui: &mut CampaignUi) {
    for _ in 0..=AI_ANIM_TICKS {
        if ui.ai_anim == 0 {
            break;
        }
        ui.tick();
    }
}

/// Close whatever the dawn opened (scroll first, then the modal).
fn clear_overlays(ui: &mut CampaignUi) {
    for _ in 0..8 {
        if ui.report_open {
            ui.event(&tap(160, 100), true); // skip, then close
        } else if ui.event_open {
            let b = event_ok_rect();
            ui.event(&tap(b.x + b.w as i32 / 2, b.y + b.h as i32 / 2), true);
        } else {
            break;
        }
    }
}

#[test]
fn the_norman_dawn_marcheth() {
    let mut ui = CampaignUi::new(13, Difficulty::Normal);
    ui.event(&end(), true);
    assert!(!ui.g.ai_moves.is_empty(), "the Normans sat idle all dawn");
    assert_eq!(ui.ai_anim, AI_ANIM_TICKS, "the replay never started");
    // every recorded move began in Norman hands
    for &(a, _) in &ui.g.ai_moves {
        assert_eq!(ui.g.strongholds[a].owner, Faction::Norman);
    }
    // the map heareth no one while the host is on the road
    let own =
        ui.g.strongholds
            .iter()
            .position(|s| s.owner == PLAYER)
            .unwrap();
    let (cx, cy) = castle_screen(&ui.g.strongholds[own]);
    ui.event(&tap(cx, cy), true);
    assert_eq!(ui.sel, None, "taps must not land mid-march");
    // a second of ticks, then England is reachable again
    run_out(&mut ui);
    assert_eq!(ui.ai_anim, 0, "the march outstayed its second");
    clear_overlays(&mut ui);
    ui.event(&tap(cx, cy), true);
    assert_eq!(ui.sel, Some(own), "the map stayed deaf");
}

#[test]
fn marching_banners_painteth_the_map() {
    let mut ui = CampaignUi::new(13, Difficulty::Normal);
    ui.event(&end(), true);
    assert!(!ui.g.ai_moves.is_empty());
    let reds = |fb: &Framebuffer| {
        (0..W)
            .flat_map(|x| (0..H).map(move |y| (x, y)))
            .filter(|&(x, y)| fb.get(x, y) == NORMAN_RED)
            .count()
    };
    let mut fb = Framebuffer::new();
    // freeze at the half, draw, then the same frame without the host
    ui.ai_anim = AI_ANIM_TICKS / 2;
    ui.draw(&mut fb);
    let with = reds(&fb);
    ui.ai_anim = 0;
    ui.draw(&mut fb);
    let without = reds(&fb);
    assert!(
        with > without,
        "marching banners painted nothing: {with} vs {without} red pixels"
    );
}

#[test]
fn the_omen_awaiteth_the_march() {
    let mut ui = CampaignUi::new(13, Difficulty::Normal);
    ui.g.tip = Some(EventKind::Plague);
    ui.event(&end(), true);
    assert!(!ui.g.ai_moves.is_empty());
    assert!(!ui.event_open, "the omen spoke over the marching host");
    if ui.report_pending {
        assert!(!ui.report_open, "the report spoke over the host");
    }
    run_out(&mut ui);
    assert!(ui.event_open, "the omen never announced itself");
    if !ui.report.is_empty() {
        assert!(ui.report_open, "the report never unrolled");
    }
}

/// Records sting calls while the scene ticks.
#[derive(Default)]
struct Horn {
    stings: usize,
    bytes: &'static [u8],
}
impl AppCtx for Horn {
    fn mute(&self) -> bool {
        false
    }
    fn play_music(&mut self, _track: Option<&'static str>) {}
    fn sfx(&mut self, _kind: Sfx) {}
    fn sting(&mut self, track: &'static [u8]) {
        self.bytes = track;
        self.stings += 1;
    }
}

#[test]
fn the_war_horn_soundeth_once_per_dawn() {
    let mut ps = PlayScene::new(13, Difficulty::Normal);
    let mut horn = Horn::default();
    ps.tick(&mut horn);
    assert_eq!(horn.stings, 0, "idle ticks must not summon the horn");
    let b = end_turn_rect();
    ps.event(&SceneEvent::Tap {
        x: b.x + b.w as i32 / 2,
        y: b.y + b.h as i32 / 2,
    });
    ps.tick(&mut horn);
    assert_eq!(horn.stings, 1, "the dawn came without its horn");
    assert_eq!(
        &horn.bytes[..12],
        b"NORMAN STING",
        "the horn blew a corrupt horn"
    );
    for _ in 0..AI_ANIM_TICKS {
        ps.tick(&mut horn);
    }
    assert_eq!(horn.stings, 1, "the horn bleated all march long");
}
