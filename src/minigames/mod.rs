//! Minigames (SPEC 6): 13 sports, one module each. The framework is pure;
//! sport modules land as they ship (T502: joust, T503: catapult, T504: sword, T601: archery, T602: rampart).

pub mod archery;
pub mod catapult;
pub mod framework;
pub mod joust;
pub mod rampart;
pub mod sword;

pub use framework::{
    pick, resolve, unlocked, Cheer, MgOutcome, MgState, Minigame, SportSpec, SPORTS,
};
