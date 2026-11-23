#!/usr/bin/env bash
# scripts/test-all.sh — the single green-light command (SPEC DONE-3).
#
#   fmt         cargo fmt --check: every workspace crate (not --all, which also checks local path
#               dependencies, i.e. ../labkit, another team's live work)
#   lint        shellcheck --severity=warning scripts/*.sh (skipped without shellcheck)
#   clippy      cargo clippy --workspace --all-targets -D warnings
#   test        cargo test --workspace: engine unit, golden and property tests, GUI tests with
#               egui_kittest, end-to-end journeys, MCP tests that spawn the real server; counted
#               per crate and by kind. A test running longer than 60 s fails the step.
#   perf        release perf gates (ENG-4, PLAT-1): every test target named `perf` or
#               `*_perf` (e.g. crates/sp-engine/tests/advanced_perf.rs), run in release with
#               --include-ignored --nocapture: the target must pass (its own asserts: 60-joint
#               Bridge solve < 1 ms, one Test tick < 8 ms, the influence line < 5 ms, …), and
#               each line it prints as
#                   PERF <label>: <value> ms (limit <limit> ms)
#               is also listed as a gate (anywhere on the line: libtest prints the first
#               one after `test name ... `). The engine's perf checks that live in other test
#               targets — test functions whose names contain `perf`, `budget`, `_is_fast` or
#               `_are_cheap` (bridge_props `performance_budget`, tower_props `calc_is_fast`,
#               dynamics `sway_steps_are_cheap`) — run in release too and must pass. Then
#               start-up → first frame of `--smoke 3` < 2 s. A gate whose test does not exist
#               yet is SKIP, never FAIL.
#   bundle      macOS only: scripts/bundle-macos.sh --check — builds "dist/Structures
#               Playground.app" (PLAT-2) and runs scripts/check-bundle.sh (signature, Info.plist,
#               icon, no machine paths, document types, sandboxed smoke runs)
#
#   scripts/test-all.sh              everything
#   scripts/test-all.sh --quick      skip the release perf gates and the bundle
#   SKIP=perf,bundle scripts/test-all.sh
#
# Works from any directory. Builds into $CARGO_TARGET_DIR (default target/test-all); logs go to
# $CARGO_TARGET_DIR/test-all-logs. Perf tests are #[ignore]d so the everyday suite stays fast.
set -uo pipefail   # not -e: every step runs and the summary reports each one
cd "$(dirname "$0")/.." || exit 2
ROOT="$(pwd -P)"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target/test-all}"
case "$CARGO_TARGET_DIR" in /*) ;; *) CARGO_TARGET_DIR="$ROOT/$CARGO_TARGET_DIR" ;; esac
LOGS="$CARGO_TARGET_DIR/test-all-logs"
mkdir -p "$LOGS"
SKIP="${SKIP:-}"
case "${1:-}" in
  --quick) SKIP="$SKIP,perf,bundle" ;;
  -h|--help) sed -n '2,32p' "$0"; exit 0 ;;
  "") ;;
  *) echo "unknown option $1 (try --help)" >&2; exit 2 ;;
esac

if [ -t 1 ]; then G=$'\e[32m'; R=$'\e[31m'; Y=$'\e[33m'; B=$'\e[1m'; N=$'\e[0m'; else G='' R='' Y='' B='' N=''; fi

STEPS=()     # "name|status|seconds|note"
PERF=()      # "gate|value|limit|status"
FAILED=0

skipped() { [[ ",$SKIP," == *",$1,"* ]]; }

# run_step <name> <log> <command…>
run_step() {
  local name="$1" log="$LOGS/$2.log"; shift 2
  printf '%s── %s%s\n' "$B" "$name" "$N"
  local t0=$SECONDS
  "$@" >"$log" 2>&1
  local rc=$? dt=$((SECONDS - t0))
  if [ $rc -eq 0 ]; then
    printf '   %sok%s (%ss)\n' "$G" "$N" "$dt"
    STEPS+=("$name|PASS|$dt|")
  else
    printf '   %sFAILED%s (%ss) — log: %s\n' "$R" "$N" "$dt" "$log"
    tail -n 25 "$log" | sed 's/^/   │ /'
    STEPS+=("$name|FAIL|$dt|see $log")
    FAILED=1
  fi
  return $rc
}

skip_step() { printf '%s── %s%s\n   %sskipped%s: %s\n' "$B" "$1" "$N" "$Y" "$N" "$2"; STEPS+=("$1|SKIP|0|$2"); }

# perf_gate <gate> <value> <op> <limit> <unit>: an empty value (the test ran, printed nothing) fails
perf_gate() {
  local gate="$1" v="$2" op="$3" lim="$4" unit="$5" st
  if [ -z "$v" ]; then st=FAIL; v="?";
  elif awk -v v="$v" -v l="$lim" -v op="$op" 'BEGIN { exit !((op == ">=") ? (v >= l) : (v < l)) }'; then st=PASS;
  else st=FAIL; fi
  [ "$st" = FAIL ] && FAILED=1
  PERF+=("$gate|$v $unit|$op $lim $unit|$st")
}
# perf_skip <gate> <op> <limit> <unit> <why>: the test / example does not exist yet
perf_skip() { PERF+=("$1|$5|$2 $3 $4|SKIP"); }

# perf_lines <log>: every `PERF <label>: <v> ms (limit <l> ms)` in it as "label|v|l" (not
# anchored: libtest prints the first one after `test name ... `)
perf_lines() {
  sed -n 's/.*PERF \([^:]*\): \([0-9.][0-9.]*\) ms (limit \([0-9.][0-9.]*\) ms).*/\1|\2|\3/p' "$1"
}

# workspace facts from cargo metadata, e.g. `has_target perf test`
META="$(cargo metadata --no-deps --format-version 1 2>/dev/null || echo '{"packages":[]}')"
has_target() { # has_target <target name> <kind> → the packages that have it, space-separated
  python3 -c '
import json, sys
name, kind = sys.argv[1], sys.argv[2]
print(" ".join(p["name"] for p in json.load(sys.stdin)["packages"]
               if any(t["name"] == name and kind in t["kind"] for t in p["targets"])))' "$1" "$2" <<<"$META"
}
# Runs a command for at most <seconds> (exit 142 when the alarm fires); called through run_step.
# shellcheck disable=SC2329
with_timeout() { local s="$1"; shift; perl -e 'alarm shift; exec @ARGV or die "exec: $!"' "$s" "$@"; }

# ------------------------------------------------------------------ environment
echo "${B}Structures Playground test-all${N}  ($(date '+%Y-%m-%d %H:%M'), target dir $CARGO_TARGET_DIR)"
echo "  rustc        $(rustc --version 2>/dev/null)"
echo "  crates       $(python3 -c 'import json,sys; print(" ".join(sorted(p["name"] for p in json.load(sys.stdin)["packages"])))' <<<"$META")"
SHELLCHECK="$(command -v shellcheck || true)"
echo "  shellcheck   ${SHELLCHECK:-absent (script lint skipped)}"
echo

# ------------------------------------------------------------------ fmt
if skipped fmt; then skip_step "fmt" "SKIP"; else
  run_step "fmt --check (workspace crates)" fmt cargo fmt --check
fi

# ------------------------------------------------------------------ lint (shell scripts)
if skipped lint; then skip_step "lint" "SKIP"
elif [ -z "$SHELLCHECK" ]; then skip_step "lint: shellcheck scripts/*.sh" "shellcheck not installed"
else
  run_step "lint: shellcheck -S warning scripts/*.sh" lint bash -c 'shellcheck --severity=warning scripts/*.sh'
fi

# ------------------------------------------------------------------ clippy
if skipped clippy; then skip_step "clippy" "SKIP"; else
  run_step "clippy --workspace --all-targets -D warnings" clippy \
    cargo clippy --workspace --all-targets -- -D warnings
fi

# ------------------------------------------------------------------ tests
if skipped test; then skip_step "test" "SKIP"; else
  if run_step "cargo test --workspace (unit · reference · golden · e2e · GUI · snapshots · MCP)" test \
      cargo test --workspace --no-fail-fast; then
    if grep -q "has been running for over 60 seconds" "$LOGS/test.log"; then
      echo "   ${R}slow tests${N}:"; grep "has been running for over 60 seconds" "$LOGS/test.log" | sed 's/^/   │ /'
      STEPS[${#STEPS[@]}-1]="cargo test --workspace|FAIL|0|a test ran longer than 60 s"
      FAILED=1
    fi
  fi
  passed=$(grep -E "^test result: " "$LOGS/test.log" | awk '{s+=$4} END {print s+0}')
  failed=$(grep -E "^test result: " "$LOGS/test.log" | awk '{s+=$6} END {print s+0}')
  ignored=$(grep -E "^test result: " "$LOGS/test.log" | awk '{s+=$8} END {print s+0}')
  echo "   $passed passed, $failed failed, $ignored ignored (perf gates run below)"
  # Attribute each test binary's result to its crate and kind: map the executables cargo built
  # (already compiled, so this is quick) to their package and target.
  if cargo test --workspace --no-run --message-format=json >"$LOGS/test-artifacts.json" 2>/dev/null; then
    python3 - "$LOGS/test.log" "$LOGS/test-artifacts.json" <<'PY' | sed 's/^/   /'
import json, os, re, sys
log, artifacts = sys.argv[1], sys.argv[2]
exe = {}  # executable basename → (package dir name, target kind, target name, src path)
for line in open(artifacts):
    try:
        m = json.loads(line)
    except ValueError:
        continue
    if m.get("reason") == "compiler-artifact" and m.get("executable"):
        t = m["target"]
        pkg = os.path.basename(os.path.dirname(m["manifest_path"]))
        exe[os.path.basename(m["executable"])] = (pkg, t["kind"][0], t["name"], t["src_path"])
per_crate, kinds, current = {}, {"GUI": 0, "e2e": 0, "MCP": 0, "snapshot": 0}, None
for line in open(log):
    r = re.search(r"^\s+Running (.+?) \((.+)\)\s*$", line)
    d = re.search(r"^\s+Doc-tests (\S+)", line)
    if r:
        current = exe.get(os.path.basename(r.group(2)))
    elif d:
        current = (d.group(1).replace("_", "-"), "doc", d.group(1), "")
    elif line.startswith("test result: ") and current:
        n = int(line.split()[3].rstrip(";"))
        pkg, kind, name, src = current
        per_crate[pkg] = per_crate.get(pkg, 0) + n
        if pkg == "sp-mcp" and kind == "test":
            kinds["MCP"] += n  # through the real structures-mcp binary
        elif ("/tests/e2e/" in src or "e2e" in name or "scenario" in name or name == "journeys"
              or pkg.endswith("e2e")):
            kinds["e2e"] += n
        elif pkg == "structures-playground" and kind == "test":
            kinds["GUI"] += n
        elif "snapshot" in name:
            kinds["snapshot"] += n
        current = None
print("by crate: " + " · ".join(f"{k} {v}" for k, v in sorted(per_crate.items())))
print(f"GUI (egui_kittest, app integration tests) {kinds['GUI']} · e2e journeys {kinds['e2e']}"
      f" · MCP {kinds['MCP']} · snapshots {kinds['snapshot']}")
PY
  fi
fi

# ------------------------------------------------------------------ perf (release)
if skipped perf; then skip_step "perf (release)" "SKIP / --quick"; else
  # test targets named perf / *_perf, as "package:target"
  perf_targets=$(python3 -c '
import json, sys
for p in json.load(sys.stdin)["packages"]:
    for t in p["targets"]:
        if "test" in t["kind"] and (t["name"] == "perf" or t["name"].endswith("_perf")):
            print(p["name"] + ":" + t["name"])' <<<"$META")
  if [ -z "$perf_targets" ]; then
    skip_step "perf: tests/*perf.rs" "no crate has a perf test yet"
    perf_skip "engine perf (ENG-4)" "<" "-" ms "no perf test yet"
  else
    for pt in $perf_targets; do
      pkg="${pt%%:*}"; tgt="${pt#*:}"
      if run_step "perf: $pkg --test $tgt (release, incl. ignored)" "perf-$tgt" \
          cargo test --release -p "$pkg" --test "$tgt" -- --include-ignored --nocapture --test-threads=1; then
        PERF+=("$pkg $tgt (its asserts)|passed|asserts|PASS")
      else
        PERF+=("$pkg $tgt (its asserts)|failed|asserts|FAIL")
      fi
      while IFS='|' read -r label value limit; do
        [ -z "$label" ] && continue
        perf_gate "$label" "$value" "<" "$limit" "ms"
      done < <(perf_lines "$LOGS/perf-$tgt.log")
    done
  fi

  # the engine's perf checks inside its other test targets (property suites, dynamics)
  if [[ " $(has_target sp_engine lib) " == *" sp-engine "* ]]; then
    if run_step "perf: sp-engine checks named perf / budget / _is_fast / _are_cheap (release)" perf-engine \
        cargo test --release -p sp-engine --tests -- --nocapture --test-threads=1 \
          perf budget _is_fast _are_cheap; then
      n=$(grep -E "^test .*(perf|budget|_is_fast|_are_cheap).* \.\.\. ok" "$LOGS/perf-engine.log" | wc -l | tr -d ' ')
      PERF+=("sp-engine in-suite perf checks ($n)|passed|asserts|PASS")
    else
      PERF+=("sp-engine in-suite perf checks|failed|asserts|FAIL")
    fi
    while IFS='|' read -r label value limit; do
      [ -z "$label" ] && continue
      perf_gate "$label" "$value" "<" "$limit" "ms"
    done < <(perf_lines "$LOGS/perf-engine.log")
  fi

  if run_step "perf: build the release app" build-release \
      cargo build --release --locked -p structures-playground --bin structures-playground; then
    BIN="$CARGO_TARGET_DIR/release/structures-playground"
    if strings -a "$BIN" | grep -F -- '--smoke' >/dev/null; then   # not -q: SIGPIPE + pipefail
      run_step "smoke: the Bridge 3 s with the GUI open" smoke with_timeout 60 "$BIN" --smoke 3
      line=$(grep '^SMOKE' "$LOGS/smoke.log" | tail -1); echo "   ${line:-no SMOKE line}"
      ff=$(sed -n 's/.*first_frame_ms=\([0-9.]*\).*/\1/p' <<<"$line")
      perf_gate "start-up → first frame (--smoke)" "$ff" "<" 2000 "ms"
    else
      perf_skip "start-up → first frame" "<" 2000 ms "app has no --smoke yet"
    fi
  fi
fi

# ------------------------------------------------------------------ bundle (macOS app)
if skipped bundle; then skip_step "bundle" "SKIP / --quick"
elif [ "$(uname)" != Darwin ]; then skip_step "bundle" "not macOS"
else
  run_step "bundle: dist/Structures Playground.app + check-bundle" bundle \
    scripts/bundle-macos.sh --check
  grep -E '^  (FAIL|SKIP) ' "$LOGS/bundle.log" | sed 's/^/  /'
  grep -E '^bundle check' "$LOGS/bundle.log" | sed 's/^/   /'
fi

# ------------------------------------------------------------------ summary
echo
echo "${B}Summary${N}"
for s in "${STEPS[@]}"; do
  IFS='|' read -r name st dt note <<<"$s"
  case "$st" in PASS) c=$G ;; FAIL) c=$R ;; *) c=$Y ;; esac
  printf '  %s%-5s%s %4ss  %s%s\n' "$c" "$st" "$N" "$dt" "$name" "${note:+  ($note)}"
done
if [ ${#PERF[@]} -gt 0 ]; then
  echo
  echo "${B}Performance gates${N}"
  for p in "${PERF[@]}"; do
    IFS='|' read -r gate v lim st <<<"$p"
    case "$st" in PASS) c=$G ;; SKIP) c=$Y ;; *) c=$R ;; esac
    printf '  %s%-5s%s %-46s %-24s (%s)\n' "$c" "$st" "$N" "$gate" "$v" "$lim"
  done
fi
echo
if [ $FAILED -eq 0 ]; then
  echo "${G}${B}ALL GREEN${N}"
else
  echo "${R}${B}FAILED${N} — logs in $LOGS"
fi
exit $FAILED
