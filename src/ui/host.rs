//! T501: `MgHost` - the scene wrapper that runs one minigame to its end.
//!
//! The host owns the shared 60 s cheer timer, the pause key, the top meter
//! band and the results screen (tier banner + score count-up loot anim). It
//! advances the pure `Minigame` core with fixed 1/60 steps during `draw`
//! (the core draws itself), announces the outcome through a shared cell so
//! the campaign scene can apply the purse after the stack pops back, and
//! leaves. `?autoplay=1` (wired by the web adapter) resolves the run at
//! once so the harness can exercise the results + reward path without a
//! per-game bot (those arrive with T502+).

use super::super::minigames::framework::{Cheer, Minigame, SportSpec};
use crate::campaign::game::Tier;
use crate::engine::audio::Sfx;
use crate::engine::fonts::draw_text_shadow;
use crate::engine::gfx::{BLACK, BLOOD, BROWN, DARKGRAY, GOLD, GRAY, NAVY, SILVER, WHITE};
use crate::engine::input::{Input, Raw, KEY_ENTER, KEY_ESC};
use crate::engine::scene::{AppCtx, Scene, SceneEvent};
use crate::minigames::framework::{resolve, MgOutcome, MgState};
use std::cell::RefCell;
use std::rc::Rc;

/// Which purse the run plays for (SPEC 5.5): paid SPORTS at a castle, or
/// the herald's free meet with double rewards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SportMode {
    Sports { sid: usize },
    Herald { sid: usize },
    /// T704 rare event: the dragon's hoard heist, no castle attached.
    Hoard,
}

/// Shared mailbox: the host writes the outcome, the campaign reads+clears.
pub type OutcomeCell = Rc<RefCell<Option<MgOutcome>>>;

#[derive(PartialEq)]
enum Phase {
    Play,
    Results,
}

pub struct MgHost {
    pub spec: &'static SportSpec,
    pub mode: SportMode,
    game: Box<dyn Minigame>,
    cheer: Cheer,
    input: Input,
    frame: u64,
    stashed: Vec<Raw>,
    paused: bool,
    phase: Phase,
    res_ticks: u32,
    cell: OutcomeCell,
    announced: bool,
    done: bool,
    autoplay: bool,
    tier: Option<Tier>,
    final_score: u32,
}

impl MgHost {
    pub fn new(
        spec: &'static SportSpec,
        mode: SportMode,
        game: Box<dyn Minigame>,
        cell: OutcomeCell,
        autoplay: bool,
    ) -> Self {
        Self {
            spec,
            mode,
            game,
            cheer: Cheer::new(),
            input: Input::new(),
            frame: 0,
            stashed: Vec::new(),
            paused: false,
            phase: Phase::Play,
            res_ticks: 0,
            cell,
            announced: false,
            done: false,
            autoplay,
            tier: None,
            final_score: 0,
        }
    }

    /// Live score of the wrapped core (results: the frozen score).
    pub fn game_score(&self) -> u32 {
        self.game.score()
    }

    /// Freeze the run: write the shared outcome, show the results screen.
    fn finish(&mut self, st: MgState) {
        let score = self.game.score();
        self.finish_at(st, score);
    }

    /// Freeze with an explicit score - the autoplay harness resolves at the
    /// gold line without playing, so the core's own score is still 0 and
    /// must not leak into the outcome (a GOLD tier with score 0 voids the
    /// purse, the best-list and the chronicle alike).
    fn finish_at(&mut self, st: MgState, score: u32) {
        self.tier = match st {
            MgState::Won(t) => Some(t),
            MgState::Lost | MgState::Running => None,
        };
        self.final_score = score;
        *self.cell.borrow_mut() = Some(MgOutcome {
            id: self.spec.id,
            score,
            tier: self.tier,
        });
        self.phase = Phase::Results;
        self.res_ticks = 0;
    }

    fn draw_band(&mut self, fb: &mut crate::engine::gfx::Framebuffer) {
        // top meter: sport label, score, cheer time
        fb.fill_rect(0, 0, 320, 12, NAVY);
        draw_text_shadow(fb, 4, 2, self.spec.label, GOLD, BLACK);
        let s = format!("{}", self.game.score());
        draw_text_shadow(fb, 120, 2, &s, WHITE, BLACK);
        let f = self.cheer.frac().clamp(0.0, 1.0);
        let w = ((1.0 - f) * 96.0) as u32;
        // Outlined cheer tube with a legend so it reads over any field.
        draw_text_shadow(fb, 160, 2, "CHEER", SILVER, BLACK);
        fb.rect(219, 2, 98, 8, BLACK);
        fb.fill_rect(220, 3, 96, 6, DARKGRAY);
        fb.fill_rect(220, 4, w, 5, crate::engine::gfx::GREEN);
    }

    fn draw_results(&mut self, fb: &mut crate::engine::gfx::Framebuffer) {
        use crate::ui::kit;
        // Heraldic plate (T410): double gold frame, crown motif, centered rows.
        fb.fill_rect(38, 54, 244, 148, BLACK);
        fb.fill_rect(40, 56, 240, 144, NAVY);
        fb.rect(40, 56, 240, 144, GOLD);
        fb.rect(43, 59, 234, 138, GOLD);
        let head = match self.tier {
            Some(Tier::Gold) => "GOLD LAUREL!",
            Some(Tier::Silver) => "SILVER LAUREL",
            Some(Tier::Bronze) => "BRONZE LAUREL",
            None => "THOU FAILED",
        };
        let col = match self.tier {
            Some(Tier::Gold) => GOLD,
            Some(Tier::Silver) => SILVER,
            Some(Tier::Bronze) => BROWN,
            None => BLOOD,
        };
        // small crown above the heading, tinted like the laurel
        fb.fill_rect(152, 66, 16, 5, col);
        for px in [152i32, 158, 164] {
            fb.fill_rect(px, 62, 3, 4, col);
        }
        kit::centered(fb, 76, head, col, BLACK);
        kit::centered(fb, 96, self.spec.label, WHITE, BLACK);
        // loot anim: score counts up over ~1 s, then the purse line settles
        let shown = ((self.res_ticks * self.final_score) / 60).min(self.final_score);
        let line = format!("SCORE {shown}");
        kit::centered(fb, 120, &line, WHITE, BLACK);
        if self.res_ticks > 60 {
            let herald = matches!(self.mode, SportMode::Herald { .. });
            let purse = match self.tier {
                Some(Tier::Gold) if herald => "PURSE DOUBLE GOLD",
                Some(Tier::Gold) => "PURSE GOLD",
                Some(Tier::Silver) if herald => "PURSE SILVER X2",
                Some(Tier::Silver) => "PURSE PROVISIONS",
                Some(Tier::Bronze) if herald => "PURSE BRONZE X2",
                Some(Tier::Bronze) => "PURSE BRONZE",
                None => "NO PURSE - NO FEE",
            };
            kit::centered(fb, 140, purse, GOLD, BLACK);
            kit::centered(fb, 176, "ENTER CONTINUES", SILVER, BLACK);
        }
    }
}

impl Scene for MgHost {
    fn init(&mut self, app: &mut dyn AppCtx) {
        // the tourney rides the title fanfare bed until its own tune exists
        app.play_music(Some("title"));
    }

    fn event(&mut self, ev: &SceneEvent) {
        let raw = match *ev {
            SceneEvent::Tap { x, y } => {
                self.stashed.push(Raw::Down { x, y });
                Raw::Up { x, y }
            }
            SceneEvent::Move { x, y } => Raw::Move { x, y },
            SceneEvent::DragStart { x, y } => Raw::Down { x, y },
            SceneEvent::DragMove { x, y, .. } => Raw::Move { x, y },
            SceneEvent::DragEnd { x1, y1, .. } => Raw::Up { x: x1, y: y1 },
            SceneEvent::Key { code, pressed } => Raw::Key { code, pressed },
            SceneEvent::Wheel { dy } => Raw::Wheel { dy },
            _ => return,
        };
        self.stashed.push(raw);
    }

    fn tick(&mut self, app: &mut dyn AppCtx) {
        if self.phase == Phase::Results && !self.announced {
            self.announced = true;
            app.mg_best(self.spec.id, self.final_score);
            match self.tier {
                Some(Tier::Gold) => app.sfx(Sfx::Crowd),
                Some(_) => app.sfx(Sfx::Coin),
                None => app.sfx(Sfx::Boom),
            }
        }
    }

    fn draw(&mut self, fb: &mut crate::engine::gfx::Framebuffer) {
        self.frame += 1;
        for r in self.stashed.drain(..) {
            self.input.push(r);
        }
        self.input.begin_frame(self.frame);
        if self.phase == Phase::Play {
            if self.input.key_pressed(b'p' as u32) || self.input.key_pressed(KEY_ESC) {
                self.paused = !self.paused;
            }
            if self.autoplay {
                // harness path: no hands on the tiller - bank the run at
                // the gold line so the reward wiring (purse, laurel, HOF)
                // gets exercised exactly as a perfect play would.
                let sc = self.spec.gold_at;
                self.finish_at(resolve(sc, self.spec), sc);
            } else if self.paused {
                fb.fill_rect(0, 0, 320, 256, BLACK);
                self.game.tick(0.0, &self.input, fb); // repaint without advancing
                self.draw_band(fb);
                draw_text_shadow(fb, 128, 112, "PAUSED", GOLD, BLACK);
                draw_text_shadow(fb, 96, 128, "P RESUMES", GRAY, BLACK);
            } else {
                self.game.tick(1.0 / 60.0, &self.input, fb);
                self.draw_band(fb);
                self.cheer.step(1.0 / 60.0);
                let st = self.game.state();
                if !matches!(st, MgState::Running) {
                    self.finish(st);
                } else if self.cheer.done() {
                    self.finish(resolve(self.game.score(), self.spec));
                }
            }
        } else {
            self.res_ticks += 1;
            self.draw_results(fb);
            if self.input.key_pressed(KEY_ENTER) || (self.autoplay && self.res_ticks > 90) {
                self.done = true;
            }
        }
    }

    fn wants_leave(&self) -> bool {
        self.done
    }
}

/// Registry factory: one arm per sport that has actually shipped
/// (T502: joust, T503: catapult, T504: sword; T505+: dragon). An id with no arm
/// yet has no game, and the SPORTS button says so honestly.
pub fn make_game(id: &str, seed: u64) -> Option<Box<dyn Minigame>> {
    match id {
        "sword" => Some(Box::new(crate::minigames::sword::Sword::new(seed))),
        "joust" => Some(Box::new(crate::minigames::joust::Joust::new(seed))),
        "catapult" => Some(Box::new(crate::minigames::catapult::Catapult::new(seed))),
        "archery" => Some(Box::new(crate::minigames::archery::Archery::new(seed))),
        "rampart" => Some(Box::new(crate::minigames::rampart::Rampart::new(seed))),
        "maze" => Some(Box::new(crate::minigames::maze::Maze::new(seed))),
        "river" => Some(Box::new(crate::minigames::river::River::new(seed))),
        "cloak" => Some(Box::new(crate::minigames::cloak::Cloak::new(seed))),
        "forge" => Some(Box::new(crate::minigames::forge::Forge::new(seed))),
        "herbs" => Some(Box::new(crate::minigames::herbs::Herbs::new(seed))),
        "falcon" => Some(Box::new(crate::minigames::falcon::Falcon::new(seed))),
        "tavern" => Some(Box::new(crate::minigames::tavern::Tavern::new(seed))),
        "dragon" => Some(Box::new(crate::minigames::dragon::Dragon::new(seed))),
        _ => None,
    }
}

/// Whether any sport can actually be played right now.
pub fn game_exists() -> bool {
    crate::minigames::framework::SPORTS
        .iter()
        .any(|s| make_game(s.id, 1).is_some())
}
