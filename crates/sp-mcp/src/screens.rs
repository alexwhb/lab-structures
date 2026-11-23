//! The Structures tools on the running app — the v2 builder — forwarded over the live bridge
//! to the builder's state (`crates/structures-playground/src/control.rs`; SPEC MCP-2):
//!
//! | tool | bridge method | what |
//! |---|---|---|
//! | `builder_level` | `builder.level` | pick a level, or the Sandbox and its scene |
//! | `builder_load_netlist` | `builder.load_netlist` | a netlist into the Sandbox |
//! | `builder_conditions` | `builder.conditions` | wind, direction, snow, quake, soil, traffic, fire |
//! | `builder_run` | `builder.run` | run to the end: the result, the first failure, comfort, stars |
//! | `builder_fast_forward` | `builder.fast_forward` | 30 years of settlement (TIME-1) |
//! | `sample_test` | `sample.test` | a piece type's sample pulled or pushed to breaking (LAB-1) |
//!
//! The arguments object is sent as the method's params unchanged (the schemas here and the
//! param structs in `sp_shared::proto` use the same names). Every result is the builder's state
//! (or the run's report) after the call, as `structuredContent`, after a one-line summary.

use crate::app::{AppLink, NAVIGATE_TIMEOUT, WORK_TIMEOUT};
use lab_mcp::mcp::{Content, RegistryError, Tool, ToolOutput, ToolRegistry};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sp_shared::input;
use sp_shared::proto::{self, BuilderState, RunReport, SampleReport};
use std::sync::Arc;
use std::time::Duration;

/// Registers a tool that forwards its arguments to `method` and shows the reply.
fn forward<T: DeserializeOwned + Serialize + 'static>(
    tools: &mut ToolRegistry,
    link: &Arc<AppLink>,
    tool: Tool,
    method: &'static str,
    timeout: Duration,
    summary: fn(&T) -> String,
) -> Result<(), RegistryError> {
    let link = Arc::clone(link);
    tools.register(tool, move |call| {
        let state: T = link.call(method, Value::Object(call.arguments.clone()), timeout)?;
        Ok(ToolOutput::json(&state).with_first(Content::text(summary(&state))))
    })
}

fn object(properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

fn with_show(mut properties: Value) -> Value {
    if let Value::Object(m) = &mut properties {
        m.insert("show".into(), crate::show_schema("Playground"));
    }
    properties
}

/// The level ids (`builder_level`, `builder_test`).
pub const LEVELS: [&str; 8] = ["river", "bus", "haul", "t1", "t2", "t3", "pisa", "sandbox"];

/// What the levels are, for tool descriptions.
pub const LEVELS_DESC: &str = "Levels: bridges river (1 · First crossing: three cars, $22k), bus \
(2 · Bus route: a bus between two cars, $30k), haul (3 · Heavy haul: a 30 t truck in a \
snowstorm, piers allowed on a soft-clay riverbed, $45k); towers t1 (4 · First tower: 40 m, \
breezy, $70k), t2 (5 · Storm tower: 64 m in a storm, comfort 300 mg, $260k), t3 (6 · Shaky \
ground: 32 m on loose wet sand, strong quake, $65k), pisa (7 · Leaning tower: fix the lean over \
30 years); sandbox (no budget or rules; scene `river` or `flat`).";

/// The conditions' JSON Schema properties (shared with the engine tool `builder_test`).
pub fn conditions_properties() -> Value {
    json!({
        "wind_ms": {"type": "number", "minimum": 0, "maximum": 50,
                    "description": "Wind at 10 m, m/s (chips: Calm 0, Breezy 15, Strong 25, Storm 35, Hurricane 50)."},
        "wind_from": {"type": "string", "enum": ["left", "right"],
                      "description": "Where the wind comes from: left (blowing →) or right (←)."},
        "snow_kg_m": {"type": "number", "minimum": 0, "maximum": 800,
                      "description": "Snow on exposed decks, kg per metre (None 0, Light 150, Heavy 400, Blizzard 800)."},
        "quake_m": {"type": "number", "minimum": 0, "maximum": 0.5,
                    "description": "Earthquake ground-motion amplitude, m: the supports shake by this × the quake's envelope at 1.1 Hz (chips: Off 0, Moderate 0.15 ≈ 0.7 g peak, Strong 0.3 ≈ 1.5 g, Huge 0.5 ≈ 2.4 g)."},
        "soil": {"type": "string", "enum": ["rock", "dense", "stiff", "soft", "loose"],
                 "description": "Rock, dense sand, stiff clay, soft clay or loose wet sand (liquefies when shaken)."},
        "traffic": {"type": "string", "enum": ["level", "rush", "buses", "trucks", "convoy"],
                    "description": "River scenes: the level's traffic, rush hour, buses, trucks or a convoy."},
        "fire_c": {"type": "number", "minimum": 0, "maximum": 900,
                   "description": "Fire under the span / at the tower's base, °C (Off 0, Small 400, Big 700, Inferno 900)."}
    })
}

/// Registers the builder tools.
pub fn register(tools: &mut ToolRegistry, link: AppLink) -> Result<(), RegistryError> {
    let link = Arc::new(link);

    forward(
        tools,
        &link,
        Tool::new(
            "builder_level",
            format!(
                "Pick a level in the running app's builder (the Playground), or the Sandbox and \
                 its scene: the structure resets to the level's anchors (and locked pieces) and \
                 its conditions, as clicking the level does. {LEVELS_DESC} Returns the builder \
                 afterwards: level, goal, budget, conditions, joints, pieces, cost, the live \
                 status, best stars."
            ),
        )
        .title("Pick a builder level")
        .input_schema(object(
            with_show(json!({
                "level": {"type": "string", "enum": LEVELS, "description": "The level."},
                "scene": {"type": "string", "enum": ["river", "flat"],
                          "description": "Sandbox only: river (River crossing) or flat (Open ground)."}
            })),
            &["level"],
        )),
        proto::BUILDER_LEVEL,
        NAVIGATE_TIMEOUT,
        |s: &BuilderState| s.summary.clone(),
    )?;

    forward(
        tools,
        &link,
        Tool::new(
            "builder_load_netlist",
            "Load a truss netlist into the running app's builder Sandbox, like pasting one (⌘V) \
             or Advanced's \"Open in Playground\": `.node`s become joints (`fix=` supports \
             become foundations), `.member`s pieces (their `.material` names a piece type: \
             Road, Floor, Wood, Steel, Concrete, Rope, Cable), static `.load`s crates. The \
             nodes must sit on whole metres in a scene: the river scene when they all fit its \
             view above ground (banks at y = 0 for x ≤ 8 and x ≥ 48 m, the riverbed at y = −12), \
             else open ground (y ≥ 0). \
             The netlist the builder's Share copies always fits. Returns the builder \
             afterwards. Then builder_run runs it.",
        )
        .title("Load a netlist into the builder")
        .input_schema(object(
            with_show(
                json!({"netlist": {"type": "string", "maxLength": input::NETLIST_MAX_LEN,
                   "description": "A .str netlist (see solve_truss)."}}),
            ),
            &["netlist"],
        )),
        proto::BUILDER_LOAD_NETLIST,
        NAVIGATE_TIMEOUT,
        |s: &BuilderState| format!("Loaded into the Sandbox — {}", s.summary),
    )?;

    forward(
        tools,
        &link,
        Tool::new(
            "builder_conditions",
            "Change the running app's builder conditions (unset ones stay as they are; none: \
             just read the builder): wind and its direction, snow, earthquake, soil, traffic \
             and fire, as the Conditions chips do. Soil and traffic change only while \
             building. Conditions easier than the level's make a practice run (no stars). \
             Returns the builder afterwards.",
        )
        .title("Set the builder's conditions")
        .idempotent()
        .input_schema(object(with_show(conditions_properties()), &[])),
        proto::BUILDER_CONDITIONS,
        NAVIGATE_TIMEOUT,
        |s: &BuilderState| s.summary.clone(),
    )?;

    forward(
        tools,
        &link,
        Tool::new(
            "builder_run",
            "Run the running app's builder structure to the end, headless and at once (as \
             pressing \"Run traffic\" / \"Run the test\" and waiting): river scenes pass when \
             every vehicle crosses, open ground when the top floor stays up, reaches the \
             target and stays under the comfort limit. Returns `pass`, the banner (\"All \
             across!\", \"5 of 8 made it\", \"It stood!\", \"It came down\", \"Too wobbly to \
             live in\"), its sentence, stars (1–3 within budget), the first piece that broke \
             (and how: snapped, buckled, crushed, …) or foundation that gave way, vehicles \
             across, height, sway and comfort in milli-g. The app then shows the run's last \
             frame and the banner. The Sandbox never ends on its own: it runs 20 s.",
        )
        .title("Run the builder")
        .input_schema(object(with_show(json!({})), &[])),
        proto::BUILDER_RUN,
        WORK_TIMEOUT,
        |r: &RunReport| r.summary(),
    )?;

    forward(
        tools,
        &link,
        Tool::new(
            "builder_fast_forward",
            "Fast-forward the running app's builder structure through the years (TIME-1; up to \
             30, default 30), as \"Fast-forward 30 years\" after a run that stood: every \
             foundation consolidates into its soil (soft clay slowly, sand fast, rock not at \
             all), unequal settlement tilts the structure and the lean moves weight to the low \
             side. Returns the result with `years`: tilt (towers), settlement and the largest \
             difference between foundations, and whether it toppled. The Leaning tower level's \
             run is this fast-forward.",
        )
        .title("Fast-forward 30 years")
        .input_schema(object(
            with_show(
                json!({"years": {"type": "number", "minimum": 0, "maximum": 30,
                   "description": "Years (default 30)."}}),
            ),
            &[],
        )),
        proto::BUILDER_FAST_FORWARD,
        WORK_TIMEOUT,
        |r: &RunReport| r.summary(),
    )?;

    forward(
        tools,
        &link,
        Tool::new(
            "sample_test",
            "Test a sample of one of the builder's piece types in the running app's sample lab \
             (LAB-1, as clicking its Materials card): `pull` until it breaks, or `push` until \
             it crushes or buckles (a 1 m sample of the builder's own section; Rope and Cable \
             only pull), at a temperature (the fire factors). Returns the peak force in t and \
             kN, the force a piece this size breaks at in the builder, the status and the \
             stress–strain curve.",
        )
        .title("Test a sample")
        .input_schema(object(
            with_show(json!({
                "piece": {"type": "string",
                          "enum": ["road", "floor", "wood", "steel", "concrete", "rope", "cable"],
                          "description": "The piece type."},
                "mode": {"type": "string", "enum": ["pull", "push"],
                         "description": "Pull until it breaks (default) or push until it crushes or buckles."},
                "temperature_c": {"type": "number", "minimum": -40, "maximum": 900,
                                  "description": "Temperature, °C (default 20)."}
            })),
            &["piece"],
        )),
        proto::SAMPLE_TEST,
        WORK_TIMEOUT,
        |r: &SampleReport| r.summary.clone(),
    )?;
    Ok(())
}
