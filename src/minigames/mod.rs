//! Minigames (SPEC 6): 13 sports, one module each. The framework is pure;
//! sport modules land as they ship (T502: joust, T503: catapult, T504: sword, T601: archery, T602: rampart, T603: maze, T604: river, T605: cloak, T606: forge, T701: herbs).

pub mod archery;
pub mod catapult;
pub mod cloak;
pub mod forge;
pub mod framework;
pub mod herbs;
pub mod joust;
pub mod maze;
pub mod rampart;
pub mod river;
pub mod sword;

pub use framework::{
    pick, resolve, unlocked, Cheer, MgOutcome, MgState, Minigame, SportSpec, SPORTS,
};
