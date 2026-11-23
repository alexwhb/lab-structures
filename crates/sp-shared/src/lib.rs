//! UI-free vocabulary shared by the Structures Playground app and its MCP server
//! (`structures-mcp`, crate `sp-mcp`):
//!
//! - the app's identity ([`APP_NAME`], [`APP_ID`]) and where the live MCP bridge listens
//!   ([`SOCKET_ENV`], [`bridge_socket_path`]);
//! - the screens and modes as the bridge names them ([`SCREENS`], [`MODES`]; the app's
//!   `structures_playground::Route` is the source of truth and a test keeps the two equal);
//! - the Structures bridge methods and their payloads ([`proto`]), on top of the four every
//!   lab-series app answers (`lab_mcp::bridge::proto`);
//! - how arguments typed by a person or a model are read and checked ([`input`]), so the app
//!   and the server accept exactly the same values;
//! - the engine reports both sides send ([`report`]): a solved truss, a tower in the wind, a
//!   building on its ground, a tensile test.
//!
//! No egui here: `sp-mcp` uses it without pulling in a GUI toolkit.

#![warn(missing_docs)]

pub mod input;
pub mod proto;
pub mod report;

use lab_mcp::bridge::proto::{ModeRef, ScreenRef};
use serde::Serialize;
use std::ffi::OsString;
use std::path::PathBuf;

/// The app's display name (matches `structures_playground::APP_NAME`).
pub const APP_NAME: &str = "Structures Playground";
/// The app's reverse-DNS id, also its macOS bundle id (matches `structures_playground::APP_ID`).
pub const APP_ID: &str = "dev.labseries.StructuresPlayground";

/// Overrides where the live bridge listens, for both the app and `structures-mcp`. Empty or
/// `off` disables the bridge.
pub const SOCKET_ENV: &str = "STRUCTURES_MCP_SOCKET";

/// How to start the app, for "not running" messages.
pub const HOW_TO_START: &str = "open Structures Playground (dist/Structures Playground.app, or \
`cargo run --release -p structures-playground` in the structures-playground checkout)";

/// The bridge socket: `$STRUCTURES_MCP_SOCKET` if set (`None` when it is empty or `off`), else
/// the per-user default (`~/Library/Application Support/dev.labseries.StructuresPlayground/
/// mcp.sock` on macOS, or a shorter fallback when that path is too long for a Unix socket).
pub fn bridge_socket_path() -> Option<PathBuf> {
    socket_from(std::env::var_os(SOCKET_ENV))
}

/// [`bridge_socket_path`] for a given value of [`SOCKET_ENV`] (`None`: unset).
pub fn socket_from(value: Option<OsString>) -> Option<PathBuf> {
    match value {
        Some(v) if v.is_empty() || v == "off" => None,
        Some(v) => Some(PathBuf::from(v)),
        None => Some(lab_mcp::bridge::default_socket_path(APP_ID)),
    }
}

/// A mode of the header's mode switcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ModeInfo {
    /// Stable id: `playground`, `advanced`.
    pub id: &'static str,
    /// The switcher label.
    pub label: &'static str,
    /// Where the mode link goes at first start (a screen id; afterwards Playground opens the
    /// last world used).
    pub home: &'static str,
}

impl ModeInfo {
    /// The mode as the bridge reports it.
    pub fn mode_ref(&self) -> ModeRef {
        ModeRef {
            id: self.id.to_owned(),
            label: self.label.to_owned(),
        }
    }
}

/// Playground: the four worlds (Bridge, Tower, Ground, Materials).
pub const PLAYGROUND: ModeInfo = ModeInfo {
    id: "playground",
    label: "Playground",
    home: "bridge",
};
/// Advanced: the truss netlist workbench.
pub const ADVANCED: ModeInfo = ModeInfo {
    id: "advanced",
    label: "Advanced",
    home: "advanced",
};
/// Both modes, in switcher order.
pub const MODES: [ModeInfo; 2] = [PLAYGROUND, ADVANCED];

/// A mockup board size in points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Board {
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

/// A screen as the bridge and `list_screens` name it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ScreenInfo {
    /// Stable id (`navigate` takes it): `bridge`, `tower`, `ground`, `materials`, `advanced`,
    /// `series`.
    pub id: &'static str,
    /// The tab label or title ("Bridge", "Advanced").
    pub label: &'static str,
    /// The mode it belongs to (`None`: the Series screen).
    pub mode: Option<ModeInfo>,
    /// The default document name in the header (`river-level`).
    pub document: &'static str,
    /// The keyboard shortcut (`⌘1`…), if any.
    pub shortcut: Option<&'static str>,
    /// What is on it, for the model.
    pub about: &'static str,
    /// The mockup that specifies it (`docs/design/mockup/`).
    pub mockup: &'static str,
    /// The mockup board size at 1440 px.
    pub board: Board,
}

impl ScreenInfo {
    /// The screen as the bridge reports it.
    pub fn screen_ref(&self) -> ScreenRef {
        ScreenRef {
            id: self.id.to_owned(),
            label: self.label.to_owned(),
            mode: self.mode.map(|m| m.mode_ref()),
        }
    }
}

const BOARD_900: Board = Board {
    width: 1440.0,
    height: 900.0,
};

/// Every screen, in shortcut order (⌘1–⌘5, then Series).
pub const SCREENS: [ScreenInfo; 6] = [
    ScreenInfo {
        id: "bridge",
        label: "Bridge",
        mode: Some(PLAYGROUND),
        document: "river-level",
        shortcut: Some("⌘1"),
        about: "Build a truss across a 40 m river and break it: joints, members (thin / \
                medium / thick), supports, hanging weights, four levels with budgets and stars, \
                a live linear-statics solve coloured tension / compression, and a Test that \
                ramps the load until it holds or collapses.",
        mockup: "Playground.dc.html",
        board: BOARD_900,
    },
    ScreenInfo {
        id: "tower",
        label: "Tower",
        mode: Some(PLAYGROUND),
        document: "tower-study",
        shortcut: Some("⌘2"),
        about: "A skyscraper in a storm: floors, structural system, shape and a tuned mass \
                damper; wind at 10 m; top sway and drift, sway period, base shear, \
                overturning, vortex lock-in, occupant comfort, cost and the 100-floor \
                challenge (a simplified cantilever model).",
        mockup: "Tower.dc.html",
        board: BOARD_900,
    },
    ScreenInfo {
        id: "ground",
        label: "Ground",
        mode: Some(PLAYGROUND),
        document: "foundation-pit",
        shortcut: Some("⌘3"),
        about: "Soils and foundations: soil, footings / mat / piles / caissons, pile length, \
                building floors and earthquake shaking; bearing pressure, capacity, safety \
                factor, settlement and tilt over the years, liquefaction and the lean of Pisa \
                challenge (simplified).",
        mockup: "Ground.dc.html",
        board: BOARD_900,
    },
    ScreenInfo {
        id: "materials",
        label: "Materials",
        mode: Some(PLAYGROUND),
        document: "tensile-test",
        shortcut: Some("⌘4"),
        about: "A tensile test: steel, aluminum, concrete, timber or carbon fiber at a \
                temperature, pulled until it breaks; the stress–strain curve, stiffness, \
                yield, peak strength, stretch at break, toughness and strength per kilogram \
                (representative curves).",
        mockup: "Materials.dc.html",
        board: BOARD_900,
    },
    ScreenInfo {
        id: "advanced",
        label: "Advanced",
        mode: Some(ADVANCED),
        document: "river-crossing",
        shortcut: Some("⌘5"),
        about: "The bridge as a truss netlist: the editable .str text (Pratt / Howe / Warren \
                generators, material, size, truck), the elevation coloured by force, the \
                influence line of a member as the truck crosses, the member results table and \
                the checks (determinacy, reactions, deflection, worst member, weight).",
        mockup: "Advanced.dc.html",
        board: Board {
            width: 1440.0,
            height: 1000.0,
        },
    },
    ScreenInfo {
        id: "series",
        label: "Series",
        mode: None,
        document: "series",
        shortcut: None,
        about: "What Structures adds to the lab series: the five apps, shared engines, bridges \
                to the other apps, netlist directives, the challenge ladder and provenance.",
        mockup: "Series.dc.html",
        board: Board {
            width: 1440.0,
            height: 1400.0,
        },
    },
];

/// The screen with this id (exact, lowercase).
pub fn screen(id: &str) -> Option<&'static ScreenInfo> {
    SCREENS.iter().find(|s| s.id == id)
}

/// The screen ids, in order (for tool schemas and error messages).
pub fn screen_ids() -> Vec<&'static str> {
    SCREENS.iter().map(|s| s.id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screens_and_modes() {
        assert_eq!(
            screen_ids(),
            [
                "bridge",
                "tower",
                "ground",
                "materials",
                "advanced",
                "series"
            ]
        );
        let adv = screen("advanced").unwrap().screen_ref();
        assert_eq!(adv.label, "Advanced");
        assert_eq!(adv.mode.unwrap().id, "advanced");
        assert!(screen("series").unwrap().mode.is_none());
        for m in MODES {
            assert_eq!(screen(m.home).unwrap().mode, Some(m));
        }
        assert!(screen("lab").is_none());
    }

    #[test]
    fn socket_paths() {
        assert_eq!(socket_from(Some("off".into())), None);
        assert_eq!(socket_from(Some("".into())), None);
        assert_eq!(
            socket_from(Some("/tmp/x.sock".into())),
            Some(PathBuf::from("/tmp/x.sock"))
        );
        let p = socket_from(None).unwrap();
        assert!(p.as_os_str().len() <= lab_mcp::bridge::MAX_SOCKET_PATH);
        assert!(p.to_string_lossy().contains("StructuresPlayground"));
    }
}
