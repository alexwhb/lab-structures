//! The six screens. Each lives in its own module as a struct (`BridgeScreen`, …) with a
//! `new()` that implements [`Screen`]; the app keeps them all alive in [`Screens`] so their
//! state survives navigation. See `docs/ARCHITECTURE.md` › "App".
//!
//! The header (NAV-1) and the world tab row (NAV-2) are drawn by the app from what the active
//! screen declares: [`Screen::document`], [`Screen::drive`] (the `.prim` drive button),
//! [`Screen::reset`], [`Screen::clock`], [`Screen::share`] and [`Screen::provenance`]. Space
//! presses the drive button (NAV-5); ⌘Z / Delete / ⌘V arrive as [`Command`]s.

use crate::ctx::{Carry, Command, Ctx};
use crate::router::Route;
use eframe::egui::{Color32, Ui};

pub mod advanced;
pub mod bridge;
pub mod ground;
pub mod materials;
pub mod series;
pub mod tower;

pub use advanced::AdvancedScreen;
pub use bridge::BridgeScreen;
pub use ground::GroundScreen;
pub use materials::MaterialsScreen;
pub use series::SeriesScreen;
pub use tower::TowerScreen;

/// The header's drive button (NAV-1): `.prim` with a play icon and `label` ("Test", "Drive
/// across", "Play gusts", "Play years", "Pull"), or a pause icon and "Pause" while `running`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drive {
    /// The label while stopped.
    pub label: &'static str,
    /// Running: the button reads "Pause".
    pub running: bool,
    /// Greyed out (e.g. nothing to test yet).
    pub enabled: bool,
}

impl Drive {
    /// A stopped, enabled drive button.
    pub const fn new(label: &'static str) -> Self {
        Self {
            label,
            running: false,
            enabled: true,
        }
    }

    /// Running ("Pause") or not.
    pub const fn running(mut self, running: bool) -> Self {
        self.running = running;
        self
    }

    /// Enabled or greyed out.
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// What the button says now (also its accessible name).
    pub fn text(&self) -> &'static str {
        if self.running { "Pause" } else { self.label }
    }
}

/// What Share copies (NAV-6): the document's text and the toast confirming it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Share {
    /// Copied to the clipboard as is.
    pub text: String,
    /// The confirmation toast (default: "Copied <document> to the clipboard").
    pub toast: Option<String>,
}

impl Share {
    /// Share `text` with the default toast.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            toast: None,
        }
    }

    /// With its own toast ("Copied the netlist").
    pub fn toast(mut self, toast: impl Into<String>) -> Self {
        self.toast = Some(toast.into());
        self
    }
}

/// How far a number can be trusted (PROV-1): the provenance badge's dot colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// Green: exact for the model.
    Exact,
    /// Sand: simplified.
    Simplified,
    /// Grey: illustrative.
    Illustrative,
}

impl Provenance {
    /// The dot colour.
    pub fn color(self) -> Color32 {
        match self {
            Provenance::Exact => sp_theme::PROV_EXACT,
            Provenance::Simplified => sp_theme::PROV_SIMPLIFIED,
            Provenance::Illustrative => sp_theme::PROV_ILLUSTRATIVE,
        }
    }
}

/// A `.badge` with a coloured dot (PROV-1): the world tab row's right-hand badges, the
/// Advanced results badges, the Series provenance row ([`crate::widgets::badge`] draws one).
#[derive(Debug, Clone, PartialEq)]
pub struct Badge {
    /// The text ("Solver: linear statics, exact").
    pub text: String,
    /// The dot's colour (`None`: no dot).
    pub dot: Option<Color32>,
}

impl Badge {
    /// A badge with the dot of `kind`.
    pub fn new(text: impl Into<String>, kind: Provenance) -> Self {
        Self {
            text: text.into(),
            dot: Some(kind.color()),
        }
    }
    /// Green "exact for the model".
    pub fn exact(text: impl Into<String>) -> Self {
        Self::new(text, Provenance::Exact)
    }
    /// Sand "simplified".
    pub fn simplified(text: impl Into<String>) -> Self {
        Self::new(text, Provenance::Simplified)
    }
    /// Grey "illustrative".
    pub fn illustrative(text: impl Into<String>) -> Self {
        Self::new(text, Provenance::Illustrative)
    }
    /// A badge with a dot of any colour from `sp_theme` (Advanced's `#E8A39E` "educational,
    /// not for design").
    pub fn dot(text: impl Into<String>, dot: Color32) -> Self {
        Self {
            text: text.into(),
            dot: Some(dot),
        }
    }
    /// A plain badge without a dot ("example: 48 m truss").
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            dot: None,
        }
    }
}

/// What a screen implements. Only [`Screen::ui`] is required.
///
/// Call order each frame for the active screen: [`Screen::enter`] (only when the route has
/// just become active, before anything else), then [`Screen::receive`] if a hand-off arrived,
/// [`Screen::command`] (⌘Z, Delete, ⌘V) and [`Screen::drive`] (Space), [`Screen::tick`] (time
/// passes), then the header reads [`Screen::document`], [`Screen::drive_button`],
/// [`Screen::clock`] (and calls [`Screen::drive`], [`Screen::reset`] or [`Screen::share`] when
/// clicked), the world tab row reads [`Screen::provenance`], then [`Screen::ui`] draws the
/// body under them.
pub trait Screen {
    /// The body under the header (and the world tab row). Lay it out with
    /// [`crate::layout::columns`].
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui);

    /// The document name after the brand (NAV-1), when it isn't the route's default
    /// ([`Route::document`]): the Bridge's `overhang-level`, the netlist's file stem.
    fn document(&self) -> Option<String> {
        None
    }

    /// The header's drive button (`None`: none).
    fn drive_button(&self) -> Option<Drive> {
        None
    }

    /// The drive button was pressed (click or Space): start or pause.
    fn drive(&mut self, _ctx: &mut Ctx) {}

    /// Whether the header shows Reset.
    fn has_reset(&self) -> bool {
        true
    }

    /// Reset was pressed: back to the screen's starting state (what Reset means per mockup).
    fn reset(&mut self, _ctx: &mut Ctx) {}

    /// The mono clock in the header (`building`, `x = 24.0 m`, `T = 9.9 s`, …; empty: none).
    fn clock(&self) -> String {
        String::new()
    }

    /// What Share copies (NAV-6). `None` greys the button out.
    fn share(&mut self, _ctx: &mut Ctx) -> Option<Share> {
        None
    }

    /// The provenance badges at the right of the world tab row (PROV-1; worlds only).
    fn provenance(&self) -> Vec<Badge> {
        Vec::new()
    }

    /// Advances the screen's simulation by the frame's time ([`Ctx::dt`]) while it runs. It
    /// runs once per frame (not again in a discarded layout pass) before anything is drawn,
    /// so the header's clock and the body show the same state. Ask for the next frame with
    /// [`Ctx::animate`] / [`Ctx::wake_in`] while something moves (PLAT-3).
    fn tick(&mut self, _ctx: &mut Ctx) {}

    /// A keyboard command (NAV-5). Return `true` if handled.
    fn command(&mut self, _cmd: Command, _ctx: &mut Ctx) -> bool {
        false
    }

    /// The route just became active (also the very first frame): seed from
    /// [`Ctx::rng_seed`], start if [`Ctx::autoplay`], … It may be called again.
    fn enter(&mut self, _ctx: &mut Ctx) {}

    /// State handed over from another screen (NAV-4).
    fn receive(&mut self, _carry: Carry, _ctx: &mut Ctx) {}

    /// A one-line description of what's on screen (stage summaries, the MCP `describe` tool).
    fn describe(&self) -> String {
        String::new()
    }
}

/// Placeholder content (wave 1): the screen's title and a note, centred on the `.graph`
/// stage under the chrome. Screen owners replace their placeholder `ui`.
#[doc(hidden)]
pub fn placeholder(ui: &mut Ui, title: &str, note: &str) {
    use crate::widgets::text::{galley, text_top};
    let rect = crate::layout::stage(ui);
    let p = ui.painter();
    let g = galley(ui, title, lab_theme::font::heading(22.0), sp_theme::TEXT);
    let c = rect.center();
    p.galley(
        eframe::egui::pos2(c.x - g.size().x / 2.0, text_top(c.y - 14.0, &g)),
        g.clone(),
        sp_theme::TEXT,
    );
    let n = galley(ui, note, lab_theme::font::body(13.0), sp_theme::TEXT_MUTED);
    p.galley(
        eframe::egui::pos2(c.x - n.size().x / 2.0, text_top(c.y + 16.0, &n)),
        n,
        sp_theme::TEXT_MUTED,
    );
    let s = title.to_string();
    ui.interact(
        rect,
        ui.scope_id().with("placeholder"),
        eframe::egui::Sense::hover(),
    )
    .widget_info(|| {
        eframe::egui::WidgetInfo::labeled(eframe::egui::accesskit::Role::Label, true, &s)
    });
}

/// Every screen's state.
pub struct Screens {
    /// SCR-1.
    pub bridge: BridgeScreen,
    /// SCR-2.
    pub advanced: AdvancedScreen,
    /// SCR-3.
    pub tower: TowerScreen,
    /// SCR-4.
    pub ground: GroundScreen,
    /// SCR-5.
    pub materials: MaterialsScreen,
    /// SCR-6.
    pub series: SeriesScreen,
}

impl Default for Screens {
    fn default() -> Self {
        Self::new()
    }
}

impl Screens {
    /// Every screen in its starting state.
    pub fn new() -> Self {
        Self {
            bridge: BridgeScreen::new(),
            advanced: AdvancedScreen::new(),
            tower: TowerScreen::new(),
            ground: GroundScreen::new(),
            materials: MaterialsScreen::new(),
            series: SeriesScreen::new(),
        }
    }

    /// The screen for `route`.
    pub fn get(&mut self, route: Route) -> &mut dyn Screen {
        match route {
            Route::Bridge => &mut self.bridge,
            Route::Advanced => &mut self.advanced,
            Route::Tower => &mut self.tower,
            Route::Ground => &mut self.ground,
            Route::Materials => &mut self.materials,
            Route::Series => &mut self.series,
        }
    }

    /// Read-only access for `describe` and tests.
    pub fn get_ref(&self, route: Route) -> &dyn Screen {
        match route {
            Route::Bridge => &self.bridge,
            Route::Advanced => &self.advanced,
            Route::Tower => &self.tower,
            Route::Ground => &self.ground,
            Route::Materials => &self.materials,
            Route::Series => &self.series,
        }
    }
}
