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
    structures_playground::run_with(structures_playground::AppOptions::default())
}
