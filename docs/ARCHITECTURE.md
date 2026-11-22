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

TODO (owner fills in): levels, game materials and sizes, cost and stars, the editing model
(joints, members, supports, loads, undo), the Test ramp (λ, progressive breaking, log), the
collapse simulation, explain sentences.

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

TODO (owner fills in).

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
| `SegGroup::new(name, &items, selected)` + `SegItem::new(label).icon(..)` | `.seg` in a well (`.style(SegStyle::Header)`), compact `.mini` | `.show(ui).clicked: Option<usize>`; items report pressed |
| `Toggle::new(label, on)` | `.chip` toggle (34 px pill; `.chip-on`) | `.dot(colour)` (material swatch), `.dimmed(b)` (`.chip-dim`), `.enabled(b)`, `.accessible_name(s)`; `.show(ui)` / `.show_at(ui, rect)`, `.clicked()` = flip; reports pressed |
| `Checkbox::new(label, checked)` | `input[type=checkbox]` row (18 px, accent; 44 px row, gap 10) | `.gap(8.0).height(40.0)` (Advanced), `.enabled(b)`; `.clicked()` = flip; an AccessKit `CheckBox` |
| `Slider::new(label, &mut v, range)` | `input[type=range]` (sand) with its label / value row | `.step(s)`, `.value_text(s)`, `.width(w)` |
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
stacked (narrow windows) each line is at least the route's `panel_height()` and only the page
scrolls. `layout::resolve(width, &cols)` is the pure maths. `layout::stage(ui)` paints the
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
recording launcher with nothing installed, a fresh temp output folder.

### Persistence (NAV-7), platform, smoke

eframe keeps the window size (`persist_window`) and `Persisted { route, last_world }` (Series
restores to the last world). A fresh start is reproducible: the seed is fixed
(`AppOptions::DEFAULT_SEED`). `structures-playground --smoke <seconds> [--headless]` opens
the Bridge, presses its drive button, runs and prints `SMOKE route=bridge mode=… first_frame_ms=…
frames=… frame_ms_avg=… frame_ms_max=… status=ok`. `STRUCTURES_BUNDLE=1` at build time
(`scripts/bundle-macos.sh`) leaves the checkout path out of the binary (`MANIFEST_DIR`).

## MCP — wave 2

TODO.
