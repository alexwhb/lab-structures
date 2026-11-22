//! SCR-1 Bridge — PLACEHOLDER from the shell (wave 1). The `bridge` agent replaces this
//! file (keep `BridgeScreen::new()` and the [`Screen`] impl; see `docs/ARCHITECTURE.md` › App).
//! Mockup: `docs/design/mockup/Playground.dc.html`.

use super::{Badge, Drive, Screen, Share};
use crate::ctx::{Carry, Ctx};
use eframe::egui::Ui;

/// SCR-1 state.
#[derive(Debug, Default)]
pub struct BridgeScreen {
    running: bool,
    /// The last hand-off received (NAV-4).
    pub received: Option<Carry>,
}

impl BridgeScreen {
    /// The starting state.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Screen for BridgeScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let _ = ctx;
        super::placeholder(
            ui,
            "Bridge",
            "Placeholder: the bridge agent builds this screen.",
        );
    }

    fn drive_button(&self) -> Option<Drive> {
        Some(Drive::new("Test").running(self.running))
    }

    fn drive(&mut self, _ctx: &mut Ctx) {
        self.running = !self.running;
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        self.running = false;
    }

    fn clock(&self) -> String {
        "building".into()
    }

    fn share(&mut self, _ctx: &mut Ctx) -> Option<Share> {
        Some(Share::new("Bridge (placeholder)"))
    }

    fn provenance(&self) -> Vec<Badge> {
        vec![
            Badge::exact("Solver: linear statics, exact"),
            Badge::simplified("Materials: representative"),
            Badge::illustrative("Game costs"),
        ]
    }

    fn receive(&mut self, carry: Carry, _ctx: &mut Ctx) {
        self.received = Some(carry);
    }

    fn describe(&self) -> String {
        "Bridge (placeholder).".into()
    }
}
