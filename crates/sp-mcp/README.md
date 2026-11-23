# structures-mcp

The Structures Playground MCP server. AI agents (Claude Code, Claude Desktop, any MCP client)
use it to run bridges and towers in the builder and solve trusses headless, and to look at and
drive the running app over the [Model Context Protocol](https://modelcontextprotocol.io).

*Educational. Not for real structural design.*

`structures-mcp` (crate `sp-mcp`) is a separate binary that speaks MCP on stdin/stdout. The
**engine tools** run `sp-engine` in-process, so they work without the app. The **app tools**
act on the running app: `structures-mcp` forwards them over the live bridge, a user-only Unix
socket the app listens on (`crates/structures-playground/src/control.rs`).

```
MCP client ──stdio──▶ structures-mcp ──Unix socket (live bridge)──▶ Structures Playground (control.rs)
                          │                                            └─ the builder's Game
                          └─ engine tools: sp-engine in-process (no app needed)
```

Both sides build their answers with `sp-shared` (`report.rs`), so a tool and the app always
give the same numbers.

## Build

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release -p sp-mcp             # → target/release/structures-mcp
target/release/structures-mcp --help
```

`scripts/bundle-macos.sh` also puts it in the app bundle:
`dist/Structures Playground.app/Contents/MacOS/structures-mcp`.

## Add it to Claude Code

```sh
claude mcp add structures-playground -- /path/to/structures-mcp
# for every project:
claude mcp add --scope user structures-playground -- /path/to/structures-mcp
```

Then open Structures Playground and ask Claude to, say, "load this truss into the Sandbox, run
the trucks across in a storm and tell me which piece breaks first", or, with no app at all,
"does a steel truss get all three cars across level 1, and for how many stars?".

## Add it to Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` and restart Claude
Desktop:

```json
{
  "mcpServers": {
    "structures-playground": {
      "command": "/Applications/Structures Playground.app/Contents/MacOS/structures-mcp",
      "args": []
    }
  }
}
```

## Options

| | |
|---|---|
| `--socket <path>` | The app's bridge socket. Default: `$STRUCTURES_MCP_SOCKET`, else `~/Library/Application Support/dev.labseries.StructuresPlayground/mcp.sock` (a shorter path under the temp dir if that one is too long for a Unix socket). |
| `-V`, `--version` | Print the version. |
| `-h`, `--help` | Print help, including the default socket path. |

The app listens on the same default socket unless `STRUCTURES_MCP_SOCKET` is empty or `off`,
or it is started with `--no-mcp`; set the variable to a path to run an instance on another
socket (both sides read it). Logs go to stderr (`RUST_LOG=sp_mcp=debug,lab_mcp=debug` for
more).

## Tools

Results are a one-line summary (first text block), then the same data as JSON
(`structuredContent` and a text copy). Mistakes (a netlist that doesn't parse, a member that
doesn't exist, a floor count out of range, an unknown soil) are tool errors (`isError: true`)
whose message says what to send instead; values outside a range are refused, never clamped;
nothing changes in the app when a call fails. Units are in the field names (`_kn`,
`_mpa`, `_m`, `_mm`, `_pct`); forces are positive in tension.

### Engine tools (no app needed)

| tool | arguments | returns |
|---|---|---|
| `solve_truss` | `netlist`, `x_m?` (truck position) | stable / the mechanism's node, status ("Holding · worst 40%"), determinacy m + r − 2j, mass, reactions (kN), every member's force, stress, limit, Euler load, utilisation and mode (yield / buckling / crushing), the worst member, the Advanced screen's checks; parse errors with line and column |
| `influence_line` | `netlist`, `member`, `truck_t?` | the member's force as the truck crosses (`[x m, N kN]`), the peak and where, the limits, whether it's over |
| `builder_test` | `level?`, `scene?`, `netlist?`, conditions (`wind_ms?`, `wind_from?`, `snow_kg_m?`, `quake_g?`, `soil?`, `traffic?`, `fire_c?`), `years?` | the structure built on the level (its rules apply: refused edits in `refused`) or loaded into the Sandbox, run to the end on the builder's settling engine → `pass`, the banner and its sentence, stars, cost against budget, the first piece that broke (and how) or foundation that gave way, vehicles across, height, sway, comfort (milli-g) with the comfort sentence; with `years`, the settlement after the run (tilt, settled, differential, toppled); plus the builder's state |
| `tower_wind` | `floors?`, `system?`, `shape?`, `wind_ms?`, `damper?` | the v1 Tower model: height, top sway and drift H/x, period, damping, base shear, overturning, peak acceleration (milli-g), vortex ratio and lock-in, cost, status, comfort, tiles, challenge goals, Share's `.story` line |
| `ground_check` | `soil?`, `foundation?`, `floors?`, `pile_length_m?`, `quake?`, `years?` | the v1 Ground model: load, pressure, capacity, safety factor and its target, settlement and tilt (now and final), consolidation time, on rock, liquefied, status, tiles, goals, Share's `.soil` line |
| `tensile_test` | `material`, `temperature_c?`, `strain_pct?`, `curve?`, `points?` | the v1 Materials bench: stress, load-cell force, broken, break strain, status, E, yield, peak, toughness, specific strength, density, the curve |

A netlist (`.str`), the format the Advanced screen edits and the builder's Share copies:

```
* triangle.str
.node A 0 0 fix=xy          ; pin
.node B 4 0 fix=y           ; roller
.node C 2 2
.material steel E=200GPa fy=250MPa rho=7850
.section s A=20cm2 I=A^2
.member M1 A C s
.member M2 C B s
.member M3 A B s
.load dead C Fx=0 Fy=-50    ; kN (or 5t: tonnes of weight)
.static linear
```

Add `.load truck 40t x=24 lane=B0..B6 share=0.5` for a moving load (`influence_line` sweeps
it), `+self` on a load for self-weight, `.buckle euler K=1.0`. The full directive list is in
`crates/sp-engine/src/netlist.rs`.

For the builder (`builder_test`, `builder_load_netlist`) the nodes sit on whole metres in a
scene — the river scene when they all fit its view above ground (banks at y = 0 for x ≤ 8 and
x ≥ 48 m, the riverbed at y = −12), else open ground (y ≥ 0) —, each `.material` names a piece
type (`road`, `floor`,
`wood`, `steel`, `concrete`, `rope`, `cable`), `fix=` joints are foundations (`* found
J1=pile J2=footing` picks their kind), static loads are crates in tonnes, and `* level river`
· scene river` / `* cond wind=25 dir=1 …` comment lines carry the level and the conditions. The builder's Share
writes exactly that, so copy a structure from the app and hand it to `builder_test`.

Levels: `river` (1 · First crossing), `bus` (2 · Bus route), `haul` (3 · Heavy haul), `t1`
(4 · First tower), `t2` (5 · Storm tower), `t3` (6 · Shaky ground), `pisa` (7 · Leaning tower:
the run is the 30-year fast-forward), `sandbox` (scene `river` or `flat`; it never ends on its
own, so a run lasts 20 s). The v1 engine tools' defaults are their old screens' starting
states: the Tower 60 floors, core, straight, 30 m/s, no damper; the Ground soft clay, mat, 6
floors, 34 m piles, no quake, year 10; the test 20 °C.

### App tools (Structures Playground must be running)

| tool | arguments | what |
|---|---|---|
| `app_status` | — | running / reachable, current screen, window, version, pid (not running is a normal result) |
| `list_screens` | — | the three screens (id, label, mode, shortcut, document, what's on it, board), the modes, the current one (works without the app) |
| `navigate` | `screen` | `playground` (the builder), `advanced`, `series` |
| `screenshot` | `screen?` | a PNG of the screen, drawn offscreen from the app's state at the window's size (works while the window is covered or minimised) |
| `describe_screen` | `screen?` | the accessibility outline (role, name, value, state per element) and the screen's one-line summary (also drawn offscreen) |
| `builder_level` | `level`, `scene?`, `show?` | pick a level (or the Sandbox and its scene): reset to its anchors and conditions → the builder (level, goal, budget, conditions, joints, pieces, cost, live status, best stars) |
| `builder_load_netlist` | `netlist`, `show?` | a netlist on the builder's grid into the Sandbox (as ⌘V or Advanced's "Open in Playground") → the builder |
| `builder_conditions` | the conditions, `show?` | wind, direction, snow, quake, soil, traffic, fire (soil and traffic only while building; none: read it) → the builder |
| `builder_run` | `show?` | run to the end at once → the result as `builder_test` gives it; the app then shows the last frame and the banner |
| `builder_fast_forward` | `years?`, `show?` | the structure's settlement over the years (TIME-1) → tilt, settled, differential, toppled, the sentence; the Leaning tower's own result (under 0.5°, stars) |
| `sample_test` | `piece`, `mode?` (pull / push), `temperature_c?`, `show?` | a piece type's sample in the sample lab to breaking (LAB-1) → peak force (t, kN), what a piece this size breaks at in the builder, status, curve |

Tools that change the builder switch the app to it (`show: false` changes it in the
background). The v1 tools `bridge_load_netlist`, `bridge_test`, `set_tower`, `set_ground` and
`set_material` are retired. When the app isn't running, app tools return a tool error that
says how to start it.

## Tests

```sh
cargo test -p sp-mcp        # the real binary over stdio, and the real app in egui_kittest
```

`tests/e2e.rs` spawns `structures-mcp` for every tool: the engine tools against closed forms
and the screens' own numbers (the Advanced defaults: "Holding · worst 40%", the top chord's
−1004 kN peak; `builder_test` agrees with the builder engine run directly), bad input, garbage
netlists; the app tools against the real `StructuresApp` on a temp socket (navigate,
screenshot pixels, the outline, a level picked in the background, the weather changed, a
structure loaded into the Sandbox and run, an empty level failing, a sample broken, the header
showing it, an app restarting under the server, "not running").
The app side of the bridge alone is tested in `crates/structures-playground/tests/mcp_bridge.rs`.
