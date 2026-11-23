#!/usr/bin/env bash
# scripts/screenshots.sh — renders the README's screenshots: every screen at its mockup board
# size (1440 px wide) with the real app in egui_kittest (wgpu, 1×), into
# docs/screenshots/<screen>.png (route ids: playground, advanced, series), plus the README's
# hero shots of the builder (docs/screenshots/builder-*.png) once it renders.
#
#   scripts/screenshots.sh              all screens
#   scripts/screenshots.sh advanced     only some
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/screenshots}"

only=""
if [[ $# -gt 0 ]]; then
  only="$(IFS=,; echo "$*")"
fi

# a build failure or a panic must not pass silently (leaving stale PNGs behind)
log="$(mktemp "${TMPDIR:-/tmp}/screenshots.XXXXXX")"
trap 'rm -f "$log"' EXIT
set +e
SP_SCREENSHOTS=1 SP_COMPARE_ONLY="$only" \
  cargo test -p structures-playground --test compare -- --nocapture readme_screenshots \
  >"$log" 2>&1
status=$?
set -e
grep -E '^screenshot:' "$log" || true
if [[ $status -ne 0 ]]; then
  tail -n 40 "$log" >&2
  echo "screenshots: the render failed (exit $status); docs/screenshots/ is not up to date" >&2
  exit "$status"
fi
