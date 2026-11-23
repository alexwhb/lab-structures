//! Engine tools: they run `sp-engine` in-process and headless, so they work without the app
//! (SPEC MCP-1).
//!
//! | tool | what |
//! |---|---|
//! | `solve_truss` | a `.str` netlist → stable / mechanism, reactions, every member's force, stress, limit, utilisation and failure mode, the worst member, determinacy, mass, the Advanced screen's checks; parse errors with line and column |
//! | `influence_line` | a netlist + a member id (+ the truck's weight) → the force in that member as the truck crosses, its peak and limits |
//! | `tower_wind` | floors, system, shape, wind, damper → sway, drift, period, base shear, overturning, vortex lock-in, comfort, cost, status |
//! | `ground_check` | soil, foundation, floors, piles, quake, years → pressure, capacity, safety factor, settlement, tilt, liquefaction, status |
//! | `tensile_test` | material, temperature, stretch → stress, force, status, E, yield, peak, toughness, specific strength, the curve |
//!
//! Every number comes from the same `sp-engine` functions the screens call, through
//! `sp_shared::report`, so a tool and the app always agree.
//!
//! # Adding an engine tool
//!
//! 1. Use `sp-engine` (and `sp-shared` for input checks and payloads), never the app or egui.
//! 2. Register the tool in [`register`]: a `snake_case` name, a title, a description written
//!    for the model (what it does, defaults, limits, what to call next) and a JSON Schema of
//!    its arguments (validated before the handler runs; deserialize with `call.args()`).
//! 3. Return [`ToolOutput::json`] with a one-line summary first, or a [`ToolError`] whose
//!    message says what to send instead.
//! 4. Test it through the real binary in `tests/e2e.rs`.

use lab_mcp::mcp::{Content, RegistryError, Tool, ToolError, ToolOutput, ToolRegistry};
use serde::Deserialize;
use serde_json::{Value, json};
use sp_engine::ground::GroundInput;
use sp_engine::tower::TowerInput;
use sp_shared::input::{self, GroundChange, TowerChange};
use sp_shared::report;

const NETLIST_DESC: &str = "A truss netlist (.str): `.node <id> <x m> <y m> [fix=xy|y|x]`, \
`.material <name> E=200GPa fy=250MPa rho=7850`, `.section <name> A=<cm²>cm2 I=A^2`, `.member \
<id> <node> <node> <section>`, `.load <case> <nodes> Fx=<kN> Fy=<kN>` (a `t` suffix is tonnes \
of weight, `+self` adds self-weight), optionally `.load truck 40t x=24 lane=B0..B6 share=0.5`, \
`.static linear`, `.buckle euler K=1.0`. `*` lines and `; …` tails are comments.";

/// The largest truck an `influence_line` call may put on the bridge, t.
pub const TRUCK_MAX_T: f64 = 1000.0;
/// The most points a `tensile_test` curve may have.
pub const CURVE_MAX_POINTS: usize = 2001;

fn netlist_schema() -> Value {
    json!({"type": "string", "description": NETLIST_DESC, "maxLength": input::NETLIST_MAX_LEN})
}

/// The Tower settings' JSON Schema properties.
pub fn tower_properties() -> Value {
    json!({
        "floors": {"type": "integer", "minimum": 5, "maximum": 120,
                   "description": "Floors, 4 m each (default 60)."},
        "system": {"type": "string", "enum": ["core", "tube", "outrigger", "braced"],
                   "description": "Structural system: a concrete core alone, a framed tube, a core with outriggers, or a braced tube (default core)."},
        "shape": {"type": "string", "enum": ["straight", "tapered", "twisted", "hollow"],
                  "description": "Shape (default straight)."},
        "wind_ms": {"type": "number", "minimum": 5, "maximum": 50,
                    "description": "Storm wind at 10 m height, m/s (default 30)."},
        "damper": {"type": "boolean", "description": "A tuned mass damper at the top (default false)."}
    })
}

/// The Ground settings' JSON Schema properties.
pub fn ground_properties() -> Value {
    json!({
        "soil": {"type": "string", "enum": ["dense", "stiff", "soft", "loose"],
                 "description": "Dense sand, stiff clay, soft clay or loose wet sand (default soft)."},
        "foundation": {"type": "string", "enum": ["footing", "mat", "piles", "caisson"],
                       "description": "Spread footings, a mat slab, piles or caissons (default mat)."},
        "floors": {"type": "integer", "minimum": 2, "maximum": 80,
                   "description": "Building floors, 12 kPa each (default 6)."},
        "pile_length_m": {"type": "number", "minimum": 8, "maximum": 40,
                          "description": "Pile length, m (piles only; 30 or more reaches bedrock; default 34)."},
        "quake": {"type": "boolean", "description": "Earthquake shaking (default false)."},
        "years": {"type": "number", "minimum": 0, "maximum": 30,
                  "description": "Years after construction, for settlement and tilt (default 10)."}
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

#[derive(Debug, Deserialize)]
struct SolveArgs {
    netlist: String,
    #[serde(default)]
    x_m: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct InfluenceArgs {
    netlist: String,
    member: String,
    #[serde(default)]
    truck_t: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TensileArgs {
    material: String,
    #[serde(default)]
    temperature_c: Option<f64>,
    #[serde(default)]
    strain_pct: Option<f64>,
    #[serde(default)]
    curve: Option<bool>,
    #[serde(default)]
    points: Option<usize>,
}

/// Parses a netlist or explains every error with its line and column.
fn parse_netlist(text: &str) -> Result<sp_engine::netlist::Netlist, ToolError> {
    input::netlist(text).map_err(|d| ToolError::new(input::netlist_errors_text(&d)))
}

/// Registers the engine tools.
pub fn register(tools: &mut ToolRegistry) -> Result<(), RegistryError> {
    tools.register(
        Tool::new(
            "solve_truss",
            "Solve a pin-jointed truss written as a .str netlist, headless (no app needed): \
             linear statics by the direct stiffness method, Euler buckling (exact for the \
             model). Returns whether it is stable (or the node of the mechanism), the status \
             (\"Holding · worst 40%\", \"M9 over its limit\", \"Mechanism\"), determinacy m + r − 2j, \
             mass, the support reactions (kN), every member's force (kN, + tension), stress \
             (MPa), limit, Euler load, utilisation and governing mode (yield / buckling / \
             crushing), the worst member and the Advanced screen's checks. With a moving load \
             (`.load truck …`) the truck stands where the netlist puts it, or at `x_m`. A \
             netlist that doesn't parse is a tool error listing each problem with its line and \
             column. Educational, not for real structural design.",
        )
        .title("Solve a truss")
        .read_only()
        .input_schema(object(
            json!({
                "netlist": netlist_schema(),
                "x_m": {"type": "number", "minimum": -1000, "maximum": 1000,
                        "description": "Where the truck stands along its lane, m (default: the netlist's x=)."}
            }),
            &["netlist"],
        )),
        |call| {
            let args: SolveArgs = call.args()?;
            let net = parse_netlist(&args.netlist)?;
            if args.x_m.is_some() && net.truck.is_none() {
                return Err(ToolError::new(
                    "`x_m` places the truck, but the netlist has no moving load \
                     (`.load truck 40t x=24 lane=B0..B6`)",
                ));
            }
            let r = report::truss_report(&net, args.x_m).map_err(|e| {
                ToolError::new(format!("the truss can't be solved: {e}"))
            })?;
            Ok(ToolOutput::json(&r).with_first(Content::text(r.summary())))
        },
    )?;

    tools.register(
        Tool::new(
            "influence_line",
            "The influence line of one member: the force in it (kN, + tension) as the netlist's \
             truck crosses the lane, one full solve per position (1 m steps, or 24 steps for a \
             big truss). Returns the samples [x m, N kN], the peak and where the truck stands \
             for it, the member's tension and compression limits and whether the peak is past \
             them. The netlist needs a moving load (`.load truck 40t x=24 lane=B0..B6 \
             share=0.5`); `truck_t` changes its weight. Headless, no app needed.",
        )
        .title("Influence line")
        .read_only()
        .input_schema(object(
            json!({
                "netlist": netlist_schema(),
                "member": {"type": "string", "maxLength": 64,
                           "description": "The member's id in the netlist, e.g. M8."},
                "truck_t": {"type": "number", "exclusiveMinimum": 0, "maximum": TRUCK_MAX_T,
                            "description": "The truck's weight, t (default: the netlist's)."}
            }),
            &["netlist", "member"],
        )),
        |call| {
            let args: InfluenceArgs = call.args()?;
            let text = match args.truck_t {
                Some(t) => sp_engine::netlist::set_truck(&args.netlist, Some(t), None).ok_or_else(
                    || {
                        ToolError::new(
                            "`truck_t` sets the truck's weight, but the netlist has no moving \
                             load line (`.load truck 40t x=24 lane=B0..B6 share=0.5`)",
                        )
                    },
                )?,
                None => args.netlist,
            };
            let net = parse_netlist(&text)?;
            let r = report::influence_report(&net, args.member.trim()).map_err(ToolError::new)?;
            Ok(ToolOutput::json(&r).with_first(Content::text(r.summary())))
        },
    )?;

    tools.register(
        Tool::new(
            "tower_wind",
            "A skyscraper in a storm, the v1 Tower model (a simplified cantilever; the builder's \
             CMF-2 shape factors come from it): power-law \
             wind V(z) = V10·(z/10)^0.2, floor forces, static top sway and drift (aim H/500), \
             first-mode period, damping (5 % with a tuned mass damper), vortex shedding and \
             lock-in, peak acceleration in milli-g (comfort bands 5 / 10 / 20), base shear, \
             overturning moment and game cost. Unset settings take the v1 screen's start (60 \
             floors, core, straight, 30 m/s, no damper). Also returns the status, the comfort \
             sentence, the six tiles, the 100-floor challenge goals and Share's .story line. \
             Headless, no app needed; builder_test runs a tower you build instead.",
        )
        .title("Tower in the wind")
        .read_only()
        .input_schema(object(tower_properties(), &[])),
        |call| {
            let change: TowerChange = call.args()?;
            let i = change
                .apply(TowerInput::default())
                .map_err(ToolError::new)?;
            let r = report::tower_report(&i);
            Ok(ToolOutput::json(&r).with_first(Content::text(r.summary())))
        },
    )?;

    tools.register(
        Tool::new(
            "ground_check",
            "A building on its ground, the v1 Ground model (simplified; the builder's soils and \
             consolidation times come from it): 12 kPa per floor, \
             footing pressure, bearing capacity (soil, pile skin friction and rock end bearing, \
             caissons), safety factor against its target (3 shallow, 2 deep), settlement with \
             consolidation over the years, differential settlement as tilt, liquefaction in a \
             quake. Unset settings take the v1 screen's start (soft clay, mat, 6 floors, 34 m \
             piles, no quake, year 10). Returns the numbers, the status (Solid / Marginal / \
             Leaning / Bearing failure / Liquefaction), the tiles, the Pisa challenge goals and \
             Share's .soil line. Headless, no app needed.",
        )
        .title("Ground check")
        .read_only()
        .input_schema(object(ground_properties(), &[])),
        |call| {
            let change: GroundChange = call.args()?;
            let i = change
                .apply(GroundInput::default())
                .map_err(ToolError::new)?;
            let r = report::ground_report(&i);
            Ok(ToolOutput::json(&r).with_first(Content::text(r.summary())))
        },
    )?;

    tools.register(
        Tool::new(
            "tensile_test",
            "A tensile test on a 50 mm² dog-bone sample, the v1 Materials model \
             (representative curves; heat effects illustrative): the stress (MPa) and load-cell \
             force (kN) at a stretch, whether it has fractured and where it breaks, the status \
             (Ready / Elastic / Plastic / Necking / Fractured), stiffness E, yield and peak \
             strength at the temperature, toughness, strength per kilogram and density. Give \
             `strain_pct` for one point; without it (or with `curve: true`) the whole curve \
             from 0 to fracture comes back as [strain %, MPa] samples. Headless; sample_test \
             tests the builder's own piece types in the app's sample lab.",
        )
        .title("Tensile test")
        .read_only()
        .input_schema(object(
            json!({
                "material": {"type": "string",
                             "description": "steel, aluminum, concrete, timber or carbon-fiber."},
                "temperature_c": {"type": "number", "minimum": -40, "maximum": 600,
                                  "description": "Temperature, °C (default 20)."},
                "strain_pct": {"type": "number", "minimum": 0, "maximum": input::STRAIN_MAX_PCT,
                               "description": "Stretch, % (0–25; default 0)."},
                "curve": {"type": "boolean",
                          "description": "Include the whole curve (default: true when strain_pct is not given)."},
                "points": {"type": "integer", "minimum": 2, "maximum": CURVE_MAX_POINTS,
                           "description": "Samples in the curve (default 101)."}
            }),
            &["material"],
        )),
        |call| {
            let a: TensileArgs = call.args()?;
            let kind = input::material(&a.material).map_err(ToolError::new)?;
            let t = input::temperature(a.temperature_c.unwrap_or(20.0)).map_err(ToolError::new)?;
            let e = input::strain(a.strain_pct.unwrap_or(0.0)).map_err(ToolError::new)?;
            let want_curve = a.curve.unwrap_or(a.strain_pct.is_none());
            let points = want_curve.then(|| a.points.unwrap_or(101));
            let r = report::tensile_report(kind, t, e, points);
            let summary = if a.strain_pct.is_none() {
                format!(
                    "{} at {} °C: peak {:.0} MPa, breaks at {} %, E = {} GPa, toughness {:.1} MJ/m³ \
                     ({} curve points)",
                    r.name,
                    report::short(r.temperature_c),
                    r.peak_mpa,
                    report::short(r.break_strain_pct),
                    report::short(r.e_gpa),
                    r.toughness_mj_m3,
                    r.curve.as_ref().map_or(0, Vec::len)
                )
            } else {
                r.summary()
            };
            Ok(ToolOutput::json(&r).with_first(Content::text(summary)))
        },
    )?;
    Ok(())
}
