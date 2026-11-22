# Structures Playground — specification

The definition of done. Every requirement has an ID; tests and commits cite them.

Structures Playground is the lab series' bench for **things that stand up** ("SPICE for
things that stand up"): next to Breadbox (circuits), Chem Playground (reactions), Bio
Playground and Automata Playground. A native macOS app written in Rust (eframe/egui 0.37,
wgpu, AccessKit), in the same family as its siblings: the same greys, graph-paper ground,
header, 2.5D stages and fonts, plus **one sand accent** (`#D6C7A1`) and the force colours
(tension blue `#7FB0E0`, compression orange `#E0905E`, "will break" red `#E5534B`).

**Educational. Not for real structural design.** The app says so (Series screen badge, About).

The mockup in `docs/design/` is the spec for layout, copy, interactions and look:
`mockup/<Screen>.dc.html` (markup + CSS + a small `class Component` with the logic),
`screenshots/<Screen>-1440.png` / `-390.png` (rendered references; animated screens are
captured at rest) and `bundles/<Screen>.html` (open in Chrome to interact). They run on a
mockup runtime (`{{holes}}`, `<sc-for>`, `<sc-if>`) we don't have: read them as a spec, not as
code to copy. `scripts/render-mockup.mjs` re-renders the references.

## Screens

| ID | Mode › tab | Mockup | Board | Document name in the header |
|---|---|---|---|---|
| SCR-1 | Playground › Bridge | `Playground.dc.html` | 1440 × 900 | `river-level` / `overhang-level` / `haul-level` / `sandbox` |
| SCR-2 | Advanced (the bridge as a truss netlist) | `Advanced.dc.html` | 1440 × 1000 | `river-crossing` (the netlist's file stem) |
| SCR-3 | Playground › Tower | `Tower.dc.html` | 1440 × 900 | `tower-study` |
| SCR-4 | Playground › Ground | `Ground.dc.html` | 1440 × 900 | `foundation-pit` |
| SCR-5 | Playground › Materials | `Materials.dc.html` | 1440 × 900 | `tensile-test` |
| SCR-6 | Series (from the brand) | `Series.dc.html` | 1440 × 1400 | `series` |

## Mockup bugs not to copy

- **MB-1** Playground: the shared stylesheet defines `.chip` twice; the first (the floating
  measurement chip: `position:absolute; transform:translate(-50%,-130%)`) leaks into the chip
  *buttons*, so in Chrome the Size chips, "Exaggerate sag", "Show forces" and the "Built from"
  material chips float over the layout. They are in-flow chips (the reference PNG is rendered
  with that fixed, see `render-mockup.mjs` `FIXES`).
- **MB-2** Playground: the level rows' star placeholders render as tiny `☆☆☆`; draw them at the
  row's mono 12 px.
- **MB-3** Tower: the mockup's `minus` floors button clamps at 5 but the slider's `min` may
  differ; floors are 5–120 everywhere.
- **MB-4** Playground: `renderVals()` returns `nM` both as the member count and as the "no
  solution, Medium" path, so the mockup never draws unstable Medium members. Draw them
  `#9AA0A8` (BR-2).
- **MB-5** Playground: the Sandbox "starter truss" is a mechanism (its middle panel has no
  diagonal: m + r − 2j = −1). Cross-brace the middle panel so the starter truss holds.
- **MB-6** Tower: "Challenge · 100 floors" can never complete — no 100-floor tower in the model
  reaches H/500 at 30 m/s (best: braced tapered tube ≈ H/367). The drift goal reads "Drift
  better than H/350 at 30 m/s" (the drift tile keeps "aim H/500"), provided a design meets all
  four goals at once; otherwise the lowest round threshold that does.
- **MB-7** Ground: the stage's soil label `#E5E0D0` is ~2:1 on the light soils; LOOK-4 wins
  (keep the look, reach 4.5:1, e.g. a chip behind it).
- **MB-8** Materials: toughness uses the heat-scaled curve (the drawn one), concrete gets
  compression wording (crushes / cracks, not "tore" / "atoms slipping"), and the explain text
  says "Press Pull" (the button's name). Advanced: the weight check is "<Material> weight"
  (Steel / Aluminum / Glulam / Carbon fiber), not "Steel weight" for every material.

## Navigation and header (NAV)

- **NAV-1** Every screen except Series has the header: logo mark (sand-outlined square with the
  truss glyph), "Structures Playground", `/`, the document name (table above); the **mode
  switcher** (Playground · Advanced with their icons); and on the right the screen's **drive
  button** (`.prim`: Test / Pause on the Bridge, Drive across / Pause in Advanced, Play gusts /
  Pause on the Tower, Play years / Pause on the Ground, Pull / Pause in Materials), **Reset**,
  the mono **clock** (`building` / `load 63%` / `held` / `collapse`; `x = 24.0 m`;
  `T = 9.9 s`; `year 10.0`; `ε = 0.00 %`) and **Share**.
- **NAV-2** Playground screens have the world tab row: Bridge, Tower, Ground, Materials (active
  tab underlined in the accent), with the screen's **provenance badges** right-aligned (see
  PROV-1). Playground opens the last-used world (Bridge at first start); Advanced opens the
  netlist workbench.
- **NAV-3** Clicking the brand (logo + name) opens the Series screen; it has a "Back" control
  and Esc returns to the previous screen.
- **NAV-4** Hand-offs carry state: Tower "Send base reactions to Ground ›" opens Ground with that
  building (floors → building load, the tower's overturning moment shown as a note); Ground
  "‹ Back to the Tower" returns. Bridge "Open in Advanced" (in the aside, below "Built from")
  opens Advanced with the player's structure written as a netlist (ADV-9).
- **NAV-5** Keyboard: Tab / Shift-Tab reach every control in reading order; Enter / Space
  activate; ⌘1–⌘4 switch worlds, ⌘5 Advanced; Space toggles the drive button when focus isn't in
  a text field; ⌘Z undo on the Bridge; Delete / Backspace deletes the selected joint or member.
- **NAV-6** Share copies the current document to the clipboard (Bridge: a `.str` netlist of the
  structure and level; Advanced: the netlist text; Tower / Ground / Materials: a one-line
  `.story` / `.soil` / `.material` style summary of the settings) and confirms with a toast.
  Pasting a netlist (⌘V) on the Bridge or in Advanced loads it.
- **NAV-7** The window remembers its size and the last screen; a fresh start is reproducible
  (no randomness without a seed).

## Provenance (PROV)

- **PROV-1** Each screen shows its badges with a coloured dot: green "exact for the model",
  sand "simplified", grey "illustrative". Bridge: "Solver: linear statics, exact" (green),
  "Materials: representative" (sand), "Game costs" (grey). Advanced: as Bridge plus the
  netlist's comment `; representative`. Tower: "Wind + sway: simplified cantilever", "Comfort
  bands: illustrative", "Game costs". Ground: "Bearing + settlement: simplified", "Soil
  presets: illustrative", "Game costs". Materials: "Curves: representative", "Heat effects:
  illustrative", "Not certified data".
- **PROV-2** `docs/DATA-SOURCES.md` lists every constant (materials, soils, wind, comfort
  bands, costs) with its provenance class and, for representative values, a public source
  (textbook / code range). Nothing from MatWeb is bundled (license); Materials Project data, if
  used, is credited.

## Bridge — Playground (BR)

Mockup `Playground.dc.html`; its `solve()`, `initSim()`, `stepSim()`, `tick()` and
`renderVals()` are the behaviour spec. Units: grid metres, tonnes for weights, $k for cost.

- **BR-1 Stage.** A 1120 × 600 stage (aspect kept, scaled to the column) on `#0C0D0F` with a
  snap-grid of dots every 2 m (36 px; 1 m = 18 px, origin x 56 px, y 380 px), two banks (top
  faces + bodies) either side of a 40 m river (x 8–48 m) with flowing dashes (animated only
  while testing), a riverbed at y = −12 m. Members draw as round-capped lines with a 2.5D back
  face (+18, −12 px, `#34373C`). Width by size: Thin 3, Medium 4.5, Thick 6.5 px.
- **BR-2 Colours.** Each member is coloured from the solve: tension blue / compression orange
  at opacity .45 / .75 / 1 for utilisation < 35 % / < 70 % / ≥ 70 %; grey `#5E5E5E` when
  |N| < 500 N; dashed red when over its limit; `#9AA0A8` when there is no solution. The legend
  (bottom-left) reads "Stretched (tension) · Squeezed (compression) · Will break · brighter =
  closer to its limit".
- **BR-3 Tools rail** (76 px, icons from the mockup): Select (inspect; drag joints to reshape),
  Member (click joint → joint, or empty grid to drop a joint there and chain; click the glowing
  joint to stop; "Finish line" in the hint bar), Joint (place loose joints), Support (cycle none
  → pin → roller on a joint on the ground), Weight (hang the chosen 1 / 5 / 10 / 25 t on a
  joint; click again to stack; 0–200 t), Erase (joint or member dot); then Undo (40 steps) and
  Clear ("Start this level again"). The hint bar shows the tool's hint (mockup `hints`), the
  Size chips (Member tool: Thin 0.004, Medium 0.008, Thick 0.016 m²) or Weight chips (Weight
  tool), and the "Exaggerate sag" (×40 displacements, on by default) and "Show forces" (kN on
  each member's handle) toggles.
- **BR-4 Rules of placement.** Snap to the 2 m grid, x 0–58, y −12–16; nothing underground
  ("That spot is underground."); no duplicate members ("Those two are already joined."); level
  joints are locked ("This joint is part of the level."; their supports and minimum loads
  too); supports only where the level allows (banks / left bank / banks or riverbed) with the
  mockup's toasts; dragging a support off the ground drops the support. Toasts fade after 2.6 s.
- **BR-5 Live solve.** While building, the structure is solved at full load every change
  (`sp_engine::truss`, self-weight lumped half per end, loads in tonnes × 9 810 N), and drawn
  displaced. A mechanism rings its free joint in dashed red. Status pill: "Nothing built yet",
  "Wobbly: not stable", "Will break at N% load", "Holding, near the limit" (> 80 %),
  "Holding"; cost `$x.xk / $Bk` (mass × material $/kg / 1000 + $0.25k per joint; red over
  budget).
- **BR-6 Test.** "Test it: add the load" (and the header Test) ramps the load factor λ by 1/55
  per tick (~30 ticks/s); each tick re-solves, and while the worst member is over 1.0 it
  breaks (up to 30 per tick, logged with λ and mode), so load redistributes and failure can
  cascade. A loading bar shows λ %. A load hanging on nothing collapses at once; a mechanism
  collapses. Reaching λ = 1 holds: stars 1 / 2 / 3 for cost ≤ budget / ≤ 75 % / ≤ 55 %, best
  stars remembered per level.
- **BR-7 Collapse.** On failure the structure becomes a position-based-dynamics simulation
  (Verlet, damping 0.996, 14 constraint passes, members tear at 5 % strain with a red spark,
  pins fixed, rollers hold their height, joints rest on the banks / riverbed, water drag below
  −8 m, 3.2 s; "Replay in slow motion" at 0.3× for 1.6 s). Result banner: "It held!" / "It
  held, but over budget" with stars, or "Collapse: M7 buckled at 63%" / "Collapse: it folded",
  with "Back to building". Building is paused while testing or looking at the result.
- **BR-8 Levels** (aside): 1 · River crossing (20 t at the middle joint, $30k, supports on the
  banks), 2 · Overhang (5 t 12 m out, supports on the left bank only, $14k), 3 · Heavy haul (two
  25 t, piers to the riverbed allowed, $60k), Sandbox (no rules, no budget; "Drop in a starter
  truss" when empty). Picking a level resets to its joints and loads. Each row shows best stars.
- **BR-9 Inspector.** Selecting a member: "M7 · J3–J5", length, utilisation bar and %, the
  sentence (mockup `mi.text`: stretched / squeezed with kN and its limit, "Carrying almost
  nothing. A candidate to delete.", "No forces until the structure is stable."), its Size chips,
  "Delete member". Selecting a joint: "Joint J3 · level", `x 28 m · y 0 m`, Support chips (dimmed
  where not allowed), Weight − / + by the chosen step, the support note, "Delete joint". With
  nothing selected: "Built from" material chips (Steel, Timber, Aluminum, Carbon fiber), the
  Joints / Members / Weight tiles, "Open in Advanced".
- **BR-10 What's happening** — the mockup's `explain` sentences for every phase (building,
  unstable, loose load, busiest member, testing, held, collapse by first failure mode).
- **BR-11 Accessibility.** Every joint and member handle is an AccessKit button with the
  mockup's labels ("Joint J3, pin support, carrying 20 t", "Member M7, 63% of its limit");
  the stage has the summary label ("Structure with 5 joints and 7 members. Holding."); the
  grid surface is a button "Building grid: click to place a joint". Joints can be moved with
  the arrow keys (2 m) when selected.

## Advanced — the truss netlist (ADV)

Mockup `Advanced.dc.html`. The netlist text is the source of truth; the toolbar edits it.

- **ADV-1 Toolbar row**: the file name `river-crossing.str`; style segmented Pratt / Howe /
  Warren (none lit for a custom netlist); "Cross-braced" checkbox; Material select (Steel,
  Aluminum, Glulam, Carbon fiber); Size 1–20 slider (default 4); Truck 0–120 t (default 40);
  x 0–48 m (default 24). Changing a generator control regenerates the netlist (undoable).
- **ADV-2 Netlist panel**: line-numbered mono editor, directives in the accent, comments
  `#7E8A7E`, an "example: 48 m truss" badge while it's the generated example. Editable: every
  edit re-parses (debounced ~150 ms) and re-solves; errors show inline on their line (red band
  + message) and the last good result stays on screen, dimmed, with the error count.
- **ADV-3 Directives** (Series "Netlist directives"): `.node name x y [fix=xy|y|x]`,
  `.member id a b section`, `.material name E=..GPa fy=..MPa rho=..` , `.section name A=..cm2
  I=..` (`I=A^2` allowed), `.load case nodes Fx Fy` (`B1..B5` ranges, `+self`) and the moving
  `.load truck <t>t x=<m> lane=B0..B6 share=0.5`, `.static linear`, `.buckle euler K=1.0`,
  `.plot N(M*) vs truck.x 0 48 1`, `.units SI kN m`; and the other worlds' directives
  (`.story`, `.wind`, `.soil`, `.foundation`, `.dynamic`) parse and are reported as "used by
  the Tower / Ground world" warnings. Comments: `*` lines and `; …` tails. Unknown directives,
  bad numbers, unknown nodes / sections, duplicate ids are errors with line and column.
- **ADV-4 Elevation · .static result**: the truss drawn from the netlist, members coloured
  tension / compression / near-zero grey / over-limit red, member and node labels, the truck
  arrow at x, supports; status pill "Holding · worst 40%" / "M9 over its limit" / "Mechanism".
- **ADV-5 Influence line** `.plot N(M8) vs truck x`: picks Top chord T2–T3, Bottom chord B2–B3,
  End post B0–T1, Vertical B2–T2 (or, for a custom netlist, the four most-used members); the
  force in the picked member as the truck moves 0 → span in 1 m steps, filled, with the limit
  dashes (+tension / −compression) and a cursor at the current x; tiles "At x = 24 m −1004 kN C",
  "Worst over the crossing −1004 kN, truck at x = 24 m", "Limit (red dashes) +2500 / −2500 kN".
- **ADV-6 Member results** sorted by use: ID, Nodes, N kN, σ MPa, Limit (yield / crush /
  buckle 1974), Use %, coloured as the mockup.
- **ADV-7 Checks**: Determinacy `m + r − 2j = …` with the determinate / indeterminate
  sentence; Reactions `A … kN · B … kN`; Midspan deflection `53.1 mm = L/903` (target L/800,
  illustrative); Worst member; Steel weight (both trusses).
- **ADV-8 Drive across** animates x 0 → 48 m (0.5 m per 90 ms) looping; dragging x pauses.
- **ADV-9 Round trip.** The Bridge's structure opens in Advanced as a netlist (`.node` with
  fix, `.member` with Thin/Medium/Thick sections, `.material`, `.load` of its weights); results
  match the Bridge's solve to the newton. "Open in Playground" (Advanced) loads a netlist whose
  nodes fit the grid into the Sandbox.
- **ADV-10** With the defaults the screen reproduces the mockup's numbers exactly (golden test:
  `crates/sp-engine/tests/mockup_pratt.rs` and its successors).

## Tower (TW)

Mockup `Tower.dc.html` (`calc()` is the model; "simplified cantilever").

- **TW-1 Controls (left)**: Floors − / value ("60 floors · 240 m") / + and slider 5–120; Structural
  system tiles Core, Tube, Outriggers, Braced tube (stiffness ×1, 2.2, 3, 4.2; cost ×1, 1.2,
  1.3, 1.45); Shape tiles Straight, Tapered, Twisted, Hollow core (drag coefficient, cross-wind
  factor, mass and cost factors from the mockup); "Tuned mass damper at the top" checkbox.
- **TW-2 Model**: 4 m floors, 40 m square plan, power-law wind `V(z) = V10·(z/10)^0.2`, floor
  forces `0.613·V²·Cd·B·w(z)·4`, cantilever deflection `Σ F·z²(3H − z)/(6EI)` with
  `EI = 4.3e13 × k`, first frequency `0.5596·√(EI/(μH⁴))`, vortex shedding `fs = 0.12·V_H/B` with
  lock-in near fs/f1 ≈ 1 (0.75–1.25), damper ζ 1.5 % → 5 %, peak acceleration in milli-g, game
  cost. All in `sp_engine::tower`, numbers exactly as the mockup's `calc()`.
- **TW-3 Stage**: "Stack it up, then let the storm in", the tower drawn floor by floor in 2.5D
  (front, side, cap; system lines; hollow-core slots; twist / taper), swaying by the static lean
  plus, while playing, the dynamic part; plumb line; wind arrows growing with height on the
  left; streaks on the right; "Wind at the top" chip (m/s · mph); pinned "top 928 mm · H/259"
  chip; "sway drawn × 40" chip; the damper drawn at the top swinging against the sway.
- **TW-4 Play gusts** animates the sway with `sp_engine::dynamics`: the first mode as a damped
  oscillator (with the damper, a two-degree-of-freedom tuned mass) driven by a gusting wind
  around the mean; the clock shows the period `T = 9.9 s`.
- **TW-5 Bottom bar**: Storm · wind at 10 m slider (m/s · mph), Show sway exaggeration slider
  (×N), "Send base reactions to Ground ›".
- **TW-6 Readouts (right)**: tiles Top sway (mm, drift H/x, aim H/500), Sway period (s, Hz), Base
  shear (MN), Overturning (GN·m), Vortex / sway (× and lock-in note), Cost ($M game money);
  Occupant comfort bar 0–30+ milli-g with bands 5 / 10 / 20 and the sentence; "Same storm, four
  shapes" bars; Challenge · 100 floors (7 / 12) with four live goals; What's happening.
- **TW-7 Status pill**: Comfortable / Too much sway / Vortex lock-in / Dangerously flexible.

## Ground (GD)

Mockup `Ground.dc.html` (`calc()` is the model; "simplified").

- **GD-1 Controls (left)**: Soil tiles Dense sand, Stiff clay, Soft clay, Loose wet sand (with
  their swatches); Foundation tiles Footings, Mat slab, Piles, Caissons; Pile length slider
  (Piles only, "34 m · on rock" at ≥ 30); Building floors slider; "Earthquake shaking"
  checkbox; Try a scenario: Leaning tower (soft clay, mat), Liquefaction (wet sand + quake),
  60 floors on piles to rock, Friction piles in soft clay.
- **GD-2 Model** (`sp_engine::ground`): load 12 kPa per floor, footing pressure ÷ 0.35,
  allowable bearing / pile skin friction × length (+ rock end bearing ≥ 30 m) / caissons 1800
  kPa, liquefaction cuts capacity to 12 %, safety factor, settlement with consolidation
  `1 − e^(−t/τ)`, differential settlement → tilt, bearing failure and liquefaction states,
  exactly as the mockup.
- **GD-3 Stage**: "What's holding it up", soil block in 2.5D with grains, bedrock band at 30 m,
  the building sinking and tilting (drawn exaggerated), its foundation (footings / mat / piles
  to length / caissons), the dashed pressure bulb, sand boils when liquefied, shake while
  quaking; pinned chip "tilt 2.37° · sank 268 mm"; soil and bedrock labels; legend.
- **GD-4 Play years** runs t 0 → 30 years (0.25 yr per 60 ms) and stops; the Years slider
  scrubs; "‹ Back to the Tower".
- **GD-5 Readouts**: Pressure, Capacity, Settlement (heads to …), Tilt (Pisa today ≈ 4°),
  Building load (MN), Consolidation (τ yr); Safety factor bar 0–5+ with target; Challenge · The
  lean of Pisa (10 / 12) with three goals; What's happening. Status: Solid / Marginal / Leaning
  / Bearing failure / Liquefaction.

## Materials (MT)

Mockup `Materials.dc.html` (`defs()`, `tempFactor()`, `stressAt()`).

- **MT-1 Controls (left)**: Sample tiles Steel, Aluminum, Concrete, Timber, Carbon fiber with
  swatches and the note; Temperature −40–600 °C slider with its note; Strain axis 0–25 % / 0–2 %
  segmented; "Coming to this bench" chips (Fatigue, Creep, Phase diagrams, Grain growth).
- **MT-2 Model** (`sp_engine::materials`): piecewise-linear representative curves, temperature
  strength factors (illustrative), fracture at the last point, necking after ultimate,
  toughness (area under the curve), specific strength; exactly as the mockup.
- **MT-3 Rig**: the tensile machine in 2.5D with the dog-bone sample stretching (exaggerated),
  necking, tearing with a crack; load cell chip "Load cell 0.0 kN · 50 mm² section"; extensometer.
- **MT-4 Plot**: stress MPa (0–700) vs strain % with the other four materials as ghosts, the
  selected curve, the done-so-far trace and the marker, curve labels, ticks per zoom.
- **MT-5 Pull** advances strain (0.08 %/tick at 25 %, 0.006 at 2 %) until fracture or the axis
  end; the Stretch slider scrubs; "Fresh sample".
- **MT-6 Readouts**: tiles Stiffness E, Yield, Peak strength, Stretch at break, Toughness,
  Density; Strength per kilogram bars; Challenge · Break all five (5 / 12) chips that light as
  each breaks; What's happening. Status: Ready / Elastic / Plastic / Necking / Fractured at N%.

## Series (SR)

- **SR-1** The Series screen reproduces `Series.dc.html`: the five app cards (Structures
  highlighted), "Shared engines, and who uses them" table, "Bridges to the other apps", "Netlist
  directives", "Challenge ladder", "Next layers", Provenance row and the "Educational. Not for
  real structural design." badge. Cards open the sibling app if installed (bundle id or the
  lab-series `dist/` folder) and otherwise say it isn't installed / planned.

## Engine (ENG)

- **ENG-1** `sp-engine` has no UI dependencies; every public item documented.
- **ENG-2** The truss solve is exact for the model and matches closed forms (triangle, cantilever
  truss) and the mockups' numbers (Advanced defaults; Playground levels with known structures).
- **ENG-3** Nothing panics on any input (fuzz / property tests per module): netlists of
  garbage, NaN, huge counts, zero-length members, empty structures.
- **ENG-4** Performance: a 60-joint Bridge solve < 1 ms; one Test tick (≤ 30 solves) < 8 ms at
  release; the influence line (49 solves of the Advanced truss) < 5 ms.
- **ENG-5** Golden tests reproduce each mockup's displayed numbers for its default state and a
  few other states (a script `scripts/mockup-goldens.mjs` may run the mockup logic in Node to
  produce them).

## MCP (MCP)

- **MCP-1** `structures-mcp` (stdio, `lab-mcp`): engine tools that run headless — `solve_truss`
  (netlist in → reactions, member forces, utilisation, determinacy), `influence_line`,
  `tower_wind`, `ground_check`, `tensile_test` — each with input validation and clear errors.
- **MCP-2** App tools over the live bridge (like the siblings): `list_screens`, `app_status`,
  `navigate`, `screenshot`, `describe_screen`, plus `bridge_load_netlist`, `bridge_test` (run a
  Test and report held / collapse), `set_tower`, `set_ground`, `set_material`.
- **MCP-3** End-to-end tests spawn the real binary (and the real app in egui_kittest for the
  bridge tools).

## Platform and packaging (PLAT)

- **PLAT-1** `structures-playground --smoke <seconds> [--headless]` prints one `SMOKE …` line
  (`first_frame_ms=`, `frames=`) like the siblings; start-up < 2 s.
- **PLAT-2** `scripts/bundle-macos.sh` builds `dist/Structures Playground.app` (icon from the
  logo mark, no machine paths in the binary, `structures-mcp` inside); `scripts/check-bundle.sh`
  verifies it sandboxed.
- **PLAT-3** Frame pacing: no busy repaint; animations only while something moves (stage
  water only while testing; ~30 fps), 0 % CPU when idle.

## Layout and look (LOOK)

- **LOOK-1** Matches the 1440 references closely (compare script per screen, mean |Δ| bound per
  stable region), and the 390 references in structure (columns stack in the mockup's order,
  one scroller).
- **LOOK-2** Colours, fonts, radii and sizes from `sp_theme` (on `lab_theme`); no hex in screens.
- **LOOK-3** No sparkle / magic-wand / four-point-star icons anywhere.
- **LOOK-4** Text ≥ 4.5:1, graphics ≥ 3:1 (tested in `sp-theme`).

## Done (DONE)

- **DONE-1** Every requirement above has a test or a documented manual check.
- **DONE-2** Every number shown comes from the engine; no mockup numbers in screens.
- **DONE-3** `scripts/test-all.sh` is green: fmt, clippy `-D warnings`, all tests, release perf
  gates, bundle check.
- **DONE-4** End-to-end journeys (egui_kittest, real app, AccessKit labels) cover: build a
  bridge for level 1 from scratch, test it, watch it hold and get stars; break one (buckling) and
  replay; open it in Advanced and see the same forces; edit the netlist and see an error, then
  fix it; tower to 100 floors with a braced tube and damper; send it to Ground and fix the lean
  of Pisa with piles; break all five materials.
- **DONE-5** MCP works end to end with the real binary.
- **DONE-6** README with screenshots, features, run / test / MCP instructions.
