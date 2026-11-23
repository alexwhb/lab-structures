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
use sp_shared::proto;
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
    assert_eq!(s.screen.id, "bridge");
    assert_eq!(s.screen.label, "Bridge");
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
        json!({"screen": "ground"}),
    );
    assert_eq!(r["screen"]["id"], "ground");
    assert_eq!(h.state().route(), Route::Ground);
    assert!(
        has_label(&h, "foundation-pit"),
        "the header shows the Ground"
    );
}

#[test]
fn bad_requests_get_error_replies_and_change_nothing() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let p = sock.path();
    let bridge_before = h.state().screens().bridge.game.summary();
    let tower_before = h.state().screens().tower.input;
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
    let (c, m) = refused(call(
        &mut h,
        &p,
        proto::BRIDGE_LOAD_NETLIST,
        json!({"netlist": ".node A 0 0 fix=xy\n.member M1 A B s\n"}),
    ));
    assert_eq!(c, code::INVALID_PARAMS);
    assert!(m.contains("line 2"), "{m}");
    let (_, m) = refused(call(
        &mut h,
        &p,
        proto::BRIDGE_TEST,
        json!({"level": "moon"}),
    ));
    assert!(
        m.contains("levels are river, overhang, haul, sandbox"),
        "{m}"
    );
    let (_, m) = refused(call(&mut h, &p, proto::TOWER_SET, json!({"floors": 121})));
    assert_eq!(m, "`floors` must be 5 to 120, not 121");
    let (_, m) = refused(call(
        &mut h,
        &p,
        proto::TOWER_SET,
        json!({"system": "glass"}),
    ));
    assert!(m.contains("core, tube, outrigger, braced"), "{m}");
    let (_, m) = refused(call(&mut h, &p, proto::GROUND_SET, json!({"years": -1})));
    assert!(m.contains("0 to 30"), "{m}");
    let (_, m) = refused(call(
        &mut h,
        &p,
        proto::MATERIALS_SET,
        json!({"material": "gold"}),
    ));
    assert!(m.contains("unknown material"), "{m}");
    let (_, m) = refused(call(
        &mut h,
        &p,
        proto::MATERIALS_SET,
        json!({"temperature_c": 900}),
    ));
    assert!(m.contains("-40 to 600"), "{m}");
    let (_, m) = refused(call(
        &mut h,
        &p,
        proto::BRIDGE_TEST,
        json!({"colour": "red"}),
    ));
    assert!(m.contains("unknown field `colour`"), "{m}");

    assert_eq!(h.state().route(), Route::Bridge, "nothing moved");
    assert_eq!(h.state().screens().bridge.game.summary(), bridge_before);
    assert_eq!(
        h.state().screens().tower.input,
        tower_before,
        "nothing changed"
    );
}

#[test]
fn screenshot_is_the_rendered_frame() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let r = ok(
        &mut h,
        &sock.path(),
        "screenshot",
        json!({"screen": "materials"}),
    );
    let img: ImagePayload = serde_json::from_value(r).unwrap();
    assert_eq!(img.screen.id, "materials");
    assert_eq!((img.width, img.height), (1440, 900));
    assert_eq!(img.mime_type, "image/png");
    assert_eq!(h.state().route(), Route::Materials);
}

#[test]
fn describe_outlines_the_accesskit_tree() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let r = ok(&mut h, &sock.path(), "describe", json!({"screen": "tower"}));
    let summary = r["summary"].as_str().unwrap_or_default().to_string();
    let d: Description = serde_json::from_value(r).unwrap();
    assert_eq!(d.screen.id, "tower");
    for label in [
        "\"Structures Playground\"",
        "\"Play gusts\"",
        "\"Playground worlds\"",
    ] {
        assert!(d.outline.contains(label), "{label} in\n{}", d.outline);
    }
    assert!(!summary.is_empty(), "the screen's one-line summary");
}

#[test]
fn screen_methods_drive_the_screens() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let p = sock.path();
    // Tower in the background
    let r = ok(
        &mut h,
        &p,
        proto::TOWER_SET,
        json!({"floors": 90, "shape": "tapered", "show": false}),
    );
    assert_eq!(r["settings"]["floors"], 90);
    assert_eq!(h.state().route(), Route::Bridge, "show: false stays put");
    assert_eq!(h.state().screens().tower.input.floors, 90);
    // the same numbers the screen shows: its tiles
    goto(&mut h, Route::Tower);
    let tile = &r["tiles"][0];
    assert!(
        has_label(
            &h,
            &format!(
                "{}: {}",
                tile["label"].as_str().unwrap(),
                tile["value"].as_str().unwrap()
            )
        ) || h
            .query_all_by_label_contains(tile["value"].as_str().unwrap())
            .next()
            .is_some(),
        "the Tower shows {tile}"
    );
    // Ground
    let r = ok(
        &mut h,
        &p,
        proto::GROUND_SET,
        json!({"soil": "dense", "floors": 20}),
    );
    assert_eq!(r["settings"]["soil"], "dense");
    assert_eq!(h.state().route(), Route::Ground);
    assert_eq!(h.state().screens().ground.input.floors, 20);
    // Materials
    let r = ok(
        &mut h,
        &p,
        proto::MATERIALS_SET,
        json!({"material": "steel", "temperature_c": 300, "strain_pct": 5}),
    );
    assert_eq!(r["test"]["temperature_c"], 300.0);
    assert_eq!(r["test"]["strain_pct"], 5.0);
    assert_eq!(h.state().screens().materials.bench().temp(), 300.0);
    // Bridge: read, load the Share of a level, test
    let r = ok(&mut h, &p, proto::BRIDGE_GET, Value::Null);
    assert_eq!(r["level"], "river");
    assert_eq!(r["document"], "river-level");
    let r = ok(&mut h, &p, proto::BRIDGE_TEST, json!({"show": false}));
    assert_eq!(r["outcome"], "refused", "{r}");
}

/// A change to a screen never shown yet survives that screen's first visit.
#[test]
fn a_change_to_an_unvisited_screen_survives_its_first_visit() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    ok(
        &mut h,
        &sock.path(),
        proto::GROUND_SET,
        json!({"foundation": "caisson", "show": false}),
    );
    goto(&mut h, Route::Ground);
    assert_eq!(
        h.state().screens().ground.input.foundation,
        sp_engine::ground::Foundation::Caisson
    );
}

fn goto(h: &mut App, r: Route) {
    h.state_mut().navigate(r);
    h.run_steps(3);
}

#[test]
fn a_client_that_leaves_mid_screenshot_does_no_harm() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    {
        use std::io::Write;
        let mut raw = std::os::unix::net::UnixStream::connect(sock.path()).unwrap();
        raw.write_all(b"{\"id\":1,\"method\":\"screenshot\",\"params\":{\"screen\":\"tower\"}}\n")
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
    assert_eq!(r["screen"]["id"], "tower");
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
    raw.write_all(b"{\"id\":1,\"method\":\"bridge.get\"}\n")
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
    assert_eq!(reply["result"]["level"], "river");
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
