//! `structures-mcp`: the Structures Playground MCP server on stdin/stdout. Logs go to stderr.

use anyhow::{Context, bail};
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn usage() -> String {
    let default = sp_mcp::Config::from_env()
        .socket
        .map_or_else(|| "disabled".to_owned(), |p| p.display().to_string());
    format!(
        "structures-mcp {VERSION} — the Structures Playground MCP server (stdio)

USAGE:
    structures-mcp [--socket <path>]

Speaks the Model Context Protocol on stdin/stdout (protocol 2026-07-28, and 2025-11-25 back
to 2024-11-05 via `initialize`). Engine tools (solve_truss, influence_line, builder_test,
tower_wind, ground_check, tensile_test) work on their own; app tools (navigate, screenshot,
builder_run, builder_conditions, …) need Structures Playground running.
Add it to Claude Code with:
    claude mcp add structures-playground -- {exe}

OPTIONS:
    --socket <path>   the running app's bridge socket
                      (default: ${env}, else {default})
    -V, --version     print the version and exit
    -h, --help        print this help and exit
",
        exe = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "/path/to/structures-mcp".into()),
        env = sp_shared::SOCKET_ENV,
    )
}

enum Command {
    Serve(sp_mcp::Config),
    Print(String),
}

fn parse(args: impl IntoIterator<Item = String>) -> anyhow::Result<Command> {
    let mut config = sp_mcp::Config::from_env();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Print(usage())),
            "-V" | "--version" => return Ok(Command::Print(format!("structures-mcp {VERSION}\n"))),
            "--socket" => {
                let p = args.next().context("--socket needs a path")?;
                config.socket = Some(PathBuf::from(p));
            }
            s if s.starts_with("--socket=") => {
                config.socket = Some(PathBuf::from(&s["--socket=".len()..]));
            }
            other => bail!("unknown argument `{other}` (try --help)"),
        }
    }
    Ok(Command::Serve(config))
}

fn main() -> anyhow::Result<()> {
    let command = match parse(std::env::args().skip(1)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("structures-mcp: {e}");
            std::process::exit(2);
        }
    };
    let config = match command {
        Command::Print(text) => {
            print!("{text}");
            return Ok(());
        }
        Command::Serve(config) => config,
    };
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("sp_mcp=info,structures_mcp=info,lab_mcp=info,warn"),
    )
    .target(env_logger::Target::Stderr)
    .init();
    match &config.socket {
        Some(p) => log::info!("structures-mcp {VERSION}: app bridge at {}", p.display()),
        None => log::info!("structures-mcp {VERSION}: app bridge disabled"),
    }
    let server = sp_mcp::server(&config).context("building the tool registry")?;
    server.serve_stdio().context("serving MCP on stdio")?;
    Ok(())
}
