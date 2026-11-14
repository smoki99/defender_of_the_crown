#!/bin/sh
# T005 docker test harness. Remote docker daemon has no bind mounts & no exec
# (websocket 403) → stage sources into a named volume via `docker cp` (staging
# container holds the volume), then `docker run` cargo with target cache in
# another named volume. Usage: ./test.sh [check|test|clippy|wasm|all] [cargo-args…]
set -eu
export PATH="$HOME/.local/bin:$PATH"
export DOCKER_HOST=tcp://tecnativa-proxy:2375
IMAGE=crown-rs:1
ROOT="$(cd "$(dirname "$0")" && pwd)"

ensure_stage() {
  docker run -d --name crown-stage -v crown-src:/work "$IMAGE" sleep infinity >/dev/null 2>&1 || true
  docker cp "$ROOT/." crown-stage:/work
}

CMD="${1:-all}"
if [ "$#" -gt 0 ]; then shift; fi
ensure_stage
BASE="docker run --rm -v crown-src:/work -v crown-target:/work/target -w /work $IMAGE cargo"
# .git-less stage: exclude by not copying (docker cp whole dir incl .git is fine)
case "$CMD" in
  check)  $BASE check "$@" ;;
  wasm)   $BASE check --target wasm32-unknown-unknown "$@" ;;
  test)   $BASE test --release "$@" ;;
  clippy) $BASE clippy --all-targets -- -D warnings ;;
  all)    $BASE test --release && $BASE clippy --all-targets -- -D warnings \
          && $BASE check --target wasm32-unknown-unknown ;;
  wasm-build) $BASE build --release --target wasm32-unknown-unknown "$@" ;;
  bindgen) docker run --rm -v crown-src:/work -v crown-target:/work/target -w /work "$IMAGE" \
      tools/bin/wasm-bindgen --target web --out-dir /work/web/pkg \
      /work/target/wasm32-unknown-unknown/release/defender_of_the_crown.wasm ;;
  *) echo "unknown: $CMD" && exit 2 ;;
esac
