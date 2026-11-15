#!/bin/sh
# SPEC §13.2 entry point: build the shipped web bundle and enforce the
# 2 MB payload budget (wasm + JS glue + embedded MOD tracks).
# The budget check lives in tools/build-web.ts and fails the build.
set -e
exec bun tools/build-web.ts "$@"
