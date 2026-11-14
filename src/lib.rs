//! The Crown Endures — a Saxon liberation. Rust→WASM retro strategy game.
//! Pure cores: [`campaign`], [`engine`] math. Wasm entry: [`start`].

pub mod campaign;
pub mod engine;

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::prelude::*;

    /// Bootstrap called from web/pkg loader. Real wiring arrives in T107/T402;
    /// until then it proves the wasm pipeline end-to-end.
    #[wasm_bindgen(start)]
    pub fn main() {
        console_error_panic_hook::set_once();
        web_sys::console::log_1(&"THE CROWN ENDURES — engine boot (stub)".into());
    }
}
