//! T301-T304: campaign playfield UI. Pure logic + painting — no `web_sys` —
//! so the whole click-to-march contract is host-testable. Layout: a 320x200
//! map band, a 56px HUD band (SPEC 7): the selected castle opens a panel
//! whose buttons overlay the map's lower edge.

pub mod kit;

use crate::campaign::game::{
    Difficulty, Faction, GameState, Stronghold, View, BAR_MAX, PLAYER, SPY_COST,
};
use crate::campaign::units::UnitKind;
use crate::engine::gfx::{
    BLACK, BLOOD, CREAM, DARKGRAY, DARKGREEN, DARKRED, DEEPWATER, GOLD, GRASS, GRAY, GREEN, LINEN,
    NAVY, SAXON_BLUE, SILVER, SKY, STONE, TEAL, W, WHITE, YELLOW,
};
pub use crate::engine::scene::SceneEvent;

/// Bottom edge of the map band; the HUD lives below it.
pub const MAP_BOTTOM: i32 = 200;

/// One hit-testable button.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Btn {
    pub id: &'static str,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Btn {
    pub fn hits(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w as i32 && y >= self.y && y < self.y + self.h as i32
    }
}

/// Castle action buttons for a selected stronghold (SPEC 7).
pub fn action_rects(sel_own: bool) -> Vec<Btn> {
    if sel_own {
        vec![
            Btn {
                id: "rec:0",
                x: 6,
                y: 140,
                w: 68,
                h: 14,
            },
            Btn {
                id: "rec:1",
                x: 78,
                y: 140,
                w: 68,
                h: 14,
            },
            Btn {
                id: "rec:2",
                x: 150,
                y: 140,
                w: 68,
                h: 14,
            },
            Btn {
                id: "rec:3",
                x: 222,
                y: 140,
                w: 68,
                h: 14,
            },
            Btn {
                id: "close",
                x: 6,
                y: 158,
                w: 68,
                h: 14,
            },
            Btn {
                id: "spy",
                x: 78,
                y: 158,
                w: 68,
                h: 14,
            },
            Btn {
                id: "sports",
                x: 150,
                y: 158,
                w: 68,
                h: 14,
            },
        ]
    } else {
        vec![
            Btn {
                id: "spy",
                x: 6,
                y: 122,
                w: 102,
                h: 14,
            },
            Btn {
                id: "close",
                x: 112,
                y: 122,
                w: 102,
                h: 14,
            },
        ]
    }
}

/// Fixed HUD control.
pub fn end_turn_rect() -> Btn {
    Btn {
        id: "end",
        x: 214,
        y: 210,
        w: 100,
        h: 14,
    }
}

fn btn_label(id: &str) -> String {
    match id {
        "rec:0" => format!("LEVY {}", UnitKind::Levy.cost()),
        "rec:1" => format!("BOW {}", UnitKind::Bowman.cost()),
        "rec:2" => format!("KNIGHT {}", UnitKind::Knight.cost()),
        "rec:3" => format!("SIEGE {}", UnitKind::Siege.cost()),
        "spy" => format!("SPY {SPY_COST}"),
        "close" => "CLOSE".to_string(),
        "sports" => "SPORTS".to_string(),
        "end" => "END TURN".to_string(),
        other => other.to_string(),
    }
}

/// Screen-space centre of a stronghold (map defs live in a 640x400 field).
pub fn castle_screen(s: &Stronghold) -> (i32, i32) {
    ((s.x * 0.5) as i32, (s.y * 0.5) as i32)
}

/// What the info line says about a castle under the given view.
fn info_line(g: &GameState, f: Faction, s: &Stronghold) -> String {
    let head = format!("{} [{}]", s.name, s.owner.name());
    match g.view(f, s.id) {
        View::Own(i) | View::Spied(i) => {
            format!(
                "{head} {} men walls {}",
                i.counts.iter().sum::<u32>(),
                i.walls
            )
        }
        View::Rough { total } => format!("{head} ~{total} men"),
        View::Dark => format!("{head} UNKNOWN"),
    }
}

/// Garrison list window height in rows (T304).
pub const LIST_ROWS: usize = 4;

/// Short ASCII unit tags: the 8x8 font has no pictographs.
const KIND_TAG: [&str; 5] = ["LEVY", "BOW", "KNIGHT", "SIEGE", "CHAMP"];

fn kind_tag(k: UnitKind) -> &'static str {
    KIND_TAG[k as usize]
}

/// Garrison roll lines for the panel list (T304): exact for known views,
/// fog estimate for rough or hidden garrisons.
fn garrison_rows(g: &GameState, sel: usize) -> Vec<String> {
    let s = &g.strongholds[sel];
    match g.view(PLAYER, sel) {
        View::Own(i) | View::Spied(i) => {
            let mut rows: Vec<String> = crate::campaign::units::ALL_KINDS
                .iter()
                .enumerate()
                .filter(|(k, _)| i.counts[*k] > 0)
                .map(|(k, _)| format!("{} X{}", KIND_TAG[k], i.counts[k]))
                .collect();
            rows.push(format!("WALLS {}", s.walls));
            rows
        }
        View::Rough { total } => vec![format!("~{total} MEN?"), "WALLS ??".to_string()],
        View::Dark => vec!["GARRISON UNKNOWN".to_string(), "WALLS ??".to_string()],
    }
}

/// Whether a control is operable with the current purse (greyed otherwise).
fn btn_enabled(g: &GameState, id: &str) -> bool {
    let gold = g.gi(PLAYER);
    match id {
        "rec:0" => gold >= UnitKind::Levy.cost(),
        "rec:1" => gold >= UnitKind::Bowman.cost(),
        "rec:2" => gold >= UnitKind::Knight.cost(),
        "rec:3" => gold >= UnitKind::Siege.cost(),
        "spy" => gold >= SPY_COST,
        "sports" => false, // minigames arrive with the tourneys (T501)
        _ => true,
    }
}

/// Tooltip line for each hoverable control (T302).
fn tip(id: &str) -> &'static str {
    match id {
        "rec:0" => "MILITIA: CHEAP, MANY, WEAK",
        "rec:1" => "BOWMEN: SHOOT FROM THE WALLS",
        "rec:2" => "KNIGHTS: HEAVY SWORD, LEAD DUELS",
        "rec:3" => "SIEGE: CATAPULTS BREACH WALLS",
        "spy" => "SPY: 7 IN 10 REVEAL A GARRISON",
        "close" => "CLOSE THIS ROLL",
        "sports" => "HELD AT THE TOURNEY GROUND",
        "end" => "SUMMON THE NEXT SEASON",
        "scr:up" | "scr:dn" => "SCROLL THE GARRISON ROLL",
        _ => "",
    }
}

/// The whole in-game screen: map, panel, HUD, endgame overlay.
pub struct CampaignUi {
    pub g: GameState,
    pub sel: Option<usize>,
    pub msg: String,
    msg_ticks: u32,
    pub anim: u32,
    pub want_restart: bool,
    /// Hovered control id for bevel highlight + tooltip (T302).
    pub hover: Option<&'static str>,
    pub hover_xy: (i32, i32),
    pub list_top: usize,
    /// Animated (tweened toward the real meters) bar fills:
    /// [wealth, provisions, loyalty, mood] (T303).
    pub bars_disp: [u32; 4],
    /// Battle report overlay: lines captured from the log, revealed count,
    /// open flag (T306).
    pub report: Vec<String>,
    pub report_shown: usize,
    pub report_open: bool,
}

impl CampaignUi {
    pub fn new(seed: u64, difficulty: Difficulty) -> Self {
        let mut g = GameState::new(seed);
        g.difficulty[1] = difficulty;
        let bars_disp = Self::bar_targets_of(&g);
        Self {
            g,
            sel: None,
            msg: String::new(),
            msg_ticks: 0,
            want_restart: false,
            anim: 0,
            hover: None,
            hover_xy: (0, 0),
            list_top: 0,
            bars_disp,
            report: Vec::new(),
            report_shown: 0,
            report_open: false,
        }
    }

    pub fn tick(&mut self) {
        self.msg_ticks = self.msg_ticks.saturating_sub(1);
        self.anim = self.anim.wrapping_add(1);
        // Bars glide toward the true meters (~8 ticks); the draw pass shows a
        // white tick at the still-unreached target, Amiga-style (T303).
        let tgt = Self::bar_targets_of(&self.g);
        for (d, t) in self.bars_disp.iter_mut().zip(tgt) {
            if *d < t {
                *d = (*d + (t - *d).div_ceil(8)).min(t);
            } else if *d > t {
                *d = (*d - (*d - t).div_ceil(8)).max(t);
            }
        }
        if self.report_open && self.report_shown < self.report.len() && self.anim.is_multiple_of(4) {
            self.report_shown += 1;
        }
    }

    /// Drawn bar values in 0..=BAR_MAX: gold is scaled 200 -> full.
    fn bar_targets_of(g: &GameState) -> [u32; 4] {
        let b = g.bars[PLAYER.index()];
        [
            g.gi(PLAYER).min(2 * BAR_MAX) / 2,
            b.provisions.min(BAR_MAX),
            b.loyalty.min(BAR_MAX),
            b.popularity.min(BAR_MAX),
        ]
    }

    /// North Sea + Channel: hatched shoreline, deep-water fill and teal
    /// wave dashes drifting at anim cadence (T301 coast shading).
    fn draw_sea(&self, fb: &mut crate::engine::gfx::Framebuffer) {
        let drift = (self.anim / 9) % 11;
        for y in 0..MAP_BOTTOM {
            let xe = coast_edge(y);
            fb.fill_rect(xe + 2, y, W - xe as u32 - 2, 1, DEEPWATER);
            for x in xe..xe + 2 {
                if ((x + y) & 1) == 0 {
                    fb.set(x, y, SKY);
                }
            }
            if (y as u32) % 11 == drift {
                fb.hline(xe + 6, xe + 13, y, TEAL);
            }
        }
    }

    /// Static forest and hill sprites (T301).
    fn draw_scenery(&self, fb: &mut crate::engine::gfx::Framebuffer) {
        for (x, y) in HILLS {
            fb.fill_circle(x, y, 4, GRAY);
            fb.fill_circle(x - 1, y - 2, 2, SILVER);
        }
        for (x, y) in TREES {
            fb.tri([(x - 3, y + 3), (x + 3, y + 3), (x, y - 4)], DARKGREEN);
            fb.tri([(x - 2, y + 1), (x + 2, y + 1), (x, y - 5)], GREEN);
        }
    }

    /// One semantic event. `own_gesture` marks real user input (the web
    /// adapter passes true; tests replay taps the same way).
    pub fn event(&mut self, ev: &SceneEvent, _own_gesture: bool) {
        if let SceneEvent::Move { x, y } = *ev {
            self.update_hover(x, y);
            return;
        }
        let SceneEvent::Tap { x, y } = ev else { return };
        let (x, y) = (*x, *y);
        if self.report_open {
            // any tap: first reveals all lines (skip), next closes (T306)
            if self.report_shown < self.report.len() {
                self.report_shown = self.report.len();
            } else {
                self.report_open = false;
            }
            return;
        }
        if let Some(sel) = self.sel {
            if self.g.strongholds[sel].owner == PLAYER {
                if let Some(d) = self.scrollbar(sel).hit(x, y) {
                    let max = garrison_rows(&self.g, sel).len().saturating_sub(LIST_ROWS);
                    self.list_top = (self.list_top as i32 + d).clamp(0, max as i32) as usize;
                    return;
                }
            }
        }
        if self.g.over.is_some() {
            self.want_restart = true;
            return;
        }
        if end_turn_rect().hits(x, y) {
            self.end_turn();
            return;
        }
        if let Some(sel) = self.sel {
            for b in action_rects(self.g.strongholds[sel].owner == PLAYER) {
                if b.hits(x, y) {
                    self.act(b.id, sel);
                    return;
                }
            }
        }
        if y >= MAP_BOTTOM {
            return;
        }
        match self.tap_castle(x, y) {
            Some(id) => {
                let name = self.g.strongholds[id].name;
                if self.g.strongholds[id].owner == PLAYER {
                    self.sel = Some(id);
                    self.list_top = 0;
                } else if let Some(sel) = self.sel {
                    if self.g.adj(sel).contains(&id) {
                        let before = self.g.log.len();
                        let msg = match self.g.march(PLAYER, sel, id) {
                            Ok(()) => format!("THE HOST MARCHES ON {name}"),
                            Err(e) => e,
                        };
                        self.capture_report(before);
                        self.say(&msg);
                    } else {
                        self.say("NO ROAD FROM YOUR HOST TO YONDER KEEP");
                    }
                } else {
                    self.sel = Some(id);
                    self.list_top = 0;
                }
            }
            None => {
                self.sel = None;
                self.list_top = 0;
                self.msg.clear();
                self.msg_ticks = 0;
            }
        }
    }

    /// Any battle log lines spawned by the action that began at log index
    /// `before` open the animated report overlay (T306).
    fn capture_report(&mut self, before: usize) {
        if self.g.over.is_some() {
            return;
        }
        let fresh = &self.g.log[before..];
        if fresh.iter().any(|l| l.contains("ROUNDS.")) {
            self.report = fresh.to_vec();
            self.report_shown = 0;
            self.report_open = true;
        }
    }

    /// Garrison scrollbar geometry (T304).
    fn scrollbar(&self, sel: usize) -> kit::Scrollbar {
        kit::Scrollbar {
            x: 200,
            y: 100,
            h: 36,
            top: self.list_top,
            total: garrison_rows(&self.g, sel).len(),
            rows: LIST_ROWS,
        }
    }

    /// Pointer-move: remember the hovered control for bevel + tooltip.
    fn update_hover(&mut self, x: i32, y: i32) {
        self.hover_xy = (x, y);
        let mut h: Option<&'static str> = if end_turn_rect().hits(x, y) {
            Some("end")
        } else {
            None
        };
        if let Some(sel) = self.sel {
            let own = self.g.strongholds[sel].owner == PLAYER;
            if own {
                let scr = self.scrollbar(sel);
                if scr.visible() {
                    if scr.up().hits(x, y) {
                        h = Some("scr:up");
                    } else if scr.down().hits(x, y) {
                        h = Some("scr:dn");
                    }
                }
            }
            if h.is_none() {
                h = action_rects(own)
                    .into_iter()
                    .find(|b| b.hits(x, y))
                    .map(|b| b.id);
            }
        }
        self.hover = h;
    }

    /// Nearest stronghold within tap reach, if any.
    fn tap_castle(&self, x: i32, y: i32) -> Option<usize> {
        let mut best: Option<(i64, usize)> = None;
        for s in &self.g.strongholds {
            let (cx, cy) = castle_screen(s);
            let dx = (x - cx) as i64;
            let dy = (y - cy) as i64;
            let d = dx * dx + dy * dy;
            if d <= 144 && best.is_none_or(|(b, _)| d < b) {
                best = Some((d, s.id));
            }
        }
        best.map(|(_, id)| id)
    }

    fn act(&mut self, id: &str, sel: usize) {
        if id == "close" {
            self.sel = None;
            return;
        }
        let outcome = if id == "spy" {
            match self.g.spy(PLAYER, sel) {
                Ok(true) => "SPIES SLIP AWAY WITH WORD".to_string(),
                Ok(false) => "EYES THERE ALREADY".to_string(),
                Err(e) => e,
            }
        } else if let Some(kind) = id
            .strip_prefix("rec:")
            .and_then(|i| UnitKind::from_index(i.parse().unwrap_or(9)))
        {
            match self.g.recruit(PLAYER, sel, kind) {
                Ok(()) => format!("{} JOIN THE GARRISON", kind_tag(kind)),
                Err(e) => e,
            }
        } else if id == "sports" {
            self.say("THE SPORTS ARE HELD AT THE TOURNEY");
            return;
        } else {
            return;
        };
        self.say(&outcome);
    }

    fn end_turn(&mut self) {
        self.sel = None;
        let before = self.g.log.len();
        self.g.end_player_turn();
        if self.g.over.is_none() {
            crate::campaign::ai::take_turn(&mut self.g, PLAYER.other());
        }
        self.g.end_player_turn();
        self.capture_report(before);
        let tail = self.g.log.last().cloned().unwrap_or_default();
        self.say(&tail);
    }

    fn say(&mut self, m: &str) {
        self.msg = m.to_string();
        self.msg_ticks = 240;
    }

    // ---------------------------------------------------------------- paint

    pub fn draw(&mut self, fb: &mut crate::engine::gfx::Framebuffer) {
        use crate::engine::fonts::draw_text_shadow;
        fb.clear(SKY);
        fb.gradient_v(0, 0, W, MAP_BOTTOM as u32, SKY, GRASS);
        self.draw_sea(fb);
        self.draw_scenery(fb);
        // roads
        for (a, b) in crate::campaign::map::roads() {
            let (ax, ay) = castle_screen(&self.g.strongholds[a]);
            let (bx, by) = castle_screen(&self.g.strongholds[b]);
            draw_line(fb, ax, ay, bx, by, STONE);
        }
        // castles
        for s in &self.g.strongholds {
            let (cx, cy) = castle_screen(s);
            let v = self.g.view(PLAYER, s.id);
            let hidden = matches!(v, View::Dark) && s.owner != PLAYER;
            let body = match s.owner {
                PLAYER => SAXON_BLUE,
                Faction::Norman => NORMAN_KEEP,
                Faction::Neutral => GRAY,
            };
            let body = if hidden { DARKGRAY } else { body };
            // keep: 14x12 block with two crenellations, capital gets a crown
            fb.fill_rect(cx - 7, cy - 4, 14, 10, body);
            fb.fill_rect(cx - 7, cy - 7, 4, 3, body);
            fb.fill_rect(cx + 3, cy - 7, 4, 3, body);
            fb.fill_rect(cx - 2, cy, 4, 6, BLACK);
            if s.capital {
                fb.set(cx, cy - 9, GOLD);
                fb.set(cx - 2, cy - 8, GOLD);
                fb.set(cx + 2, cy - 8, GOLD);
            }
            if !hidden {
                // two-frame banner fluttering from the left crenel (T301).
                fb.vline(cx - 5, cy - 13, cy - 8, BLACK);
                if ((self.anim / 8) & 1) == 0 {
                    fb.fill_rect(cx - 4, cy - 13, 3, 2, body);
                } else {
                    fb.fill_rect(cx - 4, cy - 12, 3, 2, body);
                }
            }
            if s.port && !hidden {
                fb.set(cx + 9, cy + 1, GOLD);
                fb.set(cx + 9, cy + 2, GOLD);
            }
            if self.sel == Some(s.id) {
                // two-frame pulse: the ring breathes yellow -> gold (T301).
                let p = ((self.anim / 8) & 1) == 0;
                let d: i32 = if p { 0 } else { 1 };
                fb.rect(
                    cx - 9 - d,
                    cy - 9 - d,
                    (20 + 2 * d) as u32,
                    (18 + 2 * d) as u32,
                    if p { YELLOW } else { GOLD },
                );
            }
            let label = if hidden {
                "??".to_string()
            } else {
                s.name.to_string()
            };
            draw_text_shadow(
                fb,
                cx - label.len() as i32 * 3,
                cy + 8,
                &label,
                WHITE,
                BLACK,
            );
            if !hidden {
                let n = match &v {
                    View::Own(i) | View::Spied(i) => i.counts.iter().sum::<u32>(),
                    View::Rough { total } => *total,
                    View::Dark => 0,
                };
                let t = n.to_string();
                draw_text_shadow(fb, cx - t.len() as i32 * 3 + 1, cy - 16, &t, YELLOW, BLACK);
            }
        }
        self.draw_hud(fb);
        if let Some(sel) = self.sel {
            self.draw_panel(fb, sel);
        }
        if let Some(id) = self.hover {
            let text = tip(id);
            if !text.is_empty() {
                kit::tooltip(fb, self.hover_xy.0 + 8, self.hover_xy.1 + 10, text);
            }
        }
        if let Some(winner) = self.g.over {
            self.draw_over(fb, winner);
        }
        self.draw_report(fb);
    }

    fn draw_hud(&mut self, fb: &mut crate::engine::gfx::Framebuffer) {
        use crate::engine::fonts::draw_text_shadow;
        fb.fill_rect(0, MAP_BOTTOM, W, 56, NAVY);
        fb.hline(0, W as i32 - 1, MAP_BOTTOM, GOLD);
        let me = self.g.gi(PLAYER);
        draw_text_shadow(fb, 4, 204, &format!("GOLD {me}"), GOLD, BLACK);
        let tgt = Self::bar_targets_of(&self.g);
        let disp = self.bars_disp;
        bar(fb, 4, 214, "WEALTH", disp[0], tgt[0], GOLD);
        bar(fb, 84, 214, "FOOD", disp[1], tgt[1], GREEN);
        bar(fb, 4, 224, "LOYAL", disp[2], tgt[2], SAXON_BLUE);
        bar(fb, 84, 224, "MOOD", disp[3], tgt[3], YELLOW);
        draw_text_shadow(
            fb,
            120,
            204,
            &format!("TURN {} ARMY {}", self.g.turn, self.g.troops_of(PLAYER)),
            WHITE,
            BLACK,
        );
        let e = end_turn_rect();
        kit::button(fb, &e, self.hover == Some("end"), true);
        draw_text_shadow(fb, e.x + 24, e.y + 4, "END TURN", WHITE, BLACK);
        let line = if self.msg_ticks > 0 || !self.msg.is_empty() {
            self.msg.clone()
        } else {
            self.g.log.last().cloned().unwrap_or_default()
        };
        let line: String = line.chars().take(44).collect();
        draw_text_shadow(fb, 4, 246, &line, CREAM, BLACK);
    }

    fn draw_panel(&mut self, fb: &mut crate::engine::gfx::Framebuffer, sel: usize) {
        use crate::engine::fonts::draw_text_shadow;
        let s = &self.g.strongholds[sel];
        let own = s.owner == PLAYER;
        let info = info_line(&self.g, PLAYER, s);
        kit::panel(fb, 4, 86, 292, if own { 86 } else { 52 });
        draw_text_shadow(fb, 8, 90, &info, WHITE, BLACK);
        let rows = garrison_rows(&self.g, sel);
        let top = self.list_top.min(rows.len().saturating_sub(LIST_ROWS));
        for (i, row) in rows.iter().skip(top).take(LIST_ROWS).enumerate() {
            draw_text_shadow(fb, 10, 100 + i as i32 * 9, row, CREAM, BLACK);
        }
        if own {
            let scr = self.scrollbar(sel);
            if scr.visible() {
                scr.draw(fb, self.hover);
            }
        }
        for b in action_rects(own) {
            let enabled = btn_enabled(&self.g, b.id);
            kit::button(fb, &b, self.hover == Some(b.id), enabled);
            let label = btn_label(b.id);
            draw_text_shadow(
                fb,
                b.x + (b.w as i32 - label.len() as i32 * 6) / 2,
                b.y + 4,
                &label,
                if enabled { WHITE } else { BLACK },
                BLACK,
            );
        }
    }
    fn draw_over(&mut self, fb: &mut crate::engine::gfx::Framebuffer, winner: Faction) {
        use crate::engine::fonts::draw_text_shadow;
        fb.fill_rect(0, 0, W, 256, BLACK);
        let (head, c) = if winner == PLAYER {
            ("THE CROWN ENDURES - CROWNED!", GOLD)
        } else {
            ("THE CROWN IS LOST", BLOOD)
        };
        draw_text_shadow(
            fb,
            (W as i32 - head.len() as i32 * 6) / 2,
            100,
            head,
            c,
            BLACK,
        );
        draw_text_shadow(
            fb,
            (W as i32 - "A CHRONICLE OF SAXON ENGLAND".len() as i32 * 6) / 2,
            120,
            "A CHRONICLE OF SAXON ENGLAND",
            WHITE,
            BLACK,
        );
        if self.g.turn % 60 < 40 {
            let t = "CLICK FOR A NEW WAR";
            draw_text_shadow(
                fb,
                (W as i32 - t.len() as i32 * 6) / 2,
                150,
                t,
                SILVER,
                BLACK,
            );
        }
    }

    /// Amiga-style scroll-in battle report: LINEN plate, one log line every
    /// 4 ticks, GREEN win / NORMAN_RED loss, tap skips or closes (T306).
    fn draw_report(&self, fb: &mut crate::engine::gfx::Framebuffer) {
        if !self.report_open {
            return;
        }
        use crate::engine::fonts::draw_text_shadow;
        let n = self.report.len().min(6);
        let (bx, by) = (60i32, 82i32);
        kit::panel(fb, bx, by, 200, 16 + (n as u32) * 10);
        fb.fill_rect(bx + 2, by + 2, 196, 12 + (n as u32) * 10, LINEN);
        draw_text_shadow(fb, bx + 8, by + 4, "BATTLE REPORT", NAVY, LINEN);
        for (i, line) in self
            .report
            .iter()
            .take(n)
            .take(self.report_shown)
            .enumerate()
        {
            let c = if line.contains("WON") {
                GREEN
            } else if line.contains("REPULSED") || line.contains("CHAMPION") {
                NORMAN_KEEP
            } else {
                BLACK
            };
            draw_text_shadow(fb, bx + 8, by + 16 + (i as i32) * 10, line, c, LINEN);
        }
    }
}

const NORMAN_KEEP: u8 = DARKRED;

/// East/south coastline as screen-space x per y: piecewise linear through
/// the Norfolk bulge, the London bight, Dover and the sweep into the
/// Channel corner (T301).
fn coast_edge(y: i32) -> i32 {
    const P: [(i32, i32); 10] = [
        (0, 226),
        (40, 224),
        (70, 208),
        (95, 206),
        (110, 216),
        (135, 222),
        (150, 210),
        (170, 194),
        (190, 176),
        (201, 168),
    ];
    for w in P.windows(2) {
        if y <= w[1].0 {
            let (ya, xa) = w[0];
            let (yb, xb) = w[1];
            return xa + (y - ya) * (xb - xa) / (yb - ya);
        }
    }
    P[P.len() - 1].1
}

/// Forest clusters: Sherwood, New Forest, Essex, far southwest.
const TREES: [(i32, i32); 10] = [
    (150, 30),
    (159, 36),
    (151, 43),
    (55, 146),
    (64, 150),
    (73, 146),
    (196, 50),
    (204, 57),
    (18, 162),
    (28, 168),
];

/// Hills: Midlands knolls plus the Kent/Channel ridge.
const HILLS: [(i32, i32); 5] = [(120, 58), (130, 52), (185, 135), (195, 143), (188, 166)];

fn bar(
    fb: &mut crate::engine::gfx::Framebuffer,
    x: i32,
    y: i32,
    label: &str,
    v: u32,
    tgt: u32,
    c: u8,
) {
    use crate::engine::fonts::draw_text_shadow;
    draw_text_shadow(fb, x, y, label, CREAM, BLACK);
    fb.rect(x + 34, y, 42, 6, GRAY);
    let w = (v.min(BAR_MAX) as i32 * 40) / BAR_MAX as i32;
    fb.fill_rect(x + 35, y + 1, w.max(0) as u32, 4, c);
    if tgt != v {
        // White tick where the still-gliding fill has to arrive (T303).
        let mx = x + 35 + (tgt.min(BAR_MAX) as i32 * 40) / BAR_MAX as i32;
        fb.fill_rect(mx, y, 1, 6, WHITE);
    }
}

fn draw_line(
    fb: &mut crate::engine::gfx::Framebuffer,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    c: u8,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        fb.set(x0, y0, c);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}
/// Thin adapter so the campaign screen rides the engine's `SceneStack`.
pub struct PlayScene {
    pub ui: CampaignUi,
}

impl PlayScene {
    pub fn new(seed: u64, difficulty: Difficulty) -> Self {
        Self {
            ui: CampaignUi::new(seed, difficulty),
        }
    }
}

impl crate::engine::scene::Scene for PlayScene {
    fn event(&mut self, ev: &SceneEvent) {
        self.ui.event(ev, true);
    }
    fn tick(&mut self) {
        self.ui.tick();
    }
    fn draw(&mut self, fb: &mut crate::engine::gfx::Framebuffer) {
        self.ui.draw(fb);
    }
    fn wants_leave(&self) -> bool {
        self.ui.want_restart
    }
}
