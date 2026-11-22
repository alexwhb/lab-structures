//! Navigation model (NAV-1/2/3): two modes and six routes.
//!
//! Playground holds the four worlds (Bridge, Tower, Ground, Materials) under the world tab row;
//! Advanced is the truss netlist workbench; the Series screen belongs to no mode (it is reached
//! from the brand, NAV-3, and has no header). Each route knows its mockup
//! (`docs/design/mockup/<file>`), the mockup's board size and the fixed panel height its
//! columns use when they stack.

use serde::{Deserialize, Serialize};

/// The header's mode switcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    /// Bridge · Tower · Ground · Materials (the worlds).
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
}

/// A screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Route {
    /// SCR-1 Playground › Bridge.
    Bridge,
    /// SCR-3 Playground › Tower.
    Tower,
    /// SCR-4 Playground › Ground.
    Ground,
    /// SCR-5 Playground › Materials.
    Materials,
    /// SCR-2 Advanced (the bridge as a truss netlist).
    Advanced,
    /// SCR-6 the Series screen (from the brand).
    Series,
}

impl Route {
    /// Every route, in shortcut order (⌘1 … ⌘5, then Series).
    pub const ALL: [Route; 6] = [
        Route::Bridge,
        Route::Tower,
        Route::Ground,
        Route::Materials,
        Route::Advanced,
        Route::Series,
    ];

    /// The Playground worlds, in tab order.
    pub const WORLDS: [Route; 4] = [Route::Bridge, Route::Tower, Route::Ground, Route::Materials];

    /// The mode this route belongs to (`None` for the Series screen, which highlights no mode).
    pub fn mode(self) -> Option<Mode> {
        match self {
            Route::Bridge | Route::Tower | Route::Ground | Route::Materials => {
                Some(Mode::Playground)
            }
            Route::Advanced => Some(Mode::Advanced),
            Route::Series => None,
        }
    }

    /// True for the four Playground worlds (they have the world tab row).
    pub fn is_world(self) -> bool {
        self.mode() == Some(Mode::Playground)
    }

    /// True for every route that has the header (all but Series, NAV-1).
    pub fn has_header(self) -> bool {
        self != Route::Series
    }

    /// The world tab label (`None` off the Playground).
    pub fn tab_label(self) -> Option<&'static str> {
        match self {
            Route::Bridge => Some("Bridge"),
            Route::Tower => Some("Tower"),
            Route::Ground => Some("Ground"),
            Route::Materials => Some("Materials"),
            Route::Advanced | Route::Series => None,
        }
    }

    /// A human title (the tab label for worlds).
    pub fn title(self) -> &'static str {
        match self {
            Route::Advanced => "Advanced",
            Route::Series => "Series",
            r => r.tab_label().unwrap_or_default(),
        }
    }

    /// The default document name shown after the brand in the header (NAV-1, SPEC § Screens).
    /// A screen may show another one ([`crate::Screen::document`]): the Bridge's level, the
    /// netlist's file stem.
    pub fn document(self) -> &'static str {
        match self {
            Route::Bridge => "river-level",
            Route::Tower => "tower-study",
            Route::Ground => "foundation-pit",
            Route::Materials => "tensile-test",
            Route::Advanced => "river-crossing",
            Route::Series => "series",
        }
    }

    /// The mockup file in `docs/design/mockup/`.
    pub fn mockup(self) -> &'static str {
        match self {
            Route::Bridge => "Playground.dc.html",
            Route::Tower => "Tower.dc.html",
            Route::Ground => "Ground.dc.html",
            Route::Materials => "Materials.dc.html",
            Route::Advanced => "Advanced.dc.html",
            Route::Series => "Series.dc.html",
        }
    }

    /// The mockup's board size in points (`$preview` width × height).
    pub fn board_size(self) -> (f32, f32) {
        match self {
            Route::Bridge | Route::Tower | Route::Ground | Route::Materials => (1440.0, 900.0),
            Route::Advanced => (1440.0, 1000.0),
            Route::Series => (1440.0, 1400.0),
        }
    }

    /// The height of the chrome above the body at 1440: the 56 px header, plus the 45 px
    /// world tab row (44 px tabs + its 1 px border) on the Playground; nothing on Series.
    pub fn chrome_height(self) -> f32 {
        match self {
            Route::Series => 0.0,
            Route::Advanced => sp_theme::size::HEADER,
            _ => sp_theme::size::HEADER + sp_theme::size::TAB_ROW + 1.0,
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

    /// The ⌘-digit shortcut (NAV-5): ⌘1–⌘4 worlds, ⌘5 Advanced.
    pub fn shortcut_digit(self) -> Option<u8> {
        match self {
            Route::Bridge => Some(1),
            Route::Tower => Some(2),
            Route::Ground => Some(3),
            Route::Materials => Some(4),
            Route::Advanced => Some(5),
            Route::Series => None,
        }
    }

    /// A stable lowercase id (persistence, MCP `navigate`, test filters).
    pub fn id(self) -> &'static str {
        match self {
            Route::Bridge => "bridge",
            Route::Tower => "tower",
            Route::Ground => "ground",
            Route::Materials => "materials",
            Route::Advanced => "advanced",
            Route::Series => "series",
        }
    }

    /// Parses [`Route::id`] (any case; also accepts `playground` for the Bridge, the mockup's
    /// name for it).
    pub fn from_id(id: &str) -> Option<Route> {
        let id = id.trim().to_ascii_lowercase();
        if id == "playground" {
            return Some(Route::Bridge);
        }
        Route::ALL.into_iter().find(|r| r.id() == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_and_worlds() {
        assert_eq!(
            Route::WORLDS.map(Route::mode),
            [Some(Mode::Playground); 4],
            "worlds are Playground"
        );
        assert_eq!(Route::Advanced.mode(), Some(Mode::Advanced));
        assert_eq!(Route::Series.mode(), None);
        assert!(Route::Bridge.is_world() && !Route::Advanced.is_world());
        assert!(!Route::Series.has_header() && Route::Advanced.has_header());
        assert_eq!(Mode::ALL.map(Mode::label), ["Playground", "Advanced"]);
    }

    #[test]
    fn labels_match_the_mockup() {
        assert_eq!(
            Route::WORLDS.map(|r| r.tab_label().unwrap()),
            ["Bridge", "Tower", "Ground", "Materials"]
        );
        assert_eq!(Route::Advanced.tab_label(), None);
        assert_eq!(
            Route::ALL.map(Route::document),
            [
                "river-level",
                "tower-study",
                "foundation-pit",
                "tensile-test",
                "river-crossing",
                "series"
            ]
        );
    }

    #[test]
    fn boards_and_panels() {
        assert_eq!(Route::Bridge.board_size(), (1440.0, 900.0));
        assert_eq!(Route::Advanced.board_size(), (1440.0, 1000.0));
        assert_eq!(Route::Series.board_size(), (1440.0, 1400.0));
        assert_eq!(Route::Bridge.panel_height(), 900.0 - 56.0 - 45.0);
        assert_eq!(Route::Advanced.panel_height(), 1000.0 - 56.0);
        assert_eq!(Route::Series.panel_height(), 0.0);
    }

    #[test]
    fn mockups_exist() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/design");
        for r in Route::ALL {
            assert!(dir.join("mockup").join(r.mockup()).is_file(), "{r:?}");
            let stem = r.mockup().trim_end_matches(".dc.html");
            assert!(
                dir.join(format!("screenshots/{stem}-1440.png")).is_file(),
                "{r:?}"
            );
        }
    }

    #[test]
    fn ids_round_trip() {
        for r in Route::ALL {
            assert_eq!(Route::from_id(r.id()), Some(r));
        }
        assert_eq!(Route::from_id("Advanced"), Some(Route::Advanced));
        assert_eq!(Route::from_id("playground"), Some(Route::Bridge));
        assert_eq!(Route::from_id("nope"), None);
    }

    #[test]
    fn shortcuts() {
        assert_eq!(
            Route::ALL.map(Route::shortcut_digit),
            [Some(1), Some(2), Some(3), Some(4), Some(5), None]
        );
    }
}
