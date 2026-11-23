//! External control of the running app: the app side of the live MCP bridge (SPEC MCP-2).
//!
//! The app listens on a user-only Unix socket ([`lab_mcp::bridge::BridgeServer`]) at
//! [`AppOptions::mcp_socket`](crate::AppOptions::mcp_socket); `structures-mcp` connects to it
//! to act on the running app. [`poll`] runs at the start of every frame on the UI thread: it
//! takes the requests that arrived, applies them and sends the replies.
//!
//! | method | params | reply |
//! |---|---|---|
//! | `status` | none | screen, window size, pid, version |
//! | `navigate` | `{"screen": id}` | status after navigating |
//! | `screenshot` | `{"screen"?: id}` | PNG of the screen as it stands (base64) |
//! | `describe` | `{"screen"?: id}` | outline of its AccessKit tree + the screen's one-line `summary` |
//! | `bridge.get`, `bridge.load_netlist`, `bridge.test`, `tower.set`, `ground.set`, `materials.set` | see [`sp_shared::proto`] | the screen's state after the call |
//!
//! The screen methods drive the screens' public state (`BridgeScreen::game`,
//! `TowerScreen::set_input`, `GroundScreen::set_input`, `MaterialsScreen::bench_mut`; see
//! `docs/ARCHITECTURE.md`) through [`StructuresApp::screens_mut`]. Their params are checked
//! before anything changes; with `show` (the default) the app switches to that screen. A screen
//! that has never been shown is entered first ([`StructuresApp::enter_screen`]), so its
//! first-visit set-up happens before the change, not after it.
//!
//! **Screenshots and descriptions are rendered offscreen**, at once, from the app's real state
//! ([`StructuresApp::render_offscreen`]): the same UI code draws into a second egui context at
//! the window's logical size and pixels per point (a few passes so measured layouts settle),
//! egui_kittest's wgpu renderer turns it into pixels on its own device, and the AccessKit tree
//! of that pass is the description. Nothing waits for the window to paint, so a window that is
//! covered by another app's (macOS gives it no frames) or minimised still answers. The offscreen
//! pass has no input and skips the per-frame work (enter, hand-offs, keys, `tick`, navigation),
//! so it can't advance a simulation, click anything or move the focus. The PNG is encoded on a
//! worker thread.
//!
//! Backgrounded windows: every arriving request wakes the app (`ctx.request_repaint()` from the
//! bridge thread). While its window is covered or minimised, eframe may skip `App::ui` (so
//! [`poll`] doesn't run) and call only `App::logic`. That calls [`logic`], which asks for a frame
//! (`ViewportCommand::RequestPaintWhileHidden` plus a repaint); if no frame has polled the
//! bridge for [`STALL`], it answers the queued requests itself.
//!
//! Forked from Automata Playground's `control.rs` (which mirrors Chem Playground's); the
//! AccessKit outline is a candidate for `lab-mcp`.

use crate::app::StructuresApp;
use crate::router::Route;
use eframe::egui::{self, Context, Ui, accesskit};
use lab_mcp::bridge::proto::{
    self, AccessNode, AppStatus, Description, ImagePayload, ScreenParams, ScreenRef, WindowInfo,
};
use lab_mcp::bridge::{BridgeServer, PendingRequest, RemoteError, code};
use serde_json::Value;
use sp_engine::bridge::{LevelKey, Phase};
use sp_engine::materials::curves::Kind as Sample;
use sp_shared::proto::{self as sp, MaterialsState};
use sp_shared::{input, report};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How long requests may wait for a frame before [`logic`] answers them itself (a covered or
/// minimised window gets no frames).
pub const STALL: Duration = Duration::from_millis(300);

/// Bridge state owned by the app.
#[derive(Default)]
pub struct Control {
    server: Option<BridgeServer>,
    waker_installed: bool,
    /// Requests taken from the bridge by [`logic`], for the next [`poll`].
    inbox: Vec<PendingRequest>,
    /// Screens already entered for the bridge.
    prepared: Vec<Route>,
    /// When a frame last polled the bridge.
    last_poll: Option<Instant>,
    /// The offscreen renderer (made on the first screenshot or description).
    offscreen: Option<crate::app::Offscreen>,
}

impl Control {
    /// Starts listening on `socket` (or does nothing for `None`). Never fails the app start-up:
    /// problems (another instance owns the socket, a bad path) are logged and the app runs
    /// without a bridge.
    pub fn start(socket: Option<&Path>) -> Self {
        let Some(path) = socket else {
            return Self::default();
        };
        match BridgeServer::bind(path) {
            Ok(server) => Self {
                server: Some(server),
                ..Self::default()
            },
            Err(e) => {
                log::warn!("MCP bridge off: {e}");
                Self::default()
            }
        }
    }

    /// Where the bridge listens, if it is on.
    pub fn socket(&self) -> Option<&Path> {
        self.server.as_ref().map(BridgeServer::path)
    }
}

impl std::fmt::Debug for Control {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Control")
            .field("socket", &self.socket())
            .field("inbox", &self.inbox.len())
            .finish_non_exhaustive()
    }
}

/// The socket the real app listens on: `$STRUCTURES_MCP_SOCKET`, else the per-user path
/// (`None` when the variable is empty or `off`). See [`sp_shared::bridge_socket_path`].
pub fn default_socket_path() -> Option<PathBuf> {
    sp_shared::bridge_socket_path()
}

/// Called from `App::logic`: every frame before `App::ui`, and also while the window is hidden
/// (covered or minimised), when eframe may run no UI pass. Pending requests ask for a frame; if
/// none has polled the bridge for [`STALL`], they are answered here.
pub fn logic(app: &mut StructuresApp, ctx: &Context) {
    let control = &mut app.control;
    let Some(server) = control.server.as_ref() else {
        return;
    };
    install_waker(server, &mut control.waker_installed, ctx);
    control.inbox.extend(server.poll());
    if control.inbox.is_empty() {
        return;
    }
    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
    ctx.request_repaint();
    let stalled = control.last_poll.is_none_or(|t| t.elapsed() > STALL);
    if stalled {
        for req in std::mem::take(&mut app.control.inbox) {
            handle(app, ctx, req);
        }
    }
}

/// Called at the start of every frame (before the screen is entered and drawn), so a
/// navigation or a change takes effect in the same frame.
pub fn poll(app: &mut StructuresApp, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let control = &mut app.control;
    let Some(server) = control.server.as_ref() else {
        return;
    };
    install_waker(server, &mut control.waker_installed, &ctx);
    control.last_poll = Some(Instant::now());
    let mut requests = std::mem::take(&mut control.inbox);
    requests.extend(server.poll());
    let any = !requests.is_empty();
    for req in requests {
        handle(app, &ctx, req);
    }
    if any {
        ctx.request_repaint();
    }
}

/// Requests received but not yet answered.
pub fn pending(app: &StructuresApp) -> usize {
    app.control.inbox.len()
}

fn install_waker(server: &BridgeServer, installed: &mut bool, ctx: &Context) {
    if !*installed {
        let waker = ctx.clone();
        server.set_waker(move || waker.request_repaint());
        *installed = true;
    }
}

fn handle(app: &mut StructuresApp, ctx: &Context, req: PendingRequest) {
    if req.is_abandoned() {
        // the client gave up (a timeout) and hung up before the UI got here: applying the
        // call now would change the app behind its back (and twice after a retry)
        log::info!("bridge: skipped `{}`, its client left", req.method());
        return;
    }
    match req.method() {
        proto::STATUS => {
            let reply = to_value(&status(app, ctx));
            req.reply(reply);
        }
        proto::NAVIGATE => match screen_param(&req) {
            Ok(Some(route)) => {
                app.navigate(route);
                let reply = to_value(&status(app, ctx));
                req.reply(reply);
            }
            Ok(None) => req.reply(Err(RemoteError::invalid_params(format!(
                "`screen` is required: one of {}",
                sp_shared::screen_ids().join(", ")
            )))),
            Err(e) => req.reply(Err(e)),
        },
        method @ (proto::SCREENSHOT | proto::DESCRIBE) => {
            let describe = method == proto::DESCRIBE;
            match screen_param(&req) {
                Ok(Some(route)) if route != app.route() => app.navigate(route),
                Ok(_) => {}
                Err(e) => return req.reply(Err(e)),
            }
            // a screen shown for the first time is set up before it is drawn
            app.enter_current(ctx);
            let route = app.route();
            let mut offscreen = app.control.offscreen.take();
            let shot = app.render_offscreen(ctx, &mut offscreen, describe);
            app.control.offscreen = offscreen;
            match shot {
                Err(e) => req.reply(Err(RemoteError::new(
                    code::INTERNAL_ERROR,
                    format!(
                        "couldn't draw the window offscreen ({e}); bring Structures Playground \
                         to the front and try again"
                    ),
                ))),
                Ok(shot) if describe => {
                    let nodes = shot.tree.as_ref().map(outline_nodes).unwrap_or_default();
                    let d = Description {
                        screen: screen_ref(route),
                        outline: AccessNode::outline(&nodes),
                        nodes,
                    };
                    let summary = app.screens().get_ref(route).describe();
                    let reply = to_value(&d).map(|mut v| {
                        if let Value::Object(m) = &mut v {
                            m.insert("summary".into(), Value::String(summary));
                        }
                        v
                    });
                    req.reply(reply);
                }
                Ok(shot) => encode_screenshot(req, shot, screen_ref(route)),
            }
        }
        method if sp::METHODS.contains(&method) => {
            let reply = screen_method(app, ctx, &req);
            req.reply(reply);
        }
        other => {
            let err = RemoteError::method_not_found(other);
            req.reply(Err(err));
        }
    }
}

/// PNG-encodes an offscreen frame off the UI thread and replies.
fn encode_screenshot(req: PendingRequest, shot: crate::app::OffscreenShot, screen: ScreenRef) {
    let job = move || {
        let (w, h) = (shot.width, shot.height);
        let reply = ImagePayload::png_from_rgba(w, h, &shot.rgba, screen)
            .map_err(|e| {
                RemoteError::new(code::INTERNAL_ERROR, format!("PNG encoding failed: {e}"))
            })
            .and_then(|img| to_value(&img));
        req.reply(reply);
    };
    // If no thread can be spawned the request drops, which answers it with an error.
    let _ = std::thread::Builder::new()
        .name("structures-screenshot-png".into())
        .spawn(job);
}

fn status(app: &StructuresApp, ctx: &Context) -> AppStatus {
    let rect = ctx.content_rect();
    AppStatus {
        app: crate::APP_NAME.to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        pid: std::process::id(),
        screen: screen_ref(app.route()),
        window: Some(WindowInfo {
            width: rect.width(),
            height: rect.height(),
            pixels_per_point: ctx.pixels_per_point(),
        }),
    }
}

/// A route as the bridge reports it.
pub fn screen_ref(route: Route) -> ScreenRef {
    match sp_shared::screen(route.id()) {
        Some(s) => s.screen_ref(),
        None => ScreenRef {
            id: route.id().to_owned(),
            label: route.title().to_owned(),
            mode: None,
        },
    }
}

/// The optional `screen` param as a route (`Err` for an unknown id).
fn screen_param(req: &PendingRequest) -> Result<Option<Route>, RemoteError> {
    let p: ScreenParams = req.parse_params()?;
    match p.screen {
        None => Ok(None),
        Some(id) => Route::from_id(&id).map(Some).ok_or_else(|| {
            RemoteError::invalid_params(format!(
                "unknown screen `{id}`; screens are {}",
                sp_shared::screen_ids().join(", ")
            ))
        }),
    }
}

fn to_value(v: &impl serde::Serialize) -> Result<Value, RemoteError> {
    serde_json::to_value(v).map_err(|e| RemoteError::new(code::INTERNAL_ERROR, e.to_string()))
}

fn invalid(message: impl Into<String>) -> RemoteError {
    RemoteError::invalid_params(message)
}

// ---- the screens' methods -------------------------------------------------------------------

/// Enters `route` once (if the bridge hasn't yet) and, with `show`, switches to it.
fn prepare(app: &mut StructuresApp, ctx: &Context, route: Route, show: bool) {
    if !app.control.prepared.contains(&route) {
        app.enter_screen(route, ctx);
        app.control.prepared.push(route);
    }
    if show && app.route() != route {
        app.navigate(route);
    }
}

fn screen_method(
    app: &mut StructuresApp,
    ctx: &Context,
    req: &PendingRequest,
) -> Result<Value, RemoteError> {
    match req.method() {
        sp::BRIDGE_GET => {
            prepare(app, ctx, Route::Bridge, false);
            to_value(&report::bridge_state(&app.screens().bridge.game))
        }
        sp::BRIDGE_LOAD_NETLIST => {
            let p: sp::BridgeLoadNetlist = req.parse_params()?;
            if p.netlist.chars().count() > input::NETLIST_MAX_LEN {
                return Err(invalid(format!(
                    "the netlist is longer than {} characters",
                    input::NETLIST_MAX_LEN
                )));
            }
            let (design, mat) = sp_engine::bridge::netlist::read(&p.netlist).map_err(|e| {
                invalid(format!(
                    "{e}. The Bridge's Sandbox takes netlists whose nodes sit on its 2 m grid \
                     (x 0–58 m, y −12–16 m, nothing underground), like the one its Share copies; \
                     solve_truss solves any netlist without the app."
                ))
            })?;
            prepare(app, ctx, Route::Bridge, p.show);
            let game = &mut app.screens_mut().bridge.game;
            game.open_in_sandbox(design, mat);
            to_value(&report::bridge_state(game))
        }
        sp::BRIDGE_TEST => {
            let p: sp::BridgeTest = req.parse_params()?;
            let level = match &p.level {
                Some(k) => Some(
                    LevelKey::from_key(&k.trim().to_ascii_lowercase()).ok_or_else(|| {
                        invalid(format!(
                            "unknown level `{k}`; levels are {}",
                            LevelKey::ALL.map(|l| l.key()).join(", ")
                        ))
                    })?,
                ),
                None => None,
            };
            prepare(app, ctx, Route::Bridge, p.show);
            let game = &mut app.screens_mut().bridge.game;
            if let Some(l) = level {
                game.pick_level(l);
            }
            if matches!(game.phase(), Phase::Held | Phase::Collapsed) {
                game.back_to_build();
            }
            to_value(&report::run_bridge_test(game, sp::MAX_TEST_TICKS))
        }
        sp::TOWER_SET => {
            let p: sp::TowerSet = req.parse_params()?;
            let screen = &app.screens().tower;
            let next = p.change.apply(screen.input).map_err(invalid)?;
            prepare(app, ctx, Route::Tower, p.show);
            let screen = &mut app.screens_mut().tower;
            screen.set_input(next);
            to_value(&report::tower_report(&screen.input))
        }
        sp::GROUND_SET => {
            let p: sp::GroundSet = req.parse_params()?;
            let next = p
                .change
                .apply(app.screens().ground.input)
                .map_err(invalid)?;
            prepare(app, ctx, Route::Ground, p.show);
            let screen = &mut app.screens_mut().ground;
            screen.set_input(next);
            to_value(&report::ground_report(&screen.input))
        }
        sp::MATERIALS_SET => {
            let p: sp::MaterialsSet = req.parse_params()?;
            let kind = p
                .material
                .as_deref()
                .map(input::material)
                .transpose()
                .map_err(invalid)?;
            let temp = p
                .temperature_c
                .map(input::temperature)
                .transpose()
                .map_err(invalid)?;
            if let Some(e) = p.strain_pct {
                // the axis the sample will be on (picking a sample picks the axis that fits it)
                let zoom = match kind {
                    Some(k) => sp_engine::materials::bench::Zoom::fitting(k),
                    None => app.screens().materials.bench().zoom(),
                };
                if !(e.is_finite() && (0.0..=zoom.max()).contains(&e)) {
                    return Err(invalid(format!(
                        "`strain_pct` must be 0 to {} on the {} axis, not {e}",
                        zoom.max(),
                        zoom.label()
                    )));
                }
            }
            prepare(app, ctx, Route::Materials, p.show);
            let bench = app.screens_mut().materials.bench_mut();
            if let Some(k) = kind {
                bench.pick(k);
            }
            if let Some(t) = temp {
                bench.set_temp(t);
            }
            if p.fresh {
                bench.fresh_sample();
            }
            if let Some(e) = p.strain_pct {
                bench.set_strain(e);
            }
            let mut note = None;
            if p.pull {
                if !bench.playing() {
                    bench.toggle_drive();
                }
                let mut broke = None;
                for _ in 0..100_000 {
                    if !bench.playing() {
                        break;
                    }
                    if let Some(k) = bench.tick() {
                        broke = Some(k);
                    }
                }
                note = Some(match broke {
                    Some(k) => format!(
                        "Pulled the {} sample until it broke at {} %.",
                        k.name().to_lowercase(),
                        sp_engine::materials::jsnum::to_fixed(bench.strain(), 2)
                    ),
                    None => format!(
                        "Pulled to the end of the {} axis without breaking.",
                        bench.zoom().label()
                    ),
                });
            }
            let bench = app.screens().materials.bench();
            to_value(&MaterialsState {
                test: report::tensile_from_bench(bench, Some(201)),
                axis: bench.zoom().label().into(),
                broken_so_far: Sample::ALL
                    .into_iter()
                    .filter(|k| bench.broke(*k))
                    .map(|k| k.key().to_string())
                    .collect(),
                note,
            })
        }
        other => Err(RemoteError::method_not_found(other)),
    }
}

// ---- AccessKit ------------------------------------------------------------------------------

/// Flattens an AccessKit tree into outline nodes: depth-first from the root, skipping hidden
/// subtrees and unnamed generic containers (their children move up a level).
pub fn outline_nodes(update: &accesskit::TreeUpdate) -> Vec<AccessNode> {
    use std::collections::{HashMap, HashSet};
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        update.nodes.iter().map(|(id, n)| (*id, n)).collect();
    let Some(root) = update.tree.as_ref().map(|t| t.root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    // (node, depth); a stack, children pushed in reverse to keep document order.
    let mut stack = vec![(root, 0u32)];
    let mut seen = HashSet::new();
    while let Some((id, depth)) = stack.pop() {
        if !seen.insert(id) {
            continue; // a malformed tree with a cycle
        }
        let Some(node) = by_id.get(&id) else {
            continue;
        };
        if node.is_hidden() {
            continue;
        }
        let name = node.label().map(str::to_owned).filter(|s| !s.is_empty());
        let value = node
            .value()
            .map(str::to_owned)
            .or_else(|| node.numeric_value().map(|v| format!("{v}")))
            .filter(|s| !s.is_empty());
        let role = node.role();
        let skip = name.is_none()
            && value.is_none()
            && matches!(
                role,
                accesskit::Role::GenericContainer | accesskit::Role::Unknown
            );
        let child_depth = if skip { depth } else { depth + 1 };
        if !skip {
            let mut states = Vec::new();
            if node.is_selected() == Some(true) {
                states.push("selected".to_owned());
            }
            match node.toggled() {
                Some(accesskit::Toggled::True) => states.push("checked".to_owned()),
                Some(accesskit::Toggled::Mixed) => states.push("mixed".to_owned()),
                _ => {}
            }
            if node.is_disabled() {
                states.push("disabled".to_owned());
            }
            if update.focus == id && id != root {
                states.push("focused".to_owned());
            }
            out.push(AccessNode {
                depth,
                role: format!("{role:?}"),
                name,
                value,
                states,
            });
        }
        for child in node.children().iter().rev() {
            stack.push((*child, child_depth));
        }
    }
    out
}
