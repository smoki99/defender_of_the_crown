//! T301-T304: campaign playfield UI. Pure logic + painting — no `web_sys` —
//! so the whole click-to-march contract is host-testable. Layout: a 320x200
//! map band, a 56px HUD band (SPEC 7): the selected castle opens a panel
//! whose buttons overlay the map's lower edge.

pub mod kit;
pub mod host;

pub use host::{MgHost, OutcomeCell, SportMode};

use crate::minigames::framework::{SportSpec, pick as sport_pick, SPORTS};
use std::rc::Rc;

use crate::campaign::events::EventKind;
use crate::campaign::game::{
    Difficulty, Faction, GameState, Stronghold, View, BAR_MAX, PLAYER, SPY_COST, SPORTS_FEE,
};
use crate::campaign::units::UnitKind;
use crate::engine::gfx::{
    BLACK, BLOOD, CREAM, DARKGRAY, DARKGREEN, DARKRED, DEEPWATER, EMBER, GOLD, GRASS, GRAY,
    GREEN, LINEN,
    NAVY, NORMAN_RED, SAXON_BLUE, SILVER, SKY, STONE, TEAL, W, WHITE, YELLOW,
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

/// Castle action buttons for a selected stronghold (SPEC 7). `hoard` adds
/// the T704 rare-event row while the dragon's omen stands.
pub fn action_rects(sel_own: bool, hoard: bool) -> Vec<Btn> {
    if sel_own {
        let mut v = vec![
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
                w: 84,
                h: 14,
            },
        ];
        if hoard {
            v.push(Btn {
                id: "hoard",
                x: 238,
                y: 158,
                w: 54,
                h: 14,
            });
        }
        v
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

/// Dismiss button of the omen modal (T307).
pub fn event_ok_rect() -> Btn {
    Btn {
        id: "ok",
        x: 140,
        y: 122,
        w: 40,
        h: 12,
    }
}

fn btn_label(g: &GameState, id: &str) -> String {
    match id {
        "rec:0" => format!("LEVY {}", UnitKind::Levy.cost()),
        "rec:1" => format!("BOW {}", UnitKind::Bowman.cost()),
        "rec:2" => format!("KNIGHT {}", UnitKind::Knight.cost()),
        "rec:3" => format!("SIEGE {}", UnitKind::Siege.cost()),
        "spy" => format!("SPY {SPY_COST}"),
        "close" => "CLOSE".to_string(),
        "sports" => season_sport(g).map_or("NO MEET", |s| s.label).to_string(),
        "hoard" => "HOARD".to_string(),
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

/// The sport this season's meet holds, or `None` when none is held. The
/// draw is deterministic in the turn (T706 pacing): turns 1-4 see the four
/// starter sports, 5-9 add the mid era, 10-12 the late trio; the hoard is
/// rare-event only (`unlock_turn == u32::MAX`, never drawn). The button
/// shows it so the era pacing is visible before the fee is spent.
pub fn season_sport(g: &GameState) -> Option<&'static crate::minigames::framework::SportSpec> {
    let seed = ((g.turn as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15;
    sport_pick(g.turn, seed)
}

/// Whether a control is operable with the current purse (greyed otherwise).
fn btn_enabled(g: &GameState, id: &str) -> bool {
    let gold = g.gi(PLAYER);
    match id {
        "rec:0" => gold >= UnitKind::Levy.cost(),
        "rec:1" => gold >= UnitKind::Bowman.cost(),
        "rec:2" => gold >= UnitKind::Knight.cost(),
        "rec:3" => gold >= UnitKind::Siege.cost(),
        "sports" => season_sport(g).is_some() && gold >= SPORTS_FEE,
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
        "sports" => "FEAT OF ARMS, FEE TEN GOLD",
        "hoard" => "ROB THE DRAGON - ONCE IN ALL THE GAME",
        "end" => "SUMMON THE NEXT SEASON",
        "scr:up" | "scr:dn" => "SCROLL THE GARRISON ROLL",
        _ => "",
    }
}

/// One enemy-march replay in 60 Hz ticks - a second, never longer
/// than the SPEC's 2 s (T308).
pub const AI_ANIM_TICKS: u32 = 60;

/// The Norman turn-call, played once per AI dawn (T308).
pub const NORMAN_STING: &[u8] = include_bytes!("../../assets/music/norman.mod");
/// T401 endgame one-shots (played instead of the looping bed).
pub const VICTORY_MOD: &[u8] = include_bytes!("../../assets/music/victory.mod");
pub const DEFEAT_MOD: &[u8] = include_bytes!("../../assets/music/defeat.mod");

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
    /// Omen modal: announced event, open flag, last announced turn (T307).
    pub event_kind: Option<EventKind>,
    pub event_open: bool,
    event_turn: u32,
    /// Dawn march animation (T308): ticks left, overlays deferred behind
    /// it, and the horn flag the scene hands to `AppCtx::sting`.
    pub ai_anim: u32,
    pub report_pending: bool,
    pending_event: Option<EventKind>,
    /// Triple-crown champion modal open (T707).
    pub champ_open: bool,
    pub sting_armed: bool,
    /// T501: SPORTS launch request (spec, mode), consumed by the scene layer.
    pub launch_req: Option<(&'static SportSpec, SportMode)>,
    /// Outcome mailbox written by the layered `MgHost` on exit.
    pub sport_out: OutcomeCell,
    /// Run in flight: mode + spec, set on push, applied on return.
    pub sport_ctx: Option<(SportMode, &'static SportSpec)>,
    /// Harness hook (`?autoplay=1`): the Saxons act through the same AI
    /// the Normans use, so a scripted loop can prove the full war -
    /// conquests, chronicle verdict and the crowned end screen - without
    /// a per-turn bot (T501 hook family).
    pub autoplay_player: bool,
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
            event_kind: None,
            event_open: false,
            event_turn: u32::MAX,
            ai_anim: 0,
            report_pending: false,
            pending_event: None,
            champ_open: false,
            sting_armed: false,
            launch_req: None,
            sport_out: Rc::default(),
            sport_ctx: None,
            autoplay_player: false,
        }
    }

    pub fn tick(&mut self) {
        self.msg_ticks = self.msg_ticks.saturating_sub(1);
        self.anim = self.anim.wrapping_add(1);
        // Bars glide toward the true meters (~8 ticks); the draw pass shows a
        let tgt = Self::bar_targets_of(&self.g);
        for (d, t) in self.bars_disp.iter_mut().zip(tgt) {
            if *d < t {
                *d = (*d + (t - *d).div_ceil(8)).min(t);
            } else if *d > t {
                *d = (*d - (*d - t).div_ceil(8)).max(t);
            }
        }
        // The enemy dawn marches for a second; report and omen wait (T308).
        if self.ai_anim > 0 {
            self.ai_anim -= 1;
            if self.ai_anim == 0 {
                self.open_dawn();
            }
        }
        if self.report_open && self.report_shown < self.report.len() && self.anim.is_multiple_of(4)
        {
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
        if self.ai_anim > 0 {
            // enemy banners are crossing England: none hearken (T308)
            return;
        }
        if self.report_open {
            // any tap: first reveals all lines (skip), next closes (T306)
            if self.report_shown < self.report.len() {
                self.report_shown = self.report.len();
            } else {
                self.report_open = false;
            }
            return;
        }
        if self.champ_open {
            // triple crown: the map is deaf; only OK dismisses (T707),
            // and it outranks any omen plate beneath it.
            if event_ok_rect().hits(x, y) {
                self.champ_open = false;
            }
            return;
        }
        if self.event_open {
            // omen modal: the map is deaf; only OK dismisses (T307)
            if event_ok_rect().hits(x, y) {
                self.event_open = false;
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
            for b in action_rects(
                self.g.strongholds[sel].owner == PLAYER,
                self.g.dragon_offer,
            ) {
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
                        self.capture_report(before, false);
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
    /// `before` open the animated report overlay (T306); `defer` waits for
    /// the dawn march animation to end (T308).
    fn capture_report(&mut self, before: usize, defer: bool) {
        if self.g.over.is_some() {
            return;
        }
        let fresh = &self.g.log[before..];
        if fresh.iter().any(|l| l.contains("ROUNDS.")) {
            self.report = fresh.to_vec();
            self.report_shown = 0;
            if defer {
                self.report_pending = true;
            } else {
                self.report_open = true;
            }
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
                h = action_rects(own, self.g.dragon_offer)
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

    pub fn act(&mut self, id: &str, sel: usize) {
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
        } else if id == "hoard" {
            // T704: the omen standeth - one heist, one shot at the hoard.
            self.launch_req = Some((&SPORTS[12], SportMode::Hoard));
            return;
        } else if id == "sports" {
            // The meet is a season-wide pick (same draw as the button
            // label); entry and purse differ when our own herald flies here.
            let seed = ((self.g.turn as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15;
            let herald = self.g.tournament[PLAYER.index()] == Some(sel);
            let Some(spec) = season_sport(&self.g) else {
                self.say("NO SPORT IS HELD THIS SEASON");
                return;
            };
            if host::make_game(spec.id, seed).is_none() {
                self.say(if herald {
                    "THE HERALD YET WAITS FOR A MEET"
                } else {
                    "THE SPORTS ARE HELD AT THE TOURNEY"
                });
                return;
            }
            self.launch_req = Some((
                spec,
                if herald {
                    SportMode::Herald { sid: sel }
                } else {
                    SportMode::Sports { sid: sel }
                },
            ));
            return;
        } else {
            return;
        };
        self.say(&outcome);
    }

    fn end_turn(&mut self) {
        self.sel = None;
        let before = self.g.log.len();
        // One click = one round: end_player_turn already lets the Normans
        // act, rolls the omen, and pays both sides (T308 found the old
        // double-call skipping a whole round per click).
        if self.autoplay_player {
            // the harness drives BOTH hosts: the Saxons move first, then
            // the round closes with the Norman answer as ever.
            crate::campaign::ai::take_turn(&mut self.g, PLAYER);
        }
        self.g.begin_ai_recording();
        self.g.end_player_turn();
        self.g.end_ai_recording();
        self.capture_report(before, true);
        let tail = self.g.log.last().cloned().unwrap_or_default();
        self.say(&tail);
        if self.g.over.is_none() {
            if let Some(e) = self.g.event {
                if self.event_turn != self.g.turn {
                    self.event_turn = self.g.turn;
                    self.pending_event = Some(e);
                }
            }
        }
        if self.g.over.is_none() && !self.g.ai_moves.is_empty() {
            // The enemy host marches first; report and omen follow (T308).
            self.ai_anim = AI_ANIM_TICKS;
            self.sting_armed = true;
        } else {
            self.open_dawn();
        }
    }

    /// Reveal whatever the dawn unlocked once the march animation ends.
    fn open_dawn(&mut self) {
        if std::mem::take(&mut self.report_pending) {
            self.report_open = true;
        }
        if let Some(e) = self.pending_event.take() {
            self.event_kind = Some(e);
            self.event_open = true;
        }
    }

    /// The scene announces a triple-crown champion here; the modal then
    /// holds the map until OK.
    pub fn announce_champion(&mut self) {
        self.champ_open = true;
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
            if self.g.tournament.contains(&Some(s.id)) {
                // tourney year: a yellow pennant flies from the right crenel
                // (T307) - public knowledge, so it shows through fog too.
                let p = ((self.anim / 8) & 1) == 0;
                fb.vline(cx + 5, cy - 14, cy - 8, BLACK);
                fb.fill_rect(cx + 6, cy - 14 + if p { 0 } else { 1 }, 3, 2, YELLOW);
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
        // Enemy host on the march (T308): black-bordered red dots replay
        // every Norman move of the dawn, in one second, over the map.
        if self.ai_anim > 0 {
            let t = (AI_ANIM_TICKS - self.ai_anim) as f32 / AI_ANIM_TICKS as f32;
            for &(a, b) in &self.g.ai_moves {
                let (ax, ay) = castle_screen(&self.g.strongholds[a]);
                let (bx, by) = castle_screen(&self.g.strongholds[b]);
                let x = (ax as f32 + (bx - ax) as f32 * t).round() as i32;
                let y = (ay as f32 + (by - ay) as f32 * t).round() as i32;
                fb.fill_rect(x - 1, y - 1, 3, 3, NORMAN_RED);
                fb.rect(x - 2, y - 2, 5, 5, BLACK);
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
        self.draw_event_modal(fb);
        self.draw_champion_modal(fb);
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
        for b in action_rects(own, self.g.dragon_offer) {
            let enabled = btn_enabled(&self.g, b.id);
            kit::button(fb, &b, self.hover == Some(b.id), enabled);
            let label = btn_label(&self.g, b.id);
            draw_text_shadow(
                fb,
                b.x + (b.w as i32 - label.len() as i32 * 8) / 2,
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

    /// The omen modal (T307): TAN-rim LINEN plate with a 16x16 event icon,
    /// NAVY title, two body rows and an OK button; the map is deaf until OK
    /// dismisses it.
    fn draw_event_modal(&self, fb: &mut crate::engine::gfx::Framebuffer) {
        if !self.event_open {
            return;
        }
        use crate::engine::fonts::draw_text_shadow;
        let Some(e) = self.event_kind else {
            return;
        };
        kit::panel(fb, 70, 64, 180, 74);
        fb.fill_rect(72, 66, 176, 70, LINEN);
        draw_event_icon(fb, e, 80, 72);
        draw_text_shadow(fb, 104, 72, e.title(), NAVY, LINEN);
        let (l1, l2) = event_body(e);
        draw_text_shadow(fb, 104, 86, l1, BLACK, LINEN);
        draw_text_shadow(fb, 80, 102, l2, BLACK, LINEN);
        kit::button(fb, &event_ok_rect(), false, true);
        draw_text_shadow(fb, 155, 124, "OK", CREAM, BLACK);
    }

    /// THE TRIPLE CROWN (T707): the third event-tourney gold recruits a
    /// champion - the feat is announced on the omen plate, with three
    /// crowns for a crest, and the map stays deaf until OK.
    fn draw_champion_modal(&self, fb: &mut crate::engine::gfx::Framebuffer) {
        if !self.champ_open {
            return;
        }
        use crate::engine::fonts::draw_text_shadow;
        kit::panel(fb, 70, 64, 180, 74);
        fb.fill_rect(72, 66, 176, 70, LINEN);
        fb.fill_rect(80, 72, 16, 16, NAVY);
        // three gold crowns with cream tips on a NAVY plaque
        for (cx, peak) in [(83i32, 4i32), (87, 3), (91, 4)] {
            fb.fill_rect(cx, 72 + peak, 2, 8, GOLD);
            fb.fill_rect(cx, 72 + peak, 2, 1, CREAM);
        }
        fb.fill_rect(83, 79, 10, 3, GOLD);
        fb.set(87, 80, DARKRED);
        draw_text_shadow(fb, 104, 72, "THE TRIPLE CROWN", NAVY, LINEN);
        draw_text_shadow(fb, 104, 86, "A CHAMPION RIDETH TO", BLACK, LINEN);
        draw_text_shadow(fb, 80, 102, "THY GARRISON - GLORY ALONE", BLACK, LINEN);
        kit::button(fb, &event_ok_rect(), false, true);
        draw_text_shadow(fb, 155, 124, "OK", CREAM, BLACK);
    }
}

const NORMAN_KEEP: u8 = DARKRED;

/// Two story rows for each omen (T307), kept short for the 176px plate.
fn event_body(e: EventKind) -> (&'static str, &'static str) {
    match e {
        EventKind::TournamentHerald => ("A TOURNARY IS HELD", "FREE SPORTS, GREAT GLORY"),
        EventKind::MerchantCaravan => ("TOLL GOLD SWELLS THE", "TREASURY CHEST"),
        EventKind::BadHarvest => ("ROT TAKETH THE WHEAT;", "PROVISIONS DWINDLE"),
        EventKind::Plague => ("THE PLEAGUE SILETH THE", "LANDS. HALF INCOME."),
        EventKind::KingsLevy => ("THE KING'S LEVY COMETH", "TO THE CAPITAL"),
        EventKind::Deserters => ("FEET SLINK AWAY FROM", "THY LARGEST HOST"),
        EventKind::DanishRaiders => ("HEATHEN SHIPS AT SEA;", "A COAST KEEP MAY FALL"),
        EventKind::DragonLair => ("THE DRAGON STIRRETH:", "A HOARD FOR ONE THIEF"),
    }
}

/// 16x16 event pictograph on a NAVY plaque (T307) - the 8x8 font has no
/// pictographs, so each omen gets a hand-set pixel icon.
fn draw_event_icon(fb: &mut crate::engine::gfx::Framebuffer, e: EventKind, x: i32, y: i32) {
    fb.fill_rect(x, y, 16, 16, NAVY);
    match e {
        EventKind::TournamentHerald => {
            // bugle horn
            fb.fill_rect(x + 3, y + 7, 7, 3, GOLD);
            fb.fill_rect(x + 10, y + 5, 3, 6, YELLOW);
            fb.fill_rect(x + 3, y + 6, 2, 5, CREAM);
        }
        EventKind::MerchantCaravan => {
            // toll pouch with two coins
            fb.fill_rect(x + 4, y + 7, 8, 7, STONE);
            fb.fill_rect(x + 6, y + 4, 4, 3, STONE);
            fb.fill_rect(x + 5, y + 9, 2, 2, GOLD);
            fb.fill_rect(x + 9, y + 11, 2, 2, GOLD);
        }
        EventKind::BadHarvest => {
            // three blighted stalks
            fb.vline(x + 4, y + 6, y + 12, DARKGRAY);
            fb.vline(x + 8, y + 6, y + 12, DARKGRAY);
            fb.vline(x + 12, y + 6, y + 12, DARKGRAY);
            fb.fill_rect(x + 3, y + 4, 2, 2, GRAY);
            fb.fill_rect(x + 7, y + 5, 2, 2, GRAY);
            fb.fill_rect(x + 11, y + 4, 2, 2, GRAY);
        }
        EventKind::Plague => {
            // death's head
            fb.fill_circle(x + 8, y + 7, 4, WHITE);
            fb.fill_rect(x + 5, y + 10, 6, 4, WHITE);
            fb.set(x + 6, y + 6, NAVY);
            fb.set(x + 7, y + 6, NAVY);
            fb.set(x + 9, y + 6, NAVY);
            fb.set(x + 10, y + 6, NAVY);
            fb.vline(x + 7, y + 11, y + 13, NAVY);
            fb.vline(x + 9, y + 11, y + 13, NAVY);
        }
        EventKind::KingsLevy => {
            // crown
            fb.fill_rect(x + 3, y + 9, 10, 3, GOLD);
            fb.fill_rect(x + 3, y + 5, 2, 4, GOLD);
            fb.fill_rect(x + 7, y + 4, 2, 5, GOLD);
            fb.fill_rect(x + 11, y + 5, 2, 4, GOLD);
            fb.fill_rect(x + 7, y + 10, 2, 1, BLOOD);
        }
        EventKind::Deserters => {
            // aman with a spear slipping away
            fb.fill_rect(x + 6, y + 3, 2, 2, SILVER);
            fb.fill_rect(x + 6, y + 6, 2, 4, SILVER);
            fb.fill_rect(x + 4, y + 10, 3, 1, SILVER);
            fb.fill_rect(x + 8, y + 10, 2, 1, SILVER);
            fb.vline(x + 11, y + 4, y + 10, STONE);
        }
        EventKind::DanishRaiders => {
            // longship with raven sail
            fb.fill_rect(x + 2, y + 11, 12, 3, DARKGRAY);
            fb.vline(x + 8, y + 3, y + 11, BLACK);
            fb.fill_rect(x + 9, y + 4, 4, 4, WHITE);
            fb.fill_rect(x + 10, y + 5, 2, 2, BLOOD);
        }
        EventKind::DragonLair => {
            // gold mound under a watching ember eye
            fb.fill_rect(x + 2, y + 11, 12, 3, GOLD);
            fb.fill_rect(x + 5, y + 9, 6, 2, YELLOW);
            fb.fill_rect(x + 5, y + 3, 7, 5, DARKGREEN);
            fb.fill_rect(x + 7, y + 4, 2, 2, EMBER);
            fb.fill_rect(x + 11, y + 5, 2, 2, GREEN);
        }
    }
}
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
    battle: bool,  // battle.mod bed while the report overlay is open
    endgame: bool, // victory/defeat stinger fired exactly once
    /// Harness path: minigames resolve themselves (?autoplay=1).
    autoplay: bool,
}

impl PlayScene {
    pub fn new(seed: u64, difficulty: Difficulty) -> Self {
        Self {
            ui: CampaignUi::new(seed, difficulty),
            battle: false,
            endgame: false,
            autoplay: false,
        }
    }

    pub fn with_autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self.ui.autoplay_player = on;
        self
    }

    /// Harness/debug path: open the campaign already at turn N (?turn=N),
    /// so late-season draws are reachable without replaying nine turns.
    pub fn with_turn(mut self, n: u32) -> Self {
        if n > 0 {
            self.ui.g.turn = n;
        }
        self
    }

    /// Harness hook (T707): bank a champion into the starting garrison
    /// and proclaim the triple crown at boot (?champ=1), so the modal and
    /// the panel row are reachable without winning three tourneys.
    pub fn with_champion(mut self, on: bool) -> Self {
        if on {
            self.ui.g.strongholds[0]
                .garrison
                .push(crate::campaign::units::Unit::new(UnitKind::Champion));
            self.ui.announce_champion();
        }
        self
    }

    /// Layer/collect the minigame host around the campaign screen (T501).
    fn sports_flow(&mut self, app: &mut dyn crate::engine::scene::AppCtx) {
        if let Some((spec, mode)) = self.ui.launch_req.take() {
            let seed = ((self.ui.g.turn as u64) << 32)
                ^ match mode {
                    SportMode::Sports { sid } | SportMode::Herald { sid } => {
                        (sid as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    }
                        SportMode::Hoard => 0x4452_414E_5354_4952,
                };
            if let Some(game) = host::make_game(spec.id, seed) {
                self.ui.sport_ctx = Some((mode, spec));
                let cell = self.ui.sport_out.clone();
                app.push(Box::new(MgHost::new(
                    spec, mode, game, cell, self.autoplay,
                )));
            }
        }
        let done = self.ui.sport_out.borrow_mut().take();
        if let Some(o) = done {
            if let Some((mode, spec)) = self.ui.sport_ctx.take() {
                if o.tier.is_none() {
                    if matches!(mode, SportMode::Hoard) {
                        // the dragon forgiveth no second attempt: the
                        // failed heist still burneth the one offer.
                        self.ui.g.forfeit_dragon();
                        self.ui.say("THE HOARD KEEPETH ITS SECRET - ONE TRY ALONE");
                    } else {
                        // no laurel: the meet is void - no fee was taken, the
                        // herald (if any) still waits.
                        self.ui.say("THOU FAILED THE MEET - NO FEE, NO PURSE");
                    }
                    return;
                }
                let res = match mode {
                    SportMode::Sports { sid } => self.ui.g.play_sports(
                        PLAYER,
                        sid,
                        o.score,
                        spec.silver_at,
                        spec.gold_at,
                    ),
                    SportMode::Herald { .. } => self.ui.g.play_tournament(
                        PLAYER,
                        o.score,
                        spec.silver_at,
                        spec.gold_at,
                    ),
                    SportMode::Hoard => self.ui.g.play_dragon(
                        PLAYER,
                        o.score,
                        spec.silver_at,
                        spec.gold_at,
                    ),
                };
                let (msg, crowned) = match res {
                    Ok((tier, r)) => {
                        let mut m = match tier {
                            crate::campaign::game::Tier::Gold => {
                                // a gold laurel is chronicle material (T505)
                                app.record_feat(&format!("{} GOLD - SCORE {}", spec.label, o.score));
                                format!("GOLD LAUREL: {} GOLD", r.gold)
                            }
                            crate::campaign::game::Tier::Silver => {
                                format!("SILVER LAUREL: {} PROVISIONS", r.provisions)
                            }
                            crate::campaign::game::Tier::Bronze => format!("BRONZE LAUREL: {} GOLD", r.gold),
                        };
                        if r.champion {
                            m.push_str(" AND A CHAMPION");
                        }
                        (m, r.champion)
                    }
                    Err(e) => (e, false),
                };
                if crowned {
                    // the triple crown is worth a proclamation (T707)
                    self.ui.announce_champion();
                }
                self.ui.say(&msg);
            }
        }
    }
}

impl crate::engine::scene::Scene for PlayScene {
    fn init(&mut self, app: &mut dyn crate::engine::scene::AppCtx) {
        app.play_music(Some("saxon"));
    }
    fn event(&mut self, ev: &SceneEvent) {
        self.ui.event(ev, true);
    }
    fn tick(&mut self, app: &mut dyn crate::engine::scene::AppCtx) {
        self.sports_flow(app);
        self.ui.tick();
        if self.ui.sting_armed {
            // the Norman war-horn calls across the channel (T308)
            self.ui.sting_armed = false;
            app.sting(NORMAN_STING);
        }
        // battle.mod drives the report overlay, then the hall theme returns
        if self.ui.report_open != self.battle {
            self.battle = self.ui.report_open;
            app.play_music(Some(if self.battle { "battle" } else { "saxon" }));
        }
        if !self.endgame {
            if let Some(winner) = self.ui.g.over {
                self.endgame = true;
                app.play_music(None);
                app.sting(if winner == PLAYER { VICTORY_MOD } else { DEFEAT_MOD });
                app.record_feat(&format!(
                    "{} - TURN {}",
                    if winner == PLAYER { "CROWNED" } else { "CROWN LOST" },
                    self.ui.g.turn
                ));
            }
        }
    }
    fn draw(&mut self, fb: &mut crate::engine::gfx::Framebuffer) {
        self.ui.draw(fb);
    }
    fn wants_leave(&self) -> bool {
        self.ui.want_restart
    }
}
