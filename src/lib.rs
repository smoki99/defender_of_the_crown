//! The Crown Endures — a Saxon liberation. Rust→WASM retro strategy game.
//! Pure cores: [`campaign`], [`engine`] math. Wasm entry: [`start`].

pub mod campaign;
pub mod engine;
pub mod minigames;
pub mod ui;

#[cfg(target_arch = "wasm32")]
mod web;
