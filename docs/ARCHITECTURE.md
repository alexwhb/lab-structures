# Architecture

```
lab-series/
  labkit/                          shared crates (github.com/alexwhb/labkit, a git dependency pinned
                                   to a commit; another repo — never edit from here)
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
    crates/structures-playground   the egui app: header, router, widgets, three screens (the builder,
                                   Advanced, Series), MCP bridge
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

### `builder` (v2; agents `engine`, `time`, `lab`)

The v2 Playground: `v2/mockup/Builder.dc.html`'s `class Component` without the drawing, ported
operation for operation so it is **bit-exact with the mockup** (golden test below). The screen
owns nothing but pixels. Fold-ins: `comfort`, `longterm` (agent `time`); `sample`, `fire`
(agent `lab`). **Core (agent `engine`):**

- **`builder::data`** — `PieceType::{Road, Floor, Wood, Steel, Concrete, Rope, Cable}`
  (`spec() -> &TypeSpec { name, area, e, k, f_t, f_c, mpm, rho, cost, max_len, t_only, swatch_h,
  scene, deck, ductile, blurb, tip }`, `mass_per_m()`, `allowed_in(scene)`, `for_scene(scene)`
  (palette order), `too_long()` / `too_long_add_joint()` toasts, `key()` / `from_key` /
  `from_name`), `Soil::{Rock, Dense, Stiff, Soft, Loose}` (`spec() -> SoilSpec { name, liq,
  footing: [Qc, Qt, ks], pile }`, `grip(found)`), `Foundation::{Footing, Pile}` (`cost()` 300 /
  1500 $, `chip()` "Footing · $0.3k", `tip()`), `VehicleKind::{Car, Bus, Truck}` (`spec() ->
  VehicleSpec { m, len, ht, d, v, label }`), `TrafficSet::{Level, Rush, Buses, Trucks, Convoy}`
  (`vehicles(level_list)`), chip tables `WINDS`, `SNOWS`, `QUAKES`, `FIRES` (Off / Small 400 /
  Big 700 / Inferno 900 °C), `DIRS`, `WEIGHTS`, `MAX_CRATE_T`; `Scene::{River, Flat}`
  (`ground(x)`), `Group`, `LevelKey::{River, Bus, Haul, T1, T2, T3, Pisa, Sandbox}` (`slug()` =
  header document), `Level { key, group, name, goal, scene, soil, bed, wind, snow, quake, fire,
  budget, target, comfort, sup: SupportRule::{Banks, Any}, traffic, anchors: Vec<Anchor { x, y,
  found }>, prebuilt: Option<Design> /* whole starting design, locked parts */, run:
  RunKind::{Traffic, Test, Years} }`, `levels()` (the aside's table; Pisa listed once
  `longterm` provides it), `level(key)`, `stars_for(cost, budget)`, `stars_text(n)`.
- **`builder::design`** — `Design { nodes: Vec<Joint { id "J3", x, y (world m), sup:
  Option<Foundation>, locked }>, members: Vec<Piece { id "M7", a, b, kind, locked }>, loads:
  Vec<(joint id, t)> /* first-hung order */, n_n, n_m }`: lookups, `load` / `set_load`,
  `length(piece)` (`truss::hypot` = V8 `Math.hypot`), `cost()` ($k), `top()` (highest Floor
  joint), `has(kind)`.
- **`builder::setup`** — `Conditions { wind, dir (±1), snow, quake, soil, traffic, fire (°C, 0 =
  off) }` (`for_level`), `spec(key, sb_scene)` (Sandbox scene applied), `Setup` = the mockup's
  `L()`: the level with the conditions (`scene`, `soil`, `bed`, `traffic` resolved, `budget`,
  `target`, `comfort`, `sup`, `anchors`, `run`, `sandbox`; `ground(x)`, `sup_ok(x, y)`,
  `soil_at(joint)`, `sup_refusal()`), `practice(key, sb_scene, &cond)`.
- **`builder::camera`** — `ViewState { cx, cy, z }` (`default_for(scene)`), `Camera::new(scene,
  view)` (`sc` px/m, `grid`, clamp box, 2.5D `off`; `x(wx)` / `y(wy)` / `px(p)` world → stage
  px on the 1120 × 600 stage, `to_world(px, py)`, `snap(px, py)`, `on_screen`, `zoomed(f, px,
  py)`), `fit(scene, joints, target)`; `STAGE_W/H`, `ZOOM_MIN/MAX`, `ZOOM_STEP` 1.3,
  `WHEEL_STEP` 1.15.
- **`builder::sim`** — the settling engine. `Sim::new(&Design, &Setup, &Conditions, live)`
  (`makeSim`), `step(h, damp)`, `frame(slow)` (fire hook, 20 substeps of 1/600 s (× 0.3 slow),
  damping `1 − 0.4h`, top-floor sway and smoothed acceleration), `check_done()` (river: 1.2 s
  after no vehicle drives or falls, or 45 s; open ground 18 s; Sandbox never), `support(x,
  y_hint)`, `milli_g()`, `crossed()`, `laps()`. Public state: `p: Vec<Particle { id, x, y, px,
  py, vx, vy, m, fx, fy, sup, x0, y0, ys, used, r, ripped, over_c, over_t, qc, qt, ks, liq, inv
  }>`, `c: Vec<Constraint { id, a, b, kind, len, kk, cap_t, cap_c, crush, pcr, buckles, t_only,
  deck, road, n, avg, broken, over, slack, exposed, strength, stiffness, temp, heat_mode }>`
  (`cap_t_now()`, `cap_c_now()` with the factors, `utilisation()`), `v: Vec<Vehicle { kind, m,
  len, ht, d, speed, x, y, ang, state: VehicleState::{Drive, Fall, Sunk, Crashed, Done}, vx, vy,
  ax: [Option<Axle { c, f, y, dist }>; 2], laps }>`, `setup`, `cond` (the Game copies the live
  conditions in every frame), `t`, `sparks`, `splashes`, `first_break: Option<FirstBreak { id,
  kind, mode: BreakMode, t }>`, `first_found: Option<FirstFound { id, mode: FoundMode::{Sank,
  Liquefied, RippedOut} }>`, `end_at`, `wind_l/r`, `width`, `top_i`, `top_y0`, `sway`, `peak_a`,
  `a_s`, `env`, `wind_now`, `snow_now`, `quake_start`, `sandbox`. `BreakMode::{Snapped, Cracked,
  Buckled, Crushed, Softened, BurnedThrough}.text()`. `capacity(kind, len) -> Capacity { cap_t,
  crush, pcr, cap_c, buckles, kk }` (shared with the sample drawer). Constants `SUBSTEPS`,
  `FRAME`, `SLOW`, `TEST_SECONDS`, `RIVER_MAX_SECONDS`, `RIVER_TAIL_SECONDS`.
  **Fold-in hooks** (defaults are bit-identical to the mockup): per piece `strength` (× cap_t
  and the crushing part of cap_c) and `stiffness` (× XPBD stiffness and P_cr), `temp` (°C),
  `heat_mode` (a break reports it); `Sim::fire_start` (owned by `fire::update`, called by
  `frame` every running frame); `Sim::drag` (× the floor wind force, Cd 1.3 built in) and
  `Sim::shedding` (× the 0.3 vortex gust term) for CMF-2.
- **`builder::preview`** — `preview(&Design, &Setup, &Conditions) -> Settled` (520 steps at
  1/400 s, damping 0.985, gravity over 0.3 s, snow at its setting) and `settle(…,
  &SettleOptions { steps, h, damping, offsets /* per joint, m: lowers a foundation's ys */ })`;
  `Settled { pieces: Vec<PieceForce { id, n, over, slack, cap_t, cap_c, len, exposed }>, found:
  Vec<Option<FoundForce { id, r, qc, qt, over_c, over_t }>> /* per joint */, positions, moved,
  moved_id }` (`piece(id)`, `foundation(id)`, `over_any()`, `found_over()`), `settled_of(&sim,
  &design)`.
- **`builder::game`** — **`Game`**, the state machine. `Game::new()` (level 1, Build, Road,
  building: MB-10), `Game::with_preset(&Preset)` (`Preset::board("TowerStorm")` for the seven
  boards; `PresetBuild::{Truss { chord }, Cable, Tower { h, bays, col, brace, found }}`,
  `build_preset(b)`). Reading: `level()`, `sb_scene()`, `scene()`, `cond()`, `setup()`,
  `view_state()`, `camera()`, `design()`, `pending()`, `selection() -> Option<&Selection::{Node,
  Member}>`, `phase() -> Phase::{Build, Run, Result}`, `building()`, `live()`, `hover()`,
  `dragging()`, `result() -> Option<&RunResult { pass, stars, cost, practice, first,
  first_found, outcome: Outcome::{River { crossed, total }, Tower { standing, reached, mg, comfy,
  height, sway }} }>`, `piece_type()`, `tool() -> Tool::{Select, Build, Found, Crate, Erase}`,
  `found_kind()`, `weight()`, `show_forces()`, `slow()`, `best_for(k)` / `best()` /
  `set_best(k, n)`, `toast()` / `toast_remaining()`, `sim()` / `sim_mut()`, `preview()`,
  `undo_depth()`, `revision()`, `cost()`, `practice()`, `animating()`. Editing (the mockup's
  rules and toasts; building only): `pick_level`, `pick_scene` (Sandbox chips), `clear` (=
  Clear and Reset), `pick_tool`, `pick_type`, `pick_found`, `pick_weight`, `toggle_forces`,
  `finish_line`, `click_node(id)`, `click_surface(px, py)` (stage px; snaps) / `click_at([x,
  y])` (a grid point), `pick_handle(id)`, `select`, `set_foundation(id, Option)`,
  `add_crate(id, t)` / `crate_less` / `crate_more`, `set_piece_type(id, kind)`,
  `delete_member`, `delete_joint`, `delete_selected`, `undo()` (40 steps; only edits that
  change the design leave a step), `nudge(dx, dy)` (⌥ arrows: one grid step, drag rules).
  Pointer: `joint_down(id)`, `surface_down(px, py)` (pan with Select or while running),
  `pointer_move(px, py)` (pan / drag / hover), `pointer_up()`, `clear_hover()`. Camera:
  `zoom_in`, `zoom_out`, `wheel(delta_y, px, py)`, `zoom_by`, `fit`, `pan_by(dx_px, dy_px)`,
  `set_view`. Conditions (any time; soil and traffic refused while running): `set_wind`,
  `set_dir`, `set_snow`, `set_quake`, `set_fire`, `set_soil`, `set_traffic`, `shake_now`,
  `light_fire_now`. Running: `start_run(slow)`, `stop_run`, `toggle_run`, `replay`,
  `toggle_slow`, `tick()` (one 33 ms frame; result, stars and best stars at the end),
  `advance(dt)` (toasts fade after 2.6 s), `run_test(max_frames) -> Option<&RunResult>`
  (headless). Netlists: `share_text()`, `paste_netlist(text)`, `open_in_sandbox(scene,
  design)`. Free: `run_headless(&Design, &Setup, &Conditions, practice, max_frames, tweak:
  FnOnce(&mut Sim)) -> (Option<RunResult>, Sim)` (CMF-2's runs: `tweak` sets `drag` /
  `shedding`), `result_of(&sim, cost, practice)`.
- **`builder::view`** — `Game::view() -> View`: every shape in **stage px** (1120 × 600; the
  screen maps with one scale + offset), every string (plus world metres where a fold-in paints in world space: `MemberView::{wa, wb}`, `JointView::{pos, sup}`, `VehicleView::{x, y, ang}`, `wind_now`, `wind_dir`, `snow_now`, `quake_env`, `sim_t`). Stage: `camera`, `grid { origin, step }`,
  `terrain { scene, soil, bed, soil_poly, bed_poly, water_poly, flow, tops, edges, grains }`,
  `flowing`, `members: Vec<MemberView { index, id, kind, a, b, sag /* slack rope/cable: quad
  control point */, band: Option<Band::{T1..T3, C1..C3, Over}>, deck: Option<DeckView { top,
  front, line, facade, side, window, snow }>, back, temp }>`, `snow_px`, `foundations: Vec<
  FoundationView { joint, kind, at, pad, pile, tri }>`, `crates: Vec<CrateView { joint, t,
  wire, body, top, side, tag "5 t", tag_at }>`, `vehicles: Vec<VehicleView { kind, state, body,
  cab, windows, wheels }>`, `wind: Vec<WindArrow { from, to, head, streak }>`, `flakes`
  (centres, r 1.6), `target_y`, `sparks`, `splashes`, `rings` (r 13), `ghost`, `hover` (r 5),
  `handles: Vec<HandleView { id, at, text "+2.4 t", label "Steel M7, 63% of its limit", num,
  selected, over }>`, `joints: Vec<JointView { id, at, label "Joint J3, on a pile, carrying a 5
  t crate", founded, locked, pending, selected, bad }>`, `hud: Vec<HudChip { key "Height ",
  value, tone: HudTone }>`, `shaking`, `cursor`, `stage_label`, `surface_label`, `toast`,
  `running`, `run_info`, `slow_label`, `result: Option<ResultView { title, pass, stars }>`.
  Header / rail / hint bar: `level_slug`, `clock`, `playing`, `hdr_run`, `phase`, `tools`,
  `can_undo`, `hint`, `show_types` / `types`, `show_founds` / `founds`, `show_weights` /
  `weights`, `has_pending`, `show_forces`, `zoom_pct`. Aside: `level_groups`, `goal`,
  `chips_info`, `status { label, tone: StatusTone }`, `cost_text`, `over_budget`, `run_label`,
  `is_sandbox`, `is_river`, `sb_scenes`, `conditions: ConditionsView { winds, dirs, snows,
  quakes, soils, traffic, show_traffic, fires, can_shake }` (`Chip<T> { value, name, on, dim
  }`), `inspector: Inspector::{Piece(PieceInspector { id, kind, title, len, fill, ut, tone:
  BarTone, text, types, locked }), Joint(JointInspector { id, title, pos, founds, load, note,
  can_delete }), Materials(Vec<MaterialCard { kind, name, curve /* curveFor path */,
  curve_lines, blurb, cost }>)}`, `explain`. Helpers `fmt_t`, `curve_for`, `curve_lines`,
  `Band::of`. Colours: the screen maps `PieceType` / `Soil` / `Band` / tones to
  `sp_theme::builder`.
- **`builder::netlist`** — `write(&Design, level, scene, &cond, name) -> String` (one
  `.material` per piece type used, sections from the type's area with I = k·A², foundations
  `fix=xy`, crates `.load weights … Fy=-5t`, `.load self +self`; `* level …`, `* cond …`, `*
  found J1=footing …` comment lines) and `read(text) -> Result<Imported { design, scene, level,
  cond }, ReadError>` (whole metres; river scene when every joint fits it, else open ground).
- Tests: `tests/builder_goldens.rs` replays `tests/builder_goldens.json`, written by `node
  crates/sp-engine/tests/builder_goldens.mjs` (the mockup's `Component` run verbatim in Node);
  `tests/builder_props.rs` (random edits, NaN / huge inputs, absurd zooms never panic);
  `tests/builder_perf.rs` (BLD-15, `PERF` lines).

**Fold-ins (agent `time`):**

- **`builder::comfort`** (CMF-1 / CMF-2) — the v1 Tower's comfort bands and sentences
  (`tower::Comfort`) on the builder's game scale `GAME_SCALE` = 15 (`bands()` 75 / 150 / 300 mg,
  `full_scale()` 450 "450+"; level 5's 300 mg limit is the "Seasick" line). `Meter::new(mg,
  limit) -> Meter { mg, limit, band, marker_pct, limit_pct, over_limit }` (`sentence()`,
  `tone()` → `tower::Tone`, `value()` "272 mg", `limit_text()` "limit 300 mg", `label()` the
  AccessKit summary; `Meter::stretches()` / `Meter::ticks()` with positions), `pct(mg)`,
  `band(mg)`, `mg_text`. Four shapes: `Variant::{AsBuilt, Tapered, Twisted, Hollow}` (`name()`,
  `factors() -> (drag, shedding)` = v1 `Shape::def()` `cd / BUILDER_CD` (1.3) and `cross`),
  `tapered(&Design) -> Design` (x drawn toward the centre line with height, top `TAPER_TOP` =
  60 % as wide; same pieces and crates), `applies(&Design, &Setup)` (open ground with Floors),
  `compare(&Design, &Setup, &Conditions) -> Comparison { bars: [Bar { variant, mg, standing,
  width }; 4], best, verdict }` — four 18 s `run_headless` tests on scoped threads,
  deterministic, the as-built bar equals the live run (`Bar::{value, tone(limit), label}`).
- **`builder::longterm`** (TIME-1 / TIME-2) — `fast_forward_design(&Design, &Setup,
  &Conditions) -> LongTerm` = `structure(…) -> (statics::Structure, Kind)` (joint masses,
  foundation grips and piece stiffness / capacities from `Sim::new(…, live = false)`; `Kind::
  Tower { top }` on open ground with Floors, else `Kind::Bridge`) + `fast_forward(&Structure,
  Kind)`. Each foundation consolidates toward `s∞ = R / k_long` (`k_long = ks / ratio`,
  `consolidation(soil) -> Option<Consolidation { tau, ratio }>`: τ and `ratio = KS_REF /
  ks_ground` from `sp_engine::ground`, rock `None`; overload past Qc creeps `OVERLOAD_CREEP` ×),
  exactly per step of `STEP` = 0.5 yr, never back up; after each step `statics::Structure::
  settle` finds the new equilibrium — an energy-minimising Newton solve with a positive-definite
  tangent (builder springs `E/15·A/L`, foundations `ks` + pinned x, gravity on the joint masses,
  geometric stiffness of pulled pieces) and a line search, starting from the last positions, so
  the tilt moves the loads (P-Δ) and a tower past its tipping point slides over; ropes go slack,
  resting joints lift, foundations rip out past Qt, pieces break past capacity. (Why not the
  preview: its 520 steps don't converge the rocking mode on soft soil.) `LongTerm { kind,
  frames: Vec<YearFrame { year, pos, force, broken, slack, settle, reaction, tilt, max_settle,
  differential }>, giving: Vec<Giving::{Broke, Ripped}>, ending: Ending::{Stood, Toppled(yr),
  CameDown(yr), Nothing}, joint_ids, piece_ids, found }` with `at(year) -> Snapshot`
  (interpolated), `chips(year)` ("Year " "12.0", "Tilt " "1.8°" / "Differential " "12 mm",
  "Settled " "214 mm"), `lean_line(year)` (foundations' and top floor's centroids),
  `result_line()` ("After 30 years: tilt 0.3°, settled 41 mm."), `sentence()`, `end_year()`,
  `piece_index` / `joint_index`; `has_foundations(&Design)`, `anim_year(elapsed, end)` (30
  years over `ANIM_SECONDS` = 6), `TOPPLE_DEG` 5, `LEAN_OK_DEG` 0.5. Tilt is measured from the
  line as built; "Settled" / differential count consolidation since year 0.
- **`builder::longterm::pisa`** (TIME-3) — `level() -> Level` ("7 · Leaning tower", soft clay,
  budget `BUDGET` $8k, `run: RunKind::Years`, `prebuilt`: `prebuilt()` — 2 bays × 8 storeys,
  Concrete walls and diagonals, a Floor per level, all locked; three unlocked footings `J1`–`J3`
  (swap for piles); the bells `BELLS_T` = 8 t on `bells_joint()` `J27`), `result(&LongTerm,
  cost, practice, budget) -> PisaResult { pass, stars, title, explain, tilt }` (pass: stood and
  under `GOAL_TILT` 0.5° after 30 years), `fix_cost(&Design)` (cost without locked pieces),
  `footing_ids()`, `top_ids()`. Untouched: 0.6° → 4.3° after 30 years; piles under all three
  footings: 0.16°, $4.5k, three stars; an 8 t counterweight on the top-left joint: 0.0°.
- UI (`structures-playground/src/screens/builder/{comfort,time}.rs`, `FoldIn`s): `Comfort`
  (`results`: the meter while running / after an open-ground run, "Compare shapes" → the bars
  and verdict; `comparison(&game)`, `compare(&game)` for MCP), `Time` (`results`: "Fast-forward
  30 years", progress + "Skip to the end", the "Years" scrubber, result line + sentence, "Play
  again"; `hud`, `paint_stage` plumb + lean line, `tick` / `busy` while playing, the Leaning
  tower's `drive` / `drive_button` / `adjust_view`; `Time::available(&game)`, `start(&game)`,
  `finish(&mut game)`, `set_year`, `long_term()`, `year()`, `playing()`, `pisa_result()`).
  Colours `sp_theme::time`.
- Tests: `tests/comfort.rs`, `comfort_perf.rs` (`PERF compare shapes …`, < 400 ms),
  `longterm.rs` (determinism, monotonic settlement, rock 0 / soft clay most, symmetric vs
  eccentric, topple, bridges, random structures), `longterm_model.rs` (hand-built structures),
  `longterm_pisa.rs` (the level: untouched fails, the named fix passes), `longterm_perf.rs`
  (`PERF 30-year fast-forward …`, < 300 ms); GUI `structures-playground/tests/builder_time.rs`.

**Fold-ins (agent `lab`):**

- **`builder::fire`** (FIRE-1) — `FireLevel::{Off, Small, Big, Inferno}` (`ALL`, `name()`,
  `key()`, `parse(s)`, `temp_c()` 400 / 700 / 900, `reach_m()` 8 / 12 / 16, `setting()` → the
  `Conditions::fire` °C (0 = off), `from_temp(c)`). **`fire::update(&mut Sim, dt)`** — called by
  `Sim::frame` (`fire_hook`) every running frame: lights the fire (`Sim::fire_start`)
  `FIRE_START_S` = 3 s into a run when Fire was set before it, 0.5 s after it is switched on
  mid-run, Off puts it out (`None`); then for every intact piece heads `Constraint::temp` toward
  the fire's exposure with a first-order lag (`heat::lag_s`: steel / cable 3 s, wood 4, rope 2,
  concrete 8) and sets `strength` / `stiffness` (`fire::apply(&mut Constraint)`: steel and
  cable follow the temperature now, the others keep the worst they reached) and `heat_mode`
  (`break_mode(kind, strength)` → `FireBreak::{Softened, BurnedThrough}` below 90 % strength,
  `BreakMode::from`). Returns at once on a static settle or while Fire is off and nothing is
  hot. `light_now(&mut Sim)` / `can_light(&Sim)` ("Light it now": 0.2 s later, only while set
  and not burning), `blaze(&Sim) -> heat::Blaze { source, growth, temp_c, reach }`
  (`burning()`, `target(a, b)`), `source(&Sim)` (river (28, −8); open ground: mean x of the
  founded joints at y = 0), `hottest(&Sim)`, `hud_value(&Sim)` ("650 °C"), `note(level, river)`,
  `tip(FireBreak, kind)`, `strength_factor(FireMaterial, &Factors)` (the builder's one strength
  factor: concrete's compressive). `fire::heat` — `plume_distance(source, a, b)` (only what is
  at or above the fire; height counts `PLUME` = 0.5), `exposure(d, reach)` (full inside
  `CORE` = 0.5 of the reach, linear to 0), `growth(t, start, on)` (over `GROW_S` = 5 s),
  `full_reach(c)`, `approach(temp, target, dt, m)`. `fire::factors` — `FireMaterial::{Steel,
  Cable, Concrete, Wood, Rope}` (`of(PieceType)`, Road / Floor → Concrete,
  `damage_is_permanent()`, `factors(t_c)`), `Factors { strength_t, strength_c, stiffness }`,
  the tables `STEEL_KY` / `STEEL_KE` (EN 1993-1-2 Table 3.1), `CONCRETE_KC` / `CONCRETE_EPS_C1`
  (EN 1992-1-2 Table 3.1, siliceous), `steel_ky/ke`, `concrete_kc/kct/ke`, `interp`. Sources:
  DATA-SOURCES § Fire.
- **`builder::sample`** (LAB-1) — `SampleCurve::new(kind, Test::{Pull, Push}, temp_c, len)`
  (`None` for pushing Rope / Cable): `[strain %, MPa]` corners, straight at E × k_E to the
  builder's capacity ÷ area (`capacity_n`, `limit_mpa`, `elastic_strain`), then ductile
  plateau / hardening / necking (`DUCTILE_STEEL` = v1's S275 shape scaled, `DUCTILE_WIRE`), a
  brittle snap / crack, or for Push crushing (`CONCRETE_PUSH_TAIL` to 0.35 %, `WOOD_PUSH_TAIL`),
  squashing (steel) or Euler buckling (`buckles`, `crush_n`); `failure: Failure::{Fractured,
  Snapped, Cracked, Squashed, Crushed, Buckled, BurnedThrough}`, `stress_at(e)`, `force_at(e)`
  (N on the builder's section), `necking(e)`, `peak_mpa()`, `break_strain()`.
  `curve::piece_at(kind, len, temp_c) -> Constraint` (the core's piece model heated by
  `fire::apply`) and `capacity(kind, test, len, temp_c)` = `cap_t_now()` / `cap_c_now()` — so
  "a piece this size breaks at N t" is the builder's number. **`Sample`** (the drawer's state):
  `new(kind)`, `open(kind)` (fresh Pull, 1 m, temperature kept), `pull()` / `push()` (start,
  pause, a fresh sample when switching or broken), `fresh_sample()`, `pause()`,
  `set_temp(c)` (−40…900 °C), `set_length(m)` (1 … the type's max length), `set_strain`,
  `tick()` / `advance(dt)` (20 steps a second; the straight line in 12 steps, landing exactly on
  the builder's limit, the rest of the fitted axis in 160), `next_tick_in()`, `run_to_end()`;
  `status() -> Status::{Ready, Elastic, Plastic, Necking, Squashing, Bowing, PastPeak, Failed {
  how, at }, BurnedThrough}` (`label(kind)`, `tone()`), `limit_sentence()` ("A Steel piece this
  size breaks at 153 t." / "A 14 m Steel piece buckles at 55 t."), `explain()` (v1's voice),
  `heat_note()`, `force_n()`, `capacity_n()`, `at_builder_limit()`, `axis_max()`, `describe()`.
  `Sample::view() -> SampleView`: every string (status, force "153 t" / "1500 kN", limit + note,
  temperature label / note, tiles, Pull / Push / Pause labels, push note, rig / plot labels,
  load cell) and the drawing in v1's viewBox units: `PlotGeometry` (fitted axes and ticks, the
  curve, the 20 °C ghost when heated, done trace, marker, the builder-limit line + label) and
  `RigGeometry` (crosshead, dog-bone stretching / necking / tearing, or a column squashing,
  bowing, or cracking along a diagonal; `charred`). Assumptions: DATA-SOURCES § Sample curves.
- UI (`structures-playground/src/screens/builder/{lab,fire}.rs`, `FoldIn`s): `lab::Lab`
  (`open_sample` → the drawer over the aside: `drawer`, `escape` / Close
  (`lab::CLOSE` "Close the sample drawer"), `tick` paces the pull with `ctx.wake_in`, `busy`,
  `describe`; `is_open()`, `sample()` / `sample_mut()` for MCP; the rig and plot through
  `widgets::tensile`), `fire::Fire` (`conditions`: the Fire row — chips named `fire::CHIP_NAMES`
  "Fire off" / "Small fire" / "Big fire" / "Inferno", "Light it now", the note; `hud` "Fire 650
  °C"; `paint_stage`: the bed, flames, reach dome and the heat glow on hot pieces; `describe`).
  Colours `sp_theme::fire`.
- Tests: `sp-engine/tests/fire_factors.rs` (table points exact, monotonic, 0…1),
  `fire_heat.rs` (only pieces in reach heat, lagging; steel recovers / wood stays charred;
  timing and "Light it now"; a steel A-frame over a Big fire softens and breaks,
  deterministically), `sample_curves.rs` (the straight line ends at a real `Sim`'s capacity for
  every type, length and temperature; shapes; heat), `sample_bench.rs` (status sequence, the
  limit step, controls, pacing, readouts, determinism); GUI
  `structures-playground/tests/builder_lab.rs` (card → drawer → Pull to breaking, Push, sliders,
  Esc / Close, idle pacing; the Fire row, HUD chip and glow in a run).

#### The builder screen (agent `screen`)

`crates/structures-playground/src/screens/builder.rs` + `builder/**`; scene colours in
`sp_theme::builder` (each with its CSS source). The screen draws `Game::view()` every frame and
owns nothing but pixels: every number and sentence comes from the view (DONE-2).

- **`BuilderScreen { pub game: Game, pub fold: FoldIns, .. }`** (MCP and tests reach the
  engine through `app.screens_mut().builder.game`; a change shows the next frame). Each frame
  `ui` builds the view, lets the fold-ins `adjust_view` it, collects the HUD chips, lays out the
  row on `layout::columns` (`Tools` rail · `Stage` · `Level, site and inspector` aside; stacked
  at 390), collects clicks as `builder::Action`s and applies them after drawing. Header:
  `document` = `level.slug()`, `drive_button` = a fold-in's or `Drive::new(view.hdr_run)`
  ("Run traffic" / "Run the test", "Pause" while running), `drive` = a fold-in's `drive` or
  `toggle_run`, Reset = `clear`, `clock` = `view.clock`, Share = `game.share_text()`; Carry in
  and ⌘V = `game.paste_netlist` (toast "Loaded a 10-joint structure into the Sandbox");
  "Open in Advanced" = `Carry::netlist(share_text, Some(slug), Route::Playground)`. Commands:
  Undo, Delete, ZoomIn / ZoomOut / ZoomFit, `Pan` (40 stage px per press), `Nudge`, `Escape`
  (fold-ins first, then ends the chain, then clears the selection). Best stars persist in egui
  memory (`BEST_STARS_KEY`, `Vec<(level key, stars)>`), restored on `enter`.
- **Frame pacing** (PLAT-3, BLD-15): `tick` calls `game.advance(dt)` (one wake-up for a fading
  toast), the fold-ins' `tick`, and while `game.animating()` one `game.tick()` per 1/30 s
  (`Ticker`) with `ctx.wake_in(due)`; the water / wind dashes, snowfall and the quake shake move
  only while running (decision: the mockup's CSS snowfall also falls while building; we keep
  0 % idle). Fold-ins that animate say `busy()`.
- **Modules**: `rail` (Select, Build, Found, Crate, Erase, Undo, Clear; the Found icon is drawn
  here, the rest are shared `Icon`s), `hint` (the hint bar as CSS flex-wrap: the tool's chip
  group shrinks onto its own line, the hint takes the free space, all hit targets tiled),
  `stage` (the grid surface button "Building grid": click → `click_surface`, drag →
  `surface_down` / `pointer_move` / `pointer_up` pan, wheel → one ×1.15 step per wheel event
  about the pointer, pinch → `zoom_by`; joints (`click_and_drag`) and handles as AccessKit
  buttons with the engine's labels; crate tags; HUD (right-aligned wrapping lines, 60 % wide);
  toast; run overlay; result banner with painted stars (MB-2) and "Replay in slow motion" /
  "Back to building"; the stage's `role=img` summary; the quake shake offsets the drawing),
  `scene` (the SVG layer in the mockup's paint order from the view's stage-px shapes), `paint`
  (`Xf`: stage px / world m → points; `height_for` = CSS `aspect-ratio` on the content box
  inside the 1 px border; dashes that restart per subpath, Béziers, an ear-clipping fill for
  the river's concave soil outline), `legend` (legend, zoom group, the PROV-1 line as the group
  "Where the numbers come from"), `aside` (levels, goal panel, result slot, Sandbox scenes,
  Conditions, inspector / Materials cards, Open in Advanced, What's happening), `bits` (chips
  with swatch bars / dots, chip rows, pills, level rows, tool buttons, bars, framed / section /
  paragraph blocks).
- **Fold-in hooks** (`builder/host.rs`): one trait, every method defaulted, called in the order
  comfort, time, lab, fire — `tick(&mut self, &mut Game, &mut Ctx)` (before drawing;
  `wake_in` while moving), `adjust_view(&self, &Game, &mut View)` (TIME-1), `hud(&self, &Game,
  &mut Vec<HudChip>)` (after the core's chips; `HudChip { key, value, color }`),
  `paint_stage(&self, &Game, &View, &Painter, &Xf)` (over the structure, under joints and
  overlays), `conditions(&mut self, &mut Ui, &mut Game, &mut Ctx)` (end of Conditions: Fire),
  `results(..)` (the aside slot under the goal panel), `open_sample(&mut self, PieceType) ->
  bool` (a Materials card, LAB-1), `drawer(..) -> bool` (replaces the aside while open),
  `escape(&mut self) -> bool`, `drive(..) -> bool` / `drive_button(&self, &Game) ->
  Option<Drive>` (TIME-3 takes the run), `reset(&mut self, &Game)` (new level / scene, back to
  building, Reset, Clear, a netlist loaded — the screen detects these itself), `busy(&self)`,
  `describe(&self, &Game) -> Option<String>`. `FoldIns { comfort, time, lab, fire }`
  (`each()` / `each_ref()`); helpers for fold-ins: `host::card`, `bits::{chip_row, chip_rows,
  ChipSpec, framed, section, para, eyebrow, label_node, status_pill, bar}`.
- **Tests** (`tests/builder.rs`, `tests/builder_look.rs`; `builder_time.rs` / `builder_lab.rs`
  are the fold-in owners'): level 1 built by clicks → all across with stars and the level row's
  best; placement toasts; inspectors; palette / foundations / crates / Sandbox scenes; zoom /
  pan / fit by buttons, ⌘ keys, the wheel and a drag; conditions while running, Shake now,
  refusals, slow motion; Share / ⌘V / hand-offs; ⌘Z / Delete / Esc / ⌥-arrow nudge / Reset;
  frame pacing (idle asks for no frame, a run ~30 fps); closeness per region vs
  `BridgeBuild-1440.png` and `TowerBuild-1440.png` (boards rebuilt with `Preset::board`, MB-10);
  1440 / 390 without overlapping controls. `SP_BUILDER_SHOTS=1` writes `[app | mockup]` images
  of every board (and the running ones 16 s in) to `target/compare/`.

### `bridge` (v1, retiring in v2 wave 2; agent `bridge`)

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
  2 m steps, drag rules), `starter()` / `can_starter()`, `undo()` (40 steps; only edits that change the design leave
  a step: a refused edit or a drag that didn't move leaves none),
  `open_in_sandbox(design, material)`, `set_best` (the screen restores saved best stars). Test: `start_or_stop()` (refuses with
  "Connect the loaded joint…"), `back_to_build()`, `replay()` (slow motion), `tick()` (one 33 ms
  tick: λ += 1/55, ≤ 30 breaks, loose-load / mechanism collapse, hold at λ = 1 with stars and
  best — at λ = 1 only once the cascade has settled within the 30-break cap; or 3 collapse
  steps), `advance(dt)` (toasts fade after 2.6 s), `run_test(max_ticks)`
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
  (first-hung order), n_n, n_m }`; `Design::for_level`, `starter` (18 members: the mockup's 16
  plus both diagonals of the middle panel, SPEC MB-5, so it holds), lookups, `set_load`, `to_truss(mat, λ)`,
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
- Numbers print with the shared `sp_engine::jsnum`; `view::stars(n)` is the star row.
- `truss::hypot` (added for the Bridge, used for every member length): V8's `Math.hypot`, so
  symmetric members tie-break like the mockup.
- **Screen** (`structures-playground/src/screens/bridge.rs` + `bridge/{stage,rail,aside,bits}.rs`,
  scene colours in `sp_theme::bridge`): draws `Game::view()` each frame, collects clicks /
  drags as `Action`s and applies them after drawing; `BridgeScreen { pub game: Game }` (MCP:
  `bridge_test` = `game.run_test(n)`, `bridge_load_netlist` = `bridge::netlist::read` +
  `game.open_in_sandbox`, `describe()` = `game.summary()`); `stage::Xf` maps grid metres to
  points. Best stars per level live in egui's persisted memory under
  `screens::bridge::BEST_STARS_KEY` (`[u8; 4]`, saved by eframe with the window, restored on
  `enter`); level rows report them as their AccessKit description ("Best: 1 of 3 stars").
  The Test paces frames at its 30 ticks/s (`wake_in`). Header: drive "Test" / "Pause" → `start_or_stop`, Reset → `clear`, clock, document =
  level slug, Share = `bridge::netlist::write`. `Command::{Undo, Delete, Paste}`; arrow keys
  nudge; `Carry::Netlist` in (Sandbox) and out ("Open in Advanced").
- Tests: `tests/bridge_goldens.rs` replays `tests/bridge_goldens.json` (10 scenarios, 71
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
    values); `Err` holds every error and warning sorted by line. Caps: `MAX_NODES` 500,
    `MAX_MEMBERS` 2000, `MAX_PLOT_POINTS` 400 (a truss over 120 nodes sweeps its influence line
    in at most 24 steps).
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
  - Number text comes from the shared `sp_engine::jsnum` (`to_fixed`, `minus_fixed`,
    `js_round`, `js_number`, `tidy`): JavaScript's `toFixed` (ties up), the mockup's `fmt`
    (`−1004`), `Math.round`, `String(number)`.
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
  `force_kn(&report, m)`, `sweep(&net)`. Up to `EXACT_SWEEP_NODES` (40) the line is solved in
  full per position (`influence_line_exact`, mockup-exact); above, by superposition:
  `unit_sweep(&net) -> UnitSweep { xs, base, unit, stable, .. }` (one static + one unit-truck
  solve per position, every member) and `line_from_sweep(&net, &sweep, member)` for any truck
  weight. `solve_ordered(&truss, &loads, &opts)` solves a > 40-node truss with its joints
  numbered along its long side (banded K), results in the truss's order. `plot_member(&net)`:
  the member a `.plot N(M3)` names (first in `picks`). `Status::Loose { node }`: a load on a
  node no member reaches. The weight check is "<Material> weight"
  (`generate::material_display_name`, SPEC MB-8).
- **The Advanced screen** (`screens/advanced.rs` + `screens/advanced/`, agent `advanced`):
  `AdvancedScreen { doc: Doc, playing }`; `doc::Doc` is UI-free and holds the netlist text (the
  source of truth), `generated: Option<Params>` (while the text is exactly the generator's, its
  bit-exact model is used), `origin` (Generated / Received(route) / File / Pasted / Edited),
  `good: Option<Good { netlist, report }>` (the last good model), `diagnostics`, `stale`
  (errors on screen: results dimmed), `pick` (IL), an undo stack. Mutators: `apply_generator`,
  `set_truck` (Truck / x / drive: edits the `.load … lane=` line in place), `edit` (debounced
  150 ms, `tick(now)`), `load(text, name, origin)` (hand-offs, files, ⌘V), `paste_material`
  (a lone `.material` line swaps the material), `undo`, `reset`, `drive_step`. For MCP (wave
  2): `app.screens().advanced.doc.load(..)` + `.report()` / `.influence_line()`;
  `describe()` = one line with file, size, status and x. Header: Drive across / Pause,
  Reset, `x = 24.0 m`, Share = the netlist text; Carry::Netlist in, "Open in Playground" →
  `Carry::netlist(text, None, Route::Advanced)` to the Bridge.
- Tests: `tests/advanced_goldens.rs` against `tests/golden/advanced.json`, written by
  `node scripts/mockup-goldens.mjs` (the mockup's `Component` run in Node; 84 toolbar states);
  `tests/netlist_*.rs` (parser errors, round trip, fuzz), `tests/advanced_perf.rs` (ENG-4).

### `tower`, `dynamics`, `ground` (agent `worlds`)

Both worlds reproduce their mockup's `calc()` operation for operation, and also own the copy
the screens print (tiles, statuses, sentences, goals), so every on-screen string comes from
the engine (DONE-2). Golden data: `tests/tower_ground_goldens.mjs` runs the mockups' own
`class Component` in Node and writes `tests/{tower,ground}_goldens.json` (110 + 130 states;
`tower_goldens.rs` / `ground_goldens.rs` match every number and string exactly). Inputs are
clamped (`sanitized()`); nothing panics. Colour roles are `tower::Tone` (`Plain`, `Ok`,
`Marginal`, `Caution`, `Alert`, `Danger`; `css()` names the mockup colour) — screens map them
to `sp_theme`.

- Number text goes through the shared `sp_engine::jsnum` (`fixed`, `js_round`, `round_str`,
  `num`: JavaScript's `toFixed` / `Math.round` / `String(n)` exactly).
- **`tower`**: `System` (`Core`, `Tube`, `Outriggers`, `Braced`) / `Shape` (`Straight`,
  `Tapered`, `Twisted`, `Hollow`) with `def()` → `SystemDef { key, name, k, cost, icon }` /
  `ShapeDef { key, name, cd, cross, mass, cost, icon }` (icons: the mockup's SVG paths, 40 ×
  40), `from_key`. `TowerInput { floors 5–120, system, shape, v10 5–50, damper }` (serde;
  `Default` = the mockup's start). `calc(&input) -> TowerResult { height, base_shear,
  overturning, delta, f1, zeta, mg, v_top, fs, ratio, lock, near, cost, dyn_amp }` (+
  `period()`, `drift_ratio()`, `sway_ok()`, `comfort_ok()`, `within_budget()`). Readouts:
  `tiles(&r) -> [Tile { key, value, note, tone }; 6]`, `Comfort::of(mg)` (`sentence()`,
  `tone()`), `comfort_pos`, `Status::of(&r)` (`label()`, `tone()`), `explain`, `compare(&input)
  -> [ShapeBar { shape, mg, lock, width }; 4]` (`label()`, `tone()`), `goals(&input, &r) ->
  [Goal { text, done }; 4]` (the drift goal is `DRIFT_GOAL` = H/350, SPEC MB-6; the tile and
  the status keep `DRIFT_AIM` = H/500; `CHALLENGE_DESIGN` — 100 floors, braced tapered tube,
  damper, 30 m/s — meets all four goals), labels `accel_label`, `sway_chip`, `v_top_label`, `v10_label`,
  `clock`, `floors_note`, `stage_label`, `share_line` (NAV-6 `.story … .wind …` line).
  Stage helpers: `mode_shape(z/H)`, `floor_plan(shape, i, n)` (taper / twist factors),
  `is_sky_slot`, `is_outrigger_floor`, `arrow_heights(H)`, `arrow_len(v10, z)`. Play gusts:
  `sway_params(&input, &r, seed) -> dynamics::SwayParams` (modal mass `μH·33/140`, 2 % Den
  Hartog damper, mean lean `0.85·δ`, turbulence 2 % and vortex force 1.5 % (locked onto f1
  inside the lock-in band), calibrated so the default tower's rms swing is ≈ `dyn`/3). Constants (`FLOOR_H`, `EI_CORE`, `ZETA`, `COMFORT_BANDS`, `BUDGET`,
  `CHALLENGE_RUNG`, …) are public and listed in `docs/DATA-SOURCES.md`.
- **`dynamics`**: `rk4_step` / `rk4_step2` (fixed-step RK4, const-generic `[f64; N]`, no
  allocation); `Linear<N> { m, c, k }` (`sdof(m, f, ζ)`, `with_absorber(m, f, ζ, μ, fd, ζd)`,
  `accel`, `statics`), `den_hartog(f, μ)`, `invert`; `Newmark<N>` (average acceleration:
  implicit, unconditionally stable, energy-conserving undamped; `new(sys, h, x0, v0, f0) ->
  Result<_, DynamicsError>`, `step(f_next)`, public `x`, `v`, `a`, `t`); `SplitMix64`,
  `Gusts::new(seed, tau, dt).at(t)` (seeded Ornstein–Uhlenbeck series, unit variance);
  `SwaySim::new(SwayParams)` — `advance(dt)` (48 steps per period, Newmark), `retune(params)`
  (keeps the swing), `top()`, `swing()`, `velocity()`, `accel()`, `damper_stroke()`,
  `has_damper()`, `time()`; `playback_speed(f1)` (the default tower sways at the mockup's 0.55
  Hz on screen; taller slower, 0.25–1.2 Hz). Tests (`tests/dynamics.rs`): RK4 order 4, free
  decay frequency and log decrement, statics, resonance `F₀/(2ζk)`, the absorber notch, Den
  Hartog's ≈ 3× peak cut, Newmark stability where RK4 diverges, gust statistics, TMD calms the
  gust response, lock-in swings harder, determinism, garbage in → finite out, per-frame cost.
- **`ground`**: `Soil` (`Dense`, `Stiff`, `Soft`, `Loose`) → `SoilDef { key, name, qa, ks, vr,
  fric, liquefiable, tau }`, `wet()`; `Foundation` (`Footing`, `Mat`, `Piles`, `Caisson`) →
  `FoundationDef { key, name, icon }` (44 × 34 icons under `ICON_GROUND_LINE`), `deep()`.
  Soil swatch colours are `sp_theme::ground::SOILS[soil as usize]`. `GroundInput { soil,
  foundation, floors 2–80, pile_len 8–40, quake, years 0–30, pisa_tried }` (serde).
  `calc(&input) -> GroundResult { load, pressure, capacity, fs, deep, to_rock, liquefied,
  settle_final, tilt_final, settle, tilt, tau }` (+ `target()`, `building_load_mn()`).
  `Status::of(&r)` (Solid / Marginal / Leaning / Bearing failure / Liquefaction), `explain`,
  `tiles(&input, &r)`, `fs_text`, `fs_tone`, `fs_pos`, `stage_chip`, `year_label`, `clock`,
  `pile_label`, `stage_label`, `goals(&input) -> [Goal; 3]` (the third looks 30 years ahead),
  `CHALLENGE_TEXT`, `SCENARIOS: [Scenario; 4]` (`apply(&input)`), Play years `play_tick(years)
  -> Option<f64>` / `play_start(years)`, `share_line` (`.soil … .foundation …`), and the NAV-4
  hand-off `FromTower { floors, overturning }` (`ground_floors()` clamps to 80,
  `edge_pressure()` = `M/(B³/6)` kPa, `note()`).

Screens (`src/screens/{tower,ground}.rs` + `stage.rs` each): `TowerScreen { input, ex,
playing, sim, flow }` / `GroundScreen { input, playing, from_tower, anim }`, both with
`result()` and `set_input()` (what MCP `set_tower` / `set_ground` should call). Tower Play
gusts: `tick` advances the `SwaySim` by `dt × playback_speed(f1)` and asks for ~30 fps
(`ctx.wake_in(ANIM_FRAME)`, PLAT-3);
Pause freezes the sway; changing the tower while playing retunes it, while paused drops it
(the tower rests at its lean). Ground Play years: 0.25 yr per 60 ms (`Ticker`, `wake_in`;
~30 fps while shaking or the water table flows). The stage's soil name and "bedrock" sit on
`sp_theme::ground::LABEL_CHIP` (4.5:1 on every soil, SPEC MB-7). Hand-offs: Tower → `Carry::Building`;
Ground shows `FromTower::note()` until its floors change and sends `Carry::Tower` back with
the tower's own floors while the building is unchanged. Widgets only these worlds need are in
`src/widgets/world.rs` (picture tiles with SVG icons or soil swatches, the floors stepper,
value rows, the `.lbl` readout grid, band bars, comparison rows, goal rows, the challenge
card, wrapping scenario buttons, the stage info box, title + status pill) — reusable by any
world.

### `materials` (agent `materials`)

The Materials mockup's model, number for number (`Materials.dc.html` `defs()`, `tempFactor()`,
`stressAt()`, `tick()`, `renderVals()`). Strain in %, stress in MPa.

- `materials::curves` — `Kind::{Steel, Aluminum, Concrete, Timber, CarbonFiber}` (`ALL`,
  `key()` `steel`…`carbon-fiber`, `mockup_key()`, `parse(s)`, `name()`, `curve()`);
  `CURVES: [Curve; 5]` (`points: [[strain %, MPa]]`, `e_gpa`, `rho`, `ultimate_strain`,
  `yield_mpa: Option`, `note`) with `stress_at(e, T) -> Stress { mpa, broken, break_strain }`
  (fracture past the last corner; NaN / negative strain = 0, NaN temperature = 20 °C),
  `break_strain`, `elastic_limit`, `has_plastic_range`, `is_ductile`, `necks`, `necking(e)`
  (0 → 0.55), `peak_mpa(T)`, `yield_at(T)`, `toughness()` (room temperature) /
  `toughness_at(T)` (heat-scaled: the tile, MB-8), `specific_strength(T)` (kN·m/kg). Free functions
  `temp_factor(kind, T)`, `stress_at(kind, e, T)`, `load_cell_kn(MPa)` (50 mm² section).
  Constants `TEMP_MIN_C` −40, `TEMP_MAX_C` 600, `TEMP_STEP_C` 10, `HOT_C` 200, `SECTION_MM2`.
- `materials::bench` — `Bench` (the screen's whole state; serde): `kind`, `strain`, `temp`,
  `zoom`, `playing`, `broke(kind)`, `broke_count`, `all_broken`, `now()`, `force_kn`;
  actions `pick`, `set_temp` (clamped), `set_zoom` (clips the stretch), `set_strain` (pauses),
  `fresh_sample`, `toggle_drive` (restarts a broken / maxed sample), `reset`; `tick()` (one
  Pull step: 0.08 % on `Zoom::Wide`, 0.006 % on `Zoom::Close`; returns the kind that broke),
  `advance(dt)` (20 steps/s, `TICK_SECONDS` 0.05, capped), `next_tick_in()` (for
  `Ctx::wake_in`); `status() -> Status::{Ready, Elastic, Plastic, Necking, Fractured { at }, Microcracking,
  Cracking, Crushed { at }}` (the last three: concrete, `Curve::compressive`, MB-8; `is_broken()`)
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
- `materials::jsnum` — re-exports the shared `sp_engine::jsnum` (JavaScript-exact `to_fixed`,
  `to_string`, `round`, …; owner `materials`).

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
| `src/app.rs` | `StructuresApp`: the frame (paper ground → header → screen body → toasts; no world tab row in v2), shortcuts and the builder's keys, Share, navigation + hand-offs, persistence; `AppOptions` (`::default()` real, `::test()` / `::at(route)` deterministic); `AppEvent` log |
| `src/ctx.rs` | `Ctx` (per-frame screen context), `Carry`, `Command`, `Ticker`, frame pacing |
| `src/router.rs` | `Route` (`Playground` — the builder —, `Advanced`, `Series`; v1's `Bridge` / `Tower` / `Ground` / `Materials` deserialize as `Playground`), `Mode` (`route()`); per route: title, default document, mockup + reference PNG (under `docs/design/`), board size, chrome / panel height, ⌘-digit, id; `RETIRED_IDS` |
| `src/screens/mod.rs` | the `Screen` trait, `Drive`, `Share`, `Badge`, `Provenance`, `Screens` (`builder`, `advanced`, `series`, all kept alive) |
| `src/screens/builder.rs` + `builder/**` | SCR-1, **owned by the `screen` agent** (fold-in UI files by `time` / `lab`); `BuilderScreen { pub game: sp_engine::builder::Game, .. }` |
| `src/screens/advanced.rs` + `advanced/**` | SCR-2 (v1 agent `advanced`; shell-owned in v2) |
| `src/screens/series.rs` | SCR-6 (shell) |
| `src/layout.rs` | `Column` + `columns` (the mockup's wrapping flex rows), `stage`, `graph_panel` |
| `src/widgets/*` | the widget kit (below); `header.rs` is the chrome |
| `src/launcher.rs` | the lab-series apps (`AppId`) and the injectable launcher (Series cards) |
| `src/platform.rs` | `.str` files from Finder / the command line → Advanced (`Carry::Netlist`) |
| `src/smoke.rs` | `--smoke <s> [--headless]` (PLAT-1) |
| `tests/common/mod.rs` | the egui_kittest harness |

### How a screen plugs in

Each screen is `pub struct XxxScreen` with `pub fn new() -> Self` and `impl Screen`. The app
owns one of each in `Screens` (`app.screens().builder`, `.advanced`, `.series`) for the whole
run, so state survives navigation. Only `ui` is required; everything else has a default.

```rust
impl Screen for BuilderScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) { /* the body under the header */ }
    fn document(&self) -> Option<String> { Some(self.game.level().slug()) } // header doc name; None = Route::document()
    fn drive_button(&self) -> Option<Drive> {                       // the .prim drive button
        Some(Drive::new("Run traffic").running(self.game.live()))   // "Pause" + pause icon while running
    }
    fn drive(&mut self, ctx: &mut Ctx) { self.game.toggle_run(); }  // click or Space
    fn has_reset(&self) -> bool { true }                            // show Reset (default true)
    fn reset(&mut self, ctx: &mut Ctx) { self.game.clear(); }
    fn clock(&self) -> String { "building".into() }                 // mono clock; "" = none
    fn share(&mut self, ctx: &mut Ctx) -> Option<Share> {           // NAV-6; None = "Nothing to share yet"
        Some(Share::new(self.game.share_text()))
    }
    fn tick(&mut self, ctx: &mut Ctx) { /* advance by ctx.dt(); ctx.animate() / ctx.wake_in(s) while moving */ }
    fn command(&mut self, cmd: Command, ctx: &mut Ctx) -> bool { false } // ⌘Z, Delete, ⌘V, zoom, pan, nudge, Esc
    fn enter(&mut self, ctx: &mut Ctx) {}                           // route became active (may repeat)
    fn receive(&mut self, carry: Carry, ctx: &mut Ctx) {}           // NAV-4 hand-off, right after enter
    fn describe(&self) -> String { String::new() }                  // one-line summary (MCP describe)
}
```

Per frame, for the active route: `enter` (first frame on the route) → `receive` (if a hand-off
is pending) → shortcuts / `command`s / Space → `drive` → `tick` (once per frame, not again in a
discarded layout pass) → the header reads `document`, `drive_button`, `has_reset`, `clock` and
calls `drive` / `reset` / `share` on a click → `ui`.

- **Header (NAV-1)** — drawn by the app (`widgets/header.rs`): logo mark + "Structures
  Playground" (an AccessKit `Link` → Series), `/`, the document name (a `Label`), the mode
  switcher (`Navigation` "Mode": `Link`s Playground → the builder, Advanced → Advanced; the
  current one `selected`), then the drive button (`Button` named "Run traffic" / "Pause" …),
  Reset, the clock (`Label`), Share. Not on Series (it has its own Back).
- **No world tab row (NAV-2, v2)** — screens draw their own provenance (PROV-1: the builder's
  line under its legend, Advanced's results badges). Badges: `Badge::exact / simplified /
  illustrative(text)`, `Badge::dot(text, colour)`, `Badge::plain(text)`, `Provenance::color()`;
  draw one anywhere with `widgets::badge(ui, &b)` (or `badge_with(ui, &b, BadgeLook::DANGER)`).
- **Share (NAV-6)** — the app copies `Share::text` to the clipboard (`Ctx::copy`, readable in
  tests as `app.last_copied()`) and toasts `Share::toast` (default "Copied <document> to the
  clipboard").
- **Keys (NAV-5)** — the app handles ⌘1 (Playground), ⌘2 (Advanced), Esc on Series (Back), and
  Space with nothing focused (a focused button takes Space itself; a slider lets it through:
  `widgets::space_passes_through`) → `drive` (only if `drive_button()` is enabled). With no
  text field focused it sends `Command::Undo` (⌘Z, not ⌘⇧Z), `Command::Delete` (Delete /
  Backspace) and `Command::Paste(text)` (⌘V; the paste event is consumed). **On the Playground
  only** (BLD-2, NAV-5) also `Command::ZoomIn` (⌘= / ⌘+; instead of egui's whole-window zoom),
  `ZoomOut` (⌘−), `ZoomFit` (⌘0), `Escape` (Esc), and the arrows as `Pan { dx, dy }` (⌥:
  `Nudge { dx, dy }`; `dy` up) while nothing has keyboard focus or the focused widget called
  `widgets::arrows_pass_through(ui, id)` this frame (it also locks egui's arrow-key focus
  moves for it); otherwise the arrows move focus, egui's way. Tab order is egui's: widgets in
  the order they are drawn (reading order).

### `Ctx` (per frame)

`route()`, `previous()`; `go(route)`, `go_with(route, carry)`, `back()` (to `previous`, else
the Playground); `pending()`; `toast(text)`; `copy(text)`; `save_text(name,
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
}
Carry::netlist(text, Some("river-level"), Route::Playground)   // carry.from()
```

| from → to | button | call | the target does |
|---|---|---|---|
| builder → Advanced | "Open in Advanced" | `ctx.go_with(Route::Advanced, Carry::netlist(game.share_text(), Some(slug), Route::Playground))` | parses + solves it (ADV-9); `name` → document; badge "from the Playground" |
| Advanced → builder | "Open in Playground" | `ctx.go_with(Route::Playground, Carry::netlist(text, None, Route::Advanced))` | `game.paste_netlist(text)`: into the Sandbox if it fits the grid |
| Finder / CLI `.str` → Advanced | — | the app (`platform.rs`) | as builder → Advanced, `from: None` |

(v1's `Carry::Building` / `Carry::Tower` went with the Tower and Ground screens.)

The carry is delivered at the start of the next frame, after the target's `enter`; if the
user navigates on before that, it still reaches its target. Delivered carries are logged
(`AppEvent::Carried`).

### Widget kit (`src/widgets`, colours from `sp_theme`, every control an AccessKit node)

| widget | CSS | use |
|---|---|---|
| `prim(label)`, `ghost(label)` → `Button` | `.prim`, `.ghost` 36 px; `.big()` 44 px | `.icon(Icon::Play)`, `.enabled(b)`, `.pressed(b)` (toggle look + `aria-pressed`), `.full_width()`, `.centered()`, `.height(h)`, `.accessible_name(s)`, `.link()`; `.show(ui)` / `.show_at(ui, rect)` |
| `SegGroup::new(name, &items, selected)` + `SegItem::new(label).icon(..)` | `.seg` in a well (`.style(SegStyle::Header)`), compact `.mini` | `.fill()` (equal `flex: 1` items, centred), `.wrap()` (`flex-wrap: wrap`), `.links()`, `.navigation()`; `.show(ui).clicked: Option<usize>`; items report pressed |
| `Select::new(name, &options, selected)` | `<select>` (36 px, chevron) + a menu | `.width(w)`; `.show(ui, &mut open).picked`; an AccessKit `ComboBox` (value = the option), Enter / Space open, ↑ / ↓ step |
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
| `Icon::*` | every mockup icon (play, pause, restart, truss, chart, back, tools, logo) | `icon.paint(painter, rect, colour)` |
| `tensile::{rig, plot}` | the v1 Materials tensile rig and stress–strain plot (`0 0 420 560` / `0 0 560 400` viewBoxes, fitted "meet", on a `.graph` panel) | generic scenes: `RigScene::dogbone(stretch, necking, broken, necked_break, fill)` + `.readout(..)` / `.badge(..)` / `.label(..)`; `PlotScene::axes(x_max, &x_ticks, y_max, &y_ticks)` + `curve` / `done` / `marker` / `ghosts` / `labels`, `PlotScene::point(strain, stress, ..)`; `from_bench` adapters (the builder's sample drawer, LAB-1) |
| `world::*` | the v1 Tower / Ground column widgets: `band_bar`, `compare_row`, `readout_grid`, `goal_row`, `status_pill`, `title_block`, `picture_tiles`, `stepper`, `value_row`, `challenge`, `stage_box`, `paint_path` | reused by the builder's fold-ins (CMF-1 meter, CMF-2 rows) |
| `Toasts` | app toasts (2.6 s, bottom centre) | `ctx.toast(text)` |
| `text::{galley, wrapped, text_top, lbl_galley, …}` | text helpers | |
| `widgets::hit::{target, tile}`, `interact`, `set_bounds` | 44 px hit areas, AccessKit plumbing | |

`widgets::gallery::Gallery` draws all of them (the widget tests). Screen agents may add
widget files for their own screen (one `pub mod` line in `widgets/mod.rs`); changes to the
shared widgets go to `shell` via PROGRESS Requests.

### Layout helpers (`src/layout.rs`)

`Column` is one flex column of a body row: `Column::rail("Tools")` (76 px, the builder's rail),
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
`hand_off(&mut h, r, carry)`, `events(&h)` / `carried(&h)` / `commands(&h)` (`AppEvent`s),
`last_copied(&h)`, `document(&h)`, `drive_label(&h)`, `toasts(&h)`, `step_until(&mut h, n, |h|
..)`, `h.state().screens().builder.game` (the screen's own state). Acting by AccessKit label:
`click(&mut h, label)`, `click_button`, `click_link`, `click_role(.., Role)`,
`click_drive(&mut h)`, `click_header(&mut h, "Reset")`, `header_button(&h, label)`,
`type_into(&mut h, label, text)`, `press(&mut h, Key)`, `press_cmd(&mut h, Key)`, `press_alt`,
`reveal(..)` (scrolls into view). Reading: `has_label`, `has_label_containing`, `get(&h, label,
Role)`, `pressed(&h, label)`, `selected(&h, label, Role)`, `disabled(&h, label)`,
`focused_label(&h)`, `nodes_with_roles(&h, &[Role])`, `child_labels(..)`, `text_of(&node)`.
Rendering: `render(&mut h)`, `screenshot(&mut h, name)` (into the test's tmp dir),
`render_full(route, w, min_h)`, `mockup_png(route, 1440)` (`Route::reference`: the builder's
BridgeBuild board for the Playground), `load`, `mean_diff(a, b, region)`.

`AppOptions::test()` is deterministic: seed 7, no autoplay, no persistence, no save dialog, a
recording launcher with nothing installed, a fresh temp output folder. `app.events()` logs
`AppEvent::{Drive, Reset, Shared, Carried, Command}` (the drive / Reset / Share presses, the
hand-offs delivered and the keyboard commands with whether the screen handled them).

Shell test files: `tests/nav.rs` (every route by click and key, Back / Esc, Space, Reset,
Share, hand-offs, commands, the builder's keys on the Playground only, no tab row),
`tests/a11y.rs` (chrome names, 44 px targets, Tab order), `tests/widgets.rs` (the gallery,
with the fold-in painters), `tests/series.rs`, `tests/pacing.rs` (PLAT-3: **every route at
rest must ask for no frames** — a screen that animates while idle fails it),
`tests/compare.rs` (chrome closeness; `scripts/compare-mockup.sh` renders `[app | mockup]`
side by side into `target/compare/` for any screen: `scripts/compare-mockup.sh playground`),
`tests/journeys.rs` (DONE-4), `tests/mcp_bridge.rs`, `tests/advanced*.rs`.

**Perf tests (ENG-4).** `scripts/test-all.sh` runs every test target named `perf` or `*_perf`
in release with `--include-ignored --nocapture`; it must pass, and each printed line
`PERF <label>: <value> ms (limit <limit> ms)` is listed as a gate in the summary.

### Persistence (NAV-7), platform, smoke

eframe keeps the window size (`persist_window`) and `Persisted { route }` (Series restores to
the Playground; a v1 save naming `Bridge` / `Tower` / `Ground` / `Materials` restores to the
Playground, its `last_world` ignored). A fresh start is reproducible: the seed is fixed
(`AppOptions::DEFAULT_SEED`). `structures-playground --smoke <seconds> [--headless]` opens
the Playground on level 1 with a wood truss (`smoke::SMOKE_BUILD`), presses "Run traffic",
runs and prints `SMOKE route=playground mode=… first_frame_ms=… frames=… run=run
frame_ms_avg=… frame_ms_max=… status=ok` (`run=` the builder's phase at the end). `STRUCTURES_BUNDLE=1` at build time
(`scripts/bundle-macos.sh`) leaves the checkout path out of the binary (`MANIFEST_DIR`).

## MCP (`crates/sp-shared`, `crates/sp-mcp`, the app's `control.rs`) — agent `shell`

Forked from Automata Playground's `ap-shared` / `ap-mcp` / `control.rs`, on labkit's `lab-mcp`
(a synchronous stdio MCP server + the live bridge: a user-only Unix socket with
newline-delimited JSON). Setup and the tool list for users: `crates/sp-mcp/README.md`.

```
MCP client ──stdio──▶ structures-mcp (sp-mcp) ──Unix socket──▶ the app (control.rs) ──▶ BuilderScreen::game
                         └─ engine tools: sp-engine in-process     both build replies with sp-shared
```

- **`sp-shared`** (no egui): `APP_NAME`, `APP_ID`, `SOCKET_ENV` (`STRUCTURES_MCP_SOCKET`; empty or
  `off` disables), `bridge_socket_path()`, `HOW_TO_START`; `SCREENS` (`playground`,
  `advanced`, `series`) / `MODES` (a test in `tests/mcp_bridge.rs` keeps them equal to
  `Route`), `RETIRED_SCREENS` + `retired_note(id)` (why `tower` isn't a screen any more);
  `proto` — the bridge methods beyond the four every app answers (`status`, `navigate`,
  `screenshot`, `describe`): `builder.get`, `builder.level` (`BuilderLevel`),
  `builder.load_netlist` (`BuilderLoadNetlist`), `builder.conditions` (`BuilderConditions` =
  `ConditionsChange` + `show`), `builder.run` (`BuilderRun`), `builder.fast_forward`
  (`BuilderFastForward`), `sample.test` (`SampleTest`), `MAX_RUN_FRAMES`,
  `SANDBOX_RUN_SECONDS`; `builder` — on `sp_engine::builder::Game`: arguments refused rather
  than clamped (`level`, `scene`, `apply_conditions`, `years`, `sample_args`),
  `builder_state(&Game) -> BuilderState`, `run_report(&Game) -> RunReport` (the banner's title
  and sentence from the engine's view, stars, first failure, vehicles across, height / sway /
  comfort with the CMF-1 sentence), `run_to_end` (a level to its result; the Sandbox, which
  never ends, for 20 s), `fast_forward_report(&Game, years)` (TIME-1 headless; the Leaning
  tower's own result), `run_sample` / `sample_report` (LAB-1 on the lab's `Sample`),
  `place_design` (a netlist's structure built on a *level* with the editing tools, so the
  level's rules apply and refusals come back as its toasts), `test_structure` (the engine tool);
  `input` — `netlist` (line / column diagnostics), `material`, `temperature`, `strain`,
  `TowerChange` / `GroundChange` (the v1 engine tools); `report` — the payloads both sides
  send: `truss_report`, `influence_report`, `tower_report`, `ground_report`, `tensile_report`,
  `BuilderState`, `RunReport` (+ `summary()`), `SampleReport`.
- **`sp-mcp`** (binary `structures-mcp`, `--socket`, `--version`, `--help`): `engine.rs`
  (`solve_truss`, `influence_line`, `builder_test`, `tower_wind`, `ground_check`,
  `tensile_test`), `app.rs` (the lab-series tools: `app_status`, `list_screens`, `navigate`,
  `screenshot`, `describe_screen`; `AppLink::call` opens a fresh connection per call, so the
  server survives app restarts, and turns "nothing listening" into "Structures Playground isn't
  running — open …"), `screens.rs` (`builder_level`, `builder_load_netlist`,
  `builder_conditions`, `builder_run`, `builder_fast_forward`, `sample_test`: arguments
  forwarded as the method's params; `LEVELS`, `conditions_properties()` shared with
  `builder_test`). Tool arguments are validated against their JSON Schemas by `lab-mcp` before
  the handler runs. The v1 `bridge_load_netlist`, `bridge_test`, `set_tower`, `set_ground`,
  `set_material` are retired (unknown tools).
- **The app** (`control.rs`): `AppOptions::mcp_socket` (the real app: the default socket; tests
  and the smoke run: none; `--no-mcp`); `Control::start` never fails start-up (a second
  instance logs "MCP bridge off"). `control::poll` runs first thing in every `frame`;
  `control::logic` runs from `eframe::App::logic`, also while the window is hidden, and asks for
  a painted frame (`RequestPaintWhileHidden`) while requests wait. The builder methods act on
  `app.screens_mut().builder.game` through `sp_shared::builder` — after entering the never-shown
  Playground once (`StructuresApp::enter_screen`), so its first-visit set-up doesn't undo the
  change; `show` (default) switches to it; the screen draws `game.view()` next frame. Refused
  arguments change nothing. **Screenshots and descriptions are drawn offscreen**
  (`StructuresApp::render_offscreen`): the same `frame()` runs in a second egui context (fonts +
  style installed) at the window's logical size, pixels per point and time,
  `OFFSCREEN_PASSES` (3) times so measured layouts settle, with no input and the per-frame work
  skipped (enter, hand-offs, keys, `tick`, navigation, event log): nothing moves, clicks or
  takes focus. egui_kittest's `WgpuTestRenderer` (its own wgpu device, made on the first call)
  renders the last pass; its AccessKit tree is the description. No painted frame is needed, so
  a window covered by another app's (macOS gives it no frames) still answers; if no frame has
  polled the bridge for `control::STALL` (300 ms), `control::logic` (from `App::logic`) answers
  the queued requests itself. The PNG is encoded off the UI thread. (In the offscreen frame the
  page shows from the top and nothing is hovered.)
- **Tests**: `crates/sp-mcp/tests/e2e.rs` (the real binary for every tool: engine tools with no
  app — `builder_test` agrees with the engine run directly —, app tools against the real app in
  egui_kittest on a temp socket, the GUI showing the change; argument checks; not-running
  messages), `crates/structures-playground/tests/mcp_bridge.rs` (the bridge without the
  binary: builder methods in the background and in front, errors change nothing, hidden window,
  a client that leaves, a second instance, the shared screen table), `sp-shared` unit tests (a
  netlist from the builder's Share runs on its level exactly as the game that built it; years;
  samples; checks).
- **Bundle**: `scripts/bundle-macos.sh` builds `structures-mcp` too and puts it in
  `Contents/MacOS/`; `scripts/check-bundle.sh` checks it is there.
