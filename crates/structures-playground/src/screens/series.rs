//! SCR-6 Series — PLACEHOLDER (the shell builds it out next): the five app cards, the
//! "Educational. Not for real structural design." badge and Back (NAV-3).

use super::Screen;
use crate::ctx::Ctx;
use eframe::egui::Ui;

/// The Series screen's state.
#[derive(Debug, Default)]
pub struct SeriesScreen {}

impl SeriesScreen {
    /// The starting state.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Screen for SeriesScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        if crate::widgets::ghost("Back")
            .icon(crate::widgets::Icon::Back)
            .show(ui)
            .clicked()
        {
            ctx.back();
        }
        super::placeholder(ui, "Series", "What Structures adds to the series");
    }

    fn describe(&self) -> String {
        "Series: the lab series' apps.".into()
    }
}
