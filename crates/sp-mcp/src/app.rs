//! The lab-series tools that act on the running app, forwarded over the live bridge:
//! `app_status`, `list_screens`, `navigate`, `screenshot`, `describe_screen`.
//!
//! Each call opens a fresh connection to the app's socket, so the server keeps working across
//! app restarts. When nothing is listening, a tool returns a tool error (not a protocol error)
//! saying how to start the app; `app_status` and `list_screens` report it as a normal result.

use lab_mcp::bridge::proto::{self, AppStatus, Description, ImagePayload, ScreenRef};
use lab_mcp::bridge::{BridgeClient, BridgeError};
use lab_mcp::mcp::{Content, RegistryError, Tool, ToolError, ToolOutput, ToolRegistry};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// The first sentence of every "not running" message.
pub fn not_running() -> String {
    format!(
        "Structures Playground isn't running — {} and try again.",
        sp_shared::HOW_TO_START
    )
}

/// How long `app_status` waits.
pub const STATUS_TIMEOUT: Duration = Duration::from_secs(3);
/// How long `navigate` and the screen tools wait.
pub const NAVIGATE_TIMEOUT: Duration = Duration::from_secs(5);
/// How long `screenshot` waits (a frame has to render and be encoded).
pub const SCREENSHOT_TIMEOUT: Duration = Duration::from_secs(20);
/// How long `describe_screen` waits.
pub const DESCRIBE_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a screen tool that runs the engine (steps, a Rule-lab run) waits.
pub const WORK_TIMEOUT: Duration = Duration::from_secs(60);

/// Why a bridge call failed, sorted for the tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkError {
    /// Nothing listens on the socket (or the bridge is disabled).
    NotRunning(String),
    /// Something listens but did not answer properly (timeout, dropped connection, bad reply).
    Unreachable(String),
    /// The app answered with an error (bad screen id, a rule that doesn't parse, …).
    Refused(String),
}

impl LinkError {
    /// The message for the model.
    pub fn message(&self) -> &str {
        match self {
            LinkError::NotRunning(m) | LinkError::Unreachable(m) | LinkError::Refused(m) => m,
        }
    }
}

impl From<LinkError> for ToolError {
    fn from(e: LinkError) -> Self {
        ToolError::new(e.message())
    }
}

/// How the app tools reach the app.
#[derive(Debug, Clone)]
pub struct AppLink {
    socket: Option<PathBuf>,
}

impl AppLink {
    /// A link to the app listening on `socket` (`None`: the bridge is disabled).
    pub fn new(socket: Option<PathBuf>) -> Self {
        Self { socket }
    }

    /// The socket, if the bridge is enabled.
    pub fn socket(&self) -> Option<&Path> {
        self.socket.as_deref()
    }

    /// Calls `method` on the running app and decodes the result.
    pub fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<T, LinkError> {
        let Some(path) = &self.socket else {
            return Err(LinkError::NotRunning(format!(
                "{} (The live bridge is disabled: {} is empty or `off`.)",
                not_running(),
                sp_shared::SOCKET_ENV
            )));
        };
        // the app reads one request line of at most 1 MiB: say so rather than have it cut off
        let line = serde_json::json!({"id": u64::MAX, "method": method, "params": &params})
            .to_string()
            .len()
            + 1;
        if line > lab_mcp::bridge::MAX_REQUEST_BYTES {
            return Err(LinkError::Refused(format!(
                "the arguments are {line} bytes as JSON, and Structures Playground reads at most \
                 1 MiB per call; send a smaller netlist"
            )));
        }
        let result = BridgeClient::connect(path).and_then(|mut c| c.call(method, params, timeout));
        let value = result.map_err(|e| link_error(e, method))?;
        serde_json::from_value(value).map_err(|e| {
            LinkError::Unreachable(format!(
                "Structures Playground sent an unexpected reply to `{method}` ({e}); is \
                 structures-mcp the same version as the app?"
            ))
        })
    }
}

fn link_error(e: BridgeError, method: &str) -> LinkError {
    match e {
        BridgeError::NotRunning { path } => LinkError::NotRunning(format!(
            "{} (Nothing is listening on {}.)",
            not_running(),
            path.display()
        )),
        BridgeError::Timeout { timeout, .. } => LinkError::Unreachable(format!(
            "Structures Playground didn't answer `{method}` within {}s; it may be busy. Try \
             again, or bring its window to the front.",
            timeout.as_secs()
        )),
        BridgeError::Disconnected => LinkError::Unreachable(
            "Structures Playground closed the connection before answering (did it quit?).".into(),
        ),
        BridgeError::Remote { message, .. } => LinkError::Refused(message),
        other => LinkError::Unreachable(format!("Couldn't reach Structures Playground: {other}.")),
    }
}

/// "Tower (Playground mode)".
pub fn screen_label(s: &ScreenRef) -> String {
    match &s.mode {
        Some(m) => format!("{} ({} mode)", s.label, m.label),
        None => s.label.clone(),
    }
}

/// The `describe` reply: the lab-series [`Description`] plus the screen's one-line summary.
#[derive(Debug, Clone, serde::Deserialize)]
struct DescribeReply {
    #[serde(flatten)]
    description: Description,
    #[serde(default)]
    summary: Option<String>,
}

/// Registers `app_status`, `list_screens`, `navigate`, `screenshot` and `describe_screen`.
pub fn register(tools: &mut ToolRegistry, link: AppLink) -> Result<(), RegistryError> {
    let link = Arc::new(link);

    let l = Arc::clone(&link);
    tools.register(
        Tool::new(
            "app_status",
            "Is Structures Playground running and reachable? Returns `running`, `reachable`, the \
             socket, and when it is up: the current screen (id, label, mode), the window size, \
             the app version and pid. Not running is a normal result here (`running: false`), \
             not an error.",
        )
        .title("App status")
        .read_only(),
        move |_| Ok(app_status(&l)),
    )?;

    let l = Arc::clone(&link);
    tools.register(
        Tool::new(
            "list_screens",
            "Every Structures Playground screen: its id (what `navigate`, `screenshot` and \
             `describe_screen` take), label, mode, keyboard shortcut, what is on it and its \
             mockup board size; plus the two modes. Works without the app; when the app is \
             running, `current` is the screen it shows.",
        )
        .title("List screens")
        .read_only(),
        move |_| {
            let current = l
                .call::<AppStatus>(proto::STATUS, Value::Null, Duration::from_secs(1))
                .ok()
                .map(|s| s.screen.id);
            let summary: Vec<String> = sp_shared::SCREENS
                .iter()
                .map(|s| {
                    let here = if current.as_deref() == Some(s.id) {
                        ", showing"
                    } else {
                        ""
                    };
                    match s.mode {
                        Some(m) => format!("{} ({}{here})", s.id, m.label),
                        None => format!("{} (no mode{here})", s.id),
                    }
                })
                .collect();
            let v = json!({
                "screens": sp_shared::SCREENS,
                "modes": sp_shared::MODES,
                "current": current,
            });
            Ok(ToolOutput::json(&v).with_first(Content::text(format!(
                "{} screens: {}.",
                sp_shared::SCREENS.len(),
                summary.join(", ")
            ))))
        },
    )?;

    let l = Arc::clone(&link);
    tools.register(
        Tool::new(
            "navigate",
            "Switch the running Structures Playground to a screen (ids from `list_screens`: \
             bridge, tower, ground, materials, advanced, series). Returns the app status \
             afterwards.",
        )
        .title("Navigate")
        .idempotent()
        .input_schema(json!({
            "type": "object",
            "properties": {"screen": crate::screen_schema("The screen to show.")},
            "required": ["screen"],
            "additionalProperties": false
        })),
        move |call| {
            let screen = call.str("screen").unwrap_or_default();
            let status: AppStatus =
                l.call(proto::NAVIGATE, json!({"screen": screen}), NAVIGATE_TIMEOUT)?;
            Ok(ToolOutput::json(&status).with_first(Content::text(format!(
                "Now showing {}.",
                screen_label(&status.screen)
            ))))
        },
    )?;

    let l = Arc::clone(&link);
    tools.register(
        Tool::new(
            "screenshot",
            "A PNG of the running Structures Playground's screen as it stands, drawn offscreen \
             from the app's state at the window's size (physical pixels: 2× on a Retina \
             display), so it works while the window is covered by other windows or minimised \
             (no hover highlights; the page shown from the top). Give `screen` to navigate \
             there first. For reading labels and numbers, `describe_screen` or the set_* / \
             bridge_* tools (which return the screen's numbers) are cheaper.",
        )
        .title("Screenshot")
        .idempotent()
        .input_schema(json!({
            "type": "object",
            "properties": {"screen": crate::screen_schema(
                "Optional: navigate to this screen before taking the screenshot."
            )},
            "additionalProperties": false
        })),
        move |call| {
            let img: ImagePayload = l.call(
                proto::SCREENSHOT,
                screen_params(call.str("screen")),
                SCREENSHOT_TIMEOUT,
            )?;
            let summary = format!(
                "{} — {}×{} px.",
                screen_label(&img.screen),
                img.width,
                img.height
            );
            Ok(ToolOutput::text(summary).with(Content::image_base64(img.data, img.mime_type)))
        },
    )?;

    let l = Arc::clone(&link);
    tools.register(
        Tool::new(
            "describe_screen",
            "A text outline of what is on the running app's screen, from its accessibility \
             tree: one line per element with its role, name, value and state (selected, \
             checked, disabled, focused), indented by nesting, after a one-line summary of \
             the screen. Give `screen` to navigate there first.",
        )
        .title("Describe screen")
        .idempotent()
        .input_schema(json!({
            "type": "object",
            "properties": {"screen": crate::screen_schema(
                "Optional: navigate to this screen before describing it."
            )},
            "additionalProperties": false
        })),
        move |call| {
            let d: DescribeReply = l.call(
                proto::DESCRIBE,
                screen_params(call.str("screen")),
                DESCRIBE_TIMEOUT,
            )?;
            let summary = d
                .summary
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| format!("{s}\n"))
                .unwrap_or_default();
            let text = format!(
                "{} — {} elements.\n{summary}\n{}",
                screen_label(&d.description.screen),
                d.description.nodes.len(),
                d.description.outline
            );
            Ok(ToolOutput {
                content: vec![Content::text(text)],
                structured: Some(json!({
                    "screen": d.description.screen,
                    "summary": d.summary,
                    "nodes": d.description.nodes,
                })),
            })
        },
    )?;
    Ok(())
}

fn screen_params(screen: Option<&str>) -> Value {
    match screen {
        Some(s) => json!({"screen": s}),
        None => json!({}),
    }
}

fn app_status(link: &AppLink) -> ToolOutput {
    let socket = link.socket().map(|p| p.display().to_string());
    match link.call::<AppStatus>(proto::STATUS, Value::Null, STATUS_TIMEOUT) {
        Ok(s) => {
            let text = format!(
                "{} {} is running (pid {}), showing {}.",
                s.app,
                s.version,
                s.pid,
                screen_label(&s.screen)
            );
            let mut v = json!({"running": true, "reachable": true, "socket": socket});
            if let (Value::Object(m), Ok(Value::Object(st))) = (&mut v, serde_json::to_value(&s)) {
                m.extend(st);
            }
            ToolOutput::json(&v).with_first(Content::text(text))
        }
        Err(LinkError::NotRunning(msg)) => {
            let v = json!({"running": false, "reachable": false, "socket": socket, "problem": msg});
            ToolOutput::json(&v).with_first(Content::text(not_running()))
        }
        Err(e) => {
            let v = json!({
                "running": true, "reachable": false, "socket": socket, "problem": e.message()
            });
            ToolOutput::json(&v).with_first(Content::text(e.message().to_owned()))
        }
    }
}
