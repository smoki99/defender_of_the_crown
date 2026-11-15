#!/bin/sh
# GATE ALPHA 1 (A3): minigame baselines - one PLAY frame (spawn, hands off)
# and one result PLATE (autoplay resolution) per shipped sport, driven
# through the ?scene=mg deep link so each capture is cold-boot deterministic.
#   ./tools/baseline-sports.sh              # diff vs committed baselines
#   UPDATE=1 ./tools/baseline-sports.sh     # recapture
set -e
cd "$(dirname "$0")/.."
for g in sword joust catapult archery rampart maze river cloak forge herbs falcon; do
  STATE=sport-play-$g TOL=3000 SETTLE=250 \
    CROWN_QS="scene=mg&mg=$g" bun tools/sports-baseline-one.mjs
  STATE=sport-plate-$g SETTLE=800 \
    CROWN_QS="scene=mg&mg=$g&autoplay=1" bun tools/sports-baseline-one.mjs
done
