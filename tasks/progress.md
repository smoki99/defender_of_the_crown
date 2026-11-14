# Progress Log

Format: `T<id> — <what ran> — <result> (<date>)`. Append only.
- T001 — fetch-chrome-libs.ts (fixed un-awaited exists bugs) → 111 pkgs → ~/pw-libs; chrome-headless-shell 159.0.8154.6 launches; playwright screenshot /tmp/pw-test.png 41 KB, 0 console errors (2026-11-14)
- T002 — .omp/lsp.json → /home/omp/.local/bin/rust-analyzer --stdio, rootMarker Cargo.toml (2026-11-14)
- T005 — test.sh: docker cp staging (remote daemon: no bind mounts, exec 403-filtered) + named vols crown-src/crown-target; image crown-rs:1 = rust:1.90-slim + wasm32 target (docker build works). ./test.sh wasm → Finished dev profile (2026-11-14)
- T101 — lib crate reset: src/lib.rs + campaign/* (draft reparented, 3 compile fixes: Xor derives, march() move-before-len, _atk) + engine stub + web stub; cargo check --target wasm32 GREEN (2026-11-14)

- plan+todo created; execution started (2026-11-14)
