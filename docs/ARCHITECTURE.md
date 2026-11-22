# Architecture

```
lab-series/
  labkit/                          shared crates (another repo — never edit from here)
    lab-theme                      fonts, Palette + egui style builder, contrast helpers
    lab-ui                         layout maths (flex_wrap, auto_grid), graph-paper painting, galleys
    lab-mcp                        stdio MCP server + live bridge (Unix socket) to the running app
  structures-playground/
    crates/sp-theme                Structures tokens: greys, sand ramp, force + status colours,
                                   material swatches, identities, sizes; per-screen scene modules
    crates/sp-engine               the engine (no UI): truss stiffness solve + member checks
                                   (lead), bridge game (levels, test ramp, collapse), netlist +
                                   generators + influence lines, tower wind + sway, dynamics,
                                   ground, materials
    crates/structures-playground   the egui app: header, router, widgets, six screens, MCP bridge
    crates/sp-shared               UI-free vocabulary shared by the app and sp-mcp (wave 2)
    crates/sp-mcp                  the MCP server binary `structures-mcp` (wave 2)
```

Dependency direction: `sp-theme` ← app → `sp-engine`. `sp-engine` never depends on egui or
`sp-theme`: screens adapt engine state into what they draw. MCP: app → `sp-shared` →
`sp-engine` + `lab-mcp`; `sp-mcp` → `sp-shared`, `sp-engine`, `lab-mcp` (never the app or egui;
its tests use the app as a dev-dependency).

## `sp-engine` contract

Units SI (m, N, Pa, kg); forces positive in tension. No panics on any input.

### `linalg` + `truss` (lead, done)

- `linalg::solve_dense(n, a, b) -> Result<Vec<f64>, Singular { column }>` — Gaussian
  elimination, row pivoting only (columns keep their order, so a singular column maps back to a
  degree of freedom), relative pivot tolerance `PIVOT_TOL = 1e-9 × max |A_ii|` (the mockups').
- `truss::{Truss { nodes, members, materials }, Node { id, x, y, fix: Fix }, Fix { x, y }`
  (`FREE`, `PIN`, `ROLLER`), `Member { id, a, b, area, inertia, material }`
  (`Member::box_section(id, a, b, area, mat)` sets `I = A²`, a thin square box with b/t = 24),
  `Material { name, e, fy, rho }`, `NodalLoad { node, fx, fy }` (`NodalLoad::down(node, N)`)}.
- `Truss::solve(&loads, &SolveOptions { removed: Vec<bool>, k_factor }) -> Result<Solution,
  SolveError>`. Only joints attached to an active member or supported get unknowns. A singular
  K is a valid result: `Solution { stable: false, mechanism: Some(joint), .. }` with zero forces.
  `Solution { stable, mechanism, displacements: Vec<[dx, dy]>, reactions: Vec<[Rx, Ry]>,
  members: Vec<Option<MemberResult>> (None = removed), active: Vec<bool>, loose_loads:
  Vec<joint>, worst: Option<member>, mass }`.
- `MemberResult { force, length, stress, tension_capacity, euler, compression_limit, limit,
  utilisation, mode: FailureMode::{Yield, Buckling, Crushing} }` (`mode.past_tense()` →
  "snapped" / "buckled" / "crushed"); `truss::check(force, l, A, I, &mat, K)`.
- Helpers: `Truss::self_weight(factor, removed)` (half of `A·L·ρ·g·factor` at each end),
  `Truss::mass(removed)`, `Truss::determinacy()` (`m + r − 2j`), `node_index`, `member_index`,
  `length`, `validate`. `truss::G = 9.81`, `truss::ZERO_FORCE_N = 500`.

### `bridge` (agent `bridge`)

The Playground › Bridge game: `Playground.dc.html`'s `class Component` without the drawing,
**bit-exact with the mockup** (golden test below). The screen owns nothing but pixels.

- **`bridge::Game`** — the state machine. Reading: `level()`, `design()`, `material()`,
  `tool()`, `size()`, `weight()`, `sag()`, `show_forces()`, `best()` / `best_for(level)`,
  `pending()`, `selection() -> Option<&Selection::{Joint(id), Member(id)}>`, `phase() ->
  Phase::{Build, Testing, Collapse, Collapsed, Held}`, `lam()`, `broken()`, `log() ->
  &[LogEntry { member, lam, mode, bad, loose }]`, `stars()`, `hover()`, `dragging()`,
  `toast()`, `sim()`, `analysis()` (the solve shown now), `undo_depth()`, `revision()` (bumped
  on every change), `building()`, `in_sim()`, `animating()` / `playing()`, `clock()`
  (`building` / `load 63%` / `held` / `collapse`), `summary()` (MCP one-liner), `view()`.
  Editing (the mockup's handlers, its rules and toasts): `pick_tool`, `pick_size`,
  `pick_weight`, `toggle_sag`, `toggle_forces`, `pick_material` (building only),
  `pick_level`, `clear` (= Reset), `click_joint(id)`, `click_grid(x, y)` (grid metres; see
  `data::snap(stage_px_x, stage_px_y)`), `click_member(id)`, `joint_down(id)` /
  `pointer_move(x, y)` / `pointer_up()` (drag), `clear_hover()`, `finish_line()`,
  `set_support(id, Option<Support>)`, `add_load(id, ±t)`, `set_member_size(id, size)`,
  `delete_member`, `delete_joint`, `delete_selected` (Delete), `nudge(dx, dy)` (arrow keys,
  2 m steps, drag rules), `starter()` / `can_starter()`, `undo()` (40 steps),
  `open_in_sandbox(design, material)`, `set_best`. Test: `start_or_stop()` (refuses with
  "Connect the loaded joint…"), `back_to_build()`, `replay()` (slow motion), `tick()` (one 33 ms
  tick: λ += 1/55, ≤ 30 breaks, loose-load / mechanism collapse, hold at λ = 1 with stars and
  best; or 3 collapse steps), `advance(dt)` (toasts fade after 2.6 s), `run_test(max_ticks)`
  (headless Test, for MCP `bridge_test`). Constants in `game`: `UNDO_DEPTH`, `LAMBDA_STEP`,
  `MAX_BREAKS_PER_TICK`, `TICK_SECONDS`, `TOAST_SECONDS`.
- **`bridge::view::View`** (`Game::view()`) — everything the screen shows: joint `positions`
  (grid m, displaced × 40 with sag / simulated), `members: Vec<MemberView { index, id, a, b,
  size, band: Band }>` (`Band::{T1..T3, C1..C3, Zero, Over, None}`, `band.level()` → opacity
  index into `sp_theme::FORCE_BANDS`), `supports`, `loads` (crate + tag "20 t"), `sparks`,
  `bad` (mechanism ring), `hover`, `ghost`, `handles: Vec<HandleView { text "−120", numeric,
  selected, over, label "Member M7, 63% of its limit" }>`, `joints: Vec<JointView { locked,
  pending, selected, bad, label "Joint J3, pin support, carrying 20 t" }>`, `water_flowing`,
  `crosshair`, `stage_label`, `surface_label`, `legend`, `toast`, `loading { frac, text }`,
  `result { title, held, stars, replay }`, `hint`, `show_sizes` / `show_weights` /
  `has_pending`, `levels` (rows with best stars), `goal`, `status { label, tone:
  Tone::{Muted, Ok, Warn, Danger} }`, `cost_text`, `over_budget`, `test_label`,
  `inspector: Inspector::{Member(MemberInspector { title, length, fill, ut, tone, text,
  size }), Joint(JointInspector { title, pos, support, can_support, locked, load, note,
  can_delete; chip_dim() }), Nothing(BuiltFrom { material, joints, members, weight,
  can_starter })}`, `explain`, `playing`, `slug`, `clock`. `view::status_of(&Analysis)`,
  `view::fmt_kn`.
- **`bridge::data`** — `MaterialKey::{Steel, Timber, Aluminum, CarbonFiber}` (`spec() ->
  GameMaterial { name, e, fy, rho, cost $/kg }`, `from_name`), `SizeKey::{Thin, Medium,
  Thick}` (`area()` 0.004 / 0.008 / 0.016 m², `stroke_px()`, `nearest(area)`),
  `Support::{Pin, Roller}`, `SupportRule::{Banks, LeftBank, Anywhere}` (`allows(x, y)`,
  `refusal()` toast), `LevelKey::{River, Overhang, Haul, Sandbox}` (`level() -> &Level { name,
  goal, budget, sup, nodes, loads }`, `slug()` = header document, `key()`), `stars_for(cost,
  budget)`, the stage geometry (`STAGE_W/H`, `px(x)`, `py(y)`, `snap`, `ground(x)`,
  `underground`), `WEIGHT_STEPS`, `MAX_LOAD_T`, `N_PER_TONNE` (9810), `JOINT_COST_K`.
- **`bridge::design`** — `Design { nodes: Vec<Joint { id "J3", x, y (i32 grid m), sup,
  locked }>, members: Vec<Bar { id "M7", a, b (joint ids), size }>, loads: Vec<(id, t)>
  (first-hung order), n_n, n_m }`; `Design::for_level`, `starter` (a mechanism until its
  middle panel gets a diagonal, as in the mockup), lookups, `set_load`, `to_truss(mat, λ)`,
  `analyse(mat, λ, broken) -> Analysis { ok, bad, displacement, members:
  Vec<Option<MemberResult>>, worst, mass, cost, n_act, used, reactions }` (self-weight half per
  end at λ, then weights × 9 810 N × λ, on `truss::Truss::solve`), `from_truss(&Truss, loads)
  -> Result<Design, ImportError>` (grid / underground / overlap / duplicate / sideways-load
  checks), `Band::of`.
- **`bridge::sim::Sim`** — the collapse, step for step as `initSim` / `stepSim` (Verlet,
  0.996, 14 passes, tear at 5 % with sparks, pins fixed, rollers keep height, ground, water
  drag below −8 m, 3.2 s; slow motion 0.3× for 1.6 s).
- **`bridge::netlist`** — `write(design, mat, level, name) -> String` (on
  `netlist::write_truss`: sizes as `.section`s, pins / rollers as `fix=`, weights as `.load
  weights … Fy=-20t`, `.load self +self`), `read(text) -> Result<(Design, MaterialKey),
  ReadError>` (on `netlist::parse`; static loads and the truck where it stands become
  weights). Advanced's solve of `write`'s output matches the Bridge to the newton (ADV-9).
- **`bridge::fmt`** — JavaScript-exact `to_fixed` (ties up), `js_round`, `num`, `stars`.
- `truss::hypot` (added for the Bridge, used for every member length): V8's `Math.hypot`, so
  symmetric members tie-break like the mockup.
- Tests: `tests/bridge_goldens.rs` replays `tests/bridge_goldens.json` (10 scenarios, 69
  snapshots written by `node crates/sp-engine/tests/bridge_goldens.mjs`, which runs the
  mockup's `Component` verbatim) and matches state, forces, displacements, cost, the collapse
  simulation, every string and the drawn geometry to within 2 ulps; `tests/bridge_props.rs`
  (random edits / NaN / huge loads never panic, invariants, undo, round trip, ENG-4 perf:
  60-joint solve ≈ 0.15 ms, a full 30-solve tick ≈ 4 ms).

### `netlist`, `generate`, `influence` (agent `advanced`)

All UI-free; numbers in SI inside, text exactly as the Advanced mockup prints it.

- **`netlist`** — the `.str` format (directive list and units in the module docs).
  - `netlist::parse(text) -> Result<Netlist, Vec<Diagnostic>>`. Two passes (order doesn't
    matter), never panics. `Diagnostic { line, column, len, severity: Severity::{Error,
    Warning}, message }` (1-based, columns in chars). `Ok` keeps warnings in
    `Netlist::warnings` (other worlds' `.story .wind .soil .foundation .dynamic`, filled-in
    values); `Err` holds every error and warning sorted by line. Caps: `MAX_NODES` 120,
    `MAX_MEMBERS` 600, `MAX_PLOT_POINTS` 400.
  - `Netlist { title: Option<String> /* "river-crossing.str" from the first * line */, notes,
    units, materials: Vec<truss::Material> /* name = netlist name */, sections: Vec<Section {
    name, area, inertia }>, nodes: Vec<truss::Node>, members: Vec<NetMember { id, a, b,
    section, material }>, loads: Vec<StaticLoad { case, nodes, fx, fy, self_weight }>, truck:
    Option<MovingLoad { case, weight /* N */, tonnes, x, lane, share }>, k_factor, plot:
    Option<Plot { member, range }>, warnings }`.
  - Getting a model out: `to_truss()`, `point_loads()` (the static `.load` forces, N, no
    self-weight), `member_sections()` (section name per member), `self_weight() ->
    SelfWeight::{None, MemberEnds, Shared}`, `static_loads(&truss)` (with self-weight),
    `truck_loads(x)`, `loads_at(&truss, x)`, `solve_options()` (K), `solve_at(x)`, `copies()`
    (`1/share`: 2 trusses), `stem()`, `lane_nodes()`, `lane_extent()`.
  - `+self` semantics: with nodes (`.load dead B1..B5 Fy=-80 +self`) the truss's whole
    self-weight is shared equally among them (the mockup's deck); with no nodes
    (`.load self +self`) half of each member's weight goes to each end (`Truss::self_weight`, the
    Bridge's lumping). Forces accept `t` (tonnes of weight: `t × 9.81 × 1000` N).
  - Writing: `netlist::write(&Netlist) -> String`; `netlist::write_truss(&WriteTruss { name,
    comment, truss, sections: &[String] /* per member */, loads: &[NodalLoad], self_weight })
    -> String` (= `write(&Netlist::from_truss(..))`): `.units SI kN m`, every material, one
    `.section` per (name, A, I) (`I=A^2` when I = A²), `.node … fix=xy|y|x`, `.member` (material
    named when not the first), `.load weights … Fy=-20t` (tonnes when whole to 3 decimals, else
    kN), `.load self +self`, `.static linear`, `.buckle euler K=1.0`. Names are made parseable
    and unique. `parse(write(n))` gives `n` back to ~12 significant digits.
  - `netlist::set_truck(text, tonnes: Option<f64>, x: Option<f64>) -> Option<String>`: edits
    the moving-load line in place (the Truck / x sliders).
  - `netlist::text::{to_fixed, minus_fixed, js_round, js_number, tidy}`: JavaScript's
    `toFixed` (ties up), the mockup's `fmt` (`−1004`), `Math.round`, `String(number)`.
- **`generate`** — `generate::generate(&Params { style: Style::{Pratt, Howe, Warren}, braced,
  material: MaterialKind::{Steel, Aluminum, Glulam, CarbonFiber}, size /* 1–20 */, truck_t /*
  0–120 */, x /* 0–48 */ }) -> Generated { params, lines: Vec<Line { dir, args, comment }>,
  text, netlist }`: the mockup's `build(style, braced)` and its netlist, line for line. The
  `netlist` is built with the mockup's own arithmetic (A = 0.0025·size, one ulp off the printed
  cm² for a few sizes), so the screen uses it while the text is unedited and results match the
  mockup bit for bit; `parse(text)` agrees to that ulp. `SIZE_RANGE`, `TRUCK_RANGE`, `SPAN`,
  `FILE_NAME`.
- **`influence`** — `influence::analyse(&Netlist) -> Result<Report, SolveError>` (truck where it
  stands; `analyse_at(net, x)`): `Report { truss, solution, x, copies, supports, span:
  Option<Span { left, right, length, mid }>, tones: Vec<Tone::{Zero, Tension, Compression,
  Failed}> /* elevation */, rows: Vec<Row { member, id, nodes, force, stress, limit,
  utilisation /* strings */, tone, use_tone: UseTone, result }> /* sorted by use */, status:
  Status::{Holding, Over, Mechanism} (.label(), .level()), checks: Vec<Check { label, value,
  note, level: Level::{Good, Accent, Bad, Neutral} }>, mass }`. `picks(&net, &report) ->
  Vec<Pick { key, name, member }>` (the mockup's four, or the four most-used members);
  `influence_line(&net, member) -> Option<InfluenceLine { points: [x, kN], peak, peak_at,
  limit_tension, limit_compression, scale, from, to }>` (full solve per position; `.level(kN)`
  → ±1.3 plot units, `peak_text()`, `limit_text()`, `label()`, …); `now_text(kN)`,
  `force_kn(&report, m)`, `sweep(&net)`.
- Tests: `tests/advanced_goldens.rs` against `tests/golden/advanced.json`, written by
  `node scripts/mockup-goldens.mjs` (the mockup's `Component` run in Node; 84 toolbar states);
  `tests/netlist_*.rs` (parser errors, round trip, fuzz), `tests/advanced_perf.rs` (ENG-4).

### `tower`, `dynamics`, `ground` (agent `worlds`)

TODO (owner fills in).

### `materials` (agent `materials`)

The Materials mockup's model, number for number (`Materials.dc.html` `defs()`, `tempFactor()`,
`stressAt()`, `tick()`, `renderVals()`). Strain in %, stress in MPa.

- `materials::curves` — `Kind::{Steel, Aluminum, Concrete, Timber, CarbonFiber}` (`ALL`,
  `key()` `steel`…`carbon-fiber`, `mockup_key()`, `parse(s)`, `name()`, `curve()`);
  `CURVES: [Curve; 5]` (`points: [[strain %, MPa]]`, `e_gpa`, `rho`, `ultimate_strain`,
  `yield_mpa: Option`, `note`) with `stress_at(e, T) -> Stress { mpa, broken, break_strain }`
  (fracture past the last corner; NaN / negative strain = 0, NaN temperature = 20 °C),
  `break_strain`, `elastic_limit`, `has_plastic_range`, `is_ductile`, `necks`, `necking(e)`
  (0 → 0.55), `peak_mpa(T)`, `yield_at(T)`, `toughness()` (room temperature, as the mockup
  shows) / `toughness_at(T)`, `specific_strength(T)` (kN·m/kg). Free functions
  `temp_factor(kind, T)`, `stress_at(kind, e, T)`, `load_cell_kn(MPa)` (50 mm² section).
  Constants `TEMP_MIN_C` −40, `TEMP_MAX_C` 600, `TEMP_STEP_C` 10, `HOT_C` 200, `SECTION_MM2`.
- `materials::bench` — `Bench` (the screen's whole state; serde): `kind`, `strain`, `temp`,
  `zoom`, `playing`, `broke(kind)`, `broke_count`, `all_broken`, `now()`, `force_kn`;
  actions `pick`, `set_temp` (clamped), `set_zoom` (clips the stretch), `set_strain` (pauses),
  `fresh_sample`, `toggle_drive` (restarts a broken / maxed sample), `reset`; `tick()` (one
  Pull step: 0.08 % on `Zoom::Wide`, 0.006 % on `Zoom::Close`; returns the kind that broke),
  `advance(dt)` (20 steps/s, `TICK_SECONDS` 0.05, capped), `next_tick_in()` (for
  `Ctx::wake_in`); `status() -> Status::{Ready, Elastic, Plastic, Necking, Fractured { at }}`
  (`label()`, `tone() -> Tone::{Muted, Ok, Accent, Caution, Danger}`), `explain()`, `clock()`
  (`ε = 0.00 %`), `share()` (NAV-6: `.material steel E=200GPa fy=275MPa rho=7850
  curve=steel ; tensile test at 20 °C, ε = 0.00 %, Ready`), `describe()`. `Zoom::{Wide,
  Close}` (`max`, `pull_step`, `slider_step`, `decimals`, `label`, `ticks`, `fitting(kind)`,
  `parse`). `CHALLENGE_RUNG` (5, 12).
- `materials::view` — `Bench::view() -> View`: every displayed string formatted like the
  mockup's JS (tiles, spec bars with `fill`, challenge chips, status, explain, notes, labels,
  clock, load cell, accessible labels) and the drawing model in the mockup's viewBox units:
  `PlotGeometry` (`0 0 560 400`: axes, grid, ghosts, curve, done trace, marker, ticks, curve
  labels) and `RigGeometry` (`0 0 420 560`: crosshead, specimen polygon(s), extensometer,
  crack, necking). `curve_path`, `plot_x`, `plot_y`.
- `materials::jsnum` — JavaScript-exact `to_fixed` (`toFixed`), `to_string` (`String(n)`),
  `round` (`Math.round`); usable by any module.

Tests: `tests/materials_goldens.rs` replays 27 scenarios / 253 snapshots produced by running
the mockup's own `Component` in Node (`tests/materials_goldens.mjs` →
`materials_goldens.json`; strings exact, geometry ≤ 0.05 px, strain bit-exact; a freshness test
re-runs the script when Node is present); `tests/materials_model.rs` (defaults, Pull to
fracture, timing, zoom, temperature, derived values, share, garbage input, perf).

## App (`crates/structures-playground`) — agent `shell`

Forked from Automata Playground's app crate and renamed. `lib.rs` re-exports the API screens
use: `Screen`, `Ctx`, `Carry`, `Command`, `Drive`, `Share`, `Badge`, `Provenance`, `Route`,
`Mode`, `Ticker`, `AppId`, `Launcher`.

### Files

| file | what |
|---|---|
| `src/app.rs` | `StructuresApp`: the frame (paper ground → header → world tab row → screen body → toasts), shortcuts, Share, navigation + hand-offs, persistence; `AppOptions` (`::default()` real, `::test()` / `::at(route)` deterministic); `AppEvent` log |
| `src/ctx.rs` | `Ctx` (per-frame screen context), `Carry`, `Command`, `Ticker`, frame pacing |
| `src/router.rs` | `Route` (`Bridge`, `Tower`, `Ground`, `Materials`, `Advanced`, `Series`), `Mode`; per route: tab label, default document, mockup, board size, chrome / panel height, ⌘-digit, id |
| `src/screens/mod.rs` | the `Screen` trait, `Drive`, `Share`, `Badge`, `Provenance`, `Screens` (all six kept alive) |
| `src/screens/{bridge,advanced,tower,ground,materials}.rs` | **owned by the screen agents** (placeholders from the shell: keep `XxxScreen::new()` + the `Screen` impl) |
| `src/screens/series.rs` | SCR-6 (shell) |
| `src/layout.rs` | `Column` + `columns` (the mockup's wrapping flex rows), `stage`, `graph_panel` |
| `src/widgets/*` | the widget kit (below); `header.rs`, `tabs.rs` are the chrome |
| `src/launcher.rs` | the lab-series apps (`AppId`) and the injectable launcher (Series cards) |
| `src/platform.rs` | `.str` files from Finder / the command line → Advanced (`Carry::Netlist`) |
| `src/smoke.rs` | `--smoke <s> [--headless]` (PLAT-1) |
| `tests/common/mod.rs` | the egui_kittest harness |

### How a screen plugs in

Each screen is `pub struct XxxScreen` with `pub fn new() -> Self` and `impl Screen`. The app
owns one of each in `Screens` (`app.screens().bridge`, …) for the whole run, so state survives
navigation. Only `ui` is required; everything else has a default.

```rust
impl Screen for TowerScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) { /* the body under the chrome */ }
    fn document(&self) -> Option<String> { None }                 // header doc name; None = Route::document()
    fn drive_button(&self) -> Option<Drive> {                       // the .prim drive button
        Some(Drive::new("Play gusts").running(self.playing))        // "Pause" + pause icon while running
    }
    fn drive(&mut self, ctx: &mut Ctx) { self.playing = !self.playing; } // click or Space
    fn has_reset(&self) -> bool { true }                            // show Reset (default true)
    fn reset(&mut self, ctx: &mut Ctx) { *self = Self::new(); }
    fn clock(&self) -> String { format!("T = {:.1} s", self.period) } // mono clock; "" = none
    fn share(&mut self, ctx: &mut Ctx) -> Option<Share> {           // NAV-6; None = "Nothing to share yet"
        Some(Share::new(self.story_line()).toast("Copied the tower as a .story line"))
    }
    fn provenance(&self) -> Vec<Badge> {                            // PROV-1, right of the world tabs
        vec![Badge::simplified("Wind + sway: simplified cantilever"),
             Badge::illustrative("Comfort bands: illustrative"), Badge::illustrative("Game costs")]
    }
    fn tick(&mut self, ctx: &mut Ctx) { /* advance by ctx.dt(); ctx.animate() / ctx.wake_in(s) while moving */ }
    fn command(&mut self, cmd: Command, ctx: &mut Ctx) -> bool { false } // ⌘Z, Delete, ⌘V
    fn enter(&mut self, ctx: &mut Ctx) {}                           // route became active (may repeat)
    fn receive(&mut self, carry: Carry, ctx: &mut Ctx) {}           // NAV-4 hand-off, right after enter
    fn describe(&self) -> String { String::new() }                  // one-line summary (MCP describe)
}
```

Per frame, for the active route: `enter` (first frame on the route) → `receive` (if a hand-off
is pending) → shortcuts / `command`s / Space → `drive` → `tick` (once per frame, not again in a
discarded layout pass) → the header reads `document`, `drive_button`, `has_reset`, `clock` and
calls `drive` / `reset` / `share` on a click → the world tab row reads `provenance` → `ui`.

- **Header (NAV-1)** — drawn by the app (`widgets/header.rs`): logo mark + "Structures
  Playground" (an AccessKit `Link` → Series), `/`, the document name (a `Label`), the mode
  switcher (`Navigation` "Mode": `Link`s Playground → last world, Advanced → Advanced; the
  current one `selected`), then the drive button (`Button` named "Test" / "Pause" …), Reset,
  the clock (`Label`), Share. Not on Series (it has its own Back).
- **World tab row (NAV-2)** — `widgets/tabs.rs`, on the four worlds: `TabList` "Playground
  worlds" with `Tab`s (current `selected`), and a `Group` "Provenance" with the screen's
  badges (each a `Label`). Badges: `Badge::exact / simplified / illustrative(text)`,
  `Badge::dot(text, colour)`, `Badge::plain(text)`; draw one anywhere with
  `widgets::badge(ui, &b)` (or `badge_with(ui, &b, BadgeLook::DANGER)`).
- **Share (NAV-6)** — the app copies `Share::text` to the clipboard (`Ctx::copy`, readable in
  tests as `app.last_copied()`) and toasts `Share::toast` (default "Copied <document> to the
  clipboard").
- **Keys (NAV-5)** — the app handles ⌘1–⌘4 (worlds), ⌘5 (Advanced), Esc on Series (Back), and
  Space with nothing focused (a focused button takes Space itself) → `drive` (only if
  `drive_button()` is enabled). With no text field focused it sends `Command::Undo` (⌘Z),
  `Command::Delete` (Delete / Backspace) and `Command::Paste(text)` (⌘V; the paste event is
  consumed). Screens handle other keys themselves (`ui.input`), e.g. arrow keys on a selected
  joint. Tab order is egui's: widgets in the order they are drawn (reading order).

### `Ctx` (per frame)

`route()`, `last_world()`, `previous()`; `go(route)`, `go_with(route, carry)`, `back()` (to
`previous`, else the last world); `pending()`; `toast(text)`; `copy(text)`; `save_text(name,
contents) -> Result<Option<PathBuf>>` (native dialog in the app, the output folder in tests);
`launch(AppId) -> Launch`; `rng_seed()` (fixed: `AppOptions::DEFAULT_SEED`, 7 in tests, NAV-7);
`autoplay()` (off in tests); `dt()`; `text_focus()`; `body_height()` (the window under the
chrome: what side-by-side columns fill); `egui()`; `animate()`; `wake_in(secs)`.

**Frame pacing (PLAT-3).** Nothing repaints unless asked. While something moves call
`ctx.animate()` every frame (display rate; at most 240 fps, 10 fps while the window is hidden),
or `ctx.wake_in(secs)` when only the next step matters (a 30 ticks/s test ramp:
`ctx.wake_in(ticker.due_in(30.0))`). `ctx.dt()` is the real time since the previous frame
(capped at 0.1 s; after idling it's ≤ 1/60 s, so pressing Play doesn't jump). Turn
`dt` into fixed steps with `Ticker::steps(dt, rate)` (≤ 8 per frame). Idle = no `animate` /
`wake_in` = 0 % CPU.

### Hand-offs (NAV-4) — `Carry`

```rust
pub enum Carry {
    Netlist { text: String, name: Option<String>, from: Option<Route> }, // .str text
    Building { floors: u32, base_shear_n: f64, overturning_nm: f64 },    // Tower → Ground
    Tower { floors: u32 },                                               // Ground → Tower
}
Carry::netlist(text, Some("my-bridge"), Route::Bridge)   // builder; carry.from()
```

| from → to | button | call | the target does |
|---|---|---|---|
| Tower → Ground | "Send base reactions to Ground ›" | `ctx.go_with(Route::Ground, Carry::Building { .. })` | floors → building load; the moment as a note |
| Ground → Tower | "‹ Back to the Tower" | `ctx.go_with(Route::Tower, Carry::Tower { floors })` | keeps its settings, takes the floors |
| Bridge → Advanced | "Open in Advanced" | `ctx.go_with(Route::Advanced, Carry::netlist(text, Some(name), Route::Bridge))` | parses + solves it (ADV-9); `name` → document |
| Advanced → Bridge | "Open in Playground" | `ctx.go_with(Route::Bridge, Carry::netlist(text, None, Route::Advanced))` | loads it into the Sandbox if it fits the grid |
| Finder / CLI `.str` → Advanced | — | the app (`platform.rs`) | as Bridge → Advanced, `from: None` |

The carry is delivered at the start of the next frame, after the target's `enter`; if the
user navigates on before that, it still reaches its target. Delivered carries are logged
(`AppEvent::Carried`).

### Widget kit (`src/widgets`, colours from `sp_theme`, every control an AccessKit node)

| widget | CSS | use |
|---|---|---|
| `prim(label)`, `ghost(label)` → `Button` | `.prim`, `.ghost` 36 px; `.big()` 44 px | `.icon(Icon::Play)`, `.enabled(b)`, `.pressed(b)` (toggle look + `aria-pressed`), `.full_width()`, `.centered()`, `.height(h)`, `.accessible_name(s)`, `.link()`; `.show(ui)` / `.show_at(ui, rect)` |
| `SegGroup::new(name, &items, selected)` + `SegItem::new(label).icon(..)` | `.seg` in a well (`.style(SegStyle::Header)`), compact `.mini` | `.fill()` (equal `flex: 1` items, centred), `.links()`, `.navigation()`; `.show(ui).clicked: Option<usize>`; items report pressed |
| `Toggle::new(label, on)` | `.chip` toggle (34 px pill; `.chip-on`) | `.dot(colour)` (material swatch), `.dimmed(b)` (`.chip-dim`), `.enabled(b)`, `.accessible_name(s)`; `.show(ui)` / `.show_at(ui, rect)`, `.clicked()` = flip; reports pressed |
| `Checkbox::new(label, checked)` | `input[type=checkbox]` row (18 px, accent; 44 px row, gap 10) | `.gap(8.0).height(40.0)` (Advanced), `.enabled(b)`; `.clicked()` = flip; an AccessKit `CheckBox` |
| `Slider::new(label, &mut v, range)` | `input[type=range]` (sand) with its label / value row | `.step(s)`, `.value_text(s)`, `.width(w)`; arrow keys step it (keeping focus), AccessKit increment / decrement / set value |
| `lbl(ui, text)`, `lbl_colored` | `.lbl` eyebrow | |
| `Card::panel()`, `Card::well()` | `.panel`, `.well` | `.padding()`, `.gap()`, `.fill()`, `.border()`, `.label(name)` (a `Group`); `.show(ui, \|ui\| ..)` |
| `StatTile::new(label, value)` | stat tile (a `.well`) | `.color(c)`, `.compact()` |
| `Callout::happening(text)` | "What’s happening" section | |
| `badge(ui, &Badge)`, `badge_with(.., BadgeLook)` | `.badge` + `.dot` | |
| `Chip::new(text).part(text, colour)` | the floating measurement `.chip` | `.show_at(ui, pos, Align2)` |
| `CodeBlock::new(text)` | mono block | |
| `Plot::series(&values)` | line + fill + guides + playhead | `.height()`, `.playhead(Some(i))`, `.label()` |
| `Icon::*` | every mockup icon (play, pause, restart, truss, chart, back, the Bridge's tools rail, logo) | `icon.paint(painter, rect, colour)` |
| `Toasts` | app toasts (2.6 s, bottom centre) | `ctx.toast(text)` |
| `text::{galley, wrapped, text_top, lbl_galley, …}` | text helpers | |
| `widgets::hit::{target, tile}`, `interact`, `set_bounds` | 44 px hit areas, AccessKit plumbing | |

`widgets::gallery::Gallery` draws all of them (the widget tests). Screen agents may add
widget files for their own screen (one `pub mod` line in `widgets/mod.rs`); changes to the
shared widgets go to `shell` via PROGRESS Requests.

### Layout helpers (`src/layout.rs`)

`Column` is one flex column of a body row: `Column::rail("Tools")` (76 px, the Bridge),
`Column::parts("Parts")` (1 1 250, 220–300, border-right), `Column::main("Stage")` (999 1 560,
padding 14 16, gap 10; `.flex(999.0, 1.0, 520.0).pad(16.0, 16.0).gap(12.0)` for Tower / Ground /
Materials), `Column::readouts("Readouts")` (1 1 300, 260–360, border-left), plus `.flex()`,
`.min()`, `.max()`, `.pad(y, x)`, `.gap()`, `.border_left()`, `.border_right()`, `.scroll(b)`.
`layout::columns(ui, ctx, &COLS, |ui, ctx, i| ..)` lays the row out (CSS flex-wrap with min /
max widths): side by side the columns fill `ctx.body_height()` and asides scroll on their own;
stacked (narrow windows) each line is as tall as its tallest column's content — what the
column *allocated*, measured the pass before (a column that only paints into
`available_rect_before_wrap()` must allocate it, e.g. `ui.advance_cursor_after_rect(rect)`) —
the lines share out the rest only when together shorter than the route's `panel_height()`, and
only the page scrolls. `layout::resolve(width, &cols)` is the pure maths. `layout::stage(ui)` paints the
`.graph` ground over the rest of `ui`; `layout::graph_panel(ui, rect, fill)` returns a clipped
child `Ui` for a rounded stage panel, `graph_panel_border(ui, rect)` draws its border on top.

### Test harness (`tests/common/mod.rs`, `mod common; use common::*;`)

Building: `harness(route)` (1440 × 900), `harness_at(route, w, h)`, `app().at(r).phone()`
(390 × 1000) / `.board()` / `.size(w, h)` / `.launcher(l)` / `.autoplay()` / `.options(o)`
`.build()`; `settle(&mut h)` (4 frames). State: `route(&h)`, `goto(&mut h, r)`,
`hand_off(&mut h, r, carry)`, `events(&h)` / `carried(&h)` (`AppEvent`s), `last_copied(&h)`,
`document(&h)`, `drive_label(&h)`, `toasts(&h)`, `step_until(&mut h, n, |h| ..)`,
`h.state().screens().tower` (the screen's own state). Acting by AccessKit label:
`click(&mut h, label)`, `click_button`, `click_link`, `click_tab`, `click_role(.., Role)`,
`click_drive(&mut h)`, `click_header(&mut h, "Reset")`, `header_button(&h, label)`,
`type_into(&mut h, label, text)`, `press(&mut h, Key)`, `press_cmd(&mut h, Key)`,
`reveal(..)` (scrolls into view). Reading: `has_label`, `has_label_containing`, `get(&h, label,
Role)`, `pressed(&h, label)`, `selected(&h, label, Role)`, `disabled(&h, label)`,
`focused_label(&h)`, `nodes_with_roles(&h, &[Role])`, `child_labels(..)`, `text_of(&node)`.
Rendering: `render(&mut h)`, `screenshot(&mut h, name)` (into the test's tmp dir),
`render_full(route, w, min_h)`, `mockup_png(route, 1440)`, `load`, `mean_diff(a, b,
region)`.

`AppOptions::test()` is deterministic: seed 7, no autoplay, no persistence, no save dialog, a
recording launcher with nothing installed, a fresh temp output folder. `app.events()` logs
`AppEvent::{Drive, Reset, Shared, Carried, Command}` (the drive / Reset / Share presses, the
hand-offs delivered and the keyboard commands with whether the screen handled them).

Shell test files: `tests/nav.rs` (every route by click and key, Back / Esc, Space, Reset,
Share, hand-offs, commands), `tests/a11y.rs` (chrome names, 44 px targets, Tab order),
`tests/widgets.rs` (the gallery), `tests/series.rs`, `tests/pacing.rs` (PLAT-3: **every
route at rest must ask for no frames** — a screen that animates while idle fails it),
`tests/compare.rs` (chrome closeness; `scripts/compare-mockup.sh` renders `[app | mockup]`
side by side into `target/compare/` for any screen: `scripts/compare-mockup.sh tower`).

**Perf tests (ENG-4).** `scripts/test-all.sh` runs every test target named `perf` or `*_perf`
in release with `--include-ignored --nocapture`; it must pass, and each printed line
`PERF <label>: <value> ms (limit <limit> ms)` is listed as a gate in the summary.

### Persistence (NAV-7), platform, smoke

eframe keeps the window size (`persist_window`) and `Persisted { route, last_world }` (Series
restores to the last world). A fresh start is reproducible: the seed is fixed
(`AppOptions::DEFAULT_SEED`). `structures-playground --smoke <seconds> [--headless]` opens
the Bridge, presses its drive button, runs and prints `SMOKE route=bridge mode=… first_frame_ms=…
frames=… frame_ms_avg=… frame_ms_max=… status=ok`. `STRUCTURES_BUNDLE=1` at build time
(`scripts/bundle-macos.sh`) leaves the checkout path out of the binary (`MANIFEST_DIR`).

## MCP — wave 2

TODO.
