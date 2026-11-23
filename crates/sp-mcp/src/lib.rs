//! `structures-mcp`: the Structures Playground MCP server (SPEC MCP-1, MCP-2).
//!
//! The tool layer has three parts, registered in this order (which is also `tools/list` order):
//!
//! - [`engine`]: tools that run on `sp-engine` in-process and headless, so they work without
//!   the app: solve a truss netlist, sweep a member's influence line, a tower in the wind, a
//!   building on its ground, a tensile test.
//! - [`app`]: the lab-series tools on the running app over the live bridge (`app_status`,
//!   `list_screens`, `navigate`, `screenshot`, `describe_screen`).
//! - [`screens`]: the Structures tools on the running app: load a netlist into the Bridge and
//!   Test it, set the Tower, the Ground and the Materials bench.
//!
//! App tools return a tool error saying how to start the app when it isn't running.
//! [`server`] builds the [`McpServer`]; the `structures-mcp` binary serves it on stdio.

#![warn(missing_docs)]

pub mod app;
pub mod engine;
pub mod screens;

use lab_mcp::mcp::{McpServer, RegistryError, ServerInfo, ToolRegistry};
use serde_json::{Value, json};
use std::path::PathBuf;

/// What the server needs to know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The running app's bridge socket (`None`: the bridge is disabled).
    pub socket: Option<PathBuf>,
}

impl Config {
    /// `$STRUCTURES_MCP_SOCKET`, else the app's per-user default socket.
    pub fn from_env() -> Self {
        Self {
            socket: sp_shared::bridge_socket_path(),
        }
    }
}

/// Guidance for the model, sent with `initialize` / `server/discover`.
pub const INSTRUCTIONS: &str = "Structures Playground is a desktop structural-engineering \
playground (educational, not for real structural design). Playground mode has four worlds: the \
Bridge (build a truss across a river and Test it until it holds or collapses), the Tower (a \
skyscraper in a storm), the Ground (soils and foundations) and Materials (a tensile test). \
Advanced mode is the bridge as a truss netlist (.str text).\n\nEngine tools work without the \
app: solve_truss (a .str netlist → reactions, member forces, utilisation, failure modes, \
determinacy, mass; parse errors with line and column), influence_line (a member's force as the \
truck crosses), tower_wind, ground_check and tensile_test. Netlists look like:\n.node A 0 0 \
fix=xy\n.node B 4 0 fix=y\n.node C 2 2\n.material steel E=200GPa fy=250MPa rho=7850\n.section s \
A=20cm2 I=A^2\n.member M1 A C s\n.member M2 C B s\n.member M3 A B s\n.load dead C Fx=0 \
Fy=-50\n.static linear\n(forces in kN, `t` for tonnes of weight; `fix=xy` pin, `fix=y` \
roller).\n\nApp tools act on the running app, so the user must have it open: app_status, \
list_screens, navigate, screenshot, describe_screen, bridge_load_netlist, bridge_test, \
set_tower, set_ground, set_material. Tools that change a screen switch to it by default (show: \
false to stay put). Prefer the tools' numbers or describe_screen (text) to screenshot when text \
is enough.";

/// Every tool, in `tools/list` order.
pub fn registry(config: &Config) -> Result<ToolRegistry, RegistryError> {
    let mut tools = ToolRegistry::new();
    engine::register(&mut tools)?;
    let link = app::AppLink::new(config.socket.clone());
    app::register(&mut tools, link.clone())?;
    screens::register(&mut tools, link)?;
    Ok(tools)
}

/// The MCP server for `config`.
pub fn server(config: &Config) -> Result<McpServer, RegistryError> {
    let info = ServerInfo {
        name: "structures-mcp".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        title: Some(sp_shared::APP_NAME.into()),
    };
    Ok(McpServer::new(info, registry(config)?).with_instructions(INSTRUCTIONS))
}

/// JSON Schema for a `screen` argument: one of the screen ids.
pub(crate) fn screen_schema(description: &str) -> Value {
    json!({
        "type": "string",
        "enum": sp_shared::screen_ids(),
        "description": description,
    })
}

/// JSON Schema for the `show` argument of tools that change a screen.
pub(crate) fn show_schema(screen: &str) -> Value {
    json!({
        "type": "boolean",
        "description": format!(
            "Switch the app to the {screen} so the user sees the change (default true). \
             false changes it in the background."
        ),
    })
}
