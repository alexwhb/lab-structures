//! `structures-playground --smoke <seconds> [--headless]` (PLAT-1) — a measurable start-up +
//! run path for `scripts/test-all.sh` and `scripts/check-bundle.sh`: the real
//! [`StructuresApp`] opens on the Playground (the builder) with something to run — level 1
//! with a wood truss over a road deck ([`SMOKE_BUILD`]) — presses the drive button ("Run
//! traffic") after the first frame, draws the frames it asks for during `<seconds>` of wall
//! time (it paces itself like the real app — ~30 frames a second while the cars drive and the
//! settling engine runs, none at rest — so `frames=` is its real frame count, not a spin) and
//! prints one line
//!
//! ```text
//! SMOKE route=playground mode=window first_frame_ms=412 frames=96 run=run frame_ms_avg=1.9 frame_ms_max=7.2 status=ok
//! ```
//!
//! (`run=` is the builder's phase when the smoke run ended: `run` or `result`.)
//!
//! then exits 0 (1 on an error or when the window closes early, 2 on bad arguments). With
//! `--headless` or `STRUCTURES_HEADLESS=1` — or when no window can be opened — the same app draws
//! into an egui context without a window (layout + tessellation, paced by the repaint delay the
//! app asks for, no GPU).
//!
//! A smoke run never touches the user's state: no persistence (a temp storage folder), no
//! save dialogs, a launcher that opens nothing, a fixed seed. It reads nothing from the
//! checkout at run time (fonts are compiled in), so it runs from a sandboxed bundle copy.

use crate::app::{AppOptions, StructuresApp};
use crate::launcher::Launcher;
use crate::router::Route;
use eframe::egui;
use sp_engine::builder::data::{LevelKey, PieceType};
use sp_engine::builder::game::PresetBuild;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Parsed `--smoke` arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct SmokeArgs {
    /// Wall seconds to run after the first frame.
    pub seconds: f64,
    /// Draw without a window.
    pub headless: bool,
}

impl SmokeArgs {
    /// `None` when `--smoke` is not among the arguments.
    pub fn parse(args: &[String]) -> Option<Result<SmokeArgs, String>> {
        let i = args.iter().position(|a| a == "--smoke")?;
        Some(Self::parse_after(&args[i + 1..]))
    }

    fn parse_after(rest: &[String]) -> Result<SmokeArgs, String> {
        let mut a = SmokeArgs {
            seconds: 3.0,
            headless: false,
        };
        let mut it = rest.iter().peekable();
        if let Some(s) = it.peek()
            && !s.starts_with("--")
        {
            a.seconds = s
                .parse::<f64>()
                .map_err(|_| format!("not a number of seconds: {s}"))?;
            it.next();
        }
        for flag in it {
            match flag.as_str() {
                "--headless" => a.headless = true,
                other => {
                    return Err(format!(
                        "unknown smoke option {other} (usage: structures-playground --smoke <seconds> [--headless])"
                    ));
                }
            }
        }
        if !(a.seconds > 0.0 && a.seconds.is_finite()) {
            return Err("the smoke run needs a positive number of seconds".into());
        }
        Ok(a)
    }
}

/// What a smoke run measured.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SmokeReport {
    /// The route that ran.
    pub route: String,
    /// `window` or `headless`.
    pub mode: String,
    /// Process start → first frame.
    pub first_frame_ms: f64,
    /// Frames drawn after the first.
    pub frames: u64,
    /// The builder's phase when the smoke run ended (`run`, `result`).
    pub run: String,
    /// Mean frame time (the app's `frame`, ms).
    pub frame_ms_avg: f64,
    /// Longest frame (ms).
    pub frame_ms_max: f64,
    /// What went wrong.
    pub errors: Vec<String>,
}

impl SmokeReport {
    /// No errors.
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// The one-line summary the scripts parse.
    pub fn line(&self) -> String {
        format!(
            "SMOKE route={} mode={} first_frame_ms={:.0} frames={} run={} frame_ms_avg={:.2} frame_ms_max={:.2} status={}",
            self.route,
            self.mode,
            self.first_frame_ms,
            self.frames,
            if self.run.is_empty() { "-" } else { &self.run },
            self.frame_ms_avg,
            self.frame_ms_max,
            if self.ok() {
                "ok".to_string()
            } else {
                format!("error: {}", self.errors.join("; "))
            }
        )
    }
}

/// The screen a smoke run opens.
pub const SMOKE_ROUTE: Route = Route::Playground;

/// What the smoke run builds on level 1: the preset truss (a Road deck in 8 m panels under a
/// 6 m-deep truss) in wood.
pub const SMOKE_BUILD: PresetBuild = PresetBuild::Truss {
    chord: PieceType::Wood,
};

/// Puts something to run on the Playground: level 1 with [`SMOKE_BUILD`].
pub fn prepare(app: &mut StructuresApp) {
    let game = &mut app.screens_mut().builder.game;
    game.pick_level(LevelKey::River);
    game.build_preset(SMOKE_BUILD);
}

/// The builder's phase for the report line.
fn run_state(app: &StructuresApp) -> &'static str {
    app.screens().builder.game.phase().key()
}

/// The options of a smoke run's app.
pub fn app_options() -> AppOptions {
    AppOptions {
        start: SMOKE_ROUTE,
        rng_seed: AppOptions::TEST_SEED,
        autoplay: true,
        save_dialog: false,
        load_persisted: false,
        launcher: Launcher::recording(&[]),
        out_dir: std::env::temp_dir().join(format!("structures-smoke-{}", std::process::id())),
        mcp_socket: None,
    }
}

/// Drives one run frame by frame.
struct Driver {
    seconds: f64,
    started: Instant,
    first: Option<Instant>,
    deadline: Instant,
    frame_sum: f64,
    report: SmokeReport,
    done: bool,
}

impl Driver {
    fn new(args: &SmokeArgs, mode: &str) -> Self {
        let now = Instant::now();
        Self {
            seconds: args.seconds,
            started: now,
            first: None,
            // generous for debug builds and slow machines
            deadline: now + Duration::from_secs_f64(30.0 + args.seconds * 5.0),
            frame_sum: 0.0,
            report: SmokeReport {
                route: SMOKE_ROUTE.id().into(),
                mode: mode.into(),
                ..Default::default()
            },
            done: false,
        }
    }

    /// Wall time left until the run is due to end (from now).
    fn until_end(&self) -> Duration {
        let end = match self.first {
            Some(first) => first + Duration::from_secs_f64(self.seconds),
            None => self.deadline,
        };
        end.saturating_duration_since(Instant::now())
            .max(Duration::from_millis(1))
    }

    /// After the app drew a frame that took `dt`.
    fn after(&mut self, app: &mut StructuresApp, ctx: &egui::Context, dt: Duration) {
        if self.done {
            return;
        }
        let now = Instant::now();
        match self.first {
            None => {
                self.first = Some(now);
                self.report.first_frame_ms = crate::STARTED
                    .get()
                    .unwrap_or(&self.started)
                    .elapsed()
                    .as_secs_f64()
                    * 1e3;
                // press the drive button ("Run traffic") so the run has something moving
                if !app.press_drive(ctx) {
                    self.report
                        .errors
                        .push("the Playground had no enabled drive button".into());
                }
            }
            Some(first) => {
                let ms = dt.as_secs_f64() * 1e3;
                self.report.frames += 1;
                self.frame_sum += ms;
                self.report.frame_ms_max = self.report.frame_ms_max.max(ms);
                if now.duration_since(first).as_secs_f64() >= self.seconds {
                    self.finish(app);
                }
            }
        }
        if !self.done && now > self.deadline {
            self.report.errors.push("timed out".into());
            self.finish(app);
        }
    }

    fn finish(&mut self, app: &StructuresApp) {
        let r = &mut self.report;
        r.frame_ms_avg = self.frame_sum / r.frames.max(1) as f64;
        r.route = app.route().id().into();
        r.run = run_state(app).into();
        if r.run == "build" {
            r.errors.push("the Playground's run never started".into());
        }
        if app.route() != SMOKE_ROUTE {
            r.errors.push(format!(
                "left the {} for {}",
                SMOKE_ROUTE.title(),
                app.route().id()
            ));
        }
        if r.frames == 0 {
            r.errors.push("no frames drawn".into());
        }
        self.done = true;
    }
}

/// The window around the app for a smoke run.
struct SmokeApp {
    app: StructuresApp,
    driver: Driver,
    out: Arc<Mutex<Option<SmokeReport>>>,
}

impl eframe::App for SmokeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let t0 = Instant::now();
        self.app.frame(ui);
        let ctx = ui.ctx().clone();
        self.driver.after(&mut self.app, &ctx, t0.elapsed());
        if self.driver.done {
            if let Ok(mut o) = self.out.lock() {
                *o = Some(self.driver.report.clone());
            }
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            // the app asks for its own frames; this only makes sure the run ends on time
            ui.ctx().request_repaint_after(self.driver.until_end());
        }
    }

    // never write the user's saved state
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn persist_egui_memory(&self) -> bool {
        false
    }
}

/// Runs in a real window (wgpu); `None` if no window could be opened.
pub fn run_windowed(args: &SmokeArgs) -> Option<SmokeReport> {
    crate::STARTED.get_or_init(Instant::now);
    let out = Arc::new(Mutex::new(None));
    let out2 = out.clone();
    let mut native = crate::native_options();
    native.persist_window = false;
    native.persistence_path = Some(std::env::temp_dir().join("structures-smoke-state"));
    let driver = Driver::new(args, "window");
    let r = eframe::run_native(
        "Structures Playground smoke",
        native,
        Box::new(move |cc| {
            Ok(Box::new(SmokeApp {
                app: {
                    let mut app = StructuresApp::new(cc, app_options());
                    prepare(&mut app);
                    app
                },
                driver,
                out: out2,
            }))
        }),
    );
    if let Err(e) = r {
        log::warn!("smoke: no window ({e}); running headless");
        return None;
    }
    let report = out.lock().ok().and_then(|mut o| o.take());
    Some(report.unwrap_or_else(|| SmokeReport {
        route: SMOKE_ROUTE.id().into(),
        mode: "window".into(),
        errors: vec!["the window closed before the run finished".into()],
        ..Default::default()
    }))
}

/// Runs without a window: the app draws into an egui context (layout and tessellation on the
/// CPU), sleeping between frames for the repaint delay the app asks for.
pub fn run_headless(args: &SmokeArgs) -> SmokeReport {
    crate::STARTED.get_or_init(Instant::now);
    let ctx = egui::Context::default();
    sp_theme::install(&ctx);
    let mut app = StructuresApp::with_options(app_options());
    prepare(&mut app);
    let mut d = Driver::new(args, "headless");
    let t_start = Instant::now();
    while !d.done {
        let t0 = Instant::now();
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440.0, 900.0),
            )),
            time: Some(t_start.elapsed().as_secs_f64()),
            ..Default::default()
        };
        let mut out = ctx.run_ui(raw, |ui| app.frame(ui));
        let _ = ctx.tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
        out.textures_delta.clear();
        let dt = t0.elapsed();
        d.after(&mut app, &ctx, dt);
        // what the app asked for (`Duration::MAX` when idle), at least one 60 Hz frame
        let asked = out
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map_or(Duration::MAX, |v| v.repaint_delay);
        let wait = asked
            .max(Duration::from_millis(16).saturating_sub(dt))
            .min(d.until_end());
        std::thread::sleep(wait);
    }
    d.report
}

/// `main`'s entry: runs, prints the report line, returns the exit code.
pub fn main(args: SmokeArgs) -> i32 {
    let headless = args.headless
        || std::env::var_os("STRUCTURES_HEADLESS").is_some_and(|v| !v.is_empty() && v != "0");
    let report = if headless {
        run_headless(&args)
    } else {
        run_windowed(&args).unwrap_or_else(|| run_headless(&args))
    };
    println!("{}", report.line());
    if report.ok() { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parses_arguments() {
        assert_eq!(SmokeArgs::parse(&s(&["structures-playground"])), None);
        let a = SmokeArgs::parse(&s(&["x", "--smoke", "3"]))
            .unwrap()
            .unwrap();
        assert_eq!((a.seconds, a.headless), (3.0, false));
        let a = SmokeArgs::parse(&s(&["x", "--smoke", "0.5", "--headless"]))
            .unwrap()
            .unwrap();
        assert_eq!((a.seconds, a.headless), (0.5, true));
        let a = SmokeArgs::parse(&s(&["x", "--smoke"])).unwrap().unwrap();
        assert_eq!(a.seconds, 3.0, "default");
        for bad in [
            &["x", "--smoke", "abc"][..],
            &["x", "--smoke", "-1"],
            &["x", "--smoke", "2", "--nope"],
        ] {
            assert!(SmokeArgs::parse(&s(bad)).unwrap().is_err(), "{bad:?}");
        }
    }

    /// A short headless run runs a real structure: the run starts, frames come at its pace
    /// (~30 a second, not a spin), it stays on the Playground, and the line has the fields the
    /// scripts parse.
    #[test]
    fn headless_smoke_runs_a_real_structure() {
        let r = run_headless(&SmokeArgs {
            seconds: 0.5,
            headless: true,
        });
        assert!(r.ok(), "{}", r.line());
        assert_eq!(r.run, "run", "{}", r.line());
        assert!(r.frames >= 8, "frames while running: {}", r.line());
        assert!(r.frames <= 40, "paced, not a spin: {}", r.line());
        let line = r.line();
        assert!(
            line.starts_with("SMOKE route=playground mode=headless first_frame_ms="),
            "{line}"
        );
        assert!(
            line.contains(" frames=") && line.contains(" run=") && line.ends_with("status=ok"),
            "{line}"
        );
    }

    /// The prepared structure is a real one: run to the end it gets a result.
    #[test]
    fn the_smoke_structure_runs_to_a_result() {
        let mut app = StructuresApp::with_options(app_options());
        prepare(&mut app);
        let game = &mut app.screens_mut().builder.game;
        assert!(game.design().members.len() > 10);
        assert!(game.run_test(sp_shared::proto::MAX_RUN_FRAMES).is_some());
    }
}
