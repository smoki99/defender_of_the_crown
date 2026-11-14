//! Minigame framework (SPEC 5.5 / 6 / 10). Pure core: no `web-sys`, no IO,
//! no printing — deterministic like the campaign code, so every sport module
//! and its tests run headless.
//!
//! Flow: the campaign draws one sport with [`pick`], ticks its [`Minigame`]
//! until the game's own [`MgState`] settles or the [`Cheer`] timer (the
//! 60-second "cheer threshold", SPEC 6) runs out, then funnels the final
//! score through [`resolve`] to build the [`MgOutcome`] the reward layer
//! pays out (SPEC 5.5).

pub use crate::campaign::game::Tier;
use crate::campaign::rng::Xor;
use crate::engine::gfx::Framebuffer;
use crate::engine::input::Input;

/// Live state of a sport mid-tick (SPEC 10). `Lost` can fire immediately
/// when the game itself decides (lance shatters, dragon wakes); otherwise a
/// runner keeps reporting `Running` until [`Cheer`] is done and calls
/// [`resolve`] for the verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MgState {
    Running,
    Won(Tier),
    Lost,
}

/// One minigame module (SPEC 10): one module, one spec, one test file.
pub trait Minigame {
    /// Stable id; must match a [`SPORTS`] row.
    fn id(&self) -> &'static str;
    /// Advance `dt` seconds and draw the 320x256 frame in place.
    fn tick(&mut self, dt: f32, input: &Input, fb: &mut Framebuffer);
    /// Current score; tier thresholds are pure data in [`SPORTS`].
    fn score(&self) -> u32;
    fn state(&self) -> MgState;
}

/// One catalogue row (SPEC 6): banner word, unlock pacing, reward tiers.
/// Scores are normalized per game to 0..=100 by the game module itself, so
/// thresholds stay boring pure data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SportSpec {
    pub id: &'static str,
    /// ALL-CAPS banner word shown on the sport's intro/results screen.
    pub label: &'static str,
    /// `score >= bronze_at` earns at least a bronze haul (SPEC 5.5).
    pub bronze_at: u32,
    pub silver_at: u32,
    /// Script-perfect level: a flawless run always lands Gold.
    pub gold_at: u32,
    /// First campaign turn the SPORTS draw may offer this game (SPEC 5.5);
    /// [`u32::MAX`] marks rare-event-only games that are never drawn.
    pub unlock_turn: u32,
}

/// The 13 sports in SPEC 6 order. `gold_at` sits at script-perfect on each
/// game's 0..=100 normalized scale; silver is a solid, unflashy run.
pub const SPORTS: &[SportSpec] = &[
    // Two passes of lance lane + release timing; a crowned double pass ~95.
    SportSpec {
        id: "joust",
        label: "TOURNAMENT",
        bronze_at: 40,
        silver_at: 70,
        gold_at: 95,
        unlock_turn: 0,
    },
    // 6 stones vs wind: 4 clean tower hits ~60, + ships and a perfect
    // 6-for-6 is the only way past 90.
    SportSpec {
        id: "catapult",
        label: "SIEGE",
        bronze_at: 45,
        silver_at: 70,
        gold_at: 92,
        unlock_turn: 0,
    },
    // Best-of-5 parry+riposte; win 3-0 reading every telegraph, take no hits.
    SportSpec {
        id: "sword",
        label: "DUEL",
        bronze_at: 35,
        silver_at: 65,
        gold_at: 90,
        unlock_turn: 0,
    },
    // 30-s pop-up gallery; the apple-shot bonus round gates Gold.
    SportSpec {
        id: "archery",
        label: "GALLERY",
        bronze_at: 40,
        silver_at: 70,
        gold_at: 95,
        unlock_turn: 0,
    },
    // Hold the wall vs scaling ladders; every wave survived, no breach.
    SportSpec {
        id: "rampart",
        label: "DEFEND",
        bronze_at: 40,
        silver_at: 65,
        gold_at: 90,
        unlock_turn: 5,
    },
    // Dungeon rescue scored on time: fast, guard-free exit route.
    SportSpec {
        id: "maze",
        label: "RESCUE",
        bronze_at: 35,
        silver_at: 65,
        gold_at: 90,
        unlock_turn: 5,
    },
    // Pole-vault distance + pike-joust bonuses; clearing the riverfalls
    // without a wipeout.
    SportSpec {
        id: "river",
        label: "CANOE",
        bronze_at: 40,
        silver_at: 70,
        gold_at: 92,
        unlock_turn: 5,
    },
    // 5 torch-cells + no-alarm bonus: alarm-free sweep is the only 95+.
    SportSpec {
        id: "cloak",
        label: "STEALTH",
        bronze_at: 45,
        silver_at: 75,
        gold_at: 95,
        unlock_turn: 5,
    },
    // Hammer accuracy across 3 tempos; a perfect combo chain ~95.
    SportSpec {
        id: "forge",
        label: "RHYTHM",
        bronze_at: 40,
        silver_at: 70,
        gold_at: 95,
        unlock_turn: 5,
    },
    // All pairs before the boil boils over; fast flawless brew.
    SportSpec {
        id: "herbs",
        label: "MEMORY",
        bronze_at: 40,
        silver_at: 70,
        gold_at: 90,
        unlock_turn: 10,
    },
    // Intercept rate of the crow flight; every ear of corn defended.
    SportSpec {
        id: "falcon",
        label: "FALCONER",
        bronze_at: 35,
        silver_at: 65,
        gold_at: 88,
        unlock_turn: 10,
    },
    // 3 waves, barrels used, barmaid untouched; clean sweep ~92.
    SportSpec {
        id: "tavern",
        label: "BRAWL",
        bronze_at: 40,
        silver_at: 70,
        gold_at: 92,
        unlock_turn: 10,
    },
    // Rare-event-only (SPEC 5.5): loot value; grabbing the cursed crown is
    // what lifts a perfect escape to 100.
    SportSpec {
        id: "dragon",
        label: "HOARD",
        bronze_at: 45,
        silver_at: 80,
        gold_at: 100,
        unlock_turn: u32::MAX,
    },
];

/// Sports the SPORTS button may draw at `turn` (SPEC 5.5 pacing: 1-4 from
/// the start, 5-9 from turn 5, 10-12 from turn 10). DRAGON'S HOARD is
/// rare-event-only and never appears here, at any turn.
pub fn unlocked(turn: u32) -> Vec<&'static SportSpec> {
    SPORTS
        .iter()
        .filter(|s| s.unlock_turn != u32::MAX && turn >= s.unlock_turn)
        .collect()
}

/// Deterministic SPORTS draw: same `seed` + `turn` always yields the same
/// sport. `None` only if nothing is unlocked (never happens today: games
/// 1-4 unlock at turn 0).
pub fn pick(turn: u32, seed: u64) -> Option<&'static SportSpec> {
    let pool = unlocked(turn);
    if pool.is_empty() {
        return None;
    }
    let mut rng = Xor::new(seed);
    pool[rng.below(pool.len() as u32) as usize].into()
}

/// The 60-second "cheer threshold" (SPEC 6): gold-tier runs must happen
/// while the crowd still cheers. Wall clock only — pause by not stepping.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cheer {
    elapsed: f32,
}

impl Cheer {
    /// Length of the crowd's patience, seconds.
    pub const SECONDS: f32 = 60.0;

    pub fn new() -> Self {
        Cheer { elapsed: 0.0 }
    }

    /// Advance the clock by `dt` seconds; clamps at [`Cheer::SECONDS`] so
    /// [`Cheer::frac`] stays in `0.0..=1.0`. Negative `dt` is ignored — the
    /// cheer never rewinds, so `frac()` is monotonic for any input.
    pub fn step(&mut self, dt: f32) {
        self.elapsed = (self.elapsed + dt.max(0.0)).min(Self::SECONDS);
    }

    /// 0.0..=1.0 of the cheer spent — draw this as the HUD banner bar.
    pub fn frac(&self) -> f32 {
        self.elapsed / Self::SECONDS
    }

    pub fn done(&self) -> bool {
        self.elapsed >= Self::SECONDS
    }
}

/// Final verdict from a finished score (SPEC 5.5 tiers). Pure mapping: the
/// runner decides *when* to call this (game end or cheer expiry); below
/// bronze the crowd goes home empty-handed.
pub fn resolve(score: u32, spec: &SportSpec) -> MgState {
    if score >= spec.gold_at {
        MgState::Won(Tier::Gold)
    } else if score >= spec.silver_at {
        MgState::Won(Tier::Silver)
    } else if score >= spec.bronze_at {
        MgState::Won(Tier::Bronze)
    } else {
        MgState::Lost
    }
}

/// What the reward layer receives after a sport ends: `tier == None` means
/// the run failed (Lost) and pays nothing (SPEC 5.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MgOutcome {
    pub id: &'static str,
    pub score: u32,
    pub tier: Option<Tier>,
}
