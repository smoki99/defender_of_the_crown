//! Minigames (SPEC 6): 13 sports, one module each. The framework is pure;
//! sport modules land as they ship (T502: joust).

pub mod framework;
pub mod joust;

pub use framework::{
    pick, resolve, unlocked, Cheer, MgOutcome, MgState, Minigame, SportSpec, SPORTS,
};
