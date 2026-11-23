//! Structures Playground desktop app entry point.

fn main() -> eframe::Result<()> {
    structures_playground::STARTED.get_or_init(std::time::Instant::now);
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("structures_playground=info,warn"),
    )
    .init();
    // `structures-playground --smoke <seconds> [--headless]`: start-up + run measurement
    let args: Vec<String> = std::env::args().collect();
    if let Some(smoke) = structures_playground::smoke::SmokeArgs::parse(&args) {
        match smoke {
            Ok(a) => std::process::exit(structures_playground::smoke::main(a)),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(2);
            }
        }
    }
    // `--no-mcp`: no live MCP bridge (also `STRUCTURES_MCP_SOCKET=off`)
    let mut opts = structures_playground::AppOptions::default();
    if args.iter().any(|a| a == "--no-mcp") {
        opts.mcp_socket = None;
    }
    structures_playground::run_with(opts)
}
