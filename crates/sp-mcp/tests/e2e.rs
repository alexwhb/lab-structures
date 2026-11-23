//! End to end (SPEC MCP-3): the real `structures-mcp` binary (built by Cargo for this package's
//! tests, `CARGO_BIN_EXE_structures-mcp`) speaks MCP on stdin/stdout. Engine tools run with no
//! app; app tools reach the real `StructuresApp` running in an egui_kittest harness over the
//! live bridge on a temp socket, and their effect is checked on the screens' state and through
//! the GUI (AccessKit labels).
//!
//! The harness is stepped while a tool call waits, standing in for the app's frame loop.
//! Screenshots are real: egui_kittest fulfils `ViewportCommand::Screenshot` by rendering the
//! frame with wgpu, which is what eframe does in the app.
//!
//! `SP_MCP_E2E_DUMP=<dir>` saves the screenshot and the outline there for a look.

mod common;

use common::{Mcp, TempSocket, app, data, error, text};
use serde_json::{Value, json};
use sp_engine::generate::{Params, generate};
use structures_playground::Route;

const ENGINE_TOOLS: [&str; 5] = [
    "solve_truss",
    "influence_line",
    "tower_wind",
    "ground_check",
    "tensile_test",
];
const APP_TOOLS: [&str; 11] = [
    "app_status",
    "list_screens",
    "navigate",
    "screenshot",
    "describe_screen",
    "builder_level",
    "builder_load_netlist",
    "builder_conditions",
    "builder_run",
    "builder_fast_forward",
    "sample_test",
];
/// The v1 app tools retired in v2.
const RETIRED_TOOLS: [&str; 5] = [
    "bridge_load_netlist",
    "bridge_test",
    "set_tower",
    "set_ground",
    "set_material",
];

/// A 4 m triangle: pin at A, roller at B, 50 kN down at the apex C (2 m up).
const TRIANGLE: &str = "* triangle.str
.node A 0 0 fix=xy
.node B 4 0 fix=y
.node C 2 2
.material steel E=200GPa fy=250MPa rho=7850
.section s A=20cm2 I=A^2
.member M1 A C s
.member M2 C B s
.member M3 A B s
.load dead C Fx=0 Fy=-50
.static linear
";

/// A socket nobody listens on (engine tools don't need one).
fn offline() -> (TempSocket, Mcp) {
    let sock = TempSocket::new();
    let mcp = Mcp::started(&sock.path());
    (sock, mcp)
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn member<'a>(report: &'a Value, id: &str) -> &'a Value {
    report["member_results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == id)
        .unwrap_or_else(|| panic!("no member {id} in {report}"))
}

// ---- protocol -----------------------------------------------------------------------------------

#[test]
fn initialize_and_list_every_tool() {
    let sock = TempSocket::new();
    let mut mcp = Mcp::spawn(&sock.path());
    let init = mcp.initialize();
    let info = &init["result"]["serverInfo"];
    assert_eq!(info["name"], "structures-mcp");
    assert_eq!(info["title"], "Structures Playground");
    let instructions = init["result"]["instructions"].as_str().unwrap();
    assert!(instructions.contains("solve_truss"), "{instructions}");
    assert!(instructions.contains("not for real structural design"));
    let r = mcp.call("tools/list", json!({}));
    let names: Vec<&str> = r["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    let want: Vec<&str> = ENGINE_TOOLS
        .iter()
        .chain(APP_TOOLS.iter())
        .copied()
        .collect();
    assert_eq!(names, want);
    for old in RETIRED_TOOLS {
        assert!(!names.contains(&old), "{old} is retired");
    }
    for t in r["result"]["tools"].as_array().unwrap() {
        let d = t["description"].as_str().unwrap();
        assert!(d.len() > 80, "{} has a thin description", t["name"]);
        assert_eq!(t["inputSchema"]["type"], "object", "{}", t["name"]);
    }
    mcp.close();
}

#[test]
fn version_and_help() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_structures-mcp"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("structures-mcp "));
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_structures-mcp"))
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8_lossy(&out.stdout);
    assert!(
        help.contains("claude mcp add structures-playground"),
        "{help}"
    );
    assert!(help.contains("STRUCTURES_MCP_SOCKET"), "{help}");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_structures-mcp"))
        .arg("--nope")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

// ---- engine tools -------------------------------------------------------------------------------

/// The triangle matches its closed form: ±35.36 kN in the rafters, 25 kN in the tie, 25 kN
/// up at each support, statically determinate.
#[test]
fn solve_truss_matches_the_closed_form() {
    let (_s, mut mcp) = offline();
    let r = mcp.tool("solve_truss", json!({"netlist": TRIANGLE}));
    let d = data(&r);
    assert_eq!(d["stable"], true);
    assert_eq!(d["determinacy"], 0);
    assert_eq!(d["title"], "triangle.str");
    let rafter = 50.0 / (2.0 * std::f64::consts::FRAC_1_SQRT_2);
    for id in ["M1", "M2"] {
        let m = member(d, id);
        assert!(close(m["force_kn"].as_f64().unwrap(), -rafter, 1e-6), "{m}");
        assert_eq!(m["state"], "compression");
    }
    let tie = member(d, "M3");
    assert!(
        close(tie["force_kn"].as_f64().unwrap(), 25.0, 1e-6),
        "{tie}"
    );
    assert_eq!(tie["state"], "tension");
    assert_eq!(tie["mode"], "yield");
    let reactions = d["reactions"].as_array().unwrap();
    assert_eq!(reactions.len(), 2);
    for re in reactions {
        assert!(close(re["ry_kn"].as_f64().unwrap(), 25.0, 1e-6), "{re}");
    }
    assert!(
        text(&r).starts_with("triangle.str: Holding"),
        "{}",
        text(&r)
    );
    mcp.close();
}

/// The Advanced defaults (the generated Pratt truss, 40 t at x = 24 m) give the Advanced
/// screen's numbers: "Holding · worst 40%", and the top chord's influence peak of −1004 kN
/// with the truck at 24 m (ADV-4, ADV-5, ADV-10).
#[test]
fn the_advanced_defaults_reproduce_the_mockup() {
    let (_s, mut mcp) = offline();
    let text_net = generate(&Params::default()).text;
    let r = mcp.tool("solve_truss", json!({"netlist": text_net}));
    let d = data(&r);
    assert_eq!(d["status"], "Holding · worst 40%", "{}", text(&r));
    assert_eq!(d["copies"], 2);
    assert_eq!(d["truck"]["tonnes"], 40.0);
    // the same solve the engine gives directly
    let net = sp_engine::netlist::parse(&text_net).unwrap();
    let direct = sp_shared::report::truss_report(&net, None).unwrap();
    assert_eq!(
        d["member_results"].as_array().unwrap().len(),
        direct.members
    );
    // the top chord T2–T3
    let top = d["member_results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["a"] == "T2" && m["b"] == "T3")
        .expect("a T2–T3 member");
    let id = top["id"].as_str().unwrap();
    let r = mcp.tool("influence_line", json!({"netlist": text_net, "member": id}));
    let il = data(&r);
    assert_eq!(
        il["peak_kn"].as_f64().unwrap().round(),
        -1004.0,
        "{}",
        text(&r)
    );
    assert_eq!(il["peak_at_m"], 24.0);
    assert_eq!(
        il["points"].as_array().unwrap().len(),
        49,
        "0 → 48 m in 1 m steps"
    );
    assert_eq!(il["over_limit"], false);
    // a heavier truck scales the line
    let r = mcp.tool(
        "influence_line",
        json!({"netlist": text_net, "member": id, "truck_t": 80}),
    );
    // (the deck's dead load is in the line too, so twice the truck is less than twice the peak)
    let heavy = data(&r)["peak_kn"].as_f64().unwrap();
    assert!(heavy < -1300.0, "{heavy}");
    // the truck moved: a different solve
    let r = mcp.tool("solve_truss", json!({"netlist": text_net, "x_m": 0}));
    assert_eq!(data(&r)["truck"]["x_m"], 0.0);
    mcp.close();
}

/// Bad netlists and arguments are tool errors that say what to send instead.
#[test]
fn truss_tools_explain_bad_input() {
    let (_s, mut mcp) = offline();
    // parse errors with line and column
    let r = mcp.tool(
        "solve_truss",
        json!({"netlist": ".node A 0 0 fix=xy\n.member M1 A B9 s\n.frobnicate\n"}),
    );
    let msg = error(&r);
    assert!(msg.starts_with("The netlist has"), "{msg}");
    assert!(msg.contains("line 2, column"), "{msg}");
    // a mechanism is a result, not an error
    let r = mcp.tool(
        "solve_truss",
        json!({"netlist": ".node A 0 0 fix=xy\n.node B 4 0\n.node C 2 2\n.material m E=200GPa fy=250MPa rho=7850\n.section s A=20cm2 I=A^2\n.member M1 A C s\n.member M2 C B s\n.load d C Fy=-10\n"}),
    );
    let d = data(&r);
    assert_eq!(d["stable"], false);
    assert_eq!(d["status"], "Mechanism");
    assert!(d["mechanism"].is_string());
    // x without a truck, influence line without a truck, an unknown member
    let r = mcp.tool("solve_truss", json!({"netlist": TRIANGLE, "x_m": 3}));
    assert!(error(&r).contains("no moving load"), "{}", text(&r));
    let r = mcp.tool(
        "influence_line",
        json!({"netlist": TRIANGLE, "member": "M1"}),
    );
    assert!(error(&r).contains("no moving load"), "{}", text(&r));
    let r = mcp.tool(
        "influence_line",
        json!({"netlist": TRIANGLE, "member": "M1", "truck_t": 10}),
    );
    assert!(error(&r).contains("no moving load"), "{}", text(&r));
    let net = generate(&Params::default()).text;
    let r = mcp.tool("influence_line", json!({"netlist": net, "member": "M999"}));
    assert!(error(&r).contains("no member `M999`"), "{}", text(&r));
    // schema checks: missing and out-of-range arguments
    let r = mcp.tool("solve_truss", json!({}));
    assert!(error(&r).contains("netlist"), "{}", text(&r));
    let r = mcp.tool(
        "influence_line",
        json!({"netlist": net, "member": "M1", "truck_t": -5}),
    );
    assert_eq!(r["isError"], true);
    // garbage never crashes the server
    for junk in [
        "",
        "\u{0}\u{1}",
        ".node",
        ".member M1",
        "x".repeat(5000).as_str(),
    ] {
        let r = mcp.tool("solve_truss", json!({"netlist": junk}));
        assert!(r["isError"].is_boolean(), "{r}");
    }
    let r = mcp.tool("solve_truss", json!({"netlist": TRIANGLE}));
    assert_eq!(r["isError"], false, "still serving");
    mcp.close();
}

/// tower_wind and ground_check are the screens' models: the numbers match the engine and the
/// defaults are the screens' starts.
#[test]
fn tower_and_ground_tools() {
    let (_s, mut mcp) = offline();
    let r = mcp.tool("tower_wind", json!({}));
    let d = data(&r);
    let calc = sp_engine::tower::calc(&sp_engine::tower::TowerInput::default());
    assert_eq!(d["settings"]["floors"], 60);
    assert_eq!(d["height_m"], 240.0);
    assert!(close(d["period_s"].as_f64().unwrap(), calc.period(), 1e-12));
    assert!(close(
        d["top_sway_mm"].as_f64().unwrap(),
        calc.delta * 1000.0,
        1e-9
    ));
    assert_eq!(d["tiles"].as_array().unwrap().len(), 6);
    assert_eq!(d["goals"].as_array().unwrap().len(), 4);
    assert!(
        d["share"].as_str().unwrap().starts_with(".story"),
        "{}",
        d["share"]
    );
    // a stiffer, damped tower sways less and calms down
    let r = mcp.tool(
        "tower_wind",
        json!({"floors": 60, "system": "braced", "shape": "tapered", "damper": true}),
    );
    let stiff = data(&r);
    assert!(stiff["top_sway_mm"].as_f64() < d["top_sway_mm"].as_f64());
    assert_eq!(stiff["damping_pct"], 5.0);
    // refused, not clamped
    let r = mcp.tool("tower_wind", json!({"floors": 500}));
    assert_eq!(r["isError"], true, "{r}");
    let r = mcp.tool("tower_wind", json!({"shape": "round"}));
    assert_eq!(r["isError"], true, "{r}");

    let r = mcp.tool("ground_check", json!({}));
    let g = data(&r);
    assert_eq!(g["settings"]["soil"], "soft");
    assert_eq!(g["status"], "Bearing failure", "{}", text(&r));
    // piles to rock fix it
    let r = mcp.tool(
        "ground_check",
        json!({"foundation": "piles", "pile_length_m": 34}),
    );
    let piles = data(&r);
    assert_eq!(piles["on_rock"], true);
    assert!(piles["safety_factor"].as_f64() > g["safety_factor"].as_f64());
    // liquefaction
    let r = mcp.tool(
        "ground_check",
        json!({"soil": "loose", "quake": true, "foundation": "mat"}),
    );
    assert_eq!(data(&r)["liquefied"], true);
    let r = mcp.tool("ground_check", json!({"years": 99}));
    assert_eq!(r["isError"], true, "{r}");
    mcp.close();
}

/// tensile_test: one point, the whole curve, temperature, and clear errors.
#[test]
fn tensile_test_tool() {
    let (_s, mut mcp) = offline();
    let r = mcp.tool(
        "tensile_test",
        json!({"material": "steel", "strain_pct": 12}),
    );
    let d = data(&r);
    assert_eq!(d["material"], "steel");
    assert_eq!(d["broken"], false);
    assert!(d["curve"].is_null(), "no curve unless asked");
    let direct = sp_engine::materials::curves::stress_at(
        sp_engine::materials::curves::Kind::Steel,
        12.0,
        20.0,
    );
    assert_eq!(d["stress_mpa"].as_f64().unwrap(), direct.mpa);
    assert!(d["status"].as_str().unwrap().starts_with("Plastic"), "{d}");
    // the whole curve by default without a strain; it ends at fracture
    let r = mcp.tool(
        "tensile_test",
        json!({"material": "concrete", "points": 21}),
    );
    let c = data(&r);
    let curve = c["curve"].as_array().unwrap();
    assert_eq!(curve.len(), 21);
    assert_eq!(curve[0], json!([0.0, 0.0]));
    assert_eq!(
        curve[20][0].as_f64().unwrap(),
        c["break_strain_pct"].as_f64().unwrap()
    );
    assert!(c["yield_mpa"].is_null(), "concrete has no yield");
    // heat weakens steel
    let hot = mcp.tool(
        "tensile_test",
        json!({"material": "steel", "temperature_c": 500, "strain_pct": 12}),
    );
    assert!(data(&hot)["peak_mpa"].as_f64() < d["peak_mpa"].as_f64());
    // past the break: fractured
    let r = mcp.tool(
        "tensile_test",
        json!({"material": "timber", "strain_pct": 24}),
    );
    assert_eq!(data(&r)["broken"], true);
    // errors
    let r = mcp.tool("tensile_test", json!({"material": "gold"}));
    assert!(
        error(&r).contains("unknown material `gold`"),
        "{}",
        text(&r)
    );
    let r = mcp.tool(
        "tensile_test",
        json!({"material": "steel", "strain_pct": 40}),
    );
    assert_eq!(r["isError"], true);
    let r = mcp.tool("tensile_test", json!({}));
    assert_eq!(r["isError"], true);
    mcp.close();
}

// ---- app tools ----------------------------------------------------------------------------------

#[test]
fn drive_the_running_app_over_mcp() {
    let sock = TempSocket::new();
    let mut app = app(&sock.path());
    let mut mcp = Mcp::started(&sock.path());

    // navigate → the app's route changes
    assert_eq!(app.state().route(), Route::Playground);
    let r = mcp.call_tool(&mut app, "navigate", json!({"screen": "advanced"}));
    assert_eq!(text(&r), "Now showing Advanced (Advanced mode).");
    assert_eq!(app.state().route(), Route::Advanced);

    // app_status / list_screens
    let r = mcp.call_tool(&mut app, "app_status", json!({}));
    let s = data(&r);
    assert_eq!(s["running"], true);
    assert_eq!(s["screen"]["id"], "advanced");
    assert_eq!(s["screen"]["mode"]["id"], "advanced");
    assert_eq!(s["pid"], std::process::id());
    assert_eq!(s["window"]["width"], 1440.0);
    let r = mcp.call_tool(&mut app, "list_screens", json!({}));
    assert_eq!(data(&r)["current"], "advanced");
    assert!(
        text(&r).contains("advanced (Advanced, showing)"),
        "{}",
        text(&r)
    );
    assert_eq!(data(&r)["screens"].as_array().unwrap().len(), 3);

    // screenshot (navigating first) → a PNG of the rendered frame
    let r = mcp.call_tool(&mut app, "screenshot", json!({"screen": "playground"}));
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(app.state().route(), Route::Playground);
    assert_eq!(text(&r), "Playground (Playground mode) — 1440×900 px.");
    let img = &r["content"][1];
    assert_eq!(img["type"], "image");
    assert_eq!(img["mimeType"], "image/png");
    let png = lab_mcp::base64_decode(img["data"].as_str().unwrap()).unwrap();
    let png = image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!(png.dimensions(), (1440, 900));
    let colours: std::collections::HashSet<[u8; 4]> = png.pixels().map(|p| p.0).collect();
    assert!(
        colours.len() > 50,
        "a blank frame ({} colours)",
        colours.len()
    );
    let dump = std::env::var_os("SP_MCP_E2E_DUMP").map(std::path::PathBuf::from);
    if let Some(dir) = &dump {
        png.save(dir.join("screenshot.png")).unwrap();
    }

    // describe_screen → the accessibility outline with known labels
    let r = mcp.call_tool(&mut app, "describe_screen", json!({"screen": "series"}));
    assert_eq!(app.state().route(), Route::Series);
    let outline = text(&r);
    if let Some(dir) = &dump {
        std::fs::write(dir.join("outline.txt"), outline).unwrap();
    }
    assert!(outline.starts_with("Series —"), "{outline}");
    for label in [
        "\"Back\"",
        "\"Breadbox\"",
        "\"Educational. Not for real structural design.\"",
    ] {
        assert!(outline.contains(label), "{label} missing from:\n{outline}");
    }
    assert!(r["structuredContent"]["nodes"].as_array().unwrap().len() > 20);

    // Bad arguments come back as tool errors the model can act on; nothing moves.
    let r = mcp.call_tool(&mut app, "navigate", json!({"screen": "lab"}));
    assert!(error(&r).contains("must be one of"), "{}", text(&r));
    let r = mcp.call_tool(&mut app, "navigate", json!({"screen": "tower"}));
    assert!(error(&r).contains("must be one of"), "{}", text(&r));
    assert_eq!(app.state().route(), Route::Series);
    // the retired v1 tools are gone: calling one is a protocol error (unknown tool)
    for old in RETIRED_TOOLS {
        let r = mcp.call("tools/call", json!({"name": old, "arguments": {}}));
        let msg = r["error"]["message"].as_str().unwrap_or_default();
        assert!(msg.contains("Unknown tool"), "{old}: {r}");
    }
    mcp.close();
}

#[test]
fn the_app_can_restart_under_a_running_server() {
    let sock = TempSocket::new();
    let mut mcp = Mcp::started(&sock.path());
    {
        let mut first = app(&sock.path());
        let r = mcp.call_tool(&mut first, "navigate", json!({"screen": "advanced"}));
        assert_eq!(r["isError"], false, "{r}");
    } // the app quits (its socket goes away)
    let r = mcp.tool("navigate", json!({"screen": "series"}));
    assert!(error(&r).starts_with("Structures Playground isn't running"));
    // …and comes back.
    let mut second = app(&sock.path());
    let r = mcp.call_tool(&mut second, "app_status", json!({}));
    assert_eq!(data(&r)["screen"]["id"], "playground");
    mcp.close();
}

#[test]
fn app_tools_say_clearly_when_the_app_is_not_running() {
    let (_s, mut mcp) = offline();
    let calls = [
        ("navigate", json!({"screen": "advanced"})),
        ("screenshot", json!({})),
        ("describe_screen", json!({"screen": "playground"})),
        ("builder_level", json!({"level": "river"})),
        ("builder_load_netlist", json!({"netlist": TRIANGLE})),
        ("builder_conditions", json!({"wind_ms": 25})),
        ("builder_run", json!({})),
        ("builder_fast_forward", json!({})),
        ("sample_test", json!({"piece": "steel"})),
    ];
    for (tool, args) in calls {
        let r = mcp.tool(tool, args);
        let msg = error(&r);
        assert!(
            msg.starts_with("Structures Playground isn't running — open Structures Playground"),
            "{tool}: {msg}"
        );
        assert!(
            msg.contains("cargo run --release -p structures-playground"),
            "{tool}: {msg}"
        );
    }
    // app_status reports it as a normal result; list_screens works without the app.
    let r = mcp.tool("app_status", json!({}));
    assert_eq!(data(&r)["running"], false);
    assert!(text(&r).starts_with("Structures Playground isn't running"));
    let r = mcp.tool("list_screens", json!({}));
    assert_eq!(data(&r)["screens"].as_array().unwrap().len(), 3);
    assert!(data(&r)["current"].is_null());
    mcp.close();
}

/// Arguments are checked against the tools' schemas before anything reaches the app.
#[test]
fn builder_tools_check_their_arguments() {
    let (_s, mut mcp) = offline();
    for (tool, args) in [
        ("builder_level", json!({"level": "moon"})),
        ("builder_level", json!({})),
        ("builder_conditions", json!({"wind_ms": 99})),
        ("builder_conditions", json!({"soil": "mud"})),
        ("builder_conditions", json!({"rain": 3})),
        ("builder_fast_forward", json!({"years": 31})),
        ("sample_test", json!({"piece": "gold"})),
        ("builder_load_netlist", json!({})),
    ] {
        let r = mcp.tool(tool, args.clone());
        let msg = error(&r);
        assert!(
            !msg.starts_with("Structures Playground isn't running"),
            "{tool} {args}: checked before calling the app: {msg}"
        );
    }
    mcp.close();
}
