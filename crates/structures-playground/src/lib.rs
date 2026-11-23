//! Structures Playground — the egui app: the header (brand, mode switcher, the screen's drive
//! button / Reset / clock / Share), the widget kit drawn to the mockup, and the three screens
//! (v2): the Playground builder, Advanced and Series.
//!
//! Screens are specified by the mockups under `docs/design/` ([`Route::mockup`]). How a screen
//! plugs in (state, `ui`, header hooks, keyboard commands, hand-offs, toasts) is documented on
//! [`screens::Screen`] and in `docs/ARCHITECTURE.md` › App.

pub mod app;
pub mod control;
pub mod ctx;
pub mod launcher;
pub mod layout;
pub mod platform;
pub mod router;
pub mod screens;
pub mod smoke;
pub mod widgets;

pub use app::{AppOptions, StructuresApp};
pub use ctx::{Carry, Command, Ctx, Ticker};
pub use launcher::{AppId, Launch, Launcher};
pub use router::{Mode, Route};
pub use screens::{Badge, Drive, Provenance, Screen, Share};

/// The app's name (window title, brand).
pub const APP_NAME: &str = "Structures Playground";
/// The macOS bundle id.
pub const APP_ID: &str = "dev.labseries.StructuresPlayground";

/// When the process started (set first thing in `main`; the smoke run's `first_frame_ms`).
pub static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// This checkout's crate folder, except in bundle builds (`STRUCTURES_BUNDLE=1`, set by
/// `scripts/bundle-macos.sh`) so the release binary carries no paths of the build machine.
pub const MANIFEST_DIR: Option<&str> = match option_env!("STRUCTURES_BUNDLE") {
    Some(_) => None,
    None => Some(env!("CARGO_MANIFEST_DIR")),
};

/// Window options: 1440 × 900 (the mockup's board), at least 390 × 600, wgpu presenting at
/// vsync (screens pace the frames they ask for: [`Ctx::animate`], [`Ctx::wake_in`]), the
/// window's size and position persisted.
pub fn native_options() -> eframe::NativeOptions {
    let mut options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_app_id(APP_ID)
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([390.0, 600.0]),
        persist_window: true,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    options.wgpu_options.surface.present_mode = eframe::wgpu::PresentMode::AutoVsync;
    options
}

/// Runs the app (with the MCP bridge on its default socket).
pub fn run() -> eframe::Result<()> {
    run_with(AppOptions::default())
}

/// Runs the app with `opts`.
pub fn run_with(opts: AppOptions) -> eframe::Result<()> {
    platform::install_open_documents();
    eframe::run_native(
        APP_NAME,
        native_options(),
        Box::new(|cc| {
            platform::set_receiver(&cc.egui_ctx);
            Ok(Box::new(StructuresApp::new(cc, opts)))
        }),
    )
}
