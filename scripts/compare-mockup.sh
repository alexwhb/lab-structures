#!/usr/bin/env bash
# scripts/compare-mockup.sh — renders every app screen next to its mockup reference (LOOK-1).
#
#   scripts/compare-mockup.sh                  all screens
#   scripts/compare-mockup.sh series tower     only some (route ids or mockup names)
#
# For each screen it renders the real app (egui_kittest, wgpu, 1×) at the mockup's board size
# (1440 × 900 / 1000 / 1400) and at 390 px wide (as tall as the reference or the page,
# whichever is taller), and writes
#
#   target/compare/<Screen>-1440.png       [app | mockup] side by side
#   target/compare/<Screen>-390.png
#   target/compare/<Screen>-1440-app.png   the app alone
#
# The references are docs/design/screenshots/<Screen>-<width>.png (scripts/render-mockup.mjs).
# Animated screens are captured at rest, so compare layout, sizes and colours rather than
# pixels. Each line printed is `compare: <Screen>-<width> mean |Δ| <n>`.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/compare-build}"

only=""
if [[ $# -gt 0 ]]; then
  only="$(IFS=,; echo "$*")"
fi

SP_COMPARE=1 SP_COMPARE_ONLY="$only" \
  cargo test -p structures-playground --test compare -- --nocapture side_by_side_with_mockup \
  | grep -E '^compare:' || true

echo "side-by-side renders in $repo/target/compare/"
ls -1 "$repo"/target/compare/*.png 2>/dev/null | sed 's|^|  |'
