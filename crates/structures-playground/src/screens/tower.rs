//! SCR-3 Tower — PLACEHOLDER from the shell (wave 1). The `worlds` agent replaces this
//! file (keep `TowerScreen::new()` and the [`Screen`] impl; see `docs/ARCHITECTURE.md` › App).
//! Mockup: `docs/design/mockup/Tower.dc.html`.

use super::{Badge, Drive, Screen, Share};
use crate::ctx::{Carry, Ctx};
use eframe::egui::Ui;

/// SCR-3 state.
#[derive(Debug, Default)]
pub struct TowerScreen {
    running: bool,
    /// The last hand-off received (NAV-4).
    pub received: Option<Carry>,
}

impl TowerScreen {
    /// The starting state.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Screen for TowerScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let _ = ctx;
        super::placeholder(
            ui,
            "Tower",
            "Placeholder: the worlds agent builds this screen.",
        );
    }

    fn drive_button(&self) -> Option<Drive> {
        Some(Drive::new("Play gusts").running(self.running))
    }

    fn drive(&mut self, _ctx: &mut Ctx) {
        self.running = !self.running;
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        self.running = false;
    }

    fn clock(&self) -> String {
        "T = 0.0 s".into()
    }

    fn share(&mut self, _ctx: &mut Ctx) -> Option<Share> {
        Some(Share::new("Tower (placeholder)"))
    }

    fn provenance(&self) -> Vec<Badge> {
        vec![
            Badge::simplified("Wind + sway: simplified cantilever"),
            Badge::illustrative("Comfort bands: illustrative"),
            Badge::illustrative("Game costs"),
        ]
    }

    fn receive(&mut self, carry: Carry, _ctx: &mut Ctx) {
        self.received = Some(carry);
    }

    fn describe(&self) -> String {
        "Tower (placeholder).".into()
    }
}
