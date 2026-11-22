# Progress

## Status

| Wave | Work | Status |
|---|---|---|
| 0 | repo, spec, architecture, mockup + references, `sp-theme`, `sp-engine` truss core | done (lead) |
| 1 | app shell + Series + packaging · Bridge · Advanced · Tower + Ground · Materials | in progress (shell: done — skeleton, header + world tabs + provenance badges, keyboard, Share, hand-offs, persistence, Series, smoke, pacing, packaging + scripts, icon; see shell status under Requests) |
| 2 | MCP · e2e journeys · visual pass · README · independent review + fixes | not started |

## Decisions (lead, 2026-11-22)

- Location `~/Projects/Personal/lab-series/structures-playground/`, its own git repo, like the
  siblings. Shared code comes from `../labkit` by path.
- Stack as the siblings: eframe/egui 0.37 (wgpu, AccessKit), egui_kittest, edition 2024.
- **Automata Playground is the template**: its app shell (`app.rs`, `ctx.rs`, `layout.rs`,
  `router.rs`, `widgets/`, `smoke.rs`, `platform.rs`, `control.rs`, the kittest harness), its
  `ap-shared` / `ap-mcp` and its scripts are forked and renamed (`ap` → `sp`, magenta → sand),
  not rewritten. Structures' mockup uses the same Breadbox look with 36 px controls.
- `sp-engine::truss` (lead, wave 0) is the one stiffness solver: the Bridge, Advanced and MCP all
  call it. It reproduces the Advanced mockup's numbers exactly (`tests/mockup_pratt.rs`).
- The Series board has no header; it is reached from the brand (NAV-3). The header has no app
  switcher (the mockup has none).
- Mockup bugs not to copy: SPEC "Mockup bugs not to copy" (MB-1: the floating `.chip`).

## Ownership (wave 1)

| Agent | Owns |
|---|---|
| `shell` | `crates/structures-playground/**` except the screen files below; `crates/sp-shared/**`, `crates/sp-mcp/**` (wave 2); `scripts/**` except `scripts/render-mockup.mjs` and `scripts/mockup-goldens.mjs`; `assets/**`, `packaging/**`; README (wave 2) |
| `bridge` | `crates/sp-engine/src/bridge/**` (+ `src/bridge.rs`), `crates/sp-engine/tests/bridge*.rs`; `crates/structures-playground/src/screens/bridge.rs` + `src/screens/bridge/**`; `crates/structures-playground/tests/bridge*.rs`; `crates/sp-theme/src/bridge.rs` |
| `advanced` | `crates/sp-engine/src/{netlist,generate,influence}/**` (+ their `.rs`), `crates/sp-engine/tests/{netlist,advanced}*.rs`, `scripts/mockup-goldens.mjs`; `crates/structures-playground/src/screens/advanced.rs` + `src/screens/advanced/**`; `crates/structures-playground/tests/advanced*.rs`; `crates/sp-theme/src/advanced.rs` |
| `worlds` | `crates/sp-engine/src/{tower,ground,dynamics}/**` (+ `.rs`), `crates/sp-engine/tests/{tower,ground,dynamics}*.rs`; `crates/structures-playground/src/screens/{tower,ground}.rs` + `src/screens/{tower,ground}/**`; `crates/structures-playground/tests/{tower,ground}*.rs`; `crates/sp-theme/src/{tower,ground}.rs` |
| `materials` | `crates/sp-engine/src/materials/**` (+ `.rs`), `crates/sp-engine/tests/materials*.rs`; `crates/structures-playground/src/screens/materials.rs` + `src/screens/materials/**`; `crates/structures-playground/tests/materials*.rs`; `crates/sp-theme/src/materials.rs`; `docs/DATA-SOURCES.md` |
| lead | `SPEC.md`, `PROGRESS.md` status, `docs/ARCHITECTURE.md` skeleton, `crates/sp-theme/src/lib.rs`, `crates/sp-engine/src/{lib,linalg,truss}.rs`, `crates/sp-engine/tests/mockup_pratt.rs`, `scripts/render-mockup.mjs`, root `Cargo.toml` |

Shared-file etiquette:
- `crates/sp-engine/src/lib.rs`: each engine owner may add their own `pub mod <name>;` line (and
  a re-export line) — one-line edits, re-read the file first.
- `crates/sp-engine/src/truss.rs`: lead-owned. Need a change (e.g. a faster solve, a new field)?
  Write it under Requests; small additive changes may be made by `bridge` or `advanced` if the
  lead is unavailable, with all existing tests still green, noted in Requests.
- `crates/sp-theme/src/lib.rs`: `shell` added shared widget tokens (additive, wave 1):
  `RANGE_TRACK`, `RANGE_BORDER`, `PLOT_GUIDE`, `PLOT_FILL`, `RULE`, each with its CSS source.
- `crates/structures-playground/src/screens/mod.rs` and the router: the shell creates
  placeholder screens (`bridge.rs`, `advanced.rs`, `tower.rs`, `ground.rs`, `materials.rs`) and
  the screen owners take them over (replace the placeholder body; keep the public API the shell
  documents in `docs/ARCHITECTURE.md` § App).
- Screen agents may ADD widget files under `src/widgets/` for widgets only their screen needs
  (register with a one-line `pub mod` in `src/widgets/mod.rs`); changes to existing widgets,
  the harness, `app.rs`, `ctx.rs` go under Requests to `shell`.

## Requests

- **shell (status, a57f608)**: the app skeleton is in. `cargo build -p structures-playground`
  / `cargo test -p structures-playground` pass. Screen agents: take over your placeholder
  `src/screens/<screen>.rs` (keep `XxxScreen::new()` and the `Screen` impl); the API — header
  hooks (`drive_button` / `drive` / `reset` / `clock` / `share` / `provenance` / `document`),
  `Ctx`, `Carry` hand-offs, `Command`s, widgets, layout, the kittest harness — is in
  `docs/ARCHITECTURE.md` › App. Requests for the shell go here.
- **shell (status, wave 1 done)**: header (NAV-1) + world tab row with provenance badges
  (NAV-2) match the 1440 / 390 references (closeness test in `tests/compare.rs`); Back / Esc
  from Series (NAV-3); ⌘1–⌘5, Space → drive, ⌘Z / Delete / ⌘V as `Command`s, Tab order (NAV-5);
  Share → clipboard + toast (NAV-6); last screen + window size persisted, fixed seed (NAV-7);
  Series (SR-1) with the launcher; `--smoke` (PLAT-1); pacing tests — **every route at rest
  must ask for no frames** (`tests/pacing.rs`, PLAT-3); `scripts/test-all.sh`,
  `bundle-macos.sh`, `check-bundle.sh`, `compare-mockup.sh <screen>`, `screenshots.sh`;
  `assets/icon/`. Shared widgets added for the screens: `Toggle` is now the mockup's `.chip`
  (`.dot()`, `.dimmed()`), `Checkbox` (Tower damper / Ground quake / Advanced cross-braced),
  `badge`, `Callout::happening`, `layout::{Column::rail, parts, main, readouts}`,
  `graph_panel`. Perf tests: name them `perf` / `*_perf` and optionally print
  `PERF <label>: <ms> ms (limit <ms> ms)` (ARCHITECTURE § App › harness). Note: egui panics in
  tests on a glyph the bundled fonts lack (e.g. `⇄`, `ẍ`): check unusual characters.

- **bridge → advanced** (NAV-4 / ADV-9 / NAV-6, open): the Bridge needs a netlist writer and
  parser. Proposal (rename freely, just note the final names here):
  `sp_engine::netlist::write_truss(&WriteTruss { name, comment, truss: &Truss, sections: &[String] /* per member: "Thin" / "Medium" / "Thick" */, loads: &[NodalLoad] /* point loads, N */, self_weight: bool }) -> String`
  (emits `.units SI kN m`, `.material` from `truss.materials[0]`, one `.section <name> A=..cm2 I=A^2`
  per distinct section, `.node` with `fix=xy|y`, `.member`, `.load` lines in kN, `.static`,
  `.buckle euler K=1.0`), and `netlist::parse(text) -> Result<Netlist, Vec<Diagnostic>>` with a
  way to get back `(Truss, point loads, section name per member, self_weight)`. The Bridge maps
  `Design ↔ (Truss, loads)` itself (`bridge::Design::to_truss` / `from_truss`, grid checks for
  "Open in Playground"). Until yours lands the Bridge writes the same text itself
  (`bridge::netlist::write`) so nothing blocks; I will switch to yours.
  - **advanced (d65215a): done, names as proposed.** `sp_engine::netlist::write_truss(&WriteTruss
    { name, comment, truss, sections, loads, self_weight }) -> String` and
    `netlist::parse(text) -> Result<Netlist, Vec<Diagnostic>>`; back out with
    `net.to_truss()`, `net.point_loads()` (N, no self-weight), `net.member_sections()`,
    `net.self_weight() == SelfWeight::MemberEnds` (`.load self +self` = half of each member's
    weight at each end, your lumping). Loads are written `Fy=-20t` (tonnes × 9.81 kN) when whole
    tonnes, else kN; every material in `truss.materials` is written (members name theirs when
    not the first). Details in ARCHITECTURE § netlist. Shout if you need a field.

- **materials (status, wave 1 done)**: `sp_engine::materials` (8917426, 3a90f0c) reproduces
  the Materials mockup exactly — 253 snapshots from the mockup's own JS in Node replay
  bit-for-bit (`tests/materials_goldens.{mjs,json,rs}`); `docs/DATA-SOURCES.md` (ccc6fd8,
  PROV-2); the Materials screen (e29db6f, MT-1…MT-6, NAV-1/6, PROV-1) with GUI tests
  (`tests/materials.rs`, `tests/materials_render.rs`: closeness per column vs
  `Materials-1440.png` parts 1.9 / stage 5.6 / readouts 6.2). API in ARCHITECTURE §
  materials. For everyone: `sp_engine::materials::jsnum::{to_fixed, to_string, round}` are
  JavaScript-exact (`toFixed` rounds ties up, unlike `format!`); use them for mockup-exact
  strings. **DATA-SOURCES owners**: when you add or change a constant, add its row (class +
  source) to `docs/DATA-SOURCES.md` (or tell me).
- **materials → shell** (open): (1) **Stacked lines at 390 should be content-height.** The
  mockup's row is `flex: 1 1 auto; flex-wrap: wrap` in a `min-height: 900px` column, so
  `align-content: stretch` only shares out free space when the page is shorter than the
  window; at 390 the Materials parts aside is ~612 px tall in the reference but `layout::columns`
  makes its line `panel_height()` (~794), leaving a ~180 px gap (Tower / Ground likely too).
  (2) **`Slider` loses keyboard focus on the arrow keys**: after the first ArrowRight egui's
  arrow-key focus navigation moves focus to the next widget (the value does step once), so
  NAV-5 keyboard use of every slider breaks after one step (repro: focus "Temperature" on
  Materials, press ArrowRight twice → focus is on "Stretch"). *Update:* fixed in shell's
  working-tree `slider.rs` (not yet committed when I checked); my committed test
  `tests/materials.rs::sliders_step_with_the_arrow_keys` relies on it — please commit. (3) nice-to-have: a `fill` option for `SegStyle::Header` (`.seg` 36 px
  items with `flex: 1`); Materials' Strain axis draws its own for now.
- **materials → advanced** (FYI): Materials' Share line is `.material steel E=200GPa
  fy=275MPa rho=7850 curve=steel ; tensile test at 20 °C, ε = 0.00 %, Ready` (Series'
  `.material name E fy rho [curve=…]`); pasted into Advanced it should parse (accept or warn
  on `curve=`, don't error).
- **materials → lead** (decide; implemented as the mockup until told otherwise): (a)
  **Toughness ignores temperature**: the tile shows the room-temperature area while the
  plotted curve, peak and yield are heat-scaled (timber at 300 °C: peak 0 MPa, toughness
  0.27 MJ/m³). `Curve::toughness_at(T)` is ready if you want it (MB candidate). (b)
  **Concrete copy is the metals'**: past 0.1 % it says "Plastic: permanently stretched" and
  "Past the yield point the atoms start slipping… lets steel bend", and at the end "It tore
  after stretching 0.35%"; concrete in compression crushes. (c) The explain sentence says
  "Press Play to keep pulling." but the button is "Pull". (d) SPEC MT-1 says Temperature
  20–600 °C; the mockup's slider is −40…600 (implemented −40…600). (e) The card's "5 / 12"
  doesn't match a rung of the Series challenge ladder (no "Break all five" there). (f) LOOK-4:
  the mockup's unlit challenge-chip text `#6A6A6A` is 3.4:1 on `#141414`; I used `#8A8A8A`
  (5.3:1) for the text and kept `#6A6A6A` for the dot. (g) Steel / aluminium / timber
  strengths differ between Bridge·Advanced (A36-like 250 MPa, 6061-T6 characteristic 240,
  C24/GL24h-class 30/24) and Materials (S275 275, 6061-T6 typical 276, clear wood 80):
  different grades, all representative, documented in DATA-SOURCES "Known inconsistencies".

## Candidates for labkit

- **materials**: `sp_engine::materials::jsnum` (JavaScript-exact `toFixed` / `String(n)` /
  `Math.round`) — every lab-series app that reproduces mockup numbers needs it.
