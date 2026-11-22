#!/usr/bin/env bash
# Builds "dist/Structures Playground.app" (SPEC PLAT-2) — a self-contained, ad-hoc signed macOS app.
#
#   scripts/bundle-macos.sh            # release build → dist/Structures Playground.app
#   scripts/bundle-macos.sh --dmg      # … plus dist/Structures Playground.dmg (drag to Applications)
#   scripts/bundle-macos.sh --check    # … then scripts/check-bundle.sh (signature, Info.plist,
#                                      #     no machine paths, sandboxed smoke runs)
#
# The binaries are built in release with remapped paths (--remap-path-prefix), so nothing in the
# bundle points at this machine, the checkout or the sibling labkit repo. Fonts, theme and
# mockup data are compiled into the binary, so the bundle carries no data files.
#
# Contents/
#   Info.plist                   from packaging/Info.plist.in: dev.labseries.StructuresPlayground,
#                                version from Cargo, macOS 14+, education category; .str truss
#                                netlists registered (Finder Open With → Advanced)
#   MacOS/structures-playground  the app
#   MacOS/structures-mcp         the MCP server (stdio), once the sp-mcp crate exists; point an
#                                MCP client at …/Structures Playground.app/Contents/MacOS/structures-mcp
#   Resources/
#     Structures Playground.icns the app icon (assets/icon/, rendered by assets/icon/render-icon.sh)
#     licenses/                Inter, JetBrains Mono, Space Grotesk OFL licences (the fonts come
#                              from labkit's lab-theme) + THIRD-PARTY.md (every linked crate)
#     build-info.json          version, build number, commit, build date
#
# Bundle builds get their own target dir ($CARGO_TARGET_DIR/bundle-macos, default
# target/bundle-macos): their rustflags differ from everyday release builds.
set -euo pipefail
cd "$(dirname "$0")/.."
repo="$(pwd -P)"
export PATH="$HOME/.cargo/bin:$PATH"

dmg=0
check=0
for a in "$@"; do
  case "$a" in
    --dmg) dmg=1 ;;
    --check) check=1 ;;
    -h|--help) sed -n '2,7p' "$0"; exit 0 ;;
    *) echo "unknown option $a" >&2; exit 2 ;;
  esac
done
[ "$(uname)" = Darwin ] || { echo "bundle-macos.sh: macOS only" >&2; exit 2; }

say() { printf '\033[1m==> %s\033[0m\n' "$*"; }

name="Structures Playground"   # the bundle id lives in packaging/Info.plist.in
base_target="${CARGO_TARGET_DIR:-target}"
case "$base_target" in /*) ;; *) base_target="$repo/$base_target" ;; esac
target_dir="${STRUCTURES_BUNDLE_TARGET_DIR:-$base_target/bundle-macos}"
dist="$repo/dist"
app="$dist/$name.app"

# One metadata read: the app's version, and whether the MCP server crate exists yet.
meta="$(cargo metadata --no-deps --format-version 1)"
version=$(python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"]=="structures-playground"))' <<<"$meta")
mcp_pkg=$(python3 -c '
import json, sys
for p in json.load(sys.stdin)["packages"]:
    if any(t["name"] == "structures-mcp" and "bin" in t["kind"] for t in p["targets"]):
        print(p["name"]); break' <<<"$meta")
commit=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)
build=$(git rev-list --count HEAD 2>/dev/null || echo 1)
git diff --quiet HEAD 2>/dev/null || commit="$commit-dirty"

say "Building $name $version ($commit) for the bundle${mcp_pkg:+ (+ structures-mcp)}"
# --remap-path-prefix covers debug info, panic locations and file!(), not env!() values:
# STRUCTURES_BUNDLE=1 tells the crates to compile out their env!("CARGO_MANIFEST_DIR") checkout
# fallbacks (`option_env!("STRUCTURES_BUNDLE")`, like Breadbox's BREADBOX_BUNDLE).
export STRUCTURES_BUNDLE=1
# rustc applies the last matching prefix: the checkout, then the lab-series folder (labkit),
# then anything else under $HOME (cargo registry, rustup toolchain).
lab_series="$(dirname "$repo")"
# CARGO_ENCODED_RUSTFLAGS (0x1f-separated) keeps paths with spaces intact; it replaces RUSTFLAGS,
# so any RUSTFLAGS from the environment are carried over word by word.
sep=$'\x1f'
flags="--remap-path-prefix=$HOME=~${sep}--remap-path-prefix=$lab_series=/lab-series${sep}--remap-path-prefix=$repo=/structures-playground"
extra="${CARGO_ENCODED_RUSTFLAGS:-}"
if [ -z "$extra" ] && [ -n "${RUSTFLAGS:-}" ]; then
  read -r -a words <<<"$RUSTFLAGS"
  extra=$(IFS="$sep"; echo "${words[*]}")
fi
export CARGO_ENCODED_RUSTFLAGS="${extra:+$extra$sep}$flags"
unset RUSTFLAGS
build_args=(-p structures-playground --bin structures-playground)
[ -n "$mcp_pkg" ] && build_args+=(-p "$mcp_pkg" --bin structures-mcp)
CARGO_TARGET_DIR="$target_dir" cargo build --release --locked "${build_args[@]}"

work="$(mktemp -d "${TMPDIR:-/tmp}/structures-bundle.XXXXXX")"
trap 'rm -rf "$work"' EXIT
stage="$work/$name.app"
contents="$stage/Contents"
res="$contents/Resources"
mkdir -p "$contents/MacOS" "$res/licenses"

say "Assembling $name.app"
cp "$target_dir/release/structures-playground" "$contents/MacOS/structures-playground"
if [ -n "$mcp_pkg" ]; then
  cp "$target_dir/release/structures-mcp" "$contents/MacOS/structures-mcp"
fi
for b in "$contents"/MacOS/*; do strip -x "$b"; done
printf 'APPL????' > "$contents/PkgInfo"
cp "assets/icon/$name.icns" "$res/$name.icns"

sed -e "s/@VERSION@/$version/g" -e "s/@BUILD@/$build/g" packaging/Info.plist.in > "$contents/Info.plist"
plutil -lint "$contents/Info.plist" >/dev/null

# Font licences: the fonts are embedded by labkit's lab-theme; take the OFL texts from there.
full_meta="$(cargo metadata --locked --format-version 1 --filter-platform "$(rustc -vV | sed -n 's/^host: //p')")"
lab_theme_dir=$(python3 -c '
import json, os, sys
m = json.load(sys.stdin)
print(next(os.path.dirname(p["manifest_path"]) for p in m["packages"] if p["name"] == "lab-theme"))' <<<"$full_meta")
fonts="$lab_theme_dir/../../assets/fonts"
for f in Inter JetBrainsMono SpaceGrotesk; do
  cp "$fonts/$f-OFL.txt" "$res/licenses/$f-OFL.txt"
done
[ -f LICENSE ] && cp LICENSE "$res/licenses/LICENSE-Structures-Playground.txt"

# Third-party notices: every crates.io crate linked into the shipped binaries (normal
# dependencies, this platform). This workspace and labkit (a path dependency) are lab-series code.
# shellcheck disable=SC2016  # the Python program is meant to be single-quoted
ROOTS="structures-playground${mcp_pkg:+ $mcp_pkg}" python3 -c '
import json, os, sys
m = json.load(sys.stdin)
pk = {p["id"]: p for p in m["packages"]}
nodes = {n["id"]: n for n in m["resolve"]["nodes"]}
roots = os.environ["ROOTS"].split()
seen, todo = set(), [p["id"] for p in m["packages"] if p["name"] in roots and p["source"] is None]
while todo:
    i = todo.pop()
    if i in seen:
        continue
    seen.add(i)
    for d in nodes[i]["deps"]:
        if any(k["kind"] is None for k in d["dep_kinds"]):
            todo.append(d["pkg"])
rows = sorted((pk[i]["name"], pk[i]["version"], pk[i].get("license") or "see crate",
               pk[i].get("repository") or "") for i in seen
              if (pk[i]["source"] or "").startswith("registry+"))
print("# Third-party software in Structures Playground\n")
print("Structures Playground links the Rust crates below (license expressions as published on")
print("crates.io). Fonts: Inter, JetBrains Mono and Space Grotesk under the SIL Open Font")
print("License 1.1 (`*-OFL.txt` next to this file).\n")
print("| crate | version | license | repository |\n|---|---|---|---|")
for r in rows:
    print("| %s | %s | %s | %s |" % r)
' <<<"$full_meta" > "$res/licenses/THIRD-PARTY.md"

cat > "$res/build-info.json" <<JSON
{ "name": "$name", "version": "$version", "build": "$build", "commit": "$commit", "built": "$(date -u +%Y-%m-%dT%H:%M:%SZ)", "mcp": $([ -n "$mcp_pkg" ] && echo true || echo false) }
JSON

say "Signing (ad hoc)"
codesign --force --deep --sign - "$stage"
codesign --verify --deep --strict "$stage"

mkdir -p "$dist"
rm -rf "$app"
mv "$stage" "$app"
say "dist/$name.app ($(du -sh "$app" | cut -f1))"

if [ $dmg -eq 1 ]; then
  say "Building dist/$name.dmg"
  img="$work/dmg"
  mkdir -p "$img"
  cp -R "$app" "$img/"
  ln -s /Applications "$img/Applications"
  rm -f "$dist/$name.dmg"
  hdiutil create -quiet -volname "$name $version" -srcfolder "$img" -ov -format UDZO "$dist/$name.dmg"
  say "dist/$name.dmg ($(du -h "$dist/$name.dmg" | cut -f1))"
fi

if [ $check -eq 1 ]; then
  "$repo/scripts/check-bundle.sh" "$app"
fi
