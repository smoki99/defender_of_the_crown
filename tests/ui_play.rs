//! T301-T304 click contract: taps select, march, recruit and spy through
//! the same hit-testing the browser uses; END TURN hands the world to the AI.
use defender_of_the_crown::campaign::game::{Difficulty, Faction, PLAYER, SPY_COST};
use defender_of_the_crown::ui::{action_rects, end_turn_rect, Btn, CampaignUi, SceneEvent};

fn tap(x: i32, y: i32) -> SceneEvent {
    SceneEvent::Tap { x, y }
}

fn center(b: &Btn) -> (i32, i32) {
    (b.x + b.w as i32 / 2, b.y + b.h as i32 / 2)
}

fn castle_at(ui: &CampaignUi, name: &str) -> (i32, i32) {
    let s =
        ui.g.strongholds
            .iter()
            .find(|s| s.name == name)
            .expect("castle");
    defender_of_the_crown::ui::castle_screen(s)
}

#[test]
fn tapping_a_keep_selecteth_it_and_its_buttons() {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    let (x, y) = castle_at(&ui, "Buckingham");
    ui.event(&tap(x, y), true);
    assert_eq!(ui.sel.map(|i| ui.g.strongholds[i].name), Some("Buckingham"));
    // own panel offers recruits; tapping the levy button buys when affordable
    ui.g.gold[PLAYER.index()] = 500;
    let levy = action_rects(true)
        .into_iter()
        .find(|b| b.id == "rec:0")
        .unwrap();
    let before = ui.g.strongholds[ui.sel.unwrap()].garrison.len();
    let (bx, by) = center(&levy);
    ui.event(&tap(bx, by), true);
    let after = ui.g.strongholds[ui.sel.unwrap()].garrison.len();
    assert_eq!(after, before + 1, "levy not recruited: msg {:?}", ui.msg);
}

#[test]
fn broke_levies_are_denied() {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    let (x, y) = castle_at(&ui, "Buckingham");
    ui.event(&tap(x, y), true);
    ui.g.gold[PLAYER.index()] = 0;
    let levy = action_rects(true)
        .into_iter()
        .find(|b| b.id == "rec:0")
        .unwrap();
    let (bx, by) = center(&levy);
    ui.event(&tap(bx, by), true);
    assert_eq!(
        ui.g.strongholds[ui.sel.unwrap()].garrison.len(),
        12,
        "goldless recruit went through"
    );
    assert!(!ui.msg.is_empty());
}

#[test]
fn tapping_an_adjacent_enemy_marcheth_the_host() {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    let (bx, by) = castle_at(&ui, "Buckingham");
    ui.event(&tap(bx, by), true);
    let from = ui.sel.unwrap();
    let target = *ui.g.adj(from).first().unwrap();
    let (tx, ty) = defender_of_the_crown::ui::castle_screen(&ui.g.strongholds[target]);
    ui.event(&tap(tx, ty), true);
    let dead =
        ui.g.strongholds[from].garrison.is_empty() || ui.g.strongholds[target].owner == PLAYER;
    assert!(dead || !ui.msg.is_empty(), "march neither moved nor spoke");
}

#[test]
fn distant_taps_select_and_end_turn_leteth_the_ai_answer() {
    let mut ui = CampaignUi::new(3, Difficulty::Hard);
    let turn0 = ui.g.turn;
    let (x, y) = castle_at(&ui, "London");
    ui.event(&tap(x, y), true);
    assert_eq!(
        ui.sel.map(|i| ui.g.strongholds[i].name),
        Some("London"),
        "enemy keep should open its info panel"
    );
    let e = end_turn_rect();
    let (ex, ey) = center(&e);
    ui.event(&tap(ex, ey), true);
    assert_eq!(ui.g.turn, turn0 + 1, "one click must be one round");
}

#[test]
fn the_spy_button_purchaseth_eyes() {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    ui.g.gold[PLAYER.index()] = 500;
    let (ox, oy) = castle_at(&ui, "Buckingham");
    ui.event(&tap(ox, oy), true);
    // own keeps cannot be spied: no gold moves
    let spy = action_rects(true)
        .into_iter()
        .find(|b| b.id == "spy")
        .unwrap();
    let (bx, by) = center(&spy);
    let gold0 = ui.g.gi(PLAYER);
    ui.event(&tap(bx, by), true);
    assert_eq!(ui.g.gi(PLAYER), gold0);
    // a dark enemy keep: the engine path the button drives charges 15
    let target = (0..ui.g.strongholds.len())
        .find(|t| ui.g.strongholds[*t].owner == Faction::Norman && ui.g.known[*t][0].is_none())
        .expect("dark norman keep");
    match ui.g.spy(PLAYER, target) {
        Ok(true) => assert_eq!(ui.g.gi(PLAYER), gold0 - SPY_COST),
        Ok(false) => assert_eq!(ui.g.gi(PLAYER), gold0),
        Err(e) => panic!("spy rejected: {e}"),
    }
    assert!(ui.g.known[target][0].is_some());
}
#[test]
fn a_crowned_or_lost_war_offereth_a_new_one() {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    ui.g.over = Some(Faction::Saxon);
    assert!(!ui.want_restart);
    ui.event(&tap(10, 10), true);
    assert!(ui.want_restart, "tap after game end must offer restart");
}

#[test]
fn taps_far_from_every_keep_deselecteth() {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    let (x, y) = castle_at(&ui, "Buckingham");
    ui.event(&tap(x, y), true);
    assert!(ui.sel.is_some());
    ui.event(&tap(318, 4), true); // far corner of the map band
    assert_eq!(ui.sel, None);
}
