//! The app frame: the `.paper` ground, the header (NAV-1) drawn from what the active screen
//! declares (no world tab row in v2, NAV-2), the screen's body, toasts; global shortcuts and
//! the builder's keys as [`Command`]s (NAV-5); Share (NAV-6); navigation with hand-offs
//! (NAV-4), Back / Esc from the Series screen (NAV-3); persistence of the last screen (NAV-7).

use crate::ctx::{Carry, Command, Ctx, NavRequest, Outbox, Services};
use crate::launcher::Launcher;
use crate::router::Route;
use crate::screens::Screens;
use crate::widgets::Toasts;
use crate::widgets::header::{HeaderClicks, HeaderModel, header};
use eframe::egui::{self, Key, Modifiers, Ui};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Start-up options. [`AppOptions::default`] is the real app; [`AppOptions::test`] is
/// deterministic.
#[derive(Debug, Clone)]
pub struct AppOptions {
    /// The screen to open (unless a persisted one is restored).
    pub start: Route,
    /// Seed for every random thing (gusts, grains). Fixed in tests (NAV-7).
    pub rng_seed: u64,
    /// Screens may start animating when first shown.
    pub autoplay: bool,
    /// Where files are written when there's no save dialog.
    pub out_dir: PathBuf,
    /// Use the native save dialog (off in tests).
    pub save_dialog: bool,
    /// Opens sibling apps (Series, SR-1).
    pub launcher: Launcher,
    /// Restore the last route from eframe storage (NAV-7).
    pub load_persisted: bool,
    /// Where the live MCP bridge listens (`None`: no bridge; see `control.rs`). The real app
    /// listens on [`crate::control::default_socket_path`]; tests pass their own temp socket.
    pub mcp_socket: Option<PathBuf>,
}

impl Default for AppOptions {
    fn default() -> Self {
        let out_dir = std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Documents").join(crate::APP_NAME))
            .unwrap_or_else(|| std::env::temp_dir().join("structures-playground"));
        Self {
            start: Route::Playground,
            // NAV-7: a fresh start is reproducible; screens derive their seeds from this
            rng_seed: Self::DEFAULT_SEED,
            autoplay: false,
            out_dir,
            save_dialog: true,
            launcher: Launcher::system(),
            load_persisted: true,
            mcp_socket: crate::control::default_socket_path(),
        }
    }
}

impl AppOptions {
    /// The seed of a fresh start (NAV-7: no randomness without a seed).
    pub const DEFAULT_SEED: u64 = 0x5EED;
    /// The fixed seed tests use.
    pub const TEST_SEED: u64 = 7;

    /// Deterministic options for tests: seed 7, no autoplay, no persistence, files into a
    /// fresh temp folder, a recording launcher with nothing installed.
    pub fn test() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        Self {
            start: Route::Playground,
            rng_seed: Self::TEST_SEED,
            autoplay: false,
            out_dir: std::env::temp_dir().join(format!(
                "structures-playground-test-{}-{n}",
                std::process::id()
            )),
            save_dialog: false,
            launcher: Launcher::recording(&[]),
            load_persisted: false,
            mcp_socket: None,
        }
    }

    /// [`AppOptions::test`] starting on `route`.
    pub fn at(route: Route) -> Self {
        Self {
            start: route,
            ..Self::test()
        }
    }
}

const STORAGE_KEY: &str = "structures-playground";

/// What survives a restart (NAV-7; the window size is eframe's `persist_window`). A v1 save
/// (`{"route":"Tower","last_world":"Tower"}`) restores to the Playground: the retired routes
/// are aliases of [`Route::Playground`] and unknown fields are ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Persisted {
    /// The screen shown last.
    pub route: Route,
}

/// Something the app did on a screen's behalf (tests and the MCP bridge read the log).
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// The drive button was pressed (click or Space).
    Drive(Route),
    /// Reset was pressed.
    Reset(Route),
    /// Share copied the screen's document.
    Shared(Route),
    /// A hand-off was delivered to a screen (NAV-4).
    Carried(Route, Carry),
    /// A keyboard command was sent to a screen (NAV-5), and whether it handled it.
    Command(Route, Command, bool),
}

/// The app.
pub struct StructuresApp {
    route: Route,
    previous: Option<Route>,
    entered: Option<Route>,
    screens: Screens,
    toasts: Toasts,
    services: Services,
    outbox: Outbox,
    /// Hand-offs (NAV-4) and the screens they are for, delivered in order at the start of the
    /// next frame (two files opened together, or a hand-off and an MCP call, both arrive).
    pending_carry: Vec<(Route, Carry)>,
    content_height: f32,
    chrome_height: f32,
    events: Vec<AppEvent>,
    /// The options it was started with.
    pub options: AppOptions,
    /// The live MCP bridge (owned by `control.rs`).
    pub(crate) control: crate::control::Control,
    /// Drawing an offscreen frame ([`StructuresApp::render_offscreen`]): no input, no
    /// per-frame work.
    offscreen: bool,
}

impl StructuresApp {
    /// Builds the app from eframe's creation context (installs fonts + style, restores the
    /// last screen).
    pub fn new(cc: &eframe::CreationContext<'_>, opts: AppOptions) -> Self {
        sp_theme::install(&cc.egui_ctx);
        let mut app = Self::with_options(opts);
        if app.options.load_persisted
            && let Some(storage) = cc.storage
            && let Some(p) = eframe::get_value::<Persisted>(storage, STORAGE_KEY)
        {
            app.restore(p);
        }
        app
    }

    /// Builds the app without an eframe context (call [`sp_theme::install`] yourself).
    pub fn with_options(opts: AppOptions) -> Self {
        let services = Services {
            launcher: opts.launcher.clone(),
            out_dir: opts.out_dir.clone(),
            save_dialog: opts.save_dialog,
            rng_seed: opts.rng_seed,
            autoplay: opts.autoplay,
        };
        let route = opts.start;
        Self {
            route,
            previous: None,
            entered: None,
            screens: Screens::new(),
            toasts: Toasts::default(),
            services,
            outbox: Outbox::default(),
            pending_carry: Vec::new(),
            content_height: 0.0,
            chrome_height: 0.0,
            events: Vec::new(),
            control: crate::control::Control::start(opts.mcp_socket.as_deref()),
            offscreen: false,
            options: opts,
        }
    }

    /// What would be persisted now.
    pub fn persisted(&self) -> Persisted {
        Persisted { route: self.route }
    }

    /// Restores a persisted state (the Series screen restores to the Playground: it isn't a
    /// place to start in).
    pub fn restore(&mut self, p: Persisted) {
        self.route = if p.route == Route::Series {
            Route::Playground
        } else {
            p.route
        };
        self.entered = None;
    }

    /// The current screen.
    pub fn route(&self) -> Route {
        self.route
    }

    /// The screen before this one (where Series' Back goes).
    pub fn previous(&self) -> Option<Route> {
        self.previous
    }

    /// Go to `route` (what the header does).
    pub fn navigate(&mut self, route: Route) {
        self.apply(NavRequest { route, carry: None });
    }

    /// Go to `route` handing it a carry-over (NAV-4).
    pub fn navigate_with(&mut self, route: Route, carry: Carry) {
        self.apply(NavRequest {
            route,
            carry: Some(carry),
        });
    }

    fn apply(&mut self, req: NavRequest) {
        if req.route != self.route {
            self.entered = None;
            // Series' Back returns to where it was opened from, not to itself
            if self.route != Route::Series {
                self.previous = Some(self.route);
            }
        }
        self.route = req.route;
        // delivered after `enter`, at the start of the next frame; a navigation without one
        // that comes first doesn't drop it
        if let Some(carry) = req.carry {
            self.pending_carry.push((req.route, carry));
        }
    }

    /// All screens' state.
    pub fn screens(&self) -> &Screens {
        &self.screens
    }

    /// All screens' state, mutably (tests, the MCP bridge).
    pub fn screens_mut(&mut self) -> &mut Screens {
        &mut self.screens
    }

    /// Toasts queued or showing.
    pub fn toasts(&self) -> &Toasts {
        &self.toasts
    }

    /// Queue a toast.
    pub fn toast(&mut self, text: impl Into<String>) {
        self.toasts.push(text);
    }

    /// The last text a screen copied to the clipboard (Share, NAV-6).
    pub fn last_copied(&self) -> Option<&str> {
        self.outbox.last_copied.as_deref()
    }

    /// Files saved so far.
    pub fn saved_files(&self) -> &[PathBuf] {
        &self.outbox.saved
    }

    /// The services (launcher, output folder, seed).
    pub fn services(&self) -> &Services {
        &self.services
    }

    /// What the app did on screens' behalf so far (drive / Reset / Share presses, hand-offs).
    pub fn events(&self) -> &[AppEvent] {
        &self.events
    }

    /// The page height as of the last frame: chrome + body content (screenshot tests size the
    /// window with it).
    pub fn content_height(&self) -> f32 {
        self.content_height
    }

    /// The header height as of the last frame.
    pub fn chrome_height(&self) -> f32 {
        self.chrome_height
    }

    /// Presses the current screen's drive button, as a click or Space would (the smoke run,
    /// the MCP bridge), with the app's real egui context so the screen's pacing (`animate`,
    /// `wake_in`), clipboard and toasts reach the window; asks for a frame. Returns whether
    /// there was an enabled drive button.
    pub fn press_drive(&mut self, egui_ctx: &egui::Context) -> bool {
        let route = self.route;
        let body_h = (egui_ctx.content_rect().height() - self.chrome_height).max(0.0);
        let (mut ctx, screens) = self.ctx(egui_ctx, body_h);
        let screen = screens.get(route);
        if !screen.drive_button().is_some_and(|d| d.enabled) {
            return false;
        }
        screen.drive(&mut ctx);
        let nav = ctx.nav.take();
        self.events.push(AppEvent::Drive(route));
        if let Some(req) = nav {
            self.apply(req);
        }
        egui_ctx.request_repaint();
        true
    }

    /// The document name the header shows for the current screen.
    pub fn document(&self) -> String {
        self.screens
            .get_ref(self.route)
            .document()
            .unwrap_or_else(|| self.route.document().to_string())
    }

    fn ctx<'a>(&'a mut self, egui: &egui::Context, body_height: f32) -> (Ctx<'a>, &'a mut Screens) {
        (
            Ctx {
                route: self.route,
                previous: self.previous,
                nav: None,
                toasts: &mut self.toasts,
                services: &self.services,
                outbox: &mut self.outbox,
                egui: egui.clone(),
                body_height,
            },
            &mut self.screens,
        )
    }

    /// Runs `route`'s [`Screen::enter`](crate::Screen::enter) now, whether or not it is
    /// showing: the MCP bridge calls it before driving a screen in the background, so the
    /// screen's first-visit set-up doesn't undo the change later. Screens' `enter` is safe to
    /// call again.
    pub(crate) fn enter_screen(&mut self, route: Route, egui_ctx: &egui::Context) {
        let body_h = (egui_ctx.content_rect().height() - self.chrome_height).max(0.0);
        let (mut ctx, screens) = self.ctx(egui_ctx, body_h);
        screens.get(route).enter(&mut ctx);
    }

    /// Where the live MCP bridge listens, if it is on.
    pub fn mcp_socket(&self) -> Option<&std::path::Path> {
        self.control.socket()
    }

    /// Enters the current route now if it hasn't been yet (the bridge before drawing a screen
    /// offscreen): the next frame won't enter it again.
    pub(crate) fn enter_current(&mut self, egui_ctx: &egui::Context) {
        if self.entered != Some(self.route) {
            self.enter_screen(self.route, egui_ctx);
            self.entered = Some(self.route);
        }
    }

    /// Draws the current screen offscreen as the window would show it now (the MCP
    /// `screenshot` and `describe`, which must answer when the window gets no frames: covered
    /// by another app's window, or minimised). The same UI code runs in a second egui context
    /// at `main`'s logical size, pixels per point and time, [`OFFSCREEN_PASSES`] times so
    /// measured layouts settle; that frame skips enter, hand-offs, keys, `tick` and
    /// navigation and gets no input, so nothing moves, clicks or takes focus. Rendered by
    /// egui_kittest's wgpu renderer on its own device (made once, kept in `offscreen`), with the
    /// AccessKit tree when `accesskit`.
    pub fn render_offscreen(
        &mut self,
        main: &egui::Context,
        offscreen: &mut Option<Offscreen>,
        accesskit: bool,
    ) -> Result<OffscreenShot, String> {
        if offscreen.is_none() {
            *offscreen = Some(Offscreen::new()?);
        }
        let Some(off) = offscreen.as_mut() else {
            return Err("no offscreen renderer".into());
        };
        if accesskit {
            off.ctx.enable_accesskit();
        }
        let size = main.content_rect().size();
        let ppp = main.pixels_per_point();
        let time = main.input(|i| i.time);
        let mut last = None;
        self.offscreen = true;
        for _ in 0..OFFSCREEN_PASSES {
            let mut raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                time: Some(time),
                ..Default::default()
            };
            raw.viewports
                .entry(egui::ViewportId::ROOT)
                .or_default()
                .native_pixels_per_point = Some(ppp);
            let mut out = off.ctx.run_ui(raw, |ui| self.frame(ui));
            use egui_kittest::TestRenderer as _;
            off.renderer.handle_delta(&mut out.textures_delta);
            last = Some(out);
        }
        self.offscreen = false;
        let Some(out) = last else {
            return Err("no frame drawn".into());
        };
        use egui_kittest::TestRenderer as _;
        let renderer = &mut off.renderer;
        let ctx = &off.ctx;
        let image =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| renderer.render(ctx, &out)))
                .map_err(|_| "the renderer panicked".to_string())??;
        Ok(OffscreenShot {
            width: image.width(),
            height: image.height(),
            rgba: image.into_raw(),
            tree: out.platform_output.accesskit_update,
        })
    }

    /// One frame of the whole app (also called directly by tests).
    pub fn frame(&mut self, ui: &mut Ui) {
        let offscreen = self.offscreen;
        if !offscreen {
            crate::control::poll(self, ui);
        }
        let egui_ctx = ui.ctx().clone();
        // netlists opened from Finder / the command line go to Advanced
        if !offscreen {
            crate::platform::deliver(self, &egui_ctx);
        }
        let full = ui.max_rect();
        let body_h = (full.height() - self.chrome_height).max(0.0);
        let route = self.route;

        // ---- enter + hand-off, shortcuts, Space, tick (not in an offscreen frame)
        let mut nav: Option<NavRequest> = None;
        let mut events = Vec::new();
        if !offscreen {
            let entered = self.entered;
            let carries = std::mem::take(&mut self.pending_carry);
            let (mut ctx, screens) = self.ctx(&egui_ctx, body_h);
            // a hand-off whose screen was left before it arrived still reaches that screen
            let mut here = Vec::new();
            for (target, c) in carries {
                if target == route {
                    here.push(c);
                } else {
                    let other = screens.get(target);
                    other.enter(&mut ctx);
                    events.push(AppEvent::Carried(target, c.clone()));
                    other.receive(c, &mut ctx);
                }
            }
            let screen = screens.get(route);
            if entered != Some(route) {
                screen.enter(&mut ctx);
            }
            for c in here {
                events.push(AppEvent::Carried(route, c.clone()));
                screen.receive(c, &mut ctx);
            }
            if let Some(r) = shortcut_route(&egui_ctx) {
                ctx.go(r);
            }
            let keys = keys(&egui_ctx, route);
            if keys.back {
                ctx.back();
            }
            for cmd in keys.commands {
                let handled = screen.command(cmd.clone(), &mut ctx);
                events.push(AppEvent::Command(route, cmd, handled));
            }
            if keys.drive && screen.drive_button().is_some_and(|d| d.enabled) {
                events.push(AppEvent::Drive(route));
                screen.drive(&mut ctx);
            }
            // once per frame: a discarded pass (multi-pass layout) re-runs with the same dt
            if egui_ctx.current_pass_index() == 0 {
                screen.tick(&mut ctx);
            }
            nav = nav.or(ctx.nav.take());
            self.entered = Some(route);
        }

        // ---- the page: ground, header, body (scrolls as a whole when columns stack)
        lab_ui::paint::paint_grid(ui.painter(), full, full.min, &sp_theme::PAPER);
        let document = self.document();
        let mut chrome_h = self.chrome_height;
        let mut content_h: f32 = 0.0;
        {
            let (mut ctx, screens) = self.ctx(&egui_ctx, body_h);
            let screen = screens.get(route);
            let out = egui::ScrollArea::vertical()
                .id_salt(("page", route.id()))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                    let top = ui.cursor().top();
                    if route.has_header() {
                        let model = HeaderModel {
                            route,
                            document: &document,
                            drive: screen.drive_button(),
                            reset: screen.has_reset(),
                            clock: screen.clock(),
                        };
                        let (_, clicks) = header(ui, &model);
                        handle_header(clicks, screen, &mut ctx, &mut events, &document);
                    }
                    chrome_h = ui.cursor().top() - top;
                    screen.ui(&mut ctx, ui);
                    ui.cursor().top() - top
                });
            content_h = content_h.max(out.inner).max(out.content_size.y);
            nav = ctx.nav.take().or(nav);
        }
        if (chrome_h - self.chrome_height).abs() > 0.5 {
            // columns size themselves from the chrome height: redo the pass with the new one
            egui_ctx.request_discard("chrome height changed");
        }
        self.toasts.show(&egui_ctx);
        if offscreen {
            return;
        }
        self.chrome_height = chrome_h;
        self.content_height = content_h;
        self.events.extend(events);
        if let Some(req) = nav {
            self.apply(req);
            egui_ctx.request_repaint();
        }
    }
}

/// Offscreen frames drawn per screenshot / description (the last is the one taken): enough
/// for layouts measured the pass before (remembered sizes, stacked columns) to settle.
pub const OFFSCREEN_PASSES: usize = 3;

/// The offscreen context and its wgpu renderer ([`StructuresApp::render_offscreen`]).
pub struct Offscreen {
    ctx: egui::Context,
    renderer: egui_kittest::wgpu::WgpuTestRenderer,
}

impl Offscreen {
    /// A context with the app's fonts and style, and a renderer on its own wgpu device.
    pub fn new() -> Result<Self, String> {
        let renderer = std::panic::catch_unwind(egui_kittest::wgpu::WgpuTestRenderer::new)
            .map_err(|_| "no wgpu device for offscreen rendering".to_string())?;
        let ctx = egui::Context::default();
        sp_theme::install(&ctx);
        Ok(Self { ctx, renderer })
    }
}

/// An offscreen frame: physical pixels (RGBA, unpremultiplied) and the AccessKit tree.
pub struct OffscreenShot {
    /// Width, px.
    pub width: u32,
    /// Height, px.
    pub height: u32,
    /// `width × height × 4` bytes.
    pub rgba: Vec<u8>,
    /// The frame's AccessKit tree, when asked for.
    pub tree: Option<egui::accesskit::TreeUpdate>,
}

/// Acts on the header's clicks: brand → Series (NAV-3), modes, drive, Reset, Share (NAV-6).
fn handle_header(
    clicks: HeaderClicks,
    screen: &mut dyn crate::Screen,
    ctx: &mut Ctx,
    events: &mut Vec<AppEvent>,
    document: &str,
) {
    let route = ctx.route();
    if clicks.brand {
        ctx.go(Route::Series);
    }
    if let Some(mode) = clicks.mode
        && route.mode() != Some(mode)
    {
        ctx.go(mode.route());
    }
    if clicks.drive {
        events.push(AppEvent::Drive(route));
        screen.drive(ctx);
    }
    if clicks.reset {
        events.push(AppEvent::Reset(route));
        screen.reset(ctx);
    }
    if clicks.share {
        match screen.share(ctx) {
            Some(share) => {
                ctx.copy(share.text);
                ctx.toast(
                    share
                        .toast
                        .unwrap_or_else(|| format!("Copied {document} to the clipboard")),
                );
                events.push(AppEvent::Shared(route));
            }
            None => ctx.toast("Nothing to share yet"),
        }
    }
}

/// ⌘1 Playground, ⌘2 Advanced (NAV-5).
fn shortcut_route(ctx: &egui::Context) -> Option<Route> {
    const KEYS: [(Key, u8); 2] = [(Key::Num1, 1), (Key::Num2, 2)];
    ctx.input_mut(|i| {
        KEYS.iter().find_map(|(k, d)| {
            i.consume_key(Modifiers::COMMAND, *k)
                .then(|| {
                    Route::ALL
                        .into_iter()
                        .find(|r| r.shortcut_digit() == Some(*d))
                })
                .flatten()
        })
    })
}

/// The keys the app handles this frame.
#[derive(Debug, Default)]
struct Keys {
    /// Space (see [`keys`]): the drive button.
    drive: bool,
    /// Esc on the Series screen: Back.
    back: bool,
    /// An arrow key became a command.
    arrows: bool,
    commands: Vec<Command>,
}

/// Space → drive: one press (key repeats don't toggle it again), with Shift or no modifier,
/// when no widget has focus or the focused one lets Space through (a slider:
/// [`crate::widgets::space_passes_through`]); a focused button, tab or checkbox takes Space
/// itself, and a text field types it. Esc on Series → Back. ⌘Z (not ⌘⇧Z), Delete / Backspace
/// and ⌘V (no text field focused) → [`Command`]s. On the Playground also (no text field
/// focused): ⌘= / ⌘+ / ⌘− / ⌘0 → zoom commands (instead of egui's whole-window zoom), Esc →
/// [`Command::Escape`], and the arrow keys → [`Command::Pan`] / ⌥ [`Command::Nudge`] while no
/// widget has focus or the focused one asked for them ([`crate::widgets::arrows_pass_through`];
/// otherwise they move keyboard focus, egui's way).
fn keys(ctx: &egui::Context, route: Route) -> Keys {
    let mut out = Keys::default();
    let focused = ctx.memory(|m| m.focused());
    let typing = ctx.text_edit_focused();
    let space_free = !typing && focused.is_none_or(|id| crate::widgets::space_passes(ctx, id));
    let arrows_free = !typing && focused.is_none_or(|id| crate::widgets::arrows_pass(ctx, id));
    ctx.input_mut(|i| {
        if space_free {
            let mut pressed = false;
            i.events.retain(|e| match e {
                egui::Event::Key {
                    key: Key::Space,
                    pressed: true,
                    repeat,
                    modifiers,
                    ..
                } if !(modifiers.command || modifiers.ctrl || modifiers.alt) => {
                    pressed |= !repeat;
                    false
                }
                _ => true,
            });
            out.drive = pressed;
        }
        if route == Route::Series && i.consume_key(Modifiers::NONE, Key::Escape) {
            out.back = true;
        }
        if typing {
            return;
        }
        // ⌘Z exactly: ⌘⇧Z is redo (not offered), never undo
        if !i.modifiers.shift && i.consume_key(Modifiers::COMMAND, Key::Z) {
            out.commands.push(Command::Undo);
        }
        if i.consume_key(Modifiers::NONE, Key::Delete)
            || i.consume_key(Modifiers::NONE, Key::Backspace)
        {
            out.commands.push(Command::Delete);
        }
        let mut pasted = None;
        i.events.retain(|e| match e {
            egui::Event::Paste(text) if pasted.is_none() => {
                pasted = Some(text.clone());
                false
            }
            _ => true,
        });
        if let Some(text) = pasted {
            out.commands.push(Command::Paste(text));
        }
        if route != Route::Playground {
            return;
        }
        // the builder's keys (BLD-2, NAV-5)
        if i.consume_key(Modifiers::COMMAND, Key::Equals)
            || i.consume_key(Modifiers::COMMAND, Key::Plus)
        {
            out.commands.push(Command::ZoomIn);
        }
        if i.consume_key(Modifiers::COMMAND, Key::Minus) {
            out.commands.push(Command::ZoomOut);
        }
        if i.consume_key(Modifiers::COMMAND, Key::Num0) {
            out.commands.push(Command::ZoomFit);
        }
        if i.consume_key(Modifiers::NONE, Key::Escape) {
            out.commands.push(Command::Escape);
        }
        if arrows_free {
            const ARROWS: [(Key, i8, i8); 4] = [
                (Key::ArrowLeft, -1, 0),
                (Key::ArrowRight, 1, 0),
                (Key::ArrowUp, 0, 1),
                (Key::ArrowDown, 0, -1),
            ];
            for (key, dx, dy) in ARROWS {
                if i.consume_key(Modifiers::ALT, key) {
                    out.commands.push(Command::Nudge { dx, dy });
                    out.arrows = true;
                } else if i.consume_key(Modifiers::NONE, key) {
                    out.commands.push(Command::Pan { dx, dy });
                    out.arrows = true;
                }
            }
        }
    });
    if out.arrows && focused.is_some() {
        // egui honours a focus lock only from the widget's second focused frame: on the first
        // its arrow-key navigation would move focus on — cancel that
        ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
    }
    out
}

impl eframe::App for StructuresApp {
    /// Runs even while the window is hidden; lets the MCP bridge ask for a frame (`control.rs`).
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::control::logic(self, ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.frame(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, STORAGE_KEY, &self.persisted());
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::to_normalized_gamma_f32(sp_theme::PAPER.base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_are_deterministic() {
        let (a, b) = (AppOptions::test(), AppOptions::test());
        assert_eq!(a.rng_seed, AppOptions::TEST_SEED);
        assert!(!a.autoplay && !a.save_dialog && !a.load_persisted);
        assert_ne!(a.out_dir, b.out_dir, "each test app gets its own folder");
        assert_eq!(AppOptions::at(Route::Advanced).start, Route::Advanced);
        // NAV-7: a fresh start is reproducible
        assert_eq!(
            AppOptions::default().rng_seed,
            AppOptions::default().rng_seed
        );
        assert_eq!(AppOptions::default().start, Route::Playground);
    }

    #[test]
    fn save_and_copy_go_through_the_outbox() {
        let mut app = StructuresApp::with_options(AppOptions::test());
        let egui_ctx = egui::Context::default();
        let path = {
            let (mut c, _) = app.ctx(&egui_ctx, 600.0);
            c.copy(".node A 0 0 fix=xy");
            c.toast("Copied");
            c.save_text("river-crossing.str", ".node A 0 0 fix=xy\n")
                .expect("write")
                .expect("not cancelled")
        };
        assert_eq!(app.last_copied(), Some(".node A 0 0 fix=xy"));
        assert_eq!(app.saved_files(), std::slice::from_ref(&path));
        assert!(path.starts_with(&app.services().out_dir));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            ".node A 0 0 fix=xy\n"
        );
        assert_eq!(app.toasts().items().len(), 1);
        let _ = std::fs::remove_dir_all(&app.services().out_dir);
    }

    #[test]
    fn navigation_rules() {
        let mut app = StructuresApp::with_options(AppOptions::at(Route::Advanced));
        app.navigate(Route::Playground);
        app.navigate(Route::Advanced);
        assert_eq!(app.previous(), Some(Route::Playground));
        app.navigate(Route::Series);
        assert_eq!(app.previous(), Some(Route::Advanced));
        // Back from Series: where it was opened from
        app.navigate(Route::Series);
        assert_eq!(app.previous(), Some(Route::Advanced), "Series → Series");
    }

    #[test]
    fn persistence_round_trip() {
        let mut app = StructuresApp::with_options(AppOptions::test());
        app.navigate(Route::Advanced);
        let p = app.persisted();
        assert_eq!(
            p,
            Persisted {
                route: Route::Advanced
            }
        );
        let json = serde_json::to_string(&p).unwrap();
        let back: Persisted = serde_json::from_str(&json).unwrap();
        let mut fresh = StructuresApp::with_options(AppOptions::test());
        fresh.restore(back);
        assert_eq!(fresh.route(), Route::Advanced);
        // Series isn't a place to start in
        fresh.restore(Persisted {
            route: Route::Series,
        });
        assert_eq!(fresh.route(), Route::Playground);
    }

    /// A v1 save names a retired screen: it restores to the Playground (and keeps Advanced).
    #[test]
    fn a_v1_save_restores_to_the_playground() {
        for (json, route) in [
            (
                r#"{"route":"Tower","last_world":"Tower"}"#,
                Route::Playground,
            ),
            (
                r#"{"route":"Bridge","last_world":"Bridge"}"#,
                Route::Playground,
            ),
            (
                r#"{"route":"Materials","last_world":"Ground"}"#,
                Route::Playground,
            ),
            (
                r#"{"route":"Advanced","last_world":"Ground"}"#,
                Route::Advanced,
            ),
            (
                r#"{"route":"Series","last_world":"Tower"}"#,
                Route::Playground,
            ),
        ] {
            let p: Persisted = serde_json::from_str(json).expect(json);
            let mut app = StructuresApp::with_options(AppOptions::at(Route::Advanced));
            app.restore(p);
            assert_eq!(app.route(), route, "{json}");
        }
    }
}
