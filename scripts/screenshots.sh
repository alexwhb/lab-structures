#!/usr/bin/env bash
# scripts/screenshots.sh — renders the README's screenshots: every screen at its mockup board
# size (1440 px wide) with the real app in egui_kittest (wgpu, 1×), into
# docs/screenshots/<screen>.png (route ids: bridge, tower, ground, materials, advanced, series).
#
#   scripts/screenshots.sh              all screens
#   scripts/screenshots.sh tower        only some
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/screenshots}"

only=""
if [[ $# -gt 0 ]]; then
  only="$(IFS=,; echo "$*")"
fi

SP_SCREENSHOTS=1 SP_COMPARE_ONLY="$only" \
  cargo test -p structures-playground --test compare -- --nocapture readme_screenshots \
  | grep -E '^screenshot:' || true
