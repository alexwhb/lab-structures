#!/usr/bin/env bash
# scripts/compare-mockup.sh — renders every app screen next to its mockup reference (LOOK-1).
#
#   scripts/compare-mockup.sh                  all screens
#   scripts/compare-mockup.sh series playground  only some (route ids or mockup names)
#
# For each screen it renders the real app (egui_kittest, wgpu, 1×) at the mockup's board size
# (1440 × 900 / 1000 / 1400) and at 390 px wide (as tall as the reference or the page,
# whichever is taller), and writes
#
#   target/compare/<Screen>-1440.png       [app | mockup] side by side
#   target/compare/<Screen>-390.png
#   target/compare/<Screen>-1440-app.png   the app alone
#
# The references are docs/design/screenshots/<Screen>-<width>.png (scripts/render-mockup.mjs),
# and for the Playground the builder's first board, docs/design/v2/screenshots/BridgeBuild-<width>.png
# (scripts/render-mockup-v2.mjs); `<Screen>` is Builder, Advanced, Series.
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

# a build failure or a panic must not pass silently (leaving last run's PNGs behind)
log="$(mktemp "${TMPDIR:-/tmp}/compare-mockup.XXXXXX")"
trap 'rm -f "$log"' EXIT
set +e
SP_COMPARE=1 SP_COMPARE_ONLY="$only" \
  cargo test -p structures-playground --test compare -- --nocapture side_by_side_with_mockup \
  >"$log" 2>&1
status=$?
set -e
grep -E '^compare:' "$log" || true
if [[ $status -ne 0 ]]; then
  tail -n 40 "$log" >&2
  echo "compare-mockup: the render failed (exit $status); target/compare/ is not up to date" >&2
  exit "$status"
fi

echo "side-by-side renders in $repo/target/compare/"
ls -1 "$repo"/target/compare/*.png 2>/dev/null | sed 's|^|  |'
