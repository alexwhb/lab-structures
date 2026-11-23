//! The builder's look (LOOK-1, BLD-1, BLD-14): the reference boards rebuilt through the engine
//! (SPEC MB-10: `Preset::board`), rendered at 1440 and 390, compared region by region with
//! `docs/design/v2/screenshots/<Board>-1440.png`; the columns stack at 390 in the mockup's
//! order with nothing overlapping. `SP_BUILDER_SHOTS=1` also writes `[app | mockup]` images
//! to `target/compare/`.

mod common;

use common::*;
use eframe::egui::{self, accesskit::Role};
use image::{Rgba, RgbaImage};
use sp_engine::builder::{Game, Preset};
use structures_playground::Route;

fn board_png(name: &str, width: u32) -> RgbaImage {
    load(&repo().join(format!("docs/design/v2/screenshots/{name}-{width}.png")))
}

/// The app on the Playground showing board `name` (its preset build, conditions, tool and
/// "Show forces"; build mode, MB-10), `w` × `h`.
fn board(name: &str, w: f32, h: f32) -> App {
    let mut app = harness_at(Route::Playground, w, h);
    if let Some(p) = Preset::board(name) {
        let mut p = p;
        p.run = false;
        p.demo_loop = false;
        app.state_mut().screens_mut().builder.game = Game::with_preset(&p);
    }
    settle(&mut app);
    app
}

fn side_by_side(app: &RgbaImage, mockup: &RgbaImage) -> RgbaImage {
    let h = app.height().max(mockup.height());
    let w = app.width() + 8 + mockup.width();
    let mut out = RgbaImage::from_pixel(w, h, Rgba([128, 128, 128, 255]));
    image::imageops::overlay(&mut out, app, 0, 0);
    image::imageops::overlay(&mut out, mockup, (app.width() + 8) as i64, 0);
    out
}

fn shots() -> bool {
    std::env::var_os("SP_BUILDER_SHOTS").is_some()
}

/// Mean channel difference between `app` at `region` moved down by `dy` and `mock` at
/// `region` (the aside below the level list moves down by the levels the mockup lacks).
fn shifted_diff(app: &RgbaImage, mock: &RgbaImage, region: (u32, u32, u32, u32), dy: u32) -> f64 {
    let (x, y, w, h) = region;
    let a = image::imageops::crop_imm(app, x, y + dy, w, h).to_image();
    let b = image::imageops::crop_imm(mock, x, y, w, h).to_image();
    mean_diff(&a, &b, None)
}

/// The aside's extra height for level rows the mockup doesn't have (it has 6 levels and the
/// Sandbox; TIME-3 adds "7 · Leaning tower"): 44 px per row.
fn extra_rows_dy(app: &App) -> u32 {
    let rows: usize = app
        .state()
        .screens()
        .builder
        .game
        .view()
        .level_groups
        .iter()
        .map(|g| g.items.len())
        .sum();
    (rows.saturating_sub(7) * 44) as u32
}

/// Renders board `name` at 1440 × the reference's height (the aside grows the page) and, with
/// `SP_BUILDER_SHOTS`, saves the comparison; also the aside's shift for extra level rows.
fn render_board(name: &str) -> (RgbaImage, RgbaImage, u32) {
    let mock = board_png(name, 1440);
    let mut app = board(name, 1440.0, mock.height() as f32 + 100.0);
    let dy = extra_rows_dy(&app);
    let img = render(&mut app);
    if shots() {
        save(
            &side_by_side(&img, &mock),
            &repo().join(format!("target/compare/{name}-1440.png")),
        );
    }
    (img, mock, dy)
}

/// LOOK-1: the stable regions of BridgeBuild stay close to the reference.
#[test]
fn bridge_build_is_close_to_the_reference() {
    let (img, mock, dy) = render_board("BridgeBuild");
    for (name, region, bound, below_levels) in [
        ("tool rail", (0, 62, 76, 520), 4.0, false),
        ("hint bar", (93, 75, 1032, 52), 8.0, false),
        ("stage", (93, 138, 1032, 553), 6.0, false),
        ("legend + zoom", (93, 700, 1032, 40), 8.0, false),
        ("level rows", (1152, 75, 276, 300), 6.0, false),
        ("goal panel", (1152, 462, 276, 210), 10.5, true),
        ("conditions", (1152, 686, 276, 620), 8.0, true),
    ] {
        let d = shifted_diff(&img, &mock, region, if below_levels { dy } else { 0 });
        println!("BridgeBuild {name}: mean |Δ| {d:.2}");
        assert!(d < bound, "{name}: mean |Δ| {d:.2} ≥ {bound}");
    }
}

/// LOOK-1: the stable regions of TowerBuild (level 5, the 40 m preset tower on piles, Select,
/// Show forces) stay close to the reference.
#[test]
fn tower_build_is_close_to_the_reference() {
    let (img, mock, dy) = render_board("TowerBuild");
    for (name, region, bound, below_levels) in [
        ("tool rail", (0, 62, 76, 520), 4.0, false),
        ("hint bar", (93, 75, 1032, 52), 8.0, false),
        ("stage", (93, 138, 1032, 553), 8.0, false),
        ("level rows", (1152, 75, 276, 300), 6.0, false),
        ("goal panel", (1152, 462, 276, 230), 10.5, true),
    ] {
        let d = shifted_diff(&img, &mock, region, if below_levels { dy } else { 0 });
        println!("TowerBuild {name}: mean |Δ| {d:.2}");
        assert!(d < bound, "{name}: mean |Δ| {d:.2} ≥ {bound}");
    }
}

/// BLD-1: at 1440 the rail, stage and aside sit side by side; at 390 they stack in that order;
/// no two buttons overlap (joints and handles aside).
#[test]
fn renders_at_1440_and_390_without_overlap() {
    for (w, h) in [(W, 1500.0), (PHONE_W, 2700.0)] {
        let mut app = board("BridgeBuild", w, h);
        settle(&mut app);
        let regions: Vec<(String, egui::Rect)> = nodes_with_roles(&app, &[Role::Region])
            .into_iter()
            .map(|(_, l, r)| (l, r))
            .collect();
        let find = |name: &str| {
            regions
                .iter()
                .find(|(l, _)| l == name)
                .map(|(_, r)| *r)
                .unwrap_or_else(|| panic!("region {name}: {regions:?}"))
        };
        let (tools, stage, aside) = (
            find("Tools"),
            find("Stage"),
            find("Level, site and inspector"),
        );
        if w > 1000.0 {
            assert!(tools.right() <= stage.left() + 0.5 && stage.right() <= aside.left() + 0.5);
        } else {
            assert!(
                tools.bottom() <= stage.top() + 0.5,
                "stacked: rail, then stage"
            );
            assert!(stage.bottom() <= aside.top() + 0.5, "then the aside");
        }
        let buttons: Vec<(String, egui::Rect)> = nodes_with_roles(&app, &[Role::Button])
            .into_iter()
            .filter(|(_, l, r)| {
                !l.starts_with("Building grid")
                    && !l.starts_with("Joint ")
                    && !l.contains("% of its limit")
                    && r.min.y > 60.0
            })
            .map(|(_, l, r)| (l, r))
            .collect();
        for (i, (la, a)) in buttons.iter().enumerate() {
            for (lb, bb) in &buttons[i + 1..] {
                let x = a.intersect(*bb);
                assert!(
                    !(x.width() > 0.5 && x.height() > 0.5),
                    "{w}: '{la}' {a:?} overlaps '{lb}' {bb:?}"
                );
            }
        }
        if shots() {
            let img = render(&mut app);
            let name = if w > 1000.0 { "1440" } else { "390" };
            let mock = board_png("BridgeBuild", if w > 1000.0 { 1440 } else { 390 });
            save(
                &side_by_side(&img, &mock),
                &repo().join(format!("target/compare/BridgeBuild-{name}-full.png")),
            );
        }
    }
}

/// Review images of the other boards (`SP_BUILDER_SHOTS=1`): at rest, and the boards that run
/// on their own 16 s into their run (the `-1440-16s` references; the run starts 0.7 s in).
#[test]
fn board_gallery() {
    if !shots() {
        return;
    }
    for name in [
        "BridgeStorm",
        "BridgeCollapse",
        "BridgeCable",
        "TowerStorm",
        "TowerQuake",
    ] {
        render_board(name);
    }
    for name in ["BridgeStorm", "BridgeCollapse", "TowerStorm", "TowerQuake"] {
        let mock = load(&repo().join(format!("docs/design/v2/screenshots/{name}-1440-16s.png")));
        let mut app = board(name, 1440.0, mock.height() as f32);
        {
            let g = &mut app.state_mut().screens_mut().builder.game;
            g.start_run(false);
            // 16 s of page time, the run started 0.7 s in: 15.3 s of 1/30 s frames
            for _ in 0..459 {
                g.tick();
            }
        }
        settle(&mut app);
        let img = render(&mut app);
        save(
            &side_by_side(&img, &mock),
            &repo().join(format!("target/compare/{name}-1440-16s.png")),
        );
    }
}
