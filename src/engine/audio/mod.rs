//! Audio: procedural SFX + .MOD music playback (own ProTracker player,
//! SPEC §3 fallback — xmrs is a model library, not a renderer).

pub mod modplayer;
pub mod music;
pub mod synth;

pub use modplayer::{Mod, Player};
pub use music::Music;
pub use synth::Sfx;
