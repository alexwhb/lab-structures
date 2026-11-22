//! SCR-4 Ground — PLACEHOLDER from the shell (wave 1). The `worlds` agent replaces this
//! file (keep `GroundScreen::new()` and the [`Screen`] impl; see `docs/ARCHITECTURE.md` › App).
//! Mockup: `docs/design/mockup/Ground.dc.html`.

use super::{Badge, Drive, Screen, Share};
use crate::ctx::{Carry, Ctx};
use eframe::egui::Ui;

/// SCR-4 state.
#[derive(Debug, Default)]
pub struct GroundScreen {
    running: bool,
    /// The last hand-off received (NAV-4).
    pub received: Option<Carry>,
}

impl GroundScreen {
    /// The starting state.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Screen for GroundScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let _ = ctx;
        super::placeholder(
            ui,
            "Ground",
            "Placeholder: the worlds agent builds this screen.",
        );
    }

    fn drive_button(&self) -> Option<Drive> {
        Some(Drive::new("Play years").running(self.running))
    }

    fn drive(&mut self, _ctx: &mut Ctx) {
        self.running = !self.running;
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        self.running = false;
    }

    fn clock(&self) -> String {
        "year 0.0".into()
    }

    fn share(&mut self, _ctx: &mut Ctx) -> Option<Share> {
        Some(Share::new("Ground (placeholder)"))
    }

    fn provenance(&self) -> Vec<Badge> {
        vec![
            Badge::simplified("Bearing + settlement: simplified"),
            Badge::illustrative("Soil presets: illustrative"),
            Badge::illustrative("Game costs"),
        ]
    }

    fn receive(&mut self, carry: Carry, _ctx: &mut Ctx) {
        self.received = Some(carry);
    }

    fn describe(&self) -> String {
        "Ground (placeholder).".into()
    }
}
