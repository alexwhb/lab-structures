//! SCR-5 Materials — PLACEHOLDER from the shell (wave 1). The `materials` agent replaces this
//! file (keep `MaterialsScreen::new()` and the [`Screen`] impl; see `docs/ARCHITECTURE.md` › App).
//! Mockup: `docs/design/mockup/Materials.dc.html`.

use super::{Badge, Drive, Screen, Share};
use crate::ctx::{Carry, Ctx};
use eframe::egui::Ui;

/// SCR-5 state.
#[derive(Debug, Default)]
pub struct MaterialsScreen {
    running: bool,
    /// The last hand-off received (NAV-4).
    pub received: Option<Carry>,
}

impl MaterialsScreen {
    /// The starting state.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Screen for MaterialsScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let _ = ctx;
        super::placeholder(
            ui,
            "Materials",
            "Placeholder: the materials agent builds this screen.",
        );
    }

    fn drive_button(&self) -> Option<Drive> {
        Some(Drive::new("Pull").running(self.running))
    }

    fn drive(&mut self, _ctx: &mut Ctx) {
        self.running = !self.running;
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        self.running = false;
    }

    fn clock(&self) -> String {
        "ε = 0.00 %".into()
    }

    fn share(&mut self, _ctx: &mut Ctx) -> Option<Share> {
        Some(Share::new("Materials (placeholder)"))
    }

    fn provenance(&self) -> Vec<Badge> {
        vec![
            Badge::simplified("Curves: representative"),
            Badge::illustrative("Heat effects: illustrative"),
            Badge::illustrative("Not certified data"),
        ]
    }

    fn receive(&mut self, carry: Carry, _ctx: &mut Ctx) {
        self.received = Some(carry);
    }

    fn describe(&self) -> String {
        "Materials (placeholder).".into()
    }
}
