//! Dumps march-relevant stronghold state for the browser repro:
//! every Saxon keep with its screen position, march readiness and the
//! adjacent Norman targets (also screen-positioned).
use defender_of_the_crown::campaign::game::{Difficulty, Faction};
use defender_of_the_crown::ui::{castle_screen, CampaignUi};

fn main() {
    let ui = CampaignUi::new(3, Difficulty::Normal);
    let g = &ui.g;
    for s in g.strongholds.iter().filter(|s| s.owner == Faction::Saxon) {
        let (x, y) = castle_screen(s);
        let targets: Vec<String> = s
            .adj
            .iter()
            .map(|&t| {
                let e = &g.strongholds[t];
                let (tx, ty) = castle_screen(e);
                format!(
                    "{}@{},{}({})",
                    e.name,
                    tx,
                    ty,
                    g.can_march(Faction::Saxon, s.id, t).is_ok()
                )
            })
            .collect();
        println!(
            "KEEP {} @ {},{} g={} targets={}",
            s.name,
            x,
            y,
            s.garrison.len(),
            targets.join(" ")
        );
    }
}
