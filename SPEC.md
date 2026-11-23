# Structures Playground — specification

The definition of done. Every requirement has an ID; tests and commits cite them.

Structures Playground is the lab series' bench for **things that stand up** ("SPICE for
things that stand up"): next to Breadbox (circuits), Chem Playground (reactions), Bio
Playground and Automata Playground. A native macOS app written in Rust (eframe/egui 0.37,
wgpu, AccessKit), in the same family as its siblings: the same greys, graph-paper ground,
header, 2.5D stages and fonts, plus **one sand accent** (`#D6C7A1`) and the force colours
(tension blue `#7FB0E0`, compression orange `#E0905E`, "will break" red `#E5534B`).

**Educational. Not for real structural design.** The app says so (Series screen badge, About).

**v2 (2026-11-23).** The Playground is now one builder for bridges and towers
(`docs/design/v2/`, see "Builder"), with three pieces of the retired Tower, Ground and Materials
screens folded into it (comfort, settlement over time, the sample lab + fire). Advanced and Series
are unchanged. The v1 Bridge, Tower, Ground and Materials screens are retired (git history; their
engines stay in `sp-engine` for MCP and the fold-ins).

The mockup in `docs/design/` is the spec for layout, copy, interactions and look (v2 builder:
`docs/design/v2/mockup/Builder.dc.html`, one component whose boards differ only in
`presetDef()`; references `docs/design/v2/screenshots/<Board>-1440.png` / `-390.png`, plus
`-16s` captures of the boards that run on their own; `scripts/render-mockup-v2.mjs`). v1:
`mockup/<Screen>.dc.html` (markup + CSS + a small `class Component` with the logic),
`screenshots/<Screen>-1440.png` / `-390.png` (rendered references; animated screens are
captured at rest) and `bundles/<Screen>.html` (open in Chrome to interact). They run on a
mockup runtime (`{{holes}}`, `<sc-for>`, `<sc-if>`) we don't have: read them as a spec, not as
code to copy. `scripts/render-mockup.mjs` re-renders the references.

## Screens

| ID | Mode | Mockup | Board | Document name in the header |
|---|---|---|---|---|
| SCR-1 | Playground (the builder) | `v2/mockup/Builder.dc.html` (boards BridgeBuild, BridgeStorm, BridgeCollapse, BridgeCable, TowerBuild, TowerStorm, TowerQuake) | 1440 × 900 (boards 1500 tall) | `<level>-level` (`river-level`, `bus-level`, `haul-level`, `t1-level` … `pisa-level`) / `sandbox` |
| SCR-2 | Advanced (a truss netlist) | `Advanced.dc.html` (unchanged in v2) | 1440 × 1000 | `river-crossing` (the netlist's file stem) |
| SCR-6 | Series (from the brand) | `Series.dc.html` (unchanged in v2) | 1440 × 1400 | `series` |

Retired in v2: SCR-3 Tower, SCR-4 Ground, SCR-5 Materials and the v1 Bridge (BR-*, TW-*, GD-*,
MT-*); see git history before `v2`.

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
  better than H/350 at 30 m/s" (the drift tile keeps "aim H/500"). Verified: H/350 is enough —
  `sp_engine::tower::CHALLENGE_DESIGN` (100 floors, braced tapered tube, tuned mass damper,
  30 m/s: H/367, 4.4 milli-g, $144M) meets all four goals at once
  (`tower_goldens.rs::the_100_floor_challenge_is_completable`).
- **MB-7** Ground: the stage's soil label `#E5E0D0` is ~2:1 on the light soils; LOOK-4 wins
  (keep the look, reach 4.5:1, e.g. a chip behind it).
- **MB-8** Materials: toughness uses the heat-scaled curve (the drawn one), concrete gets
  compression wording (crushes / cracks, not "tore" / "atoms slipping"), and the explain text
  says "Press Pull" (the button's name). Advanced: the weight check is "<Material> weight"
  (Steel / Aluminum / Glulam / Carbon fiber), not "Steel weight" for every material.

- **MB-9** v2 builder: the same double `.chip` rule as MB-1 floats the material palette, Show
  forces and every Conditions chip; they are in-flow chips (references rendered with the fix).
- **MB-10** v2 builder: the boards that "run on their own" (`presetDef().run/loop`) are demo
  states for the design board, not app behaviour; the app starts in build mode. Their builds
  (`buildPreset`: truss, cable-stayed, tower) are reproducible through the engine for tests.

## Navigation and header (NAV)

- **NAV-1** Every screen except Series has the header: logo mark, "Structures Playground", `/`,
  the document name (table above); the mode switcher (Playground · Advanced); on the right the
  screen's drive button (`.prim`: builder "Run traffic" (river scenes) / "Run the test" (open
  ground) / Pause; Advanced "Drive across" / Pause), Reset, the mono clock (builder `building` /
  `t = 3.1 s`; Advanced `x = 24.0 m`) and Share.
- **NAV-2** v2: there is no world tab row. Playground is the builder; Advanced the netlist
  workbench. Provenance moves to one quiet line under the builder's legend (PROV-1).
- **NAV-3** Clicking the brand (logo + name) opens the Series screen; it has a "Back" control
  and Esc returns to the previous screen.
- **NAV-4** Hand-offs carry state: builder "Open in Advanced" (in the aside under the Materials
  list) writes the structure as a netlist (one `.material` per piece type, foundations as
  supports, crates as loads); Advanced "Open in Playground" loads a netlist that fits the grid into
  the Sandbox (river scene if its nodes sit on the river grid, else open ground). Finder-opened
  `.str` files go to Advanced.
- **NAV-5** Keyboard: Tab / Shift-Tab reach every control in reading order; Enter / Space
  activate; ⌘1 Playground, ⌘2 Advanced; Space toggles the drive button when focus isn't in a text
  field; in the builder ⌘Z undo, Delete / Backspace deletes the selection, ⌘= / ⌘− zoom, ⌘0 fit,
  arrow keys pan the view (and nudge a selected joint by one grid step with ⌥), Esc ends a member
  chain or closes the sample drawer.
- **NAV-6** Share copies the current document to the clipboard (builder: a `.str` netlist of the
  structure with the level and conditions as comments; Advanced: the netlist text) and confirms
  with a toast. Pasting a netlist (⌘V) in the builder loads it into the Sandbox; in Advanced it
  replaces the netlist.
- **NAV-7** The window remembers its size and the last screen; a fresh start is reproducible
  (no randomness without a seed).

## Provenance (PROV)

- **PROV-1** Provenance with coloured dots: green "exact for the model", sand "simplified", grey
  "illustrative". Builder (one line under the legend): "Physics: simplified dynamic settling"
  (sand), "Materials: representative" (sand), "Soil, weather, fire: illustrative" (grey), "Game
  costs" (grey). Advanced keeps "Solver: linear statics, exact", "Materials: representative",
  "Game costs".
- **PROV-2** `docs/DATA-SOURCES.md` lists every constant (materials, soils, wind, comfort
  bands, costs) with its provenance class and, for representative values, a public source
  (textbook / code range). Nothing from MatWeb is bundled (license); Materials Project data, if
  used, is credited.

## Builder — Playground (BLD)

Mockup `v2/mockup/Builder.dc.html`. Its `class Component` is the behaviour spec: `types()`,
`soils()`, `vehicles()`, the condition lists, `levels()`, `spec()` / `L()`, `levelState()`,
`cam()`, `cost()`, `practice()`, the settling engine (`makeSim`, `support`, `step`, `preview`),
the run loop (`tick`, `startRun`, `stopRun`), the editing methods and every string in
`renderVals()`. Reproduce its numbers, states and copy exactly (golden tests run the mockup's
own JS in Node, as in v1). Units: metres, tonnes for crates, $k for cost.

- **BLD-1 Layout.** Header (NAV-1, doc name `<level>-level` / `sandbox`), no tab row. Tool rail
  (76 px): Select, Build, Found, Crate, Erase, then Undo and Clear. Main column: the hint bar
  (the tool's hint; Build shows the piece palette — one chip per piece type with its swatch bar,
  Road only in river scenes and Floor only on open ground; Found shows "Footing · $0.3k" /
  "Pile · $1.5k"; Crate shows 1 / 5 / 10 / 25 t; "Finish line" while a chain is open; "Show
  forces" on the right), the stage (1120 × 600, aspect kept), then the legend ("Pulled", "Pushed",
  "Slack / idle", "Will break", "Brighter means closer to breaking.") with the zoom controls (−,
  zoom %, +, Fit) and the provenance line (PROV-1). Aside (260–360 px): level groups, the goal
  panel, the Sandbox scene chips, Conditions, the inspector or the Materials list, "Open in
  Advanced", What's happening. At 390 px the columns stack in that order (references `-390`).
- **BLD-2 Scenes and camera.** River scene: banks at x ≤ 8 and x ≥ 48 m, riverbed at −12 m, water
  from −8 to −12 m with flowing dashes (animated only while running), soil body with grains,
  18 px per metre at 100 %, view x −20…76, y −12…40. Open ground: flat soil at 0, 5 px per metre,
  x −40…240, y 0…200, the level's target height as a dashed line. Zoom 35–500 % (wheel at the
  pointer ×1.15, buttons ×1.3, Fit frames the structure + ground + target); pan by dragging empty
  space with Select, or any time while running. Snap grid 2 m (river) / 4 m (open ground), halved
  at ≥ 250 %, doubled below 60 %; the grid dots and everything else are drawn in world coordinates
  and zoom together.
- **BLD-3 Pieces** (`types()`): Road, Floor, Wood, Steel, Concrete, Rope, Cable — area, E, the
  buckling factor k (I = k·A²), tension and compression strength, mass per metre or density,
  cost $/m, maximum length, tension-only (Rope, Cable), ductile (Steel, Cable). Too long →
  "Steel pieces can be at most 14 m. Add a joint in between."; building over an existing pair
  changes its material. Road and Floor are decks: a 2.5D slab (top face + front); Floor adds a
  façade, side face and window line. Other pieces draw as lines with a 2.5D back copy; Rope and
  Cable sag visibly when slack while running.
- **BLD-4 Foundations** (Found tool, `soils()`): Footing ($0.3k) or Pile ($1.5k) at a joint or
  on the ground: river levels allow banks only (1, 2) or banks and riverbed (3, Sandbox); open
  ground anywhere at y = 0. Each soil gives a foundation its push capacity, pull capacity and
  stiffness; river levels' anchors are locked ("Level foundations stay put."). Piles are drawn
  14 m deep. The joint inspector says "Soft clay under a footing: holds up to N t pushing down
  and M t pulling up. Carrying X t right now."
- **BLD-5 Crates** (Crate tool): 1 / 5 / 10 / 25 t per click on a joint, 0–100 t, drawn with
  their wire and a "5 t" tag; a crate on a rope near a tower top is a pendulum damper (the hint
  says so).
- **BLD-6 Editing** as the mockup: Build chains joint → joint (click the grid to drop a joint),
  click the glowing joint or "Finish line" to stop; Select inspects and drags joints (refusing
  underground spots, occupied spots and moves that make a piece too long; dragging a foundation
  off the ground drops it); Erase deletes joints or a piece's dot; Undo (40 steps); Clear resets
  the level; toasts fade after 2.6 s ("That spot is underground.", "Level foundations stay
  put.", "Foundations go on the ground.", …).
- **BLD-7 Live preview.** While building, every design change runs the settling engine headless
  (520 steps at h = 1/400 s, damping 0.985, gravity ramped over 0.3 s; cached by design) and the
  stage shows the settled forces: pulled blue / pushed orange at three brightness bands by
  utilisation (< 35 %, < 70 %, ≥ 70 %), slack or |N| < 500 N not coloured, over capacity dashed
  red. A joint that drops more than 1.5 m is ringed red ("Sags and folds"), as is an overloaded
  foundation. Status pill: "Nothing built yet", "Sags and folds", "Breaks under its own weight
  (and snow)", "A foundation is overloaded", "Stands, but strained" (> 70 %), "Stands up". "Show
  forces" puts the force in tonnes on each piece's handle (`+2.4 t`, `slack`).
- **BLD-8 The engine** (`makeSim` / `step`, one settling engine for everything): XPBD distance
  constraints with stiffness E/15·A/L; tension-only pieces go slack; compression capacity
  min(A·fC, π²·E·k·A²/L²) (buckling flagged); foundations are soil springs that creep down when
  pushed past capacity ("sank"), rip out when pulled past it ("ripped out"), and on loose wet sand
  lose most of their grip while shaking ("liquefied": footing ×0.15, pile ×0.6, stiffness ×0.3);
  a piece breaks when its smoothed force passes capacity ("snapped", "cracked" for concrete,
  "buckled", "crushed"). Loads: gravity on member and joint masses (ramped over 0.8 s live),
  crates, snow on exposed decks (rising at most 90 kg/m per second toward the setting), wind
  (calm for the first second, then easing toward the setting; power-law profile on the windward
  floor edge joints with gusts including a shedding term at 0.12·V_top / width; road decks
  flutter with an aerodynamic lift), earthquake (ground motion 1.1 Hz × the setting, envelope
  2 s up / 5 s / 2 s down, starting 3 s in or on "Shake now"), vehicles (Car 1.5 t, Bus 12 t,
  Truck 30 t with length, axle half-spacing and speed; they drive on Road pieces or the banks,
  their axle weight split onto the two joints of the piece under each axle; with no road under an
  axle they fall, sink with a splash or crash; in the Sandbox they loop and count crossings).
  Running steps 20 substeps per 1/30 s frame (×0.3 in slow motion).
- **BLD-9 Running.** "Run traffic" / "Run the test" (header and goal panel) starts the run (a
  toast warns when there is no Road / no Floor yet); the button becomes "Stop", then "Back to
  building"; "Slow motion" / "Normal speed" in the run overlay; the overlay reads "2 / 8 across",
  "t = 3.1 / 18 s" (open ground, 18 s test) or "t = 3 s · runs until you stop" (Sandbox). HUD
  chips (top right of the stage): Height `40 m / 64` (green when reached), Sway, Comfort
  (`15 mg`, red over the limit), Wind `21 m/s →`, Snow `150 kg/m`, Shaking `80%`, Crossings.
  Wind arrows grow with height on the windward side, streaks cross the sky, snowflakes fall, the
  stage shakes during a quake; broken pieces leave a red ✕ spark, sunk vehicles a splash.
  Conditions can change while running (wind eases in, snow piles up, a quake can start any time
  or with "Shake now"); soil and traffic change only while building ("Soil can only change while
  building.", "Traffic changes on the next run."). Building is paused while running.
- **BLD-10 Results.** River scenes pass when every vehicle crosses (a run ends 1.2 s after the
  last vehicle is done, or at 45 s); open ground passes when the top floor stays above 80 % of
  its height, reached the target and stayed under the comfort limit (peak top-floor acceleration
  in milli-g). Stars 1 / 2 / 3 at cost ≤ budget / ≤ 80 % / ≤ 60 %, none for a practice run
  (any condition easier than the level's; the goal panel shows "Practice: easier than the
  level"). Best stars per level persist (NAV-7). Result banner titles and explain sentences as the
  mockup ("All across!", "5 of 8 made it", "It stood!", "It came down", "Too short", "Too
  wobbly to live in", "…, but over budget"; the first failure with its tip), with "Replay in slow
  motion" and "Back to building".
- **BLD-11 Levels** (`levels()`): Bridges — 1 · First crossing (three cars, $22k), 2 · Bus route
  (a bus between two cars, piles set back on each bank, $30k), 3 · Heavy haul (a 30 t truck in a
  snowstorm, piers allowed on a soft-clay riverbed, $45k); Towers — 4 · First tower (40 m,
  breezy, $70k), 5 · Storm tower (64 m in a storm, comfort 300 mg, $260k), 6 · Shaky ground
  (32 m on loose wet sand, strong quake, $65k), 7 · Leaning tower (TIME-3, new); Sandbox (no
  budget, rules or time limit; scene chips "River crossing" / "Open ground"; traffic Car, Bus,
  Truck looping). Picking a level resets to its anchors and conditions.
- **BLD-12 Conditions** ("Conditions · change them any time"): Wind Calm 0 / Breezy 15 / Strong
  25 / Storm 35 / Hurricane 50 m/s with "From the left →" / "← From the right"; Snow None 0 /
  Light 150 / Heavy 400 / Blizzard 800 kg/m; Earthquake Off / Moderate 0.15 / Strong 0.3 / Huge
  0.5 g (with "Shake now" while running); Soil Rock / Dense sand / Stiff clay / Soft clay / Loose
  wet sand (swatch dots); Traffic (river scenes only) Level traffic / Rush hour / Buses / Trucks /
  Convoy; Fire (FIRE-1, new).
- **BLD-13 Inspector.** Piece: "Steel · M7", length, utilisation bar and %, the sentence
  ("Pulled with 2.4 t of force. It breaks at 15 t." / "Pushed with … It gives way at …" /
  "Slack: it is being pushed, so it carries nothing. Ropes and cables only pull." / "Doing almost
  nothing. A candidate to delete." + "Snow is piling on it." + "Running the test adds more."),
  material chips (scene-filtered, max length checked), "Delete piece". Joint: "Joint J3 · level
  foundation", position, foundation chips None / Footing / Pile (dimmed where not allowed), crate
  − / +, the note, "Delete joint". Nothing selected: "Materials · how each one pulls and pushes":
  per piece type its pull/push curve glyph, name, blurb, max length and $/m (each card opens the
  sample lab, LAB-1).
- **BLD-14 Accessibility.** Joints and piece handles are AccessKit buttons with the mockup's
  labels ("Steel M7, 63% of its limit", "Joint J3, on a pile, carrying a 5 t crate"); the stage
  has the summary label ("Bridge with 9 joints and 15 pieces. Stands up."); the grid surface is a
  button "Building grid"; zoom and pan by keyboard (NAV-5); every chip is a toggle button with its
  pressed state.
- **BLD-15 Performance.** A live-preview solve of a 60-joint design < 20 ms in release (it runs on
  edits, not every frame); a running frame (20 substeps) of a 60-joint structure with 8 vehicles
  < 4 ms; no busy repaint when nothing moves (PLAT-3).

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

## Fold-ins from the retired screens (CMF, TIME, LAB, FIRE)

The v1 Tower, Ground and Materials screens are retired; these three ideas from them live on inside
the builder, drawn in the builder's look (reuse the v1 widgets and painters where they fit:
`widgets/world.rs` band bars and comparison rows, the v1 tensile rig and plot painters).

- **CMF-1 Comfort meter.** On open-ground runs and their result: a colour-banded meter 0–30+
  milli-g with bands at 5 / 10 / 20 and the level's comfort limit marked, the peak value, and the
  v1 sentence: "Nobody can feel it." (< 5), "Some people on the top floors notice." (5–10),
  "Hanging lamps swing. People get uneasy." (10–20), "Seasick. Nobody would rent this floor."
  (≥ 20).
- **CMF-2 Same storm, four shapes.** After an open-ground run, "Compare shapes" runs the same
  conditions headless on four versions of the player's tower — as built, Tapered (each joint's
  x drawn toward the centre line in proportion to its height so the top is 60 % as wide; same
  pieces), Twisted (drag ×1.15/1.3 and the shedding gust term ×0.45) and Hollow core (drag
  ×1.0/1.3, shedding ×0.55) — and shows peak milli-g as bars, the player's first, with the
  verdict sentence ("Tapering cuts the sway the most: the top catches less wind."). Factors from
  the v1 tower model (`sp_engine::tower` shapes), documented as simplified. Deterministic; all
  four runs < 400 ms in release.
- **TIME-1 Fast-forward 30 years.** After a run that stood (any scene with foundations),
  "Fast-forward 30 years" animates years 0 → 30 (about 6 s; a years scrubber afterwards). Each
  foundation consolidates: s(t) = s∞·(1 − e^(−t/τ)), with s∞ from its long-term load (the settled
  reaction) over the soil's long-term stiffness, and τ the soil's consolidation time (from
  `sp_engine::ground` soils: dense sand 0.3 yr, stiff clay 3, soft clay 9, loose wet sand 0.5,
  rock none). Unequal settlement tilts the structure, the tilt moves weight toward the low side,
  which sinks more (P-Δ feedback, re-settled each step). HUD chips "Year 12.0", "Tilt 1.8°",
  "Settled 214 mm"; pieces can still break from differential settlement; a tower leaning past
  5° topples. The result adds "After 30 years: tilt 0.3°, settled 41 mm." with a sentence.
- **TIME-2 Tilt** is the angle between vertical and the line from the foundations' centroid to
  the top floor's centroid; bridges report the largest differential settlement instead.
- **TIME-3 New level "7 · Leaning tower"** (Towers group, `pisa-level`): soft clay; a pre-built
  stone-heavy tower (locked Floors and walls) on shallow footings with an eccentric load that
  leans past 4° within 30 years if untouched. Goal: "It leans more every year. Fix it without
  removing floors: under 0.5° after 30 years." The run is the 30-year fast-forward. Allowed:
  foundations (piles under the low side), pieces, and crates as a counterweight on the high side
  (what engineers did at Pisa). Budget set so a sensible fix earns three stars; an engine test
  names one passing fix.
- **LAB-1 Test a sample.** Clicking a material card opens the sample drawer (a panel over the
  aside, Esc / "Close"): the v1 tensile rig drawing with a sample of that material, its
  stress–strain curve built from the piece type's own properties (linear to strength, then for
  ductile materials a yield plateau, hardening and necking to break; brittle materials snap at
  strength; Rope and Cable only pull), "Pull" until it breaks and, for materials that resist
  pushing, "Push" until it crushes or buckles (a 1 m sample of the builder's own section). The
  readout shows force in t and kN against the builder's breaking force for that piece ("a piece
  this size breaks at 15 t", equal to the engine's capacity), status, a temperature slider
  (−40…900 °C, the FIRE-1 factors), "Fresh sample".
- **FIRE-1 Fire** (a Conditions row: Off / Small 400 °C / Big 700 °C / Inferno 900 °C, "Light it
  now" while running): on river scenes a fire burns under the middle of the span, on open ground
  at the base of the tower. Pieces within the fire's reach (growing over the first seconds)
  heat toward the fire temperature with a lag of a few seconds; strength and stiffness scale by
  the material's temperature factor — Steel and Cable per EN 1993-1-2 Table 3.1 (k_y: 1.0 up to
  400 °C, 0.78 at 500, 0.47 at 600, 0.23 at 700, 0.11 at 800, 0.06 at 900), Concrete per EN
  1992-1-2 (siliceous), Wood loses strength from 100 °C and burns through by 300 °C, Rope by
  250 °C; Road and Floor as concrete. Hot pieces glow; HUD chip "Fire 650 °C"; new break modes
  "softened" (steel, cable) and "burned through" (wood, rope) with tips ("Steel keeps only about
  half its strength at 600 °C. Real steel frames are wrapped in fireproofing."). Factors and
  sources in `docs/DATA-SOURCES.md` (illustrative heating, published reduction factors).

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
- **ENG-4** Performance: the truss solve as before (influence line < 5 ms); the builder per
  BLD-15; CMF-2 < 400 ms; TIME-1 30-year fast-forward computed < 300 ms.
- **ENG-5** Golden tests reproduce each mockup's numbers and strings for its default state and
  many others (Node runs the mockup's own JS: `Advanced`, and every builder level, preset build,
  edit sequence, live preview and run — positions, forces, results and text; chaotic collapses
  agree on the first failure, its time to the frame and the result).
## MCP (MCP)

- **MCP-1** `structures-mcp` (stdio, `lab-mcp`): engine tools that run headless —
  `solve_truss`, `influence_line`, `tower_wind`, `ground_check`, `tensile_test` (unchanged), and
  `builder_test` (a level or Sandbox scene + a structure + conditions → run to the end → result,
  first failure, comfort, stars; optional `years` for TIME-1).
- **MCP-2** App tools over the live bridge: `list_screens`, `app_status`, `navigate`,
  `screenshot`, `describe_screen`, `builder_level` (pick a level / Sandbox scene),
  `builder_load_netlist` (into the Sandbox), `builder_conditions` (set wind, direction, snow,
  quake, soil, traffic, fire), `builder_run` (run to the end, report the result), plus
  `builder_fast_forward` and `sample_test`. The v1 `bridge_*`, `set_tower`, `set_ground`,
  `set_material` app tools are retired.
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
- **DONE-4** End-to-end journeys (egui_kittest, real app, AccessKit labels) cover: level 1 built
  from scratch with Road + Wood triangles, run, all across with stars; Heavy haul in a wood truss
  collapses under the truck and snow, replay; a cable-stayed Sandbox bridge with looping traffic
  and a wind change mid-run; a tower built to 64 m that rides out the storm, comfort meter and
  "Compare shapes"; Shaky ground footings liquefy, piles fix it; Leaning tower fixed with piles
  and a counterweight over 30 years; a material card's sample pulled to breaking; a fire softens a
  steel truss; Open in Advanced shows the same structure; keyboard-only build and zoom.
- **DONE-5** MCP works end to end with the real binary.
- **DONE-6** README with screenshots, features, run / test / MCP instructions.
