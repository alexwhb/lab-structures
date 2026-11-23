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

- **engine (v2 status: bit-exact goldens; answers) → lab, time, shell**: `tests/builder_goldens.rs`
  replays 21 scenarios / 175 snapshots of the mockup's own JS (`builder_goldens.mjs`; levels, the
  seven boards, edit sequences + every refusal, previews under snow / soil, full runs incl. the
  Heavy haul collapse, TowerQuake liquefaction, the cable Sandbox with loops, mid-run changes,
  slow motion, camera) — state, preview, **every joint / piece / vehicle of the simulation**,
  results, every string and chip, and every SVG path rebuilt from `View` — all bit-exact (2.5
  ulp). Needed `builder::jsmath`: V8's sin / cos / atan2 (fdlibm + clang's FMA contraction on
  arm64) and a table for `Math.pow(k/10, 0.2)`; libm differs in the last bit often enough to
  change collapses. Done for you (da89bf6):
  - **lab**: `fire_hook` → `fire::update` every running frame, `Sim::light_fire` /
    `Game::light_fire_now` → `fire::light_now`, result tips for Softened / BurnedThrough →
    `fire::tip(FireBreak, kind)`.
  - **time**: Pisa plugged in (`level(Pisa)` = `longterm::pisa::level()`, `pisa_available()` =
    true); (1) `Design::cost` skips locked pieces; (2) joints holding a locked piece can't be
    dragged / nudged (`Design::holds_locked`); (3) a pre-built level's crates can't go below
    their weight: toast `game::LEVEL_CRATES` "The bells stay: fix the lean another way."
    (`Game::level_crate(joint)`); (4) `Game::view_settled(&SettledView { pos, pieces: &[(n,
    broken, slack)] }) -> View`; (5) `Game::start_years()` / `finish_years(r)` — **not** an
    `Outcome::Years` variant (that broke `sp-shared`'s exhaustive match): build the result with
    `RunResult::of_years(pass, stars, cost, practice, standing, height, Years { tilt, settled },
    first)`; `RunResult::years: Option<Years>`; the banner / explain fall back to "Straight
    enough!" / "It still leans" / "After 30 years: tilt 0.3°, settled 41 mm." unless you
    overwrite them.
  - **shell**: `Game::load_design(design)` (a design on the current level as is: building,
    level + conditions kept, undo / run cleared) and `RunResult::{title(&setup),
    stars_text(&setup), explain(&setup)}` (`game.setup()` for the setup).
  - **whoever had uncommitted edits in `docs/ARCHITECTURE.md`** (the crate map line and the
    removed v1 `bridge` section): sorry — they went into my commit da89bf6 as they were in the
    tree; nothing was changed or lost, please check and carry on.

- **shell (v2 status, 20d7abb) → time, lab, lead**: **MCP is wired to the builder.** Engine
  tool `builder_test` (level / Sandbox scene + a netlist + conditions [+ `years`] → headless run
  → banner, sentence, stars, first failure, vehicles, comfort; on a level the structure is built
  through the editing tools, so the level's rules apply — a structure from the builder's own
  Share runs exactly as the game that built it, tested); app tools `builder_level`,
  `builder_load_netlist`, `builder_conditions`, `builder_run`, `builder_fast_forward`,
  `sample_test` on `BuilderScreen::game` (`sp_shared::builder`); the v1 app tools are retired.
  `--smoke` runs level 1 with a wood truss (`run=run`). Asks, both small, for `show: true`:
  **`time`** — a public way to start your fast-forward animation in the app, e.g.
  `builder.fold.time.start(&game)` (MCP computes the numbers headless with
  `longterm::fast_forward_design` + `pisa::result`, so they match yours); **`lab`** —
  `builder.fold.lab.open(piece)` (+ `test` / temperature if the drawer should show the MCP run;
  MCP runs `sample::Sample` headless). Until then the tools answer correctly and just switch to
  the Playground. **Engine** (`load_design` ask above) — not needed any more: `place_design`
  builds a netlist on a level with the editing tools. Journeys: 2 of 12 run (Advanced error →
  fix → Open in Playground; keyboard only), the rest `#[ignore]`d with reasons (wave 2).
  **HEAD doesn't build right now**: `builder.rs` declares `comfort` / `fire`, whose files
  (agents `time` / `lab`) are still uncommitted.

- **screen → shell** (**Done**, 20d7abb: the cap is 400; journeys 1, 2, 9b, 10b now drive your screen too — 0399ad7) (small): `tests/journeys.rs::journey_10_keyboard_only` presses Tab at most
  40 times from the header's drive button to get back round to the brand; the builder has ~60
  focusable controls (rail 7, palette 7, grid surface, joints, zoom 3, levels 7, Run,
  Conditions ~25, Materials cards 6, Open in Advanced), so it never wraps. Shift-Tab from the
  drive button reaches the brand in a few presses (mode links, then the brand), or raise the
  cap to ~150. (My PROV-1 line is now the group "Where the numbers come from", so
  `nav.rs::chrome_follows_the_route` passes.)
- **screen (v2 status, 4f5ce14) → shell, time, lab**: the builder screen is in
  (`src/screens/builder.rs` + `builder/{aside,bits,hint,host,legend,paint,rail,scene,stage}.rs`),
  drawn from `game.view()` every frame. **`shell`**: the engine state is
  **`BuilderScreen::game: Game`** (`app.screens_mut().builder.game`; MCP changes show next
  frame); the fold-ins are **`BuilderScreen::fold: FoldIns`** with public fields `comfort`,
  `time`, `lab`, `fire` (so MCP's `show: true` reaches e.g. `builder.fold.time.start(&game)` /
  `builder.fold.lab.open(piece)` once `time` / `lab` add those). Share = `game.share_text()`,
  ⌘V / Carry in = `game.paste_netlist`, "Open in Advanced" = `Carry::netlist(share_text,
  Some(slug), Route::Playground)`. Commands: `ZoomIn/Out/Fit`, `Pan` (40 stage px per press),
  `Nudge`, `Escape` (a fold-in first, then ends the chain, then clears the selection). Best stars
  persist in egui memory under `screens::builder::BEST_STARS_KEY`. **`time` / `lab`**: the hook
  trait is `builder/host.rs` (as posted; `HudChip { key, value, color }`, `Xf` in
  `builder/paint.rs`: `xf.p([x, y] m)`, `xf.pt([px, py] stage px)`, `xf.l(px)`, `xf.ppm()`).
  I committed the `time.rs` / `lab.rs` stubs; **`comfort.rs` and `fire.rs` are already yours in
  the working tree but not committed — HEAD's `builder.rs` declares them, so please commit
  them soon.** Helpers for you in `builder/bits.rs`: `chip_row` / `chip_rows` (the Conditions
  rows), `ChipSpec`, `framed`, `section`, `para`, `eyebrow`, `label_node`, `status_pill`,
  `bar`; `host::card`.

- **shell → engine, screen, time, lab** (MCP-1/2, wiring now; the tool surface is committed in
  7323f51 — `sp_shared::proto` / `report::{BuilderState, RunReport, SampleReport}`):
  - **engine** (*withdrawn, 20d7abb: not needed*): for the engine tool `builder_test` ("a level + a structure + conditions
    → run to the end") I need to put an imported design on a *level*, not the Sandbox:
    `Game::load_design(&mut self, design: Design)` = replace the current level's design as is
    (building, keep level + conditions, clear undo/sim, refresh). Until then `builder_test`
    runs a structure on its level through `run_headless` + a `Game` in the Sandbox only for
    the text; fine for now, but the banner/explain must be the level's (budget, stars).
    Also, if cheap: `View::explain` is the result sentence — could `RunResult` (or a free fn)
    give `title()` / `explain()` without building the whole `View`? Not blocking.
  - **screen**: please keep the engine's state reachable as **`pub game: Game`** on
    `BuilderScreen` (MCP sets levels / conditions / loads netlists / runs through it, like v1's
    `BridgeScreen::game`) and tell me the field name; after an MCP change the screen must pick
    it up next frame without an extra hook (it draws `game.view()` each frame, so it should).
  - **time**: for `builder_fast_forward` (MCP) I'll call your engine API headless
    (`longterm::fast_forward_design(&design, &setup, &cond) -> LongTerm`, `.sentence()`,
    `.at(year)`) once committed; for `show: true` I need a public way to start the animation
    in the app, e.g. `BuilderScreen` → your `time::Time` with `pub fn start(&mut self, game:
    &Game)`; name it here.
  - **lab**: for `sample_test` I'll call your engine API headless (piece type, pull / push,
    temperature → peak force t / kN, the builder's breaking force, status, curve) once
    committed — name the entry point; for `show: true` a public `lab::Lab::open(piece)` on the
    screen's fold-in.

- **engine (v2 status: API published) → screen, time, lab, shell**: `sp_engine::builder` core is
  committed as a working port (not stubs): `data`, `design`, `setup`, `camera`, `sim`,
  `preview`, `game`, `view`, `netlist` — full list in ARCHITECTURE § builder. The state machine
  is **`builder::Game`** (`Game::new()` = level 1 in build mode, MB-10; `Game::with_preset(
  &Preset::board("TowerStorm"))` for the seven boards), `game.view() -> View` (every shape in
  **stage px** on the 1120 × 600 stage plus world metres where useful, every string), piece enum
  **`PieceType`**, chips `Chip<T> { value, name, on, dim }`. Goldens against the mockup's JS are
  next; names may still gain fields but won't be renamed without a note here. Answers:
  - **screen**: positions are stage px (`MemberView::{a, b}`, `JointView::at`, …) with world
    metres alongside (`MemberView::{wa, wb}`, `JointView::pos`, `VehicleView::{x, y, ang}`);
    the camera is `game.camera()` (`x()/y()/px()/to_world()/snap()`), pan with
    `pan_by(dx_px, dy_px)` / pointer drag, zoom `zoom_by(f, Some(px), Some(py))` / `wheel` /
    `zoom_in` / `zoom_out` / `fit`. Pointer: `joint_down(id)`, `surface_down(px, py)`,
    `pointer_move(px, py)`, `pointer_up()`, `click_surface(px, py)` (ignores the click that ends
    a pan), `click_node(id)`, `pick_handle(id)`. Band `z` is `band: None`. Shell's `Nudge` →
    `game.nudge(dx, dy)`, `Pan` → `pan_by`, `Paste(text)` → `game.paste_netlist(text)`, Share
    → `game.share_text()`. Toasts: call `game.advance(dt)` each frame (`toast_remaining()` for
    `wake_in`), `game.tick()` per 1/30 s while `game.animating()`.
  - **lab**: done as asked except the two lines that call into your file — `frame()` now calls
    `fire_hook(sim, dt)` **every** running frame and `Sim::new` leaves `fire_start: None`;
    `capacity(kind, len) -> Capacity { cap_t, crush, pcr, cap_c, buckles, kk }` is what
    `Sim::new` uses; `Game::sim()` / `sim_mut()` exist. **Commit `fire.rs` now** (it compiles
    with this commit); tell me here and I'll switch `fire_hook` to `super::fire::update`,
    `light_fire_now` to `fire::light_now`, and the result tip for Softened / BurnedThrough to
    `fire::tip(FireBreak, kind)` the same hour.
  - **time**: CMF-2 = `Sim::drag` / `Sim::shedding` (drag multiplies at the end of the wind line;
    bit-identical at 1.0) and `run_headless(&Design, &Setup, &Conditions, practice, max_frames,
    |sim| { sim.drag = …; sim.shedding = … }) -> (Option<RunResult>, Sim)` (no allocation per
    substep; `Setup::new(level, sb_scene, &cond)` = `L()`); everything you listed on `Sim` /
    `Particle` / `Constraint` is public, `Setup::soil_at(joint)`. `settle` with offsets stays
    (`preview::settle`). TIME-3: yes — add `pub fn pisa_level() -> Level` and `pub const
    PISA_AVAILABLE: bool` to `longterm.rs`, tell me here, and I'll call them from
    `data::level(LevelKey::Pisa)` / `pisa_available()`. `Level::prebuilt: Option<Design>` is the
    whole starting design (anchors ignored when set; `Joint::locked` / `Piece::locked` are
    refused by edits with "Level pieces stay put."), `run: RunKind::Years` makes
    `Game::run_test` return `None` (your drive owns it). `Game::set_best(level, stars)` exists.
    For `adjust_view`: every `View` field is `pub`, mutate the copy before drawing.

- **shell → lab, time** (d6a8373): the v1 painters you reuse are in `src/widgets/`.
  **`lab`: `widgets::tensile`** — `tensile::rig(ui, rect, &RigScene)` and
  `tensile::plot(ui, rect, &PlotScene)` draw on a `.graph` panel in the mockup's viewBox units
  (rig `0 0 420 560`, plot `0 0 560 400`, fitted "meet"). Build a rig with
  `RigScene::dogbone(stretch /* rig units, negative = pushed */, necking /* 0–0.55 */, broken,
  necked_break, fill)` + `.readout("Load cell", "12.4 kN", note)` + `.badge(..)` + `.label(..)`
  (its `specimen` polygons are public if you want a buckled shape); a plot with
  `PlotScene::axes(x_max, &x_ticks, y_max, &y_ticks)` then set `curve` / `curve_color` / `done`
  / `marker` / `ghosts` / `labels` / `y_title` / `x_title` / `label`, points via
  `PlotScene::point(strain, stress, x_max, y_max)`. Colours: `sp_theme::materials` (frame,
  axes, `SAMPLES[..]`); add yours in `sp_theme::fire` if you need piece-type sample colours.
  **`time`: `widgets::world`** stays — `band_bar(ui, &bands, marker_pct, &ticks, name)`,
  `compare_row(ui, name, selected, pct, colour, value)`, `readout_grid`, `goal_row`,
  `status_pill`, … (both shown in the widget gallery). The v1 Materials / Tower / Ground screens
  are gone from the tree (git history before 2234b1f).

- **shell (v2 status, 2234b1f) → screen, time, lab**: routes are now **Playground (the
  builder) · Advanced · Series** (`Route::{Playground, Advanced, Series}`; a v1 saved route
  restores to the Playground; ⌘1 / ⌘2). **`screen`: `src/screens/builder.rs` is yours now** — a
  placeholder `BuilderScreen` (`new()`, `Screen` impl: drive "Run traffic" / "Pause", Reset,
  clock `building`, `describe`); replace the body, keep `BuilderScreen::new()` + the `Screen`
  impl (add your `pub mod` lines there). App hooks for BLD: no world tab row any more (draw
  PROV-1 yourself; `Screen::provenance` is gone; `Badge` / `Provenance::color()` /
  `widgets::badge` remain); the builder's keys arrive as `Command`s on the Playground only —
  `ZoomIn` (⌘= / ⌘+), `ZoomOut` (⌘−), `ZoomFit` (⌘0), `Pan { dx, dy }` (arrows, dy up),
  `Nudge { dx, dy }` (⌥ arrows), `Escape` (Esc), plus `Undo` / `Delete` / `Paste(text)` as
  before (none while a text field has focus). The arrows arrive while nothing has keyboard
  focus; for a focused stage / joint button call `widgets::arrows_pass_through(ui, id)` each
  frame it has focus (it also stops egui moving focus with the arrows). Hand-offs:
  `Carry::Netlist { text, name, from }` only (`Carry::netlist(text, Some(name),
  Route::Playground)` for "Open in Advanced"; Advanced's "Open in Playground" sends you one with
  `from: Some(Route::Advanced)`). `Drive` is unchanged (`label`, `.running(b)` → "Pause",
  `.enabled(b)`); ask if you need a "Stop" label instead of "Pause". Tests: `tests/common`
  has `commands(&h)` and `press_alt`. MCP will call the engine's `Game` through
  `app.screens_mut().builder` — please keep a public handle to it (e.g. `pub game: Game`) and
  tell me its name.

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

- **lab (v2 status)**: LAB-1 + FIRE-1 engine, UI, theme, docs and tests committed.
  Engine: `builder::fire` (ffffb49 + later) — EN 1993-1-2 / EN 1992-1-2 reduction tables, wood /
  rope burn-through, the plume heat model, `fire::update(&mut Sim, dt)` on the core's hooks,
  `light_now` / `can_light`, `break_mode` + `tip`, `hud_value`, `note`; `builder::sample` —
  curves from each type's builder properties whose straight line ends exactly at the core's
  `sim::capacity` × fire factors, the Pull / Push stepper, status, explain, view + rig / plot
  geometry. UI (4019636): `screens/builder/lab.rs` (the drawer over the aside, Esc / Close,
  paced at 20 steps/s, idle = no frames) and `fire.rs` (Fire row + "Light it now" + note, flames,
  reach dome, heat glow, HUD "Fire 650 °C"). `sp_theme::fire` (ddd35ce). DATA-SOURCES (PROV-2:
  builder tables, fire, sample curves, time's rows, retired v1 rows). ARCHITECTURE § builder ›
  lab fold-ins (f41db06). Tests: `sp-engine/tests/{fire_factors,fire_heat,sample_curves,
  sample_bench}.rs` (26), sp-theme fire (3); GUI `structures-playground/tests/builder_lab.rs`
  (4) — **not committed yet: its Fire test needs the engine's `fire_hook` wiring (in engine's
  working copy) to be committed**; green against that copy.
- **lab → engine**: thanks — `fire_start: None`, `capacity()` (now used by the sample drawer),
  `sim()` / `sim_mut()`, `light_fire` → `fire::light_now` and the result tips are all as hoped.
  Only left: **commit** `fire_hook` → `super::fire::update(sim, dt)` (it's in your working
  copy); my GUI test lands right after.
- **lab → shell** (**Done**, b1fd84a: with `show` the test runs in `fold.lab`'s drawer; `builder_fast_forward` with `show` calls `fold.time.start` + `set_year`) (MCP-2, optional): `sample_test` runs a headless `Sample` today; to show it in
  the app (`show: true`) open the drawer: `app.screens_mut().builder.fold.lab.open(kind)` then
  drive `fold.lab.sample_mut()` (`push()` for Push, `set_temp`, `run_to_end()`; `describe()` is
  the one-line summary) — the user then sees the rig and plot where the result came from.

- **time (v2 status)**: engine `builder::comfort` + `builder::longterm` (+ `statics`, `pisa`)
  committed (6bb9f43, 06752bb) with `sp_theme::time` (589b69d), the UIs `screens/builder/
  {comfort,time}.rs` (cf3a4c2) and ARCHITECTURE § builder › Fold-ins (3c146e6). CMF-1 meter (live
  and after the run), CMF-2 "Compare shapes" (four headless runs, ~35 ms release), TIME-1/2
  fast-forward (exact settle per 0.5 yr with P-Δ, 3–30 ms release; HUD chips, plumb + lean line,
  scrubber, result line), TIME-3 level `longterm::pisa::level()` (waiting for `engine` to plug it
  in). Tests: engine 34 (comfort, longterm, longterm_model, longterm_pisa, statics units) + 2
  perf targets (`comfort_perf`, `longterm_perf`, `PERF` lines); GUI `tests/builder_time.rs` (4).
  **Waiting on**: engine hooks 4 (draw the structure at given positions — until then the stage
  shows the design / last run while the years play) and 5 (`start_years` / `finish_years`),
  plugging Pisa in, the three Leaning tower rules; lab: the DATA-SOURCES rows below.
  **For `shell` (MCP)**: headless = `longterm::fast_forward_design(&design, &setup, &cond)` →
  `LongTerm` (`result_line()`, `sentence()`, `chips(year)`, `at(year)`, `ending`); Leaning tower
  scoring `longterm::pisa::result(&lt, pisa::fix_cost(&design), practice, budget)`; four shapes
  `comfort::compare(&design, &setup, &cond)`. In the app: `builder.fold.time.start(&mut game)`
  (shows it, and on the Leaning tower starts the run; no-op unless `Time::available(&game)`),
  `.finish(&mut game)` (skip to year 30; the Leaning tower's result goes to `finish_years`),
  `.set_year(y)`, `.long_term()`, `.pisa_result()`; `builder.fold.comfort.compare(&game)`.
- **time → engine** (CMF-2 / TIME-1 / TIME-3 hooks — rename freely, I adapt):
  1. **CMF-2 factors**: two per-run factors on the sim, default 1.0, applied exactly at the
     mockup's wind line: `fx += dir·0.613·v²·1.3·drag·4·4·gust` and `gust = 1 + 0.3·shedding·
     sin(2π·fg·t) + 0.15·sin(…) + 0.1·sin(…)` (only the vortex term scales). And a **headless
     run of any design** that doesn't touch the `Game`: e.g. `run_headless(&Design, &Level (or
     the resolved L), &Conditions, RunFactors { drag, shedding }) -> RunOutcome` with at least
     the result fields of `tick()`'s end (`pass`, `standing`, `reached`, `mg`, `height`, `sway`,
     first break / foundation). Deterministic, no allocation per substep if you can (four 18 s
     runs must fit in 400 ms release). I call it for the as-built tower too.
  2. **TIME-1: I don't need the "settle with foundation offsets" hook** — a quick port of the
     mockup engine to Node showed the 520-step preview hasn't converged on soft soil (a tower's
     rocking mode on its soil springs is over-damped, τ ≈ 2 s ≫ the 1.3 s settle), so reactions
     barely shift and the P-Δ feedback depends on the step count. `longterm` solves the settled
     state exactly instead (linear stiffness with the builder's own springs: pieces `E/15·A/L`,
     foundations `ks` vertical + pinned horizontally, gravity on the sim's joint masses, snow on
     exposed decks; tension-only slack and breaks iterated; geometry updated each year step =
     P-Δ). What I need instead: read access to the **sim's setup** for a design — `Sim::new(&
     Design, &L, &Conditions, live: false)` (or whatever builds `makeSim`) with public per-joint
     `x, y, m, sup, Qc, Qt, ks, inv`(used) and per-piece `a, b, L, kk, capT, capC, tOnly, deck,
     exposed, buckles`, plus `soil_at(joint)`. If you already built the offsets hook, keep it;
     I just won't depend on it.
  3. **TIME-3 level**: I'll provide `longterm::pisa_level() -> Level` (full entry incl.
     `prebuilt: Some(Design)` with locked joints + pieces, crate(s), `run: RunKind::Years`) and
     `longterm::PISA_AVAILABLE`-style switch; you call it from `level(LevelKey::Pisa)` and
     `pisa_available()`. Fine? (I'll tell you when it exists so your placeholder can go.) The run
     is mine: the screen routes the Pisa drive to `time::Time::drive`; I need `Game::set_best(
     level, stars)` (or similar) and `stars_for` (have it).
- **time → engine** (TIME-3 Leaning tower, follow-up — the level is designed and tested against
  your uncommitted `game.rs` / `sim.rs`; `longterm::pisa::level()` is ready to plug into
  `level(LevelKey::Pisa)` + `pisa_available() = true`): the prebuilt `Design` has three
  **unlocked** footings `J1`–`J3` (so the player can swap them for piles — the intended fix),
  locked joints above and locked pieces, plus the bells crate (8 t on `J27`). Three small rules
  please, none of which touch mockup behaviour (no mockup level has locked pieces or prebuilt
  crates):
  1. `Design::cost()` skips **locked pieces** like it skips locked foundations ("a level's own
     piece costs nothing") — otherwise the chip reads $31.4k for an untouched tower and the
     budget would have to include the tower. Pisa's budget is $8k for the fix.
  2. A joint that holds a locked piece can't be **dragged / nudged** either (`joint_down`,
     arrow-nudge: you already refuse deleting it with `LOCKED_PIECE`); re-founding it stays
     allowed.
  3. The level's own crates can't go **below the level's amount** (`addLoad` on `J27` under
     8 t → toast e.g. "The bells stay: fix the lean another way."); adding more is fine.
- **time → engine** (TIME-1 drawing + TIME-3 run — two hooks, please; status: `comfort` /
  `longterm` / `longterm::pisa` are written against your uncommitted `game` / `sim` / `setup`
  API and pass their tests; I commit when your modules are in HEAD):
  4. **Draw the structure at given positions**: the fast-forward has, per frame, every joint's
     world position, every piece's axial force (N), broken and slack flags. Something like
     `Game::view_settled(&Settle { pos: &[[f64; 2]] /* design joint order */, pieces: &[(n,
     broken, slack)] /* design piece order */ }) -> View` — `view()` with these instead of the
     design positions + preview forces (members / decks / backs / bands, foundations, crates,
     joints, handles, rings), the rest unchanged. My `adjust_view` then swaps it in while the
     years play and on the scrubber. (Re-deriving decks, crates and foundations from positions
     in the screen would duplicate your geometry.)
  5. **The Leaning tower's run**: `Game::start_years()` (phase `Run` with no sim: editing
     paused, selection cleared, like `start_run`) and `Game::finish_years(RunResult)` (phase
     `Result`, the result stored, best stars updated as in `tick`), with `Outcome::Years { tilt,
     settled }` (degrees, m). `stop_run` / Back to building as usual. I fill the banner title /
     explain / run info in `adjust_view` (`view.result`, `view.explain`, `view.run_info`).
- **time → lab** (DATA-SOURCES, § "Comfort and settlement fold-ins" — thanks for the
  pre-filled rows; they're right. Please add / adjust these, all from `builder::{comfort,
  longterm}` as committed in 6bb9f43 / 06752bb):
  | Constant | Value | Class | Note |
  |---|---|---|---|
  | Comfort value | the engine's peak smoothed top-floor acceleration of the run (live: so far) | Simplified | the meter shows exactly the HUD chip's number |
  | Same storm, four shapes — runs | four headless 18 s open-ground tests at the run's level and conditions (the as-built bar equals the live run) | Simplified | `comfort::compare` |
  | Long-term stiffness | `k_long = ks / ratio`, `ratio = 1 kPa/mm ÷ ks_ground` (v1 Ground ks): soft clay 1.67, loose wet sand 0.67, stiff clay 0.25, dense sand 0.125, rock 0 | Illustrative | so the final consolidation settlement is the first (immediate) one × ratio: soft clay settles tens of cm, sand millimetres, rock nothing (`longterm::consolidation`) |
  | Overload creep | + 3 × (R − Qc) / k_long when a foundation is pushed past its capacity | Illustrative | `longterm::OVERLOAD_CREEP` |
  | Consolidation stepping | every 0.5 yr: s ← s∞ + (s − s∞)·e^(−Δt/τ) for the current reaction (exact for a constant load), never rebounding | Simplified | [Terzaghi] first term; `longterm::STEP` |
  | Long-term loads | self-weight (joint masses as the settling engine's) and crates; no wind, snow, quakes, traffic | Simplified | they come and go |
  | Settled state | exact static equilibrium of the builder's springs (pieces E/15·A/L, foundations ks vertical and pinned horizontally), geometry updated each step (P-Δ); ropes slack, resting joints lift, foundations rip out past Qt, pieces break past capacity | Exact for the model | `longterm::statics` (energy minimisation; the preview's 520 steps don't converge the rocking mode on soft soil) |
  | Tilt | angle between vertical and the foundations' centroid → top floor's centroid line, from the line as built; bridges: largest differential settlement | Exact (geometry) | SPEC TIME-2 |
  | 7 · Leaning tower | soft clay; 2 bays × 8 storeys (8 m × 32 m), Concrete walls + diagonals, Floors, all locked; three footings; 8 t bells top right; goal under 0.5° after 30 years; budget $8k | Game | left alone 0.6° → 4.3° in 30 years; piles under all three footings 0.16° ($4.5k, three stars); an 8 t counterweight top left 0.0°. The real tower leaned ≈ 5.5° by 1990; 600 t of lead counterweights (1993) and soil extraction (1999–2001) took it back to ≈ 4° [Burland 2003: J. B. Burland, M. B. Jamiolkowski, C. Viggiani, "The stabilisation of the Leaning Tower of Pisa", Soils and Foundations 43(5), 2003] |
- **time → engine** (thanks for `view_settled`, `start_years` / `finish_years`, `Outcome::Years`
  and plugging Pisa in — adapting now): one more small one — on a `RunKind::Years` level
  `start_run(slow)` (so `toggle_run`, the banner's "Watch again, slow" → `replay`) should do
  `start_years()` (keeping `slow`) instead of building a live `Sim`; my `tick` starts the
  fast-forward whenever a Years level is in `Run` without a sim.
- **time → engine** (thanks — all five landed and are in use, ce3e305): two small view things for
  the Leaning tower's years run (phase `Run` / `Result` with no sim): (a) `view_settled` /
  `view` draw the **joint buttons** then (they're `live`-less), so the white joint dots sit on
  the leaning tower and are clickable; the mockup hides joints whenever it isn't building —
  please leave `joints` empty outside `Phase::Build`. (b) Optional: a closer starting view for
  Pisa (e.g. `Level::view: Option<ViewState>`; `fit()` gives z ≈ 2.5 on the tower) — at 5 px/m
  the footings the player re-founds are 20 px apart. (Still open: `start_run` on a Years level →
  `start_years`, so "Replay in slow motion" replays the years; my `tick` already picks that up.)
- **time → screen**: thanks, `FoldIn` works for me. `comfort::Comfort` implements `results`
  (meter + "Compare shapes" bars after an open-ground run), `reset`, `describe`; `time::Time`
  `results` ("Fast-forward 30 years" button, years scrubber, result line), `tick` + `busy` (6 s
  animation, ~30 fps), `hud` ("Year 12.0", "Tilt 1.8°", "Settled 214 mm"), `adjust_view`
  (settled joint positions + broken pieces + each piece's band for that year), `drive` /
  `drive_button` (Leaning tower), `reset`, `describe`. From `Game` I need read access to: the
  design, the resolved level/conditions, the phase and the last run's result (`mg`, `standing`,
  `pass`), and — for `adjust_view` — a way to override joint positions / piece band / broken in
  `View`. Please create the stubs when you need them.
- **time → lead** (decide; implemented as below until you say otherwise): CMF-1's meter "0–30+
  milli-g, bands 5 / 10 / 20" is v1's real-world scale, but the builder's comfort number is the
  mockup's game scale: the TowerStorm preset reads **272 mg** and passes level 5 (limit 300);
  level 4's limit is 100. On a 0–30 meter every builder tower is "Seasick" and the level limit
  is off the scale. I keep the v1 bands and sentences but **scale them ×15 for the builder**
  (`comfort::GAME_SCALE`): bands 75 / 150 / 300 mg, meter 0–450+ mg, so level 5's 300 mg limit is
  exactly the "Seasick" line ("…without making everyone seasick") and level 4's 100 sits in
  "Some people notice". Documented as illustrative in DATA-SOURCES. One constant to change.

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
