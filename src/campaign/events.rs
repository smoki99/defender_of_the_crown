//! Event table data (SPEC 5.4). Rolling and effects land with T206; this is
//! the pure data table the UI announces from. The hoard omen (T704) joins
//! the pool from turn 20 and the caller keeps it once-per-campaign.

use crate::campaign::rng::Xor;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventKind {
    /// Castle banner: a free featured minigame with big rewards.
    TournamentHerald,
    /// +gold.
    MerchantCaravan,
    /// -provisions.
    BadHarvest,
    /// Two turns of halved income.
    Plague,
    /// +3 levies to the capital.
    KingsLevy,
    /// -2 troops from the largest army.
    Deserters,
    /// From turn 12: strike on a weak coast castle.
    DanishRaiders,
    /// From turn 20, once per campaign: the dragon stirs - a rare-event
    /// heist opens for the Saxon player (SPEC 5.5).
    DragonLair,
}

pub const ALL_EVENTS: [EventKind; 8] = [
    EventKind::TournamentHerald,
    EventKind::MerchantCaravan,
    EventKind::BadHarvest,
    EventKind::Plague,
    EventKind::KingsLevy,
    EventKind::Deserters,
    EventKind::DanishRaiders,
    EventKind::DragonLair,
];

impl EventKind {
    pub fn title(self) -> &'static str {
        match self {
            EventKind::TournamentHerald => "TOURNAMENT HERALD",
            EventKind::MerchantCaravan => "MERCHANT CARAVAN",
            EventKind::BadHarvest => "BAD HARVEST",
            EventKind::Plague => "PLEAGUE!",
            EventKind::KingsLevy => "KING'S LEVY",
            EventKind::Deserters => "DESERTERS",
            EventKind::DanishRaiders => "DANISH RAIDERS",
            EventKind::DragonLair => "THE DRAGON STIRS",
        }
    }

    /// Base weight; DanishRaiders and DragonLair are additionally gated by
    /// turn (the hoard from 20 - and only once, via `roll_excluding`).
    pub fn weight(self, turn: u32) -> u32 {
        match self {
            EventKind::TournamentHerald => 20,
            EventKind::MerchantCaravan => 18,
            EventKind::BadHarvest => 16,
            EventKind::Plague => 10,
            EventKind::KingsLevy => 12,
            EventKind::Deserters => 12,
            EventKind::DanishRaiders => {
                if turn >= 12 {
                    14
                } else {
                    0
                }
            }
            EventKind::DragonLair => {
                if turn >= 20 {
                    8
                } else {
                    0
                }
            }
        }
    }

    /// Weighted draw over `ALL_EVENTS` for the given turn. Deterministic in
    /// the caller's RNG.
    pub fn roll(rng: &mut Xor, turn: u32) -> EventKind {
        Self::roll_excluding(rng, turn, &[])
    }

    /// Weighted draw with some kinds withheld (the campaign keeps
    /// `DragonLair` out of the pool once it has fired). A zero-weight kind
    /// is withheld anyway; `skip` must not cover the whole positive pool.
    pub fn roll_excluding(rng: &mut Xor, turn: u32, skip: &[EventKind]) -> EventKind {
        let total: u32 = ALL_EVENTS
            .iter()
            .filter(|e| !skip.contains(e))
            .map(|e| e.weight(turn))
            .sum();
        let mut pick = rng.below(total);
        for &e in &ALL_EVENTS {
            let w = if skip.contains(&e) { 0 } else { e.weight(turn) };
            if pick < w {
                return e;
            }
            pick -= w;
        }
        ALL_EVENTS[0]
    }
}
