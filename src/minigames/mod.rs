//! Minigames (SPEC 6): 13 sports, one module each. T501 lands the shared
//! pure framework; sport modules build on it per the capability map.

pub mod framework;

pub use framework::{
    pick, resolve, unlocked, Cheer, MgOutcome, MgState, Minigame, SportSpec, SPORTS,
};
