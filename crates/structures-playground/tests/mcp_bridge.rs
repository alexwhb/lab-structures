//! The app side of the live MCP bridge (`control.rs`, SPEC MCP-2): a real `StructuresApp` in an egui_kittest
//! harness listens on a temp socket, and a `lab_mcp` bridge client drives it. The harness is
//! stepped while the client waits, as frames would run in the real app. Changes are checked
//! both on the screens' state and through the GUI (AccessKit labels).
//!
//! The full stack (the real `structures-mcp` binary speaking MCP) is tested in
//! `crates/sp-mcp/tests/e2e.rs`.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use lab_mcp::bridge::proto::{AppStatus, Description, ImagePayload};
use lab_mcp::bridge::{BridgeClient, BridgeError, code};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use structures_playground::{AppOptions, Route, StructuresApp};

type App = Harness<'static, StructuresApp>;

/// A private temp dir with a socket path in it, removed on drop.
struct TempSocket {
    dir: PathBuf,
}

impl TempSocket {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "sp-bridge-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn path(&self) -> PathBuf {
        self.dir.join("app.sock")
    }
}

impl Drop for TempSocket {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn harness_with(socket: &Path, opts: AppOptions) -> App {
    let opts = AppOptions {
        mcp_socket: Some(socket.to_path_buf()),
        ..opts
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(64)
        .wgpu()
        .build_eframe(move |cc| StructuresApp::new(cc, opts));
    h.run_steps(3);
    h
}

fn harness(socket: &Path) -> App {
    harness_with(socket, AppOptions::test())
}

/// Calls the app from another thread, stepping frames until the reply arrives.
fn call(h: &mut App, socket: &Path, method: &str, params: Value) -> Result<Value, BridgeError> {
    let (socket, method) = (socket.to_path_buf(), method.to_owned());
    let client = thread::spawn(move || {
        BridgeClient::connect(&socket)?.call(&method, params, Duration::from_secs(15))
    });
    let t0 = Instant::now();
    while !client.is_finished() {
        assert!(t0.elapsed() < Duration::from_secs(20), "no reply");
        h.step();
        thread::sleep(Duration::from_millis(2));
    }
    let r = client.join().unwrap();
    h.run_steps(2); // let the GUI draw the change
    r
}

fn ok(h: &mut App, socket: &Path, method: &str, params: Value) -> Value {
    call(h, socket, method, params).unwrap_or_else(|e| panic!("{method}: {e}"))
}

fn refused(r: Result<Value, BridgeError>) -> (i64, String) {
    match r {
        Err(BridgeError::Remote { code, message, .. }) => (code, message),
        other => panic!("expected an error reply, got {other:?}"),
    }
}

fn has_label(h: &App, label: &str) -> bool {
    h.query_all_by_label(label).next().is_some()
}

#[test]
fn status_and_navigate() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());

    let s: AppStatus =
        serde_json::from_value(ok(&mut h, &sock.path(), "status", Value::Null)).unwrap();
    assert_eq!(s.app, "Structures Playground");
    assert_eq!(s.pid, std::process::id());
    assert_eq!(s.screen.id, "playground");
    assert_eq!(s.screen.label, "Playground");
    assert_eq!(s.screen.mode.as_ref().unwrap().id, "playground");
    let w = s.window.unwrap();
    assert_eq!(
        (w.width, w.height, w.pixels_per_point),
        (1440.0, 900.0, 1.0)
    );
    let r = ok(
        &mut h,
        &sock.path(),
        "navigate",
        json!({"screen": "advanced"}),
    );
    assert_eq!(r["screen"]["id"], "advanced");
    assert_eq!(h.state().route(), Route::Advanced);
    assert!(
        has_label(&h, "river-crossing"),
        "the header shows Advanced's netlist"
    );
    let r = ok(
        &mut h,
        &sock.path(),
        "navigate",
        json!({"screen": "builder"}),
    );
    assert_eq!(r["screen"]["id"], "playground", "`builder` names it too");
}

#[test]
fn bad_requests_get_error_replies_and_change_nothing() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let p = sock.path();
    let invalid = |r| refused(r).0;
    assert_eq!(
        invalid(call(&mut h, &p, "navigate", json!({"screen": "lab"}))),
        code::INVALID_PARAMS
    );
    assert_eq!(
        invalid(call(&mut h, &p, "navigate", json!({}))),
        code::INVALID_PARAMS
    );
    assert_eq!(
        invalid(call(&mut h, &p, "screenshot", json!({"screen": "nope"}))),
        code::INVALID_PARAMS
    );
    assert_eq!(
        invalid(call(&mut h, &p, "launch_rockets", Value::Null)),
        code::METHOD_NOT_FOUND
    );
    // the retired v1 screens say where their ideas went
    for old in ["bridge", "tower", "ground", "materials"] {
        let (c, m) = refused(call(&mut h, &p, "navigate", json!({"screen": old})));
        assert_eq!(c, code::INVALID_PARAMS);
        assert!(
            m.contains("retired in v2") && m.contains("playground"),
            "{m}"
        );
    }
    assert_eq!(h.state().route(), Route::Playground, "nothing moved");
}

#[test]
fn screenshot_is_the_rendered_frame() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let r = ok(
        &mut h,
        &sock.path(),
        "screenshot",
        json!({"screen": "advanced"}),
    );
    let img: ImagePayload = serde_json::from_value(r).unwrap();
    assert_eq!(img.screen.id, "advanced");
    assert_eq!((img.width, img.height), (1440, 900));
    assert_eq!(img.mime_type, "image/png");
    assert_eq!(h.state().route(), Route::Advanced);
}

#[test]
fn describe_outlines_the_accesskit_tree() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let r = ok(
        &mut h,
        &sock.path(),
        "describe",
        json!({"screen": "playground"}),
    );
    let summary = r["summary"].as_str().unwrap_or_default().to_string();
    let d: Description = serde_json::from_value(r).unwrap();
    assert_eq!(d.screen.id, "playground");
    for label in ["\"Structures Playground\"", "\"Run traffic\""] {
        assert!(d.outline.contains(label), "{label} in\n{}", d.outline);
    }
    assert!(
        !d.outline.contains("Playground worlds"),
        "no world tab row in v2:\n{}",
        d.outline
    );
    assert!(!summary.is_empty(), "the screen's one-line summary");
}

#[test]
fn a_client_that_leaves_mid_screenshot_does_no_harm() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    {
        use std::io::Write;
        let mut raw = std::os::unix::net::UnixStream::connect(sock.path()).unwrap();
        raw.write_all(
            b"{\"id\":1,\"method\":\"screenshot\",\"params\":{\"screen\":\"advanced\"}}\n",
        )
        .unwrap();
        // the app takes the request while the client is there (one that already left isn't
        // applied at all)…
        let ctx = h.ctx.clone();
        let t0 = Instant::now();
        while structures_playground::control::pending(h.state()) == 0 {
            assert!(
                t0.elapsed() < Duration::from_secs(5),
                "request never arrived"
            );
            structures_playground::control::logic(h.state_mut(), &ctx);
            thread::sleep(Duration::from_millis(2));
        }
        h.step();
    } // …and the client hangs up before the frame it waits for
    h.run_steps(5);
    let r = ok(&mut h, &sock.path(), "status", Value::Null);
    assert_eq!(r["screen"]["id"], "advanced");
}

/// Whether the last frame asked eframe to paint even if the window is hidden.
fn asks_for_paint_while_hidden(h: &App) -> bool {
    h.output()
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .is_some_and(|v| {
            v.commands
                .contains(&egui::ViewportCommand::RequestPaintWhileHidden)
        })
}

#[test]
fn a_hidden_window_asks_for_a_frame_to_answer_in() {
    use std::io::{BufRead, BufReader, Write};
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    h.step();
    assert!(!asks_for_paint_while_hidden(&h), "idle: no extra frames");

    // A request arrives while eframe runs only `App::logic`, as it does for a minimized or
    // covered window.
    let mut raw = std::os::unix::net::UnixStream::connect(sock.path()).unwrap();
    raw.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    raw.write_all(b"{\"id\":1,\"method\":\"status\"}\n")
        .unwrap();
    let ctx = h.ctx.clone();
    let t0 = Instant::now();
    while structures_playground::control::pending(h.state()) == 0 {
        assert!(
            t0.elapsed() < Duration::from_secs(5),
            "request never arrived"
        );
        structures_playground::control::logic(h.state_mut(), &ctx);
        thread::sleep(Duration::from_millis(2));
    }

    // The next frame carries `RequestPaintWhileHidden`, so eframe runs the UI and paints, and
    // the request is answered.
    h.step();
    assert!(asks_for_paint_while_hidden(&h));
    let mut line = String::new();
    BufReader::new(raw).read_line(&mut line).unwrap();
    let reply: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(reply["id"], 1);
    assert_eq!(reply["result"]["app"], "Structures Playground");
    assert_eq!(structures_playground::control::pending(h.state()), 0);
}

#[test]
fn a_second_instance_starts_without_a_bridge() {
    let sock = TempSocket::new();
    let mut first = harness(&sock.path());
    let mut second = harness(&sock.path()); // logs "MCP bridge off", runs normally
    second.run_steps(2);
    let r = ok(&mut first, &sock.path(), "status", Value::Null);
    assert_eq!(r["pid"], std::process::id());
    // A socket path that can't work doesn't stop the app either.
    let mut bad = harness(Path::new("/dev/null/app.sock"));
    bad.run_steps(2);
}

#[test]
fn defaults_and_shared_identity() {
    assert!(
        AppOptions::test().mcp_socket.is_none(),
        "tests never touch the real socket"
    );
    assert!(
        structures_playground::smoke::app_options()
            .mcp_socket
            .is_none()
    );
    assert_eq!(structures_playground::APP_ID, sp_shared::APP_ID);
    assert_eq!(structures_playground::APP_NAME, sp_shared::APP_NAME);
    if std::env::var_os(sp_shared::SOCKET_ENV).is_none() {
        let p = AppOptions::default()
            .mcp_socket
            .expect("the real app listens");
        assert!(p.as_os_str().len() <= lab_mcp::bridge::MAX_SOCKET_PATH);
        assert!(p.to_string_lossy().contains("StructuresPlayground"));
    }
    // The bridge's screen table is the router's.
    assert_eq!(Route::ALL.len(), sp_shared::SCREENS.len());
    for (route, s) in Route::ALL.iter().zip(sp_shared::SCREENS) {
        assert_eq!(route.id(), s.id);
        assert_eq!(route.title(), s.label);
        assert_eq!(route.document(), s.document);
        assert_eq!(route.mockup(), s.mockup);
        assert_eq!(route.board_size(), (s.board.width, s.board.height));
        assert_eq!(
            route.mode().map(|m| m.label()),
            s.mode.map(|m| m.label),
            "{route:?}"
        );
        assert_eq!(
            route.shortcut_digit().map(|d| format!("⌘{d}")).as_deref(),
            s.shortcut
        );
        assert_eq!(
            structures_playground::control::screen_ref(*route),
            s.screen_ref()
        );
    }
}

/// Calls the app from another thread while **no frame is ever drawn** — only `App::logic`
/// runs, as eframe does for a window covered by another app's (macOS gives it no frames).
fn call_without_frames(h: &mut App, socket: &Path, method: &str, params: Value) -> Value {
    let (socket, method) = (socket.to_path_buf(), method.to_owned());
    let client = thread::spawn(move || {
        BridgeClient::connect(&socket)?.call(&method, params, Duration::from_secs(15))
    });
    let ctx = h.ctx.clone();
    let t0 = Instant::now();
    while !client.is_finished() {
        assert!(
            t0.elapsed() < Duration::from_secs(10),
            "no reply without frames"
        );
        structures_playground::control::logic(h.state_mut(), &ctx);
        thread::sleep(Duration::from_millis(5));
    }
    client.join().unwrap().unwrap()
}

/// A covered window still answers `screenshot` and `describe` (rendered offscreen from the
/// app's state), and `navigate` too.
#[test]
fn a_covered_window_still_answers_screenshots() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let r = call_without_frames(
        &mut h,
        &sock.path(),
        "screenshot",
        json!({"screen": "advanced"}),
    );
    let img: ImagePayload = serde_json::from_value(r).unwrap();
    assert_eq!(img.screen.id, "advanced");
    assert_eq!((img.width, img.height), (1440, 900));
    let png = lab_mcp::base64_decode(&img.data).unwrap();
    let png = image::load_from_memory(&png).unwrap().to_rgba8();
    let colours: std::collections::HashSet<[u8; 4]> = png.pixels().map(|p| p.0).collect();
    assert!(
        colours.len() > 50,
        "a blank frame ({} colours)",
        colours.len()
    );
    assert_eq!(h.state().route(), Route::Advanced);
    let r = call_without_frames(
        &mut h,
        &sock.path(),
        "describe",
        json!({"screen": "playground"}),
    );
    let d: Description = serde_json::from_value(r).unwrap();
    assert_eq!(d.screen.id, "playground");
    for label in ["\"Structures Playground\"", "\"Run traffic\""] {
        assert!(d.outline.contains(label), "{label} in\n{}", d.outline);
    }
    let r = call_without_frames(
        &mut h,
        &sock.path(),
        "navigate",
        json!({"screen": "series"}),
    );
    assert_eq!(r["screen"]["id"], "series");
}

/// An offscreen screenshot doesn't advance a running simulation, fire events or move the
/// keyboard focus.
#[test]
fn an_offscreen_screenshot_changes_nothing() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    h.state_mut().navigate(Route::Advanced);
    h.run_steps(3);
    // Drive across is running; focus sits on a slider
    let ctx = h.ctx.clone();
    assert!(h.state_mut().press_drive(&ctx));
    h.step();
    h.get_by_role_and_label(egui::accesskit::Role::Slider, "Truck")
        .focus();
    h.step();
    assert!(
        h.state().screens().advanced.playing,
        "Drive across is running"
    );
    let x = h.state().screens().advanced.doc.x();
    let events = h.state().events().len();
    let focused = h.ctx.memory(|m| m.focused());
    // the screenshot is answered from `logic` alone, so no real frame runs in between
    let r = call_without_frames(&mut h, &sock.path(), "screenshot", Value::Null);
    assert_eq!(r["screen"]["id"], "advanced");
    assert_eq!(h.state().screens().advanced.doc.x(), x, "no tick");
    assert_eq!(h.state().events().len(), events, "no clicks or keys");
    assert_eq!(h.ctx.memory(|m| m.focused()), focused, "focus stays");
}

/// The builder's methods drive `BuilderScreen::game` through `sp_shared::builder` (MCP-2):
/// in the background with `show: false`, switching to the Playground otherwise; the header
/// shows the change.
#[test]
fn builder_methods_drive_the_builder() {
    use sp_shared::proto;
    let sock = TempSocket::new();
    let mut h = harness_with(&sock.path(), AppOptions::at(Route::Advanced));
    let p = sock.path();
    let r = ok(&mut h, &p, proto::BUILDER_GET, Value::Null);
    assert_eq!(r["level"], "river");
    assert_eq!(r["phase"], "build");
    assert_eq!(r["document"], "river-level");
    assert_eq!(h.state().route(), Route::Advanced, "get doesn't switch");

    let r = ok(
        &mut h,
        &p,
        proto::BUILDER_LEVEL,
        json!({"level": "t2", "show": false}),
    );
    assert_eq!(r["level"], "t2");
    assert_eq!(r["scene"], "flat");
    assert_eq!(h.state().route(), Route::Advanced, "show: false stays put");
    // the change survives the builder's first visit
    h.state_mut().navigate(Route::Playground);
    h.run_steps(3);
    assert!(has_label(&h, "t2-level"), "the header shows the level");

    let r = ok(
        &mut h,
        &p,
        proto::BUILDER_CONDITIONS,
        json!({"wind_ms": 25, "wind_from": "right", "snow_kg_m": 150}),
    );
    assert_eq!(r["conditions"]["wind"], "Strong");
    assert_eq!(r["conditions"]["wind_from"], "right");
    assert_eq!(r["conditions"]["snow"], "Light");
    assert_eq!(h.state().screens().builder.game.cond().wind, 25.0);

    // a structure from the builder's own Share loads into the Sandbox and runs
    let mut g = sp_engine::builder::game::Game::new();
    g.build_preset(sp_engine::builder::game::PresetBuild::Truss {
        chord: sp_engine::builder::data::PieceType::Steel,
    });
    let r = ok(
        &mut h,
        &p,
        proto::BUILDER_LOAD_NETLIST,
        json!({"netlist": g.share_text()}),
    );
    assert_eq!(r["level"], "sandbox");
    assert_eq!(r["joints"], g.design().nodes.len());
    assert!(has_label(&h, "sandbox"), "the header shows the Sandbox");
    let r = ok(&mut h, &p, proto::BUILDER_RUN, json!({}));
    assert!(r["t_s"].as_f64().unwrap() > 15.0, "{r}");

    // level 1 with nothing built: the cars can't cross
    ok(&mut h, &p, proto::BUILDER_LEVEL, json!({"level": "river"}));
    let r = ok(&mut h, &p, proto::BUILDER_RUN, json!({}));
    assert_eq!(r["pass"], false);
    assert_eq!(r["crossed"], 0);
    assert_eq!(h.state().screens().builder.game.phase().key(), "result");

    // the sample lab
    let r = ok(
        &mut h,
        &p,
        proto::SAMPLE_TEST,
        json!({"piece": "wood", "show": false}),
    );
    assert_eq!(r["broken"], true, "{r}");
    assert_eq!(r["piece"], "wood");
}

/// Bad builder arguments are error replies and change nothing.
#[test]
fn bad_builder_requests_change_nothing() {
    use sp_shared::proto;
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let p = sock.path();
    let before = ok(&mut h, &p, proto::BUILDER_GET, Value::Null);
    for (method, params, says) in [
        (
            proto::BUILDER_LEVEL,
            json!({"level": "moon"}),
            "levels are river",
        ),
        (
            proto::BUILDER_LEVEL,
            json!({"level": "river", "scene": "flat"}),
            "Sandbox only",
        ),
        (
            proto::BUILDER_LOAD_NETLIST,
            json!({"netlist": ".node A 1.5 0 fix=xy"}),
            "grid",
        ),
        (
            proto::BUILDER_CONDITIONS,
            json!({"wind_ms": 30, "soil": "mud"}),
            "unknown soil",
        ),
        (
            proto::BUILDER_CONDITIONS,
            json!({"quake_g": 2}),
            "`quake_g` must be 0 to 0.5",
        ),
        (
            proto::SAMPLE_TEST,
            json!({"piece": "rope", "mode": "push"}),
            "only pulls",
        ),
        (
            proto::BUILDER_RUN,
            json!({"colour": "red"}),
            "unknown field",
        ),
    ] {
        let (c, m) = refused(call(&mut h, &p, method, params.clone()));
        assert_eq!(c, code::INVALID_PARAMS, "{method} {params}");
        assert!(m.contains(says), "{method} {params}: {m}");
    }
    assert_eq!(ok(&mut h, &p, proto::BUILDER_GET, Value::Null), before);
}

/// With `show`, a sample test runs in the sample drawer (LAB-1) and a fast-forward plays on
/// the stage (TIME-1): the numbers are the headless ones.
#[test]
fn shown_samples_and_years_reach_the_fold_ins() {
    use sp_shared::proto;
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let p = sock.path();
    let r = ok(
        &mut h,
        &p,
        proto::SAMPLE_TEST,
        json!({"piece": "steel", "mode": "push"}),
    );
    let lab = &h.state().screens().builder.fold.lab;
    assert!(lab.is_open(), "the drawer opened");
    assert_eq!(lab.sample().kind().key(), "steel");
    assert_eq!(r["mode"], "push");
    // a tower that stands on soft clay, then 12 years
    let mut g = sp_engine::builder::game::Game::new();
    g.pick_level(sp_engine::builder::data::LevelKey::T1);
    g.build_preset(sp_engine::builder::game::PresetBuild::Tower {
        h: 40.0,
        bays: 2,
        col: sp_engine::builder::data::PieceType::Steel,
        brace: sp_engine::builder::data::PieceType::Wood,
        found: sp_engine::builder::data::Foundation::Pile,
    });
    let text = g.share_text();
    ok(
        &mut h,
        &p,
        proto::BUILDER_LOAD_NETLIST,
        json!({"netlist": text}),
    );
    ok(&mut h, &p, proto::BUILDER_RUN, json!({}));
    let r = ok(
        &mut h,
        &p,
        proto::BUILDER_FAST_FORWARD,
        json!({"years": 12}),
    );
    assert_eq!(r["years"]["years"], 12.0, "{r}");
    let (_, m) = refused(call(
        &mut h,
        &p,
        proto::BUILDER_FAST_FORWARD,
        json!({"years": 40}),
    ));
    assert!(m.contains("`years` must be 0 to 30"), "{m}");
}
