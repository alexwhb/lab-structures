# structures-mcp

The Structures Playground MCP server. AI agents (Claude Code, Claude Desktop, any MCP client)
use it to solve trusses, towers, foundations and tensile tests headless, and to look at and
drive the running app over the [Model Context Protocol](https://modelcontextprotocol.io).

*Educational. Not for real structural design.*

`structures-mcp` (crate `sp-mcp`) is a separate binary that speaks MCP on stdin/stdout. The
**engine tools** run `sp-engine` in-process, so they work without the app. The **app tools**
act on the running app: `structures-mcp` forwards them over the live bridge, a user-only Unix
socket the app listens on (`crates/structures-playground/src/control.rs`).

```
MCP client ──stdio──▶ structures-mcp ──Unix socket (live bridge)──▶ Structures Playground (control.rs)
                          │                                            └─ the screens' state
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

Then open Structures Playground and ask Claude to, say, "build a truss across the river level,
test it and tell me which member fails first", or, with no app at all, "what's the safety
factor of a 40-floor building on piles in soft clay?".

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
whose message says what to send instead; values outside a screen's range are refused, never
clamped; nothing changes in the app when a call fails. Units are in the field names (`_kn`,
`_mpa`, `_m`, `_mm`, `_pct`); forces are positive in tension.

### Engine tools (no app needed)

| tool | arguments | returns |
|---|---|---|
| `solve_truss` | `netlist`, `x_m?` (truck position) | stable / the mechanism's node, status ("Holding · worst 40%"), determinacy m + r − 2j, mass, reactions (kN), every member's force, stress, limit, Euler load, utilisation and mode (yield / buckling / crushing), the worst member, the Advanced screen's checks; parse errors with line and column |
| `influence_line` | `netlist`, `member`, `truck_t?` | the member's force as the truck crosses (`[x m, N kN]`), the peak and where, the limits, whether it's over |
| `tower_wind` | `floors?`, `system?`, `shape?`, `wind_ms?`, `damper?` | height, top sway and drift H/x, period, damping, base shear, overturning, peak acceleration (milli-g), vortex ratio and lock-in, cost, status, comfort, tiles, challenge goals, Share's `.story` line |
| `ground_check` | `soil?`, `foundation?`, `floors?`, `pile_length_m?`, `quake?`, `years?` | load, pressure, capacity, safety factor and its target, settlement and tilt (now and final), consolidation time, on rock, liquefied, status, tiles, goals, Share's `.soil` line |
| `tensile_test` | `material`, `temperature_c?`, `strain_pct?`, `curve?`, `points?` | stress, load-cell force, broken, break strain, status, E, yield, peak, toughness, specific strength, density, the curve |

A netlist (`.str`), the format the Advanced screen edits and the Bridge's Share copies:

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

Defaults are the screens' starting states: the Tower 60 floors, core, straight, 30 m/s, no
damper; the Ground soft clay, mat, 6 floors, 34 m piles, no quake, year 10; the test 20 °C.

### App tools (Structures Playground must be running)

| tool | arguments | what |
|---|---|---|
| `app_status` | — | running / reachable, current screen, window, version, pid (not running is a normal result) |
| `list_screens` | — | the six screens (id, label, mode, shortcut, document, what's on it, board), the modes, the current one (works without the app) |
| `navigate` | `screen` | `bridge`, `tower`, `ground`, `materials`, `advanced`, `series` |
| `screenshot` | `screen?` | a PNG of the screen, drawn offscreen from the app's state at the window's size (works while the window is covered or minimised) |
| `describe_screen` | `screen?` | the accessibility outline (role, name, value, state per element) and the screen's one-line summary (also drawn offscreen) |
| `bridge_load_netlist` | `netlist`, `show?` | a netlist on the Bridge's 2 m grid into its Sandbox → the Bridge (status, cost, joints, members) |
| `bridge_test` | `level?`, `show?` | the Test run to the end → held / collapsed / refused, the banner, stars, the first failure (member, load %, mode), every broken member |
| `set_tower` | the `tower_wind` settings, `show?` | change the Tower (none: read it) → its numbers |
| `set_ground` | the `ground_check` settings, `show?` | change the Ground → its numbers |
| `set_material` | `material?`, `temperature_c?`, `fresh?`, `strain_pct?`, `pull?`, `show?` | drive the Materials bench (pull to fracture) → the test, the axis, samples broken so far |

Tools that change a screen switch the app to it (`show: false` changes it in the background).
When the app isn't running, app tools return a tool error that says how to start it.

## Tests

```sh
cargo test -p sp-mcp        # the real binary over stdio, and the real app in egui_kittest
```

`tests/e2e.rs` spawns `structures-mcp` for every tool: the engine tools against closed forms
and the screens' own numbers (the Advanced defaults: "Holding · worst 40%", the top chord's
−1004 kN peak), bad input, garbage netlists; the app tools against the real `StructuresApp` on
a temp socket (navigate, screenshot pixels, the outline, a netlist loaded and Tested, the
worlds set and read back through the GUI, an app restarting under the server, "not running").
The app side of the bridge alone is tested in `crates/structures-playground/tests/mcp_bridge.rs`.
