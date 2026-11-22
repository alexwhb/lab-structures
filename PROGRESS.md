# Progress

## Status

| Wave | Work | Status |
|---|---|---|
| 0 | repo, spec, architecture, mockup + references, `sp-theme`, `sp-engine` truss core | done (lead) |
| 1 | app shell + Series + packaging · Bridge · Advanced · Tower + Ground · Materials | in progress (shell: skeleton + ARCHITECTURE § App done; header / tabs / keyboard, Series, platform, packaging next) |
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

## Candidates for labkit

(none yet)
