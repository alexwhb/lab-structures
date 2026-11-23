//! SCR-1 the Playground: the builder for bridges and towers (SPEC "Builder", BLD-*;
//! `docs/design/v2/mockup/Builder.dc.html`).
//!
//! **Placeholder from the `shell` agent** — the `screen` agent takes this file over (PROGRESS
//! "v2 — the builder"). Keep `BuilderScreen::new()` and the [`Screen`] impl; the hooks the app
//! calls are documented in `docs/ARCHITECTURE.md` › App: the header's drive button ("Run
//! traffic" / "Run the test", "Pause" while running), Reset, the clock (`building` /
//! `t = 3.1 s`), the document name (`<level>-level` / `sandbox`), Share, `Command`s (⌘Z,
//! Delete, ⌘V, ⌘= / ⌘− / ⌘0, arrows / ⌥ arrows, Esc) and `Carry::Netlist` in (Advanced "Open
//! in Playground") and out ("Open in Advanced").

use crate::ctx::Ctx;
use crate::screens::{Drive, Screen, placeholder};
use eframe::egui::Ui;

/// The builder's state.
#[derive(Debug, Default)]
pub struct BuilderScreen {
    /// Running ("Pause" in the header) — the placeholder only toggles it.
    running: bool,
}

impl BuilderScreen {
    /// The builder in build mode on level 1 (SPEC MB-10: the app starts in build mode).
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a run is going (the header says "Pause").
    pub fn running(&self) -> bool {
        self.running
    }
}

impl Screen for BuilderScreen {
    fn ui(&mut self, _ctx: &mut Ctx, ui: &mut Ui) {
        placeholder(
            ui,
            "Playground",
            "The builder for bridges and towers is on its way.",
        );
    }

    fn drive_button(&self) -> Option<Drive> {
        Some(Drive::new("Run traffic").running(self.running))
    }

    fn drive(&mut self, _ctx: &mut Ctx) {
        self.running = !self.running;
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        *self = Self::new();
    }

    fn clock(&self) -> String {
        if self.running {
            "t = 0.0 s"
        } else {
            "building"
        }
        .to_string()
    }

    fn describe(&self) -> String {
        "Playground (the builder), river-level, building".to_string()
    }
}
