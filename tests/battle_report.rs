//! T306 animated battle report: a fight opens the scroll-in overlay, one
//! line per 4 ticks; first tap skips, second closes. Pixel-checked rows.
use defender_of_the_crown::campaign::game::{Difficulty, Faction};
use defender_of_the_crown::campaign::units::{Unit, UnitKind};
use defender_of_the_crown::engine::gfx::{Framebuffer, GREEN, LINEN};
use defender_of_the_crown::ui::{castle_screen, end_turn_rect, Btn, CampaignUi, SceneEvent};

const BY: i32 = 82;

fn tap(x: i32, y: i32) -> SceneEvent {
    SceneEvent::Tap { x, y }
}

fn center(b: &Btn) -> (i32, i32) {
    (b.x + b.w as i32 / 2, b.y + b.h as i32 / 2)
}

fn castle_at(ui: &CampaignUi, name: &str) -> (i32, i32) {
    let id =
        ui.g.strongholds
            .iter()
            .position(|s| s.name == name)
            .unwrap();
    castle_screen(&ui.g.strongholds[id])
}

/// Non-LINEN pixels in report line i (text landed there?).
fn ink(fb: &Framebuffer, line: usize) -> usize {
    let y0 = (BY + 16) as u32 + (line * 10) as u32;
    (y0..y0 + 8)
        .flat_map(|y| (68u32..250).map(move |x| (x, y)))
        .filter(|(x, y)| fb.get(*x, *y) != LINEN)
        .count()
}

fn green_ink(fb: &Framebuffer, line: usize) -> usize {
    let y0 = (BY + 16) as u32 + (line * 10) as u32;
    (y0..y0 + 8)
        .flat_map(|y| (68u32..250).map(move |x| (x, y)))
        .filter(|(x, y)| fb.get(*x, *y) == GREEN)
        .count()
}

/// Buckingham host attacks a first neighbour massed with green Norman levy.
fn battle_setup() -> (CampaignUi, usize) {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    let (bx, by) = castle_at(&ui, "Buckingham");
    ui.event(&tap(bx, by), true);
    let from = ui.sel.unwrap();
    let target = *ui.g.adj(from).first().unwrap();
    ui.g.strongholds[target].owner = Faction::Norman;
    ui.g.strongholds[target].garrison = vec![
        Unit {
            kind: UnitKind::Levy,
            hp: 30
        };
        5
    ];
    (ui, target)
}

#[test]
fn battle_scrollteth_lines_in() {
    let (mut ui, target) = battle_setup();
    let (tx, ty) = castle_screen(&ui.g.strongholds[target]);
    ui.event(&tap(tx, ty), true);
    assert!(ui.report_open, "a fight spawned no report");
    assert_eq!(ui.report_shown, 0, "lines shown before any tick");
    assert!(
        ui.report.iter().any(|l| l.contains("WON")),
        "no WON line in {:?}",
        ui.report
    );

    let mut fb = Framebuffer::new();
    ui.draw(&mut fb);
    assert_eq!(
        ink(&fb, 0),
        0,
        "plate carries text before the scroll starts"
    );

    for _ in 0..4 {
        ui.tick();
    }
    assert_eq!(ui.report_shown, 1, "first line missed its 4-tick slot");
    ui.draw(&mut fb);
    assert!(green_ink(&fb, 0) > 4, "WON line not painted GREEN");
    assert_eq!(ink(&fb, 1), 0, "second line jumped the queue");

    for _ in 0..28 {
        ui.tick();
    }
    assert_eq!(ui.report_shown, ui.report.len(), "scroll never finished");
}

#[test]
fn tap_skipeth_then_closethe() {
    let (mut ui, target) = battle_setup();
    let (tx, ty) = castle_screen(&ui.g.strongholds[target]);
    ui.event(&tap(tx, ty), true);
    assert!(ui.report_open);

    ui.event(&tap(10, 10), true); // skip: every line at once
    assert_eq!(ui.report_shown, ui.report.len(), "skip showed nothing");
    let mut fb = Framebuffer::new();
    ui.draw(&mut fb);
    if ui.report.len() > 1 {
        assert!(ink(&fb, 1) > 4, "skipped lines stayed invisible");
    }

    let turn0 = ui.g.turn;
    ui.event(&tap(10, 10), true); // close
    assert!(!ui.report_open, "second tap did not close");
    ui.event(&tap(10, 10), true); // stray: must NOT burn the turn
    assert_eq!(ui.g.turn, turn0, "closed report ate a tap");

    let (ex, ey) = center(&end_turn_rect());
    ui.event(&tap(ex, ey), true);
    assert!(ui.g.turn > turn0, "report left the map deaf");
}

#[test]
fn a_battleless_tap_openeth_nothing() {
    let mut ui = CampaignUi::new(9, Difficulty::Normal);
    let (bx, by) = castle_at(&ui, "Buckingham");
    ui.event(&tap(bx, by), true);
    ui.tick();
    assert!(!ui.report_open, "selecting a keep faked a battle");
}
