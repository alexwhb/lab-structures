//! Side-by-side `[app | mockup]` images of every screen, for human and agent review, run
//! through `scripts/compare-mockup.sh` (which sets `SP_COMPARE=1`; without it that test does
//! nothing), plus closeness bounds for the shell's own regions (LOOK-1).
//!
//! For each route it renders the app at the mockup's board size (`Route::board_size`) and at
//! 390 px wide (as tall as the reference or the app's content, whichever is taller), puts the
//! reference (`Route::reference`: the builder's BridgeBuild board for the Playground) next to
//! it and writes `target/compare/<Screen>-1440.png` / `<Screen>-390.png` (+ `-1440-app.png`,
//! the app alone). `SP_COMPARE_ONLY=series,playground` limits it to some routes.
//!
//! `readme_screenshots` writes `docs/screenshots/*.png` with `SP_SCREENSHOTS=1`
//! (`scripts/screenshots.sh`). `chrome_stays_close_to_the_mockup` always runs: the header (the
//! shell's) must stay within a mean channel difference of the references.

mod common;

use common::*;
use image::{Rgba, RgbaImage};
use structures_playground::Route;

/// Gap between the two images, filled with mid grey so the seam is obvious.
const SEAM: u32 = 8;

fn side_by_side(app: &RgbaImage, mockup: &RgbaImage) -> RgbaImage {
    let h = app.height().max(mockup.height());
    let w = app.width() + SEAM + mockup.width();
    let mut out = RgbaImage::from_pixel(w, h, Rgba([128, 128, 128, 255]));
    image::imageops::overlay(&mut out, app, 0, 0);
    image::imageops::overlay(&mut out, mockup, (app.width() + SEAM) as i64, 0);
    out
}

fn wanted(route: Route) -> bool {
    match std::env::var("SP_COMPARE_ONLY") {
        Ok(list) if !list.trim().is_empty() => list.split(',').any(|s| {
            let s = s.trim().to_ascii_lowercase();
            s == route.id() || s == mockup_name(route).to_ascii_lowercase()
        }),
        _ => true,
    }
}

/// The app on `route` at `w` × at least `min_h` (grown to its content) and its render.
fn app_render(route: Route, w: f32, min_h: f32) -> (App, RgbaImage) {
    let mut app = harness_at(route, w, min_h);
    let need = app.state().content_height().ceil();
    if need > min_h + 0.5 {
        app.set_size(eframe::egui::vec2(w, need));
        settle(&mut app);
    }
    let img = render(&mut app);
    (app, img)
}

fn out_dir() -> std::path::PathBuf {
    let d = repo().join("target/compare");
    let _ = std::fs::create_dir_all(&d);
    d
}

#[test]
fn side_by_side_with_mockup() {
    if std::env::var_os("SP_COMPARE").is_none() {
        return;
    }
    for route in Route::ALL.into_iter().filter(|r| wanted(*r)) {
        let name = mockup_name(route);
        let mock = load(&mockup_png(route, 1440));
        let (w, h) = route.board_size();
        let (_, img) = app_render(route, w, h.max(mock.height() as f32));
        save(&img, &out_dir().join(format!("{name}-1440-app.png")));
        save(
            &side_by_side(&img, &mock),
            &out_dir().join(format!("{name}-1440.png")),
        );
        println!(
            "compare: {name}-1440 mean |Δ| {:.1}",
            mean_diff(&img, &mock, None)
        );
        let phone = load(&mockup_png(route, 390));
        let (_, img) = app_render(route, PHONE_W, phone.height() as f32);
        save(
            &side_by_side(&img, &phone),
            &out_dir().join(format!("{name}-390.png")),
        );
        println!(
            "compare: {name}-390 mean |Δ| {:.1}",
            mean_diff(&img, &phone, None)
        );
    }
}

/// The builder boards the README shows (`docs/screenshots/builder-<board>.png`): the mockup's
/// presets (SPEC MB-10), the running ones 16 s into their run like the `-16s` references.
pub const README_BOARDS: [&str; 5] = [
    "BridgeStorm",
    "BridgeCable",
    "BridgeCollapse",
    "TowerBuild",
    "TowerStorm",
];

/// `scripts/screenshots.sh` (`SP_SCREENSHOTS=1`): every screen at its board size into
/// `docs/screenshots/<route id>.png`, and the builder boards (`README_BOARDS`) into
/// `docs/screenshots/builder-<board>.png` (the README's).
#[test]
fn readme_screenshots() {
    if std::env::var_os("SP_SCREENSHOTS").is_none() {
        return;
    }
    for route in Route::ALL.into_iter().filter(|r| wanted(*r)) {
        let (w, h) = route.board_size();
        let (_, img) = app_render(route, w, h);
        let path = repo().join(format!("docs/screenshots/{}.png", route.id()));
        save(&img, &path);
        println!("screenshot: {}", path.display());
    }
    if !wanted(Route::Playground) {
        return;
    }
    use sp_engine::builder::{Game, Preset};
    for board in README_BOARDS {
        let preset = Preset::board(board).expect("a board");
        let (w, h) = Route::Playground.board_size();
        let mut app = harness_at(Route::Playground, w, h);
        {
            let game = &mut app.state_mut().screens_mut().builder.game;
            *game = Game::with_preset(&preset);
            if preset.run {
                game.start_run(false);
                for _ in 0..16 * 30 {
                    game.tick();
                }
            }
        }
        app.run_steps(2);
        let img = render(&mut app);
        let path = repo().join(format!(
            "docs/screenshots/builder-{}.png",
            board.to_ascii_lowercase()
        ));
        save(&img, &path);
        println!("screenshot: {}", path.display());
    }
}

/// The header (56 px) at 1440 stays close to the builder's and Advanced's references (LOOK-1,
/// the shell's region; no world tab row in v2).
#[test]
fn chrome_stays_close_to_the_mockup() {
    for route in [Route::Playground, Route::Advanced] {
        let (w, h) = route.board_size();
        let mut app = harness_at(route, w, h);
        let img = render(&mut app);
        let mock = load(&mockup_png(route, 1440));
        // brand + mode switcher (the right-hand actions depend on the screen's clock text)
        let header = mean_diff(&img, &mock, Some((0, 0, 560, 56)));
        assert!(header < 6.0, "{route:?} header: {header:.2}");
    }
}
