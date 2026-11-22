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

TODO (owner fills in): `netlist::parse(text) -> Result<Netlist, Vec<Diagnostic>>` with line /
column diagnostics, `netlist::write(...)`, `Netlist::to_truss()` + load cases, the moving truck,
`generate::{pratt, howe, warren}(braced, material, size)`, `influence::line(...)`, checks.

### `tower`, `dynamics`, `ground` (agent `worlds`)

TODO (owner fills in).

### `materials` (agent `materials`)

TODO (owner fills in).

## App (`crates/structures-playground`) — agent `shell`

TODO (shell fills in): the Screen trait / screen context API, router (`Route`), header and tab
row, widgets, layout helpers, harness for tests (`tests/common`), how a screen registers its
drive button / clock / Reset / Share, toasts, hand-offs between screens (NAV-4), smoke mode.

## MCP — wave 2

TODO.
