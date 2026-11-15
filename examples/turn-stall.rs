//! Throwaway probe: does the campaign stall after the first END click?
use defender_of_the_crown::campaign::game::Difficulty;
use defender_of_the_crown::ui::{end_turn_rect, CampaignUi, SceneEvent};

fn main() {
    let mut ui = CampaignUi::new(7, Difficulty::Normal);
    let e = end_turn_rect();
    let (ex, ey) = (e.x + e.w as i32 / 2, e.y + e.h as i32 / 2);
    for round in 1..=4 {
        ui.event(&SceneEvent::Tap { x: ex, y: ey }, true);
        for _ in 0..300 {
            ui.tick();
        }
        println!(
            "round {round}: turn {} report {} event {} champ {} ai_anim {} over {:?}",
            ui.g.turn,
            ui.report_open,
            ui.event_open,
            ui.champ_open,
            ui.ai_anim,
            ui.g.over,
        );
        // dismiss whatever the dawn revealed, exactly like the harness
        if ui.report_open {
            ui.event(&SceneEvent::Tap { x: 10, y: 10 }, true);
            ui.event(&SceneEvent::Tap { x: 10, y: 10 }, true);
        }
        if ui.event_open {
            ui.event(&SceneEvent::Tap { x: 160, y: 128 }, true);
        }
        for _ in 0..30 {
            ui.tick();
        }
        println!(
            "  after dismiss: report {} event {} turn {}",
            ui.report_open, ui.event_open, ui.g.turn
        );
    }
}
