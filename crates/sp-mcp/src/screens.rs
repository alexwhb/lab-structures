//! The Structures tools on the running app, forwarded over the live bridge to the screens'
//! public state (`crates/structures-playground/src/control.rs`; SPEC MCP-2):
//!
//! | tool | bridge method | what |
//! |---|---|---|
//! | `bridge_load_netlist` | `bridge.load_netlist` | a netlist on the 2 m grid into the Bridge's Sandbox |
//! | `bridge_test` | `bridge.test` | run the Test to the end: held / collapsed, the first failure, stars |
//! | `set_tower` | `tower.set` | floors, system, shape, wind, damper → the Tower's numbers |
//! | `set_ground` | `ground.set` | soil, foundation, floors, piles, quake, years → the Ground's numbers |
//! | `set_material` | `materials.set` | sample, temperature, stretch, pull → the bench |
//!
//! The arguments object is sent as the method's params unchanged (the schemas here and the
//! param structs in `sp_shared::proto` use the same names). Every result is the screen's state
//! after the call, as `structuredContent`, after a one-line summary.

use crate::app::{AppLink, NAVIGATE_TIMEOUT, WORK_TIMEOUT};
use lab_mcp::mcp::{Content, RegistryError, Tool, ToolOutput, ToolRegistry};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sp_shared::input;
use sp_shared::proto::{
    self, BridgeState, BridgeTestResult, GroundReport, MaterialsState, TowerReport,
};
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

fn with_show(mut properties: Value, screen: &str) -> Value {
    if let Value::Object(m) = &mut properties {
        m.insert("show".into(), crate::show_schema(screen));
    }
    properties
}

/// Registers the Bridge, Tower, Ground and Materials tools.
pub fn register(tools: &mut ToolRegistry, link: AppLink) -> Result<(), RegistryError> {
    let link = Arc::new(link);

    forward(
        tools,
        &link,
        Tool::new(
            "bridge_load_netlist",
            "Load a truss netlist into the running app's Bridge (the Sandbox level: no budget, \
             supports anywhere), like pasting one or Advanced's \"Open in Playground\". Its \
             nodes must sit on the Bridge's 2 m grid (x 0–58 m, y −12–16 m, nothing \
             underground: the banks are at y = 0 left of x = 8 and right of x = 48, the riverbed \
             at y = −12); static loads become hanging weights. The netlist the Bridge's Share \
             copies always fits. Returns the Bridge afterwards (status, cost, joints, members). \
             Then bridge_test runs the Test.",
        )
        .title("Load a netlist into the Bridge")
        .input_schema(object(
            with_show(
                json!({"netlist": {"type": "string", "maxLength": input::NETLIST_MAX_LEN,
                       "description": "A .str netlist (see solve_truss)."}}),
                "Bridge",
            ),
            &["netlist"],
        )),
        proto::BRIDGE_LOAD_NETLIST,
        NAVIGATE_TIMEOUT,
        |s: &BridgeState| format!("Loaded into the Sandbox — {}", s.summary),
    )?;

    forward(
        tools,
        &link,
        Tool::new(
            "bridge_test",
            "Run the running app's Bridge Test to the end, headless and instantly (as pressing \
             Test and waiting): the load ramps from 0 to 100 % of the level's weights; members \
             past their limit break one after another and the load redistributes; it holds, or \
             it collapses (a mechanism, a weight hanging on nothing, or a cascade). Returns \
             `outcome` (held / collapsed), the banner (\"It held!\", \"Collapse: M7 buckled at \
             63%\"), stars (1–3 when it holds within budget: ≤ budget, ≤ 75 %, ≤ 55 %), the \
             first failure (member, load %, mode yield / buckling / crushing), every broken \
             member and the Bridge afterwards. `level` picks a level first (its starting joints \
             only).",
        )
        .title("Test the Bridge")
        .input_schema(object(
            with_show(
                json!({"level": {"type": "string", "enum": ["river", "overhang", "haul", "sandbox"],
                       "description": "Pick this level first (resets the structure to the level's joints)."}}),
                "Bridge",
            ),
            &[],
        )),
        proto::BRIDGE_TEST,
        WORK_TIMEOUT,
        |r: &BridgeTestResult| r.summary(),
    )?;

    forward(
        tools,
        &link,
        Tool::new(
            "set_tower",
            "Change the running app's Tower (unset settings stay as they are; none: just read \
             it): floors (5–120), structural system, shape, storm wind at 10 m, tuned mass \
             damper. Returns the Tower's numbers as tower_wind does: sway and drift, period, \
             base shear, overturning, lock-in, comfort, cost, status, tiles and the 100-floor \
             challenge goals.",
        )
        .title("Set the Tower")
        .idempotent()
        .input_schema(object(
            with_show(crate::engine::tower_properties(), "Tower"),
            &[],
        )),
        proto::TOWER_SET,
        NAVIGATE_TIMEOUT,
        |r: &TowerReport| r.summary(),
    )?;

    forward(
        tools,
        &link,
        Tool::new(
            "set_ground",
            "Change the running app's Ground (unset settings stay as they are; none: just read \
             it): soil, foundation, building floors (2–80), pile length, earthquake shaking, \
             years after construction. Returns the Ground's numbers as ground_check does: \
             pressure, capacity, safety factor, settlement, tilt, liquefaction, status, tiles \
             and the Pisa challenge goals.",
        )
        .title("Set the Ground")
        .idempotent()
        .input_schema(object(
            with_show(crate::engine::ground_properties(), "Ground"),
            &[],
        )),
        proto::GROUND_SET,
        NAVIGATE_TIMEOUT,
        |r: &GroundReport| r.summary(),
    )?;

    forward(
        tools,
        &link,
        Tool::new(
            "set_material",
            "Drive the running app's Materials bench (a tensile test): pick a sample (a fresh one \
             on the axis that fits it: 0–25 %, or 0–2 % for concrete and carbon fiber), set the \
             temperature, start a fresh sample, stretch it to a strain, and/or `pull` until it \
             breaks or reaches the axis end (as pressing Pull and waiting). Applied in that \
             order. Returns the test (stress, force, status, E, yield, peak, toughness, specific \
             strength and the curve), the axis and the samples broken so far (\"Break all \
             five\").",
        )
        .title("Set the Materials bench")
        .input_schema(object(
            with_show(
                json!({
                    "material": {"type": "string",
                                 "description": "steel, aluminum, concrete, timber or carbon-fiber."},
                    "temperature_c": {"type": "number", "minimum": -40, "maximum": 600,
                                      "description": "Temperature, °C."},
                    "fresh": {"type": "boolean", "description": "A fresh, unstretched sample."},
                    "strain_pct": {"type": "number", "minimum": 0, "maximum": input::STRAIN_MAX_PCT,
                                   "description": "Stretch to this, % (within the sample's axis)."},
                    "pull": {"type": "boolean",
                             "description": "Pull until it breaks or reaches the axis end."}
                }),
                "Materials",
            ),
            &[],
        )),
        proto::MATERIALS_SET,
        WORK_TIMEOUT,
        |s: &MaterialsState| {
            let mut t = s.test.summary();
            if let Some(n) = &s.note {
                t = format!("{n} {t}");
            }
            t
        },
    )?;
    Ok(())
}
