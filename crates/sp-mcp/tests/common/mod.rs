//! Helpers for the end-to-end tests: a temp socket, the real app in an egui_kittest harness,
//! and the real `structures-mcp` binary spoken to over its stdin/stdout.
#![allow(dead_code)]

use eframe::egui;
use egui_kittest::Harness;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use structures_playground::{AppOptions, StructuresApp};

/// The app under test.
pub type App = Harness<'static, StructuresApp>;

/// The longest any request may take.
pub const T: Duration = Duration::from_secs(30);

/// A private temp dir with a socket path in it, removed on drop.
pub struct TempSocket {
    dir: PathBuf,
}

impl TempSocket {
    pub fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "sp-mcp-e2e-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join("app.sock")
    }
}

impl Drop for TempSocket {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The real app at 1440 × 900 with the test options (seed 7, no autoplay), listening on `socket`.
pub fn app(socket: &Path) -> App {
    let opts = AppOptions {
        mcp_socket: Some(socket.to_path_buf()),
        ..AppOptions::test()
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(64)
        .wgpu()
        .build_eframe(move |cc| StructuresApp::new(cc, opts));
    h.run_steps(3);
    h
}

/// The `structures-mcp` binary Cargo built for this package's tests.
pub struct Mcp {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: mpsc::Receiver<Value>,
    next_id: u64,
}

impl Mcp {
    pub fn spawn(socket: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_structures-mcp"))
            .arg("--socket")
            .arg(socket)
            .env("RUST_LOG", "warn")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn structures-mcp");
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let v: Value = serde_json::from_str(&line).unwrap_or_else(|e| {
                    panic!("structures-mcp wrote non-JSON to stdout {line:?}: {e}")
                });
                if tx.send(v).is_err() {
                    break;
                }
            }
        });
        Self {
            stdin: child.stdin.take(),
            child,
            lines,
            next_id: 1,
        }
    }

    /// Spawns the server and does the `initialize` handshake.
    pub fn started(socket: &Path) -> Self {
        let mut m = Self::spawn(socket);
        m.initialize();
        m
    }

    pub fn send(&mut self, msg: &Value) {
        let w = self.stdin.as_mut().unwrap();
        writeln!(w, "{msg}").unwrap();
        w.flush().unwrap();
    }

    /// Sends a request; returns its id.
    pub fn request(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        id
    }

    /// Waits for the response to `id`, calling `step` between polls (to run app frames).
    pub fn response_with(&mut self, id: u64, mut step: impl FnMut()) -> Value {
        let t0 = Instant::now();
        loop {
            match self.lines.recv_timeout(Duration::from_millis(2)) {
                Ok(msg) if msg["id"] == json!(id) => return msg,
                Ok(other) => panic!("unexpected message while waiting for {id}: {other}"),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => panic!("structures-mcp exited"),
            }
            assert!(t0.elapsed() < T, "no response to request {id}");
            step();
        }
    }

    /// Request + response with no app to step.
    pub fn call(&mut self, method: &str, params: Value) -> Value {
        let id = self.request(method, params);
        self.response_with(id, || {})
    }

    /// `tools/call` with no app: the result object.
    pub fn tool(&mut self, name: &str, arguments: Value) -> Value {
        let r = self.call("tools/call", json!({"name": name, "arguments": arguments}));
        assert!(r.get("error").is_none(), "{name}: protocol error {r}");
        r["result"].clone()
    }

    /// `tools/call` while stepping the app's frames until the result comes back; then a
    /// couple more frames so the GUI shows the change.
    pub fn call_tool(&mut self, app: &mut App, name: &str, arguments: Value) -> Value {
        let id = self.request("tools/call", json!({"name": name, "arguments": arguments}));
        let r = self.response_with(id, || app.step());
        assert!(r.get("error").is_none(), "{name}: protocol error {r}");
        app.run_steps(2);
        r["result"].clone()
    }

    /// The legacy handshake, as Claude Code and Claude Desktop do it.
    pub fn initialize(&mut self) -> Value {
        let r = self.call(
            "initialize",
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "sp-mcp-e2e", "version": "0"}
            }),
        );
        self.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        r
    }

    /// Closes stdin and checks that the server exits cleanly.
    pub fn close(mut self) {
        drop(self.stdin.take());
        let t0 = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "structures-mcp exited with {status}");
                return;
            }
            assert!(
                t0.elapsed() < Duration::from_secs(10),
                "structures-mcp did not exit"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The text of a tool result's first content block (the summary).
pub fn text(result: &Value) -> &str {
    result["content"][0]["text"].as_str().unwrap_or_default()
}

/// A successful tool result's structured content (panics on a tool error).
pub fn data(result: &Value) -> &Value {
    assert_eq!(result["isError"], false, "tool error: {}", text(result));
    &result["structuredContent"]
}

/// A tool error's message (panics on success).
pub fn error(result: &Value) -> &str {
    assert_eq!(
        result["isError"], true,
        "expected a tool error, got {result}"
    );
    text(result)
}

/// Whether any AccessKit node has exactly this label.
pub fn has_label(app: &App, label: &str) -> bool {
    use egui_kittest::kittest::Queryable;
    app.query_all_by_label(label).next().is_some()
}
