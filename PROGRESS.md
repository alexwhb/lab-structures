# Progress

## Status

| Wave | Work | Status |
|---|---|---|
| 0 | repo, spec, architecture, mockup + references, `sp-theme`, `sp-engine` truss core | done (lead) |
| 1 | app shell + Series + packaging · Bridge · Advanced · Tower + Ground · Materials | done |
| 2 | MCP · e2e journeys · README · independent reviews (advanced → bridge, worlds → advanced, materials → worlds + shell) + fixes · lead's live MCP e2e against the bundle | done (2026-11-22): `scripts/test-all.sh` ALL GREEN; see "Lead verification" below |

## v2 — the builder (started 2026-11-23)

The user's updated mockup (`docs/design/v2/`) turns the Playground into one builder for bridges
and towers. Decisions (user + lead):
- The v1 Bridge, Tower, Ground and Materials screens are **retired** (no study tabs). Their
  engines (`sp_engine::{tower, ground, materials}`) stay for MCP and the fold-ins; the v1 bridge
  engine (`sp_engine::bridge`) is removed once nothing uses it (wave 2).
- Three ideas from the retired screens are folded into the builder: the comfort meter + "same
  storm, four shapes" (CMF), settlement over 30 years + a Leaning tower level (TIME), the sample
  lab + Fire (LAB, FIRE). SPEC "Builder" and "Fold-ins".
- Advanced and Series are unchanged; NAV/MCP/DONE updated in SPEC.

| v2 wave | Work | Status |
|---|---|---|
| 0 | v2 mockup + references, SPEC v2, module stubs | done (lead) |
| 1 | builder engine · builder screen · app frame + MCP + removals · time + comfort · lab + fire | in progress |
| 2 | integration, e2e journeys, visual pass, reviews + fixes, docs | not started |

### Ownership (v2 wave 1)

| Agent | Owns |
|---|---|
| `engine` | `crates/sp-engine/src/builder.rs` (core; keep the four `pub mod` fold-in lines) and `crates/sp-engine/src/builder/**` except the four fold-in files below; `crates/sp-engine/tests/builder*.{rs,mjs,json}` |
| `screen` | `crates/structures-playground/src/screens/builder.rs` + `src/screens/builder/**` except the fold-in UI files below; `crates/structures-playground/tests/builder*.rs`; `crates/sp-theme/src/builder.rs` |
| `shell` | everything else in `crates/structures-playground/**` (app, router, ctx, widgets, control, removals of the retired screens and their tests, journeys), `crates/sp-shared/**`, `crates/sp-mcp/**`, `scripts/**` (except `render-mockup*.mjs`, `mockup-goldens.mjs`), README, `docs/screenshots/**` |
| `time` | `crates/sp-engine/src/builder/{comfort,longterm}.rs` (+ submodules), `crates/sp-engine/tests/{comfort,longterm}*.rs`; UI `crates/structures-playground/src/screens/builder/{comfort,time}.rs`; `crates/sp-theme/src/time.rs` |
| `lab` | `crates/sp-engine/src/builder/{sample,fire}.rs` (+ submodules), `crates/sp-engine/tests/{sample,fire}*.rs`; UI `crates/structures-playground/src/screens/builder/{lab,fire}.rs`; `crates/sp-theme/src/fire.rs`; `docs/DATA-SOURCES.md` |
| lead | `SPEC.md`, `PROGRESS.md` status, `docs/ARCHITECTURE.md` skeleton, `docs/design/**`, `scripts/render-mockup*.mjs`, root `Cargo.toml`, `crates/sp-theme/src/lib.rs`, `crates/sp-engine/src/{lib,linalg,truss}.rs` |

Hook points (agreed up front, details in ARCHITECTURE § builder):
- `engine` builds the fold-in hooks into the core: per-piece strength and stiffness factors in
  the simulation (default 1.0) that `fire::update` sets each frame when Fire is on; a `fire`
  field in the conditions; wind drag and shedding factors (default 1.0) for CMF-2's variants; a
  static settle with per-foundation settlement offsets for TIME-1; a level table that `time`
  can add the Leaning tower to (`pisa`), and break-mode names "softened" / "burned through".
- `screen` exposes the places the fold-in UIs render into: the result banner / aside result
  card (comfort meter, Compare shapes, Fast-forward), the HUD chips, the Conditions panel (Fire
  row), the Materials list cards (open the sample drawer), the stage overlay (fire, glow, years).
  Fold-in agents own the UI files `builder/{comfort,time,lab,fire}.rs` and the screen calls them.

## Lead verification (2026-11-22)

- `scripts/test-all.sh` on the final tree: fmt, shellcheck, clippy `-D warnings`, the whole
  workspace test suite (engine goldens against the mockups' own JS, property / fuzz tests, GUI
  tests and the cross-screen journeys in egui_kittest, MCP end-to-end with the real binary),
  release perf gates, the real-window smoke run and the sandboxed bundle check.
- Live end to end against `dist/Structures Playground.app` opened with `open -n`, driven by its
  own `structures-mcp` over stdio while the window was covered by another app: a netlist loaded
  into the Sandbox holds under Test ("It held!"), a weakened copy collapses ("Collapse: M5
  buckled at 33%"), Advanced / Tower (100 floors, all four goals lit) / Ground (piles to rock,
  FS 14.86) / Materials (concrete "Crushed at 0.35%") / Series all answer, and every screenshot
  comes back (offscreen render, 2880 × 1800).
- Mockup bugs and lead decisions: SPEC "Mockup bugs not to copy" MB-1…MB-8.

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

- **screen (v2, status + fold-in hook API — for `time` and `lab`)**: the builder screen hosts
  the fold-ins through one trait in `src/screens/builder/host.rs` (screen-owned). Each fold-in
  UI file defines one `Default` struct and implements it; every method has a default, so
  implement only what you need. The screen holds `comfort::Comfort`, `time::Time`, `lab::Lab`,
  `fire::Fire` and calls them in that order. If your file doesn't exist when I need it I create
  it once as a stub (`#[derive(Debug, Default)] pub struct Comfort; impl FoldIn for Comfort
  {}`); after that it's yours. (`Game` / `View` / `PieceType` = whatever `engine` names the
  state machine, its view and the piece-type enum; I'll fix the names when it publishes.)
  ```rust
  pub trait FoldIn {
      fn tick(&mut self, game: &mut Game, ctx: &mut Ctx) {}            // once a frame before drawing; fire heats pieces here; ctx.wake_in(1/30) while moving
      fn adjust_view(&self, game: &Game, view: &mut View) {}            // TIME-1: settled joint positions / broken pieces drawn this frame
      fn hud(&self, game: &Game, chips: &mut Vec<HudChip>) {}           // after the core's: "Fire 650 °C", "Year 12.0", "Tilt 1.8°", "Settled 214 mm"
      fn paint_stage(&self, game: &Game, view: &View, painter: &Painter, xf: &Xf) {} // over the structure: flames, glow; xf.p([x, y] m) -> Pos2, xf.ppm() px per metre
      fn conditions(&mut self, ui: &mut Ui, game: &mut Game, ctx: &mut Ctx) {} // rows at the end of Conditions (FIRE-1 Fire row + "Light it now")
      fn results(&mut self, ui: &mut Ui, game: &mut Game, ctx: &mut Ctx) {}    // the aside's result slot under the goal panel (CMF-1 meter, Compare shapes, Fast-forward 30 years + scrubber); draw at the cursor, allocate what you draw, nothing when there's nothing
      fn open_sample(&mut self, piece: PieceType) -> bool { false }     // a Materials card was clicked (LAB-1)
      fn drawer(&mut self, ui: &mut Ui, game: &mut Game, ctx: &mut Ctx) -> bool { false } // open: draw the drawer in the aside column instead of the aside, return true
      fn escape(&mut self) -> bool { false }                            // Esc: true = consumed (close the drawer) before Esc ends a chain
      fn drive(&mut self, game: &mut Game, ctx: &mut Ctx) -> bool { false } // take the drive / Run press (TIME-3: Leaning tower's run is the fast-forward)
      fn drive_button(&self, game: &Game) -> Option<Drive> { None }     // the header button while you drive
      fn reset(&mut self, game: &Game) {}                               // back to building, Reset, Clear, level / scene picked: drop run-only state
      fn busy(&self) -> bool { false }                                  // something you animate is moving
      fn describe(&self, game: &Game) -> Option<String> { None }       // extra words for the MCP summary
  }
  pub struct HudChip { pub key: String, pub value: String, pub color: Color32 } // colour from sp_theme
  ```
  Helpers you may use from `host`: `card(ui, label, |ui| ..)` (`.well` block, padding 12, gap
  10), `chip_row(ui, group, caption, &chips) -> Option<usize>` (the Conditions rows' look),
  `para`, `label_node`. Shout under Requests (tag `screen`) if you need another hook.
- **screen → engine** (what the screen will draw from; rename freely, I adapt): as v1, one
  `Game` with `view() -> View` holding every string and number of `renderVals()` — in
  particular joint / member / crate / vehicle positions in **world metres** (displaced while
  running) with the camera in the engine (`cam()`: `cx, cy, z`, px per metre, grid, view bounds;
  `zoom_by(f, px, py)` in stage px 1120 × 600, `pan_to`, `fit`, `snap(stage px) -> world m`),
  member band (`t1..c3 / z / x`) + piece type + broken + slack sag + `exposed`, the handles
  (`text`, `label`, `over`, `selected`) and joints (`label`, `pending`, `selected`, `bad`,
  `sup`, `locked`), vehicle kind / x / y / angle / state, sparks, splashes with age, wind now +
  dir, snow now, quake envelope, the HUD list, run info, result, every chip row (palette with
  swatch key, founds, weights, levels with best stars, conditions with on / dim, scenes),
  inspector (member / joint / Materials list with `curveFor` path), explain, stage / surface
  labels, slug, clock; and the editing / run methods of the mockup (`clickNode`,
  `clickSurface`, drag, `setFound`, `addLoad`, type change, deletes, undo, Clear, levels,
  scenes, conditions, Shake now, `startRun` / `stopRun` / replay / slow, `tick()` per 1/30 s
  frame, toasts fading), Share / ⌘V netlist write + read into the Sandbox, best stars get / set,
  and `buildPreset` (MB-10) so tests can rebuild the reference boards.

- **lab → engine** (FIRE-1 hook — please confirm or rename; I adapt): `builder::fire` (mine)
  defines `FireLevel::{Off, Small, Big, Inferno}` (400 / 700 / 900 °C; `ALL`, `name()`,
  `temp_c()`, `Default = Off`) and `FireState` (`Default`; per-piece temperatures, when it was
  lit, reach). What I need in the core:
  1. `Conditions { .., fire: fire::FireLevel }` (the condition chips set it like `quake`; it
     never makes a run a practice run) and the sim holds `fire: fire::FireState`.
  2. Per piece in the sim, three factors, default 1.0, that I set:
     `strength_t` (× `A·fT`), `strength_c` (× `A·fC`, the crushing term) and `stiffness`
     (× the XPBD `kk` and × the Euler load). So capT = A·fT·strength_t and capC =
     min(A·fC·strength_c, π²·E·k·A²/L²·stiffness). Concrete needs the two strengths (EN 1992-1-2
     tension and compression differ). Please expose that formula as one pub fn (e.g.
     `capacity(type, len, strength_t, strength_c, stiffness) -> { tension, compression, crush,
     euler, buckles }`, all N) and use it in `makeSim`: the sample drawer (LAB-1) calls the same
     fn so "a piece this size breaks at N t" equals the engine's capacity by construction.
  3. Call `fire::update(&mut sim, h)` once per **live** substep (before the break checks),
     always — it returns at once when the fire is off and nothing is hot (pieces must cool after
     Fire → Off). Not in the build-mode preview. It reads `sim.t`, the scene (river / open
     ground), joint positions + supports, each piece's ends, type and `broken`, `cond.fire`.
  4. Breaks: when a piece breaks, `mode = fire::break_mode(piece_type, strength_t.min(strength_c))
     .unwrap_or(usual)` → `Some(Softened)` (steel, cable) / `Some(BurnedThrough)` (wood, rope)
     when the fire took a real part of its strength; and a piece whose strength factor is 0 (I set
     0 when wood / rope has burned through) breaks whatever its force. Break-mode names "softened"
     / "burned through"; tips from `fire::tip(mode) -> Option<&'static str>` in the result explain.
  5. `Game::sim_mut() -> Option<&mut Sim>` (or a `fire_light_now()` passthrough) for "Light it
     now", and read access to the sim for the glow / HUD.
  Piece types: I read your type table (area, E, k, fT, fC, ductile, tension-only, max length,
  name) for `builder::sample` — tell me its name when it lands; until then `sample` has its own
  copy of `types()` that a test will check equal to yours.
- **lab → screen**: thanks — `lab::Lab` will implement `open_sample` / `drawer` / `escape` /
  `tick` / `busy` / `describe`; `fire::Fire` `conditions` / `hud` / `paint_stage` / `reset` /
  `describe`. For the glow I need each member's two end positions (world m), piece type and
  broken flag from the `View` (as you listed) and the piece index into the sim. Create the two
  files as stubs when you need them; after that I fill them.

- **shell → advanced** (small, 2e1eda8): `crates/sp-engine/tests/advanced_perf.rs:85` prints Debug
  durations (`parse+solve 1.2ms, …`); `scripts/test-all.sh` lists a line as a perf gate only
  when it reads `PERF <label>: <value> ms (limit <limit> ms)` (like `materials_perf.rs`). Could
  you print the 450-node solve and influence line that way? The target's asserts gate it
  already; this only adds the numbers to the summary.

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

- **shell (status, wave 2)**: **MCP** (7086c2e, ffca6fa; MCP-1…MCP-3): `crates/sp-shared`
  (identity, screens, socket, bridge methods, input checks that refuse rather than clamp, and
  the engine reports both sides send), `crates/sp-mcp` (binary `structures-mcp`: engine tools
  `solve_truss`, `influence_line`, `tower_wind`, `ground_check`, `tensile_test`; app tools
  `app_status`, `list_screens`, `navigate`, `screenshot`, `describe_screen`,
  `bridge_load_netlist`, `bridge_test`, `set_tower`, `set_ground`, `set_material`), the app's
  `control.rs` (answered each frame and from `App::logic` when hidden; `--no-mcp`). Tests: 12
  e2e through the real binary + the real app in egui_kittest (`cargo test -p sp-mcp`), 10
  app-side bridge tests (`tests/mcp_bridge.rs`); `crates/sp-mcp/README.md`, ARCHITECTURE § MCP;
  `bundle-macos.sh` bundles `structures-mcp`. **README** (b59e257) + `docs/screenshots/`
  (47b76ce, from HEAD). Frame fixes from review probes (7086c2e): hand-offs queue, ⌘⇧Z isn't
  undo, held Space drives once, Space drives while a slider has focus
  (`widgets::space_passes_through`), long document names ellipsized at 390. advanced's
  request done (39f813c: `widgets::Select`, `SegGroup::wrap`). 390 stacked-column gap: gone
  (page heights within 1–16 px of the references on Tower / Ground / Materials).
  **`scripts/test-all.sh` on 47b76ce (with the bundle): ALL GREEN** — fmt, shellcheck, clippy,
  334 tests (sp-engine 135, structures-playground 168, sp-mcp 12, sp-theme 12, sp-shared 7),
  perf gates, smoke start-up 264 ms, bundle with `structures-mcp`, check-bundle (Finder .str,
  sandboxed smokes, `open -a … truss.str` → Advanced).
- **shell (status, review round)**: the reviewer's ten findings fixed. 2e1eda8: a disabled
  `Slider` ignores pointer, keys and AccessKit Increment / Decrement / SetValue and stops
  advertising them; inverted / NaN ranges are safe; `Menu` item targets tile (no 8 px overlap);
  `Plot` keeps negative series in the box; test-all finds PERF lines anywhere and gates the
  engine's in-suite perf checks (`performance_budget`, `calc_is_fast`, `sway_steps_are_cheap`)
  in release; compare-mockup / screenshots exit with cargo's status; check-bundle's
  LaunchServices run needs `status=ok` and no checkout paths; `press_drive(ctx)` uses the real
  context; the smoke run Tests the Sandbox starter truss under 150 t (`bridge=collapse`, ~30
  fps). 3d7fdcf: **MCP `screenshot` / `describe` are drawn offscreen** from the app's state
  (second egui context, 3 passes, no input, no tick / clicks / focus changes; egui_kittest's
  wgpu renderer on its own device), so a window covered by another app answers; `App::logic`
  answers the queue itself after 300 ms without a frame; summaries print clean numbers (`22.08
  %`). Verified live: the release app + `structures-mcp`, a 2880×1800 Tower screenshot.
  **test-all on 3d7fdcf (with the bundle): ALL GREEN**, 343 tests.
- **bridge → advanced** (NAV-4 / ADV-9 / NAV-6, **done** — the Bridge uses it since ea3e3db): the Bridge needs a netlist writer and
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

- **bridge (status, wave 1 done)**: `sp_engine::bridge` (ca336a9, ea3e3db) is the mockup's
  `class Component` without the drawing, **bit-exact**: `tests/bridge_goldens.rs` replays 10
  scenarios / 69 snapshots of the mockup's own JS run in Node (`tests/bridge_goldens.mjs`) and
  matches state, forces, displacements, cost, the whole collapse simulation, every string and
  the drawn geometry to 2 ulps; property tests (random edits, NaN, huge loads), undo, round
  trip, ENG-4 perf (60-joint solve ≈ 0.15 ms, a full 30-solve tick ≈ 4 ms). The Bridge screen
  (0043bd9, e8d7a36: BR-1…BR-11, NAV-4/5/6, PROV-1) with `tests/bridge.rs` (11 journeys:
  level 1 by clicks → held ★, thin timber buckles + slow-motion replay, mechanism, every
  placement toast, inspector, undo / Delete / arrow nudge, drag, levels, Share / ⌘V /
  hand-offs, no overlaps at 1440 / 390, closeness per region vs `Playground-1440.png`: rail
  2.4, stage 1.2, levels 2.9, hint bar 6.7, goal 8.0, built from 3.7). API in ARCHITECTURE §
  bridge. **For MCP (wave 2)**: `app.screens().bridge.game` is public — `bridge_test` =
  `game.run_test(max_ticks)` then `phase()` / `view().result`; `bridge_load_netlist` =
  `bridge::netlist::read(text)` + `game.open_in_sandbox(design, mat)`; `describe()` =
  `game.summary()`. `SP_BRIDGE_SHOTS=1 cargo test --test bridge state_gallery` writes review
  screenshots of every state.
- **bridge (status, wave 2 done)**: MB-5 (68a55e9: the starter truss is cross-braced, holds,
  also with 20 t at a middle joint; goldens apply the same two members after the mockup's
  `starter()`, noted); `bridge::fmt` replaced by `sp_engine::jsnum` and deleted (247c55e);
  advanced's four review findings fixed (247c55e, replies under Review findings); **DONE-4
  journeys** in `crates/structures-playground/tests/journeys.rs` (1f370cf), all 7 green in
  < 1 s, driven and read through AccessKit only (the header through its `Banner` node), with
  expected numbers from the engine run independently: level 1 by clicks → held ★ → best stars
  on the row; thin timber → "Collapse: M6 buckled at 7%" → replay → back; Open in Advanced →
  all 13 members' kN and use % equal the Bridge inspector's, Share → ⌘V round trip; netlist
  error on its line with the last result dimmed → fix → Open in Playground; Tower 100 floors
  braced tapered + damper → all four goals (MB-6) → Ground note → Pisa scenario, piles to rock,
  30 years → three goals → back to the Tower with 6 floors; Materials all five broken (the
  status text comes from a headless `Bench`, so MB-8's "Crushed at" for concrete is followed);
  keyboard only (⌘1–⌘5, Space, Tab to the drive button + Enter, Tab to the brand → Series →
  Esc). The journeys found no bugs in other screens.
- **bridge → lead** (FYI, small change made with tests green): `truss.rs` gained `pub fn
  hypot(dx, dy)` — V8's `Math.hypot` algorithm (scaled, Kahan-summed), used for every member
  length. A correctly rounded hypot differs from the mockups' in the last bit (e.g. 10 × 6 m
  diagonals), which flipped tie-breaks between symmetric members ("M7 buckled" instead of the
  mockup's "M6"). All truss / Advanced tests stayed green.
- **bridge → lead** (new mockup bug, please add as **MB-4**): `Playground.dc.html`
  `renderVals()` returns `nM` both as the member count (the Members tile) and as the "no
  solution, Medium size" path (`<path d="{{nM}}">`), so the mockup never draws unstable
  Medium members (or any Medium member during the collapse). SPEC BR-2 says they're grey-blue
  `#9AA0A8`; the app draws them (the golden test skips that one key).
- **bridge → lead** (decide; implemented as below): (a) water dashes flow while testing and
  during the collapse only (mockup: in every phase but build) — SPEC BR-1 / PLAT-3 say "only
  while testing" and 0 % CPU at rest; (b) material chips and the starter truss work only while
  building (the mockup lets you change material mid-test; BR-7 "building is paused"); (c) the
  hover dot clears when the pointer leaves the stage (the mockup leaves it); (d) a plain click
  on a joint with Select doesn't push an undo step, only a drag does (the mockup pushes one on
  every pointer-down, so ⌘Z sometimes does nothing visible); (e) "Open in Advanced" is a
  full-width ghost below the Built-from tiles (not in the mockup; NAV-4 / BR-9); (f) arrow-key
  nudge (BR-11) follows the drag rules and refuses level joints with "This joint is part of the
  level."; (g) the starter truss is a mechanism (middle panel without a diagonal: m + r − 2j =
  −1) — reproduced as the mockup, a teaching moment; (h) stars are painted five-point stars
  (MB-2), not font glyphs.
- **bridge → advanced** (FYI, **done** 0ecd67f: `MAX_NODES` 500, `MAX_MEMBERS` 2000; a 450-node netlist parses + solves in ~10 ms, its influence line is swept in ≤ 24 steps): `netlist::MAX_NODES` was 120 but a Bridge design can have up to
  450 joints (the 2 m grid): a huge Sandbox structure opened in Advanced gets your parse error.
  Fine by me unless you want to raise the cap.
- **bridge → lead** (labkit candidate / tidy-up): there are now four JavaScript-exact number
  formatters (`materials::jsnum`, `netlist::text`, `tower::fmt`, `bridge::fmt`); one shared
  module would do.

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
- **materials (status, wave 2 done)**: shared `sp_engine::jsnum` (cfd834b; superset of the three
  old formatters, checked against Node; bridge / advanced / worlds have switched). **MB-8** done
  (cc3b907): heat-scaled toughness tile, concrete's Microcracking / Cracking / Crushed status and
  compression explain copy, "Press Pull"; the goldens override exactly those fields in JS
  (`mb8()` in `materials_goldens.mjs`). DATA-SOURCES (cc3b907, 02661e6): edge pressure, MB-6,
  the new gust numbers. Review of `worlds` under "Review findings" (1 med gust calibration, 1 low
  display-rate repaint; both fixed by worlds); the shell review went to the lead. Nothing tagged
  `materials` open. Optional follow-up: switch the Strain axis control to the new
  `SegGroup::fill()`.
- **materials → shell** (**Done**, 4fe97a3: stacked lines are content-height — every column must allocate what it draws; the slider keeps focus from the first arrow press; `SegGroup::fill()` for `.seg` groups, centred items): (1) **Stacked lines at 390 should be content-height.** The
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
- **materials → advanced** (FYI, **done** e0e4252: `curve=` is a warning; ⌘V of a lone `.material` line in Advanced swaps the netlist's material, a netlist with nodes replaces the netlist; tests `materials_bench_share_line_parses`, `pasting_a_netlist_or_a_material`): Materials' Share line is `.material steel E=200GPa
  fy=275MPa rho=7850 curve=steel ; tensile test at 20 °C, ε = 0.00 %, Ready` (Series'
  `.material name E fy rho [curve=…]`); pasted into Advanced it should parse (accept or warn
  on `curve=`, don't error).
- **materials → lead** (answered in SPEC a38bb39 / MB-8; **done** in cc3b907: heat-scaled
  toughness, concrete's cracking / crushing wording, "Press Pull"): (a)
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

- **worlds (status, wave 1 done)**: `sp_engine::{tower, dynamics, ground}` (a4e1932) reproduce
  the Tower / Ground mockups exactly — `tests/tower_ground_goldens.mjs` runs the mockups' own
  `Component` in Node; 110 tower + 130 ground states match every number and string
  (`tower_goldens.rs`, `ground_goldens.rs`), plus property / no-panic / perf tests. `dynamics`:
  RK4, Newmark (unconditionally stable), seeded gusts, `SwaySim` (SDOF / 2-DOF Den Hartog TMD),
  tested against closed forms (`tests/dynamics.rs`). Screens SCR-3 / SCR-4 (f3f6b00, 26dc44c,
  17b241e): every TW-* / GD-* control, stage, readout, Play gusts / Play years, clocks, Share,
  badges, NAV-4 hand-offs; GUI tests `tests/{tower,ground}.rs` (16; closeness per region vs the
  1440 references: stage 0.6–0.9, columns 2.2–6.4, bars 4.2–5.5; 390 heights within 16 px).
  API in ARCHITECTURE § tower, dynamics, ground. For others: `src/widgets/world.rs` (picture
  tiles with SVG icons / swatches, `.lbl` readout grid, band bars, comparison / goal rows,
  challenge card, stepper, value rows, title + status pill) is reusable. For MCP (wave 2):
  `TowerInput` / `GroundInput` are serde; drive the screens with `TowerScreen::set_input` /
  `GroundScreen::set_input`; `tower::share_line` / `ground::share_line` are the one-line
  summaries.
- **worlds (status, wave 2 done)**: review of `advanced` posted (2 med, 2 low; all four fixed by
  advanced in 7a38a91 / 3c0d110). Fixes (aafbc65): **MB-6** drift goal H/350 — **winning design
  for the journeys: `sp_engine::tower::CHALLENGE_DESIGN` = 100 floors, Braced tube, Tapered,
  Tuned mass damper on, storm 30 m/s → H/367, 4.4 milli-g, $144M, all four goals lit** (the
  status pill still says "Too much sway" and the tile "aim H/500", by the lead's decision); SPEC
  MB-6 line updated with it. **MB-7** soil name + "bedrock" on `sp_theme::ground::LABEL_CHIP`
  (≥ 5.2:1 on every soil; "bedrock" also sits over the soil, so it needed it too). `tower::fmt`
  deleted, both worlds on `sp_engine::jsnum`. materials' two findings fixed (gust calibration,
  30 fps). Engine, app and journey suites green; clippy clean.
- **worlds → materials** (DATA-SOURCES, small): the "Gust playback" row now reads turbulence
  **2 %** and vortex force **1.5 %** (was 8 % / 6 %); and MB-7's label chip `rgba(10,10,10,.7)`
  if you list drawing constants.
  - **materials: Done** (see the commit after cc3b907) — row updated (2 % / 1.5 %, with the
    calibration note). DATA-SOURCES lists data constants, not colours, so the label chip stays
    in `sp_theme::ground` with its CSS source.
- **worlds → lead** (decide; implemented as the mockup until told otherwise): (a) **"Challenge ·
  100 floors" can't light all four goals at once**: in the mockup's own model no 100-floor
  tower reaches H/500 at 30 m/s (best: braced tapered tube H/367; the drift goal needs ≤ 77
  floors), so the card never completes (`tower_goldens.rs::the_100_floor_challenge_…`). A fix
  would be a goal like "Drift better than H/350" or a stiffer top system; your call. (b) LOOK-4:
  the stage's soil label `#E5E0D0` (mono 11) is ~2:1 on the lighter soils (dense sand, loose
  wet sand), the mockup's colours; kept. (c) Play gusts runs faster than real time (the default
  tower sways at the mockup's 0.55 Hz; `dynamics::playback_speed`); the header clock shows the
  real period `T = 9.9 s` as the mockup does. (d) Lock-in in the sway animation forces at f1
  in the stage's plane (real shedding pushes across the wind) — noted in the docs.
- **worlds → materials** (DATA-SOURCES, small): please add the Tower → Ground note's edge
  pressure, `ground::FromTower::edge_pressure` = M / (B³/6) over the 40 m footprint
  (Simplified: rigid base, linear pressure), and, under "Known inconsistencies", the
  100-floor challenge above. Thanks for the Tower / Ground rows — they match the code.
  - **materials: Done** (cc3b907) — edge-pressure row in the Ground table, MB-6 under "Known
    inconsistencies" (resolved by `worlds`), the Tower challenge row now lists `DRIFT_GOAL` H/350.

- **advanced (status, wave 1 done)**: engine `sp_engine::{netlist, generate, influence}`
  (d65215a, f814297, 0ecd67f) and the Advanced screen (d859a0c, e0e4252 + this commit). The
  generator writes the mockup's netlist line for line and its model reproduces the mockup's
  `solve()` **bit for bit** (forces, displacements, utilisations, mass) for 84 toolbar states
  (`scripts/mockup-goldens.mjs` runs the mockup's `Component` in Node →
  `crates/sp-engine/tests/golden/advanced.json`; `tests/advanced_goldens.rs`: every table row,
  check, status, IL tile, IL path and limit dashes string-equal). Parser: line/column
  diagnostics, order-independent, other worlds' directives warn, one mistake = one message,
  never panics (seeded fuzz: 1500 mutated netlists + garbage + huge counts); writer + round trip
  (a Bridge structure solves the same to the newton); ENG-4 influence line ≈ 0.2 ms (< 5 ms).
  Screen: ADV-1…ADV-9 as the mockup at 1440 and 390 (`tests/advanced_look.rs`: mean |Δ| vs
  `Advanced-1440.png` toolbar 3.3, netlist 1.5, elevation 5.8, influence line 5.9, table 8.0,
  checks 7.4, whole board 3.9), editable netlist with inline errors (red band, underline,
  message chip, dimmed last good result, error badge), Drive across, IL picks, Share, Reset,
  ⌘Z for toolbar changes, ⌘V, hand-offs both ways, Tab out of the editor (`tests/advanced.rs`, 12 journeys).
  Tests: sp-engine 6 goldens + 11 parser + 3 round-trip + 4 fuzz + 3 perf; app 7 doc unit + 12
  journeys + 2 look. Not done: nothing required by ADV-*; the IL for a custom netlist without
  the standard T2–T3 / B2–B3 / B0–T1 / B2–T2 members picks the four most-used members.
- **advanced (status, wave 2)**: review of bridge + truss/linalg posted under Review findings
  (a688a24: 1 med, 3 low, none for lead). MB-8 done (a289877: "<Material> weight" — Steel /
  Aluminum / Glulam / Carbon fiber, other names title-cased, "Weight" for a mix; the goldens
  override the mockup's "Steel weight" for non-steel states). Also: a load on a node no member
  reaches now shows "Load on D hangs on nothing" in the status (the solver ignores such loads).
  jsnum switch done. worlds' 4 findings fixed (7a38a91, 3c0d110).
- **advanced → lead** (decide; implemented as the mockup until told otherwise): (a) **slider
  ranges**: SPEC ADV-1 says Size 1–10 and Truck 0–60 t; the mockup (and the reference PNG's
  thumb positions) use 1–20 and 0–120 — implemented 1–20 / 0–120. (b) The mockup colours a `*`
  description line's text as arguments (`#D9D9D9`) and only the title line `* river-crossing.str`
  fully in the comment colour; ADV-2 says comments `#7E8A7E` — implemented as the mockup renders
  it. (c) "Steel weight" is the label for every material (mockup copy); the note says "Both
  trusses" from `share=0.5` (`1/share` trusses). (d) Determinacy uses the real reaction count
  (the mockup hard-codes `+ 3`; same for its trusses). (e) The mockup's editor has no error state:
  the red band / underline / chip follow Automata's script editor; "Open in Playground" sits in a
  footer under the netlist (not in the mockup; ADV-9). (f) `+self` with nodes shares the truss's
  self-weight among them (the mockup's deck); `.load self +self` (no nodes) lumps half per member
  end (the Bridge's) — documented in the module docs.
- **advanced → shell** (**Done**, 39f813c: `widgets::Select` (your select with shared tokens: `ROW`, `BORDER_STRONG`, `SELECT_CHEVRON`) and `SegGroup::new(..).style(SegStyle::Header).wrap()`; switch when convenient, then `controls::{select, SegWrap}` can go) (nice-to-have): the IL picks need a `.seg` (36 px) group that wraps
  (`flex-wrap: wrap`) — `screens/advanced/controls.rs::SegWrap` meanwhile; and a `<select>`
  widget (Material) — `controls::select` (ComboBox + your `Menu`) could move to `widgets/`.

- **materials → bridge, advanced, worlds**: `sp_engine::jsnum` is in (cfd834b); switch your
  formatter and delete the duplicate. It is a superset of `bridge::fmt`, `netlist::text` and
  `tower::fmt` with your names as aliases: `to_fixed` / `fixed`, `round` / `js_round`,
  `round_str`, `to_string` / `num` / `js_number`, `minus_fixed`, `tidy` (checked against Node
  on ~10 000 values). Differences from the old copies: `tower::fmt::num(-0.0)` printed `"-0"`
  (now `"0"`, as JavaScript); `bridge::fmt::to_fixed` had no ≥ 1e21 case (now `"1e+21"`). The
  Bridge's `stars()` is not a number format and stays with the Bridge.
  - **advanced: done** (a289877): `netlist::text` deleted; generate / influence / netlist / the
    Advanced screen use `sp_engine::jsnum`; goldens green.

## Review findings

- **advanced → bridge** [severity: med] `crates/structures-playground/src/app.rs:92` (`Persisted`)
  + `crates/sp-engine/src/bridge/game.rs:471` (`Game::set_best`) — best stars per level are
  lost on restart: `set_best` ("restoring saved progress") has no caller and `Persisted` holds
  only `route` / `last_world`. — Win ★★★ on level 1, quit, relaunch: the level row shows no stars
  (BR-6 "best stars remembered per level", NAV-7). — Persist `game.best()` (e.g. a `best: [u8; 4]`
  in `Persisted` with `#[serde(default)]`, written by the app, restored with `set_best`); needs
  **shell** for `Persisted`.
  - **Fixed** (247c55e): the Bridge keeps best stars in egui's persisted memory
    (`screens::bridge::BEST_STARS_KEY`, saved by eframe with the window), restored on `enter`
    (never lowering this run's) — no shell change needed. Test `tests/bridge.rs::
    best_stars_survive_a_restart`; level rows also expose them ("Best: 1 of 3 stars").
- **advanced → bridge** [severity: low] `crates/sp-engine/src/bridge/game.rs:981` (`test_tick`) — on
  the λ = 1 tick, if the break loop uses all `MAX_BREAKS_PER_TICK` (30) breaks, `finish_hold`
  runs without re-solving: the structure is declared "It held!" (with stars) although it may
  still be over its limit or a mechanism after the 30th break. (The mockup's `tick()` has the
  same hole.) — A large Sandbox truss whose cascade starts on the last ramp step (λ 54/55 → 1)
  and breaks > 30 members in that tick ends Held. — Before `finish_hold`, re-analyse at λ = 1
  with `broken`: if `!ok` collapse, if worst > 1 keep `Testing` at λ = 1 for another tick.
  - **Fixed** (247c55e): at λ = 1 the Test holds only once the cascade settled within the
    tick; if the 30 breaks run out, the next tick goes on at λ = 1. Regression test
    `a_cascade_on_the_last_tick_is_not_called_held` (60 thin members, first break inside the
    last ramp step: the old code said "It held!" on a mechanism, now it collapses); the random
    edit test checks Held ⇒ stable and nothing over its limit.
- **advanced → bridge** [severity: low] `crates/sp-engine/src/bridge/game.rs:529, 550, 557, 709` —
  undo steps that change nothing (the class your decision (d) fixed for Select clicks):
  `click_grid` with Joint / Member pushes before `add_node` refuses an underground spot;
  `click_joint` with Member pushes before `add_member` refuses an already-joined pair;
  `add_load` pushes when the joint is already at 200 t. Verified with a probe (`click_grid(2,
  -4)` on level 1: toast "That spot is underground.", `undo_depth` 0 → 1, design unchanged; same
  for 200 t and "Those two are already joined."). — ⌘Z then appears to do nothing. — Push only
  after the change is accepted (compare the design before / after, or check first).
  - **Fixed** (247c55e): every edit snapshots and records a step only if the design changed
    (refused spot / pair / 200 t / same size, and a drag that didn't move, leave none); the
    golden harness drops the mockup's no-op steps the same way, so undo depths stay compared.
    Test `refused_edits_leave_no_undo_step`.
- **advanced → bridge** [severity: low] `crates/structures-playground/src/screens/bridge.rs:509` —
  `ctx.animate()` every frame while testing / collapsing repaints at display rate (60–240 fps)
  although the simulation ticks at 30 /s; PLAT-3 asks for ~30 fps while something moves. —
  A 5 s Test + collapse on a 120 Hz display draws ~600 frames instead of ~150. — `ctx.wake_in(
  self.ticker.due_in(1.0 / TICK_SECONDS as f32))` (the Ctx docs' pattern), or cap the flow
  animation at 30 fps.
  - **Fixed** (247c55e): `ctx.wake_in(ticker.due_in(30.3))` while testing / collapsing.
- **advanced → lead** [none] `crates/sp-engine/src/{truss,linalg}.rs` — reviewed (incl. bridge's
  `hypot`, matches V8 for NaN / ∞ / 0); no defects found. Note only: a supported joint with no
  members (e.g. a lone roller placed in the Sandbox) keeps a free x unknown and makes the whole
  solve a mechanism — identical to the mockup's `solve()`, so left as is.
- Also checked on the Bridge, no findings: hand-offs both ways (Advanced's 48 m truss →
  "Open in Playground" loads 12 joints into the Sandbox and holds; ids / counters stay unique),
  ⌘V, collapse sim (zero / NaN lengths guarded), snapping (NaN-safe), drag start / release
  outside the stage, AccessKit (37 controls at 1440 and 390, no duplicate or empty labels; Tab
  reaches every tool, chip, joint, level and material), test suites (all green, < 0.2 s).

- **worlds → advanced** [severity: med] `crates/sp-engine/src/netlist/edit.rs:87` (`set_truck`) +
  `crates/structures-playground/src/screens/advanced/doc.rs:357` — the x slider / Drive across
  write the truck position in **metres as a bare number** (`x=6`), but bare lengths are read in
  the netlist's `.units` length unit. — Verified with a probe: a netlist with `.units SI kN cm`
  (nodes in cm, lane 0–800 cm) after `set_truck(None, Some(6.0))` reads `x=6` → truck at
  **0.06 m**; the slider and clock jump to 0.1 m, and Drive across stalls near 0 (each step writes
  x + 0.5 "m" = 0.005 m). — Write the unit (`x=6m`, which the parser already accepts) or divide
  by `units.length.per_metre()`; add a cm/mm test to `netlist_roundtrip.rs`.
  - **Fixed** (7a38a91): `set_truck` writes x in the `.units` length unit (6.5 m → `x=650`
    under cm); tests `x_is_written_in_the_netlists_length_unit`,
    `truck_edits_respect_the_length_unit`.
- **worlds → advanced** [severity: med] `crates/structures-playground/src/screens/advanced/doc.rs:233`
  (`influence_line` cache) + `advanced.rs:79` + `crates/sp-engine/src/influence.rs:297` — the
  cache key includes the truck weight, so **every Truck-slider step** (and every pick change)
  re-runs the whole sweep synchronously inside `ui`. On a Bridge-sized custom netlist (450 nodes,
  25 full solves) a release build measures **64 ms per drag step** with the nodes interleaved
  B0,T0,B1,T1… and **506 ms** with the same truss declared bottom chord then top chord (21 ms vs
  6 ms per solve: `solve_dense`'s zero-skip only helps a narrow band) — the UI freezes while
  dragging Truck on a sandbox handed over by the Bridge (whose joint order is arbitrary). —
  Linear statics: N(x) = N_static + W·n(x), so cache the unit-truck line per (model without the
  truck weight, member) and scale by W (no re-solve on a weight change); and number the DOFs by
  x (or RCM) before the dense solve to keep K banded (`to_truss`, or ask the lead for
  `truss.rs`).
  - **Fixed** (7a38a91): above 40 nodes the line comes from `influence::unit_sweep` (one static
    + one unit-truck solve per position, all members) cached per model without truck x / weight,
    so Truck drags and pick changes don't solve (29 µs); `influence::solve_ordered` numbers a
    big truss's joints along its long side before the dense solve (450 nodes declared bottom
    chord then top chord: solve + sweep 84 ms). Small trusses keep the mockup-exact full solves
    (goldens unchanged). Tests `tests/advanced_sweep.rs` (superposition = full solves to 1e-6,
    reordered solve = plain solve, perf).
- **worlds → advanced** [severity: low] `crates/sp-engine/src/influence.rs:132` (`picks`) +
  `crates/sp-engine/src/netlist.rs:250` (`Plot::member`) — `.plot N(M3) vs truck.x 0 48 1`
  parses and stores the member, but nothing reads it (only `plot.range` is used, `influence.rs:277`).
  — Change the generated netlist's last line to `.plot N(M3) vs truck.x 0 48 1`: the panel
  still plots the standard pick and is titled `.plot N(M8) vs truck x`, contradicting the
  netlist. — When `plot.member` is set, put that member first in `picks` and select it on
  parse / load.
  - **Fixed** (7a38a91): the member `.plot N(M3)` names comes first in `picks` (added if not a
    standard one) and is selected when the line (re)names it; the user's later pick stays.
    Test `a_plot_line_naming_a_member_selects_it`.
- **worlds → advanced** [severity: low] `crates/structures-playground/src/screens/advanced/doc.rs:498`
  (`drive_step`) — Drive across on a custom netlist **with errors** never moves: `x()` reads
  the last good netlist, each step patches the text with that x + 0.5, the re-parse fails, so
  the next step writes the same x again; the button says Pause and the clock stays put. —
  Edit `.node B1 8 0` to `.node B1 8 oops`, press Drive across: `x = 24.0 m` forever. — Step
  from `params.x` (which `set_truck` already updates) or pause the drive while `stale`.
  - **Fixed** (7a38a91, 3c0d110): Drive across is disabled — and a running drive pauses —
    while the netlist has errors (the dimmed results can't follow the truck).
- **worlds → advanced** — also checked, no findings: the parser (two passes, cascades
  suppressed, ranges bounded, CRLF, U+2212, units for E/fy/A/I/rho/forces/tonnes, every
  directive's errors on the right line/column), writer → parser round trips (materials,
  non-box I in cm4, static + `+self`, moving load, `.plot`, other units), `edit::set_truck`
  text preservation, the report strings and checks against `Advanced.dc.html`, picks, the
  editor (Tab leaves it, AccessKit name + error description, error bands / chips, typing while
  Drive across runs keeps the cursor), the Material select (Enter / Space / arrows, ComboBox),
  1440 and 390 renders against the references (crowded member labels at 390 are the mockup's).

- **materials → worlds** [severity: med] `crates/sp-engine/src/tower.rs:905` (`TURBULENCE`) +
  `crates/sp-engine/src/dynamics.rs:466` (`GUST_TAU_PERIODS = 1`) — Play gusts swings about 4×
  the mockup's dynamic amplitude, enough to carry the drawn top **through vertical into the
  wind** at the default settings. Verified with a probe on the default tower (60 floors, core,
  30 m/s, seed 7): mean lean 0.789 m, `dyn_amp` 0.232 m, simulated rms swing 0.326 m
  (1.40 × dyn); over 500 periods the top ranges −0.44 … 1.94 m and is upwind of vertical ~1 %
  of the time (5 seeds: min −0.23 … −0.57 m) — ×40 on the stage, ±46 px against a 33 px lean. The
  doc comment says "gust peaks (≈ 3σ) come out near the mockup's dynamic amplitude `dyn`", i.e.
  rms ≈ dyn/3, but 3σ ≈ 0.98 m ≈ 4.2 × dyn; the test
  `dynamics.rs::play_gusts_sways_around_the_drawn_lean…` only bounds rms to 0.15–1.5 × dyn, so
  it passes. (A 1-period gust correlation time puts most of the gust energy at f1, where ζ =
  1.5 % amplifies it ~1/(2ζ).) — Calibrate: `TURBULENCE ≈ 0.08 × (0.232/3)/0.326 ≈ 0.02` (or a
  longer gust τ, a few periods), and tighten the test to rms ≈ 0.2–0.5 × dyn plus "the top never
  crosses x = 0 at the defaults".
  - **worlds: Fixed** (aafbc65): `TURBULENCE` 0.08 → 0.02 and `VORTEX_FORCE` 0.06 → 0.015 (same
    factor, so lock-in keeps its relative boost); the default tower's rms swing is now
    0.32 × dyn; `dynamics.rs` bounds rms to 0.2–0.5 × dyn and checks the top never reaches
    x = 0 at the defaults (5 seeds × 500 periods; peaks < 1.6 × dyn). Thanks — good catch.
- **materials → worlds** [severity: low] `crates/structures-playground/src/screens/tower.rs:480`
  (and `screens/ground.rs:460` while quaking / wet) — `ctx.animate()` every frame while Play
  runs repaints at display rate: the harness measures `repaint_delay` 4.2 ms (the 240 fps cap)
  for Play gusts, vs 20 ms for Ground's paced years; PLAT-3 asks ~30 fps while something moves.
  A 120-floor tower redraws ~900 shapes per frame at 120 Hz on ProMotion. — `ctx.wake_in(1.0 /
  30.0)` (SwaySim already integrates in fixed internal steps, so the sway is unchanged).
  - **worlds: Fixed** (aafbc65): Play gusts asks `ctx.wake_in(1/30 s)` (`tower::ANIM_FRAME`);
    Ground's shake / water table the same (or the next years tick if sooner). Tests assert a
    30–40 ms `repaint_delay` for both.
- Also checked on Tower / Ground, no findings: Newmark (γ ½, β ¼) effective-load and update
  formulas, Den Hartog tuning, the OU gust discretisation (unit variance), `MODAL_MASS_FACTOR`
  33/140 (∫φ² for the tip-load shape), SwaySim step-size independence (600 × 1/60 s and
  60 × 10/60 s give bit-identical states), TMD effect (rms −37 %), long runs at 5 / 35 / 120
  floors (bounded, lock-in amplifies), `calc()` vs the mockups (both), `FromTower::edge_pressure`
  (M / (B³/6) = 248 kPa for the default), Tower → Ground with 100 floors (80 + note) and back,
  scrubbing pauses Play years, Play from year 30 restarts at 0, every soil × foundation × 2 /
  80 floors × quake finite, Tower / Ground / Materials at 120 × 200, 260 × 300 and 2600 × 400
  px with Play running (no panic).

## Candidates for labkit

- **materials**: `sp_engine::materials::jsnum` (JavaScript-exact `toFixed` / `String(n)` /
  `Math.round`) — every lab-series app that reproduces mockup numbers needs it. (`worlds`:
  `sp_engine::tower::fmt` is the same thing, written in parallel; fold both into one there.)
