//! SCR-2 Advanced — PLACEHOLDER from the shell (wave 1). The `advanced` agent replaces this
//! file (keep `AdvancedScreen::new()` and the [`Screen`] impl; see `docs/ARCHITECTURE.md` › App).
//! Mockup: `docs/design/mockup/Advanced.dc.html`.

use super::{Drive, Screen, Share};
use crate::ctx::{Carry, Ctx};
use eframe::egui::Ui;

/// SCR-2 state.
#[derive(Debug, Default)]
pub struct AdvancedScreen {
    running: bool,
    /// The last hand-off received (NAV-4).
    pub received: Option<Carry>,
}

impl AdvancedScreen {
    /// The starting state.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Screen for AdvancedScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let _ = ctx;
        super::placeholder(
            ui,
            "Advanced",
            "Placeholder: the advanced agent builds this screen.",
        );
    }

    fn drive_button(&self) -> Option<Drive> {
        Some(Drive::new("Drive across").running(self.running))
    }

    fn drive(&mut self, _ctx: &mut Ctx) {
        self.running = !self.running;
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        self.running = false;
    }

    fn clock(&self) -> String {
        "x = 0.0 m".into()
    }

    fn share(&mut self, _ctx: &mut Ctx) -> Option<Share> {
        Some(Share::new("Advanced (placeholder)"))
    }

    fn receive(&mut self, carry: Carry, _ctx: &mut Ctx) {
        self.received = Some(carry);
    }

    fn describe(&self) -> String {
        "Advanced (placeholder).".into()
    }
}
