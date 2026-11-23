//! Navigation model (NAV-1/2/3): two modes and three routes.
//!
//! v2: the Playground is one builder for bridges and towers (no world tab row); Advanced is
//! the truss netlist workbench; the Series screen belongs to no mode (it is reached from the
//! brand, NAV-3, and has no header). Each route knows its mockup (under `docs/design/`), the
//! mockup's board size and the fixed panel height its columns use when they stack.
//!
//! The v1 Bridge, Tower, Ground and Materials screens are retired: a saved route naming one of
//! them restores to the Playground (the serde aliases on [`Route::Playground`]).

use serde::{Deserialize, Serialize};

/// The header's mode switcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    /// The builder.
    Playground,
    /// The truss netlist workbench.
    Advanced,
}

impl Mode {
    /// In switcher order.
    pub const ALL: [Mode; 2] = [Mode::Playground, Mode::Advanced];

    /// The switcher label (and accessible name).
    pub fn label(self) -> &'static str {
        match self {
            Mode::Playground => "Playground",
            Mode::Advanced => "Advanced",
        }
    }

    /// The route the switcher opens.
    pub fn route(self) -> Route {
        match self {
            Mode::Playground => Route::Playground,
            Mode::Advanced => Route::Advanced,
        }
    }
}

/// A screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Route {
    /// SCR-1 the Playground: the builder for bridges and towers (v2). A route saved by v1
    /// (`Bridge`, `Tower`, `Ground`, `Materials`) restores here.
    #[serde(
        alias = "Bridge",
        alias = "Tower",
        alias = "Ground",
        alias = "Materials"
    )]
    Playground,
    /// SCR-2 Advanced (a truss netlist).
    Advanced,
    /// SCR-6 the Series screen (from the brand).
    Series,
}

/// The ids of the v1 screens retired in v2 (MCP `navigate` explains instead of guessing).
pub const RETIRED_IDS: [&str; 4] = ["bridge", "tower", "ground", "materials"];

impl Route {
    /// Every route, in shortcut order (⌘1, ⌘2, then Series).
    pub const ALL: [Route; 3] = [Route::Playground, Route::Advanced, Route::Series];

    /// The mode this route belongs to (`None` for the Series screen, which highlights no mode).
    pub fn mode(self) -> Option<Mode> {
        match self {
            Route::Playground => Some(Mode::Playground),
            Route::Advanced => Some(Mode::Advanced),
            Route::Series => None,
        }
    }

    /// True for every route that has the header (all but Series, NAV-1).
    pub fn has_header(self) -> bool {
        self != Route::Series
    }

    /// A human title.
    pub fn title(self) -> &'static str {
        match self {
            Route::Playground => "Playground",
            Route::Advanced => "Advanced",
            Route::Series => "Series",
        }
    }

    /// The default document name shown after the brand in the header (NAV-1, SPEC § Screens).
    /// A screen may show another one ([`crate::Screen::document`]): the builder's level, the
    /// netlist's file stem.
    pub fn document(self) -> &'static str {
        match self {
            Route::Playground => "river-level",
            Route::Advanced => "river-crossing",
            Route::Series => "series",
        }
    }

    /// The mockup file, relative to `docs/design/`.
    pub fn mockup(self) -> &'static str {
        match self {
            Route::Playground => "v2/mockup/Builder.dc.html",
            Route::Advanced => "mockup/Advanced.dc.html",
            Route::Series => "mockup/Series.dc.html",
        }
    }

    /// The rendered reference at `width` (1440 or 390), relative to `docs/design/`: the
    /// builder's first board (BridgeBuild) for the Playground.
    pub fn reference(self, width: u32) -> String {
        match self {
            Route::Playground => format!("v2/screenshots/BridgeBuild-{width}.png"),
            Route::Advanced => format!("screenshots/Advanced-{width}.png"),
            Route::Series => format!("screenshots/Series-{width}.png"),
        }
    }

    /// The mockup's board size in points (`$preview` width × height).
    pub fn board_size(self) -> (f32, f32) {
        match self {
            Route::Playground => (1440.0, 900.0),
            Route::Advanced => (1440.0, 1000.0),
            Route::Series => (1440.0, 1400.0),
        }
    }

    /// The height of the chrome above the body at 1440: the 56 px header (no tab row in v2);
    /// nothing on Series.
    pub fn chrome_height(self) -> f32 {
        match self {
            Route::Series => 0.0,
            Route::Playground | Route::Advanced => sp_theme::size::HEADER,
        }
    }

    /// The fixed height of the screen's columns when they stack at narrow widths (the board
    /// height under the chrome). Side by side they fill the window under the chrome instead.
    /// The Series screen has no columns (0).
    pub fn panel_height(self) -> f32 {
        match self {
            Route::Series => 0.0,
            r => r.board_size().1 - r.chrome_height(),
        }
    }

    /// The ⌘-digit shortcut (NAV-5): ⌘1 Playground, ⌘2 Advanced.
    pub fn shortcut_digit(self) -> Option<u8> {
        match self {
            Route::Playground => Some(1),
            Route::Advanced => Some(2),
            Route::Series => None,
        }
    }

    /// A stable lowercase id (persistence, MCP `navigate`, test filters).
    pub fn id(self) -> &'static str {
        match self {
            Route::Playground => "playground",
            Route::Advanced => "advanced",
            Route::Series => "series",
        }
    }

    /// Parses [`Route::id`] (any case; `builder` also names the Playground). The retired v1
    /// ids ([`RETIRED_IDS`]) are not routes.
    pub fn from_id(id: &str) -> Option<Route> {
        let id = id.trim().to_ascii_lowercase();
        if id == "builder" {
            return Some(Route::Playground);
        }
        Route::ALL.into_iter().find(|r| r.id() == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes() {
        assert_eq!(Route::Playground.mode(), Some(Mode::Playground));
        assert_eq!(Route::Advanced.mode(), Some(Mode::Advanced));
        assert_eq!(Route::Series.mode(), None);
        assert!(!Route::Series.has_header() && Route::Advanced.has_header());
        assert_eq!(Mode::ALL.map(Mode::label), ["Playground", "Advanced"]);
        assert_eq!(
            Mode::ALL.map(Mode::route),
            [Route::Playground, Route::Advanced]
        );
    }

    #[test]
    fn documents_match_the_spec() {
        assert_eq!(
            Route::ALL.map(Route::document),
            ["river-level", "river-crossing", "series"]
        );
    }

    #[test]
    fn boards_and_panels() {
        assert_eq!(Route::Playground.board_size(), (1440.0, 900.0));
        assert_eq!(Route::Advanced.board_size(), (1440.0, 1000.0));
        assert_eq!(Route::Series.board_size(), (1440.0, 1400.0));
        assert_eq!(Route::Playground.panel_height(), 900.0 - 56.0);
        assert_eq!(Route::Advanced.panel_height(), 1000.0 - 56.0);
        assert_eq!(Route::Series.panel_height(), 0.0);
    }

    #[test]
    fn mockups_exist() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/design");
        for r in Route::ALL {
            assert!(dir.join(r.mockup()).is_file(), "{r:?}");
            for w in [1440, 390] {
                assert!(dir.join(r.reference(w)).is_file(), "{r:?} {w}");
            }
        }
    }

    #[test]
    fn ids_round_trip() {
        for r in Route::ALL {
            assert_eq!(Route::from_id(r.id()), Some(r));
        }
        assert_eq!(Route::from_id("Advanced"), Some(Route::Advanced));
        assert_eq!(Route::from_id("builder"), Some(Route::Playground));
        assert_eq!(Route::from_id("nope"), None);
        for id in RETIRED_IDS {
            assert_eq!(Route::from_id(id), None, "{id} is retired");
        }
    }

    #[test]
    fn retired_routes_restore_to_the_playground() {
        for old in ["Bridge", "Tower", "Ground", "Materials", "Playground"] {
            let r: Route = serde_json::from_str(&format!("\"{old}\"")).unwrap();
            assert_eq!(r, Route::Playground, "{old}");
        }
        assert_eq!(
            serde_json::from_str::<Route>("\"Advanced\"").unwrap(),
            Route::Advanced
        );
        assert_eq!(
            serde_json::to_string(&Route::Playground).unwrap(),
            "\"Playground\""
        );
    }

    #[test]
    fn shortcuts() {
        assert_eq!(
            Route::ALL.map(Route::shortcut_digit),
            [Some(1), Some(2), None]
        );
    }
}
