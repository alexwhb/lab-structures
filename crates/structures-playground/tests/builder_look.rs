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

/// Renders board `name` at 1440 × the reference's height (the aside grows the page) and, with
/// `SP_BUILDER_SHOTS`, saves the comparison.
fn render_board(name: &str) -> (RgbaImage, RgbaImage) {
    let mock = board_png(name, 1440);
    let mut app = board(name, 1440.0, mock.height() as f32);
    let img = render(&mut app);
    if shots() {
        save(
            &side_by_side(&img, &mock),
            &repo().join(format!("target/compare/{name}-1440.png")),
        );
    }
    (img, mock)
}

/// LOOK-1: the stable regions of BridgeBuild stay close to the reference.
#[test]
fn bridge_build_is_close_to_the_reference() {
    let (img, mock) = render_board("BridgeBuild");
    for (name, region, bound) in [
        ("tool rail", (0, 62, 76, 520), 4.0),
        ("hint bar", (93, 75, 1032, 52), 8.0),
        ("stage", (93, 138, 1032, 553), 6.0),
        ("legend + zoom", (93, 700, 1032, 40), 8.0),
        ("level rows", (1152, 75, 276, 380), 6.0),
        ("goal panel", (1152, 462, 276, 210), 10.5),
        ("conditions", (1152, 686, 276, 620), 8.0),
    ] {
        let d = mean_diff(&img, &mock, Some(region));
        println!("BridgeBuild {name}: mean |Δ| {d:.2}");
        assert!(d < bound, "{name}: mean |Δ| {d:.2} ≥ {bound}");
    }
}

/// LOOK-1: the stable regions of TowerBuild (level 5, the 40 m preset tower on piles, Select,
/// Show forces) stay close to the reference.
#[test]
fn tower_build_is_close_to_the_reference() {
    let (img, mock) = render_board("TowerBuild");
    for (name, region, bound) in [
        ("tool rail", (0, 62, 76, 520), 4.0),
        ("hint bar", (93, 75, 1032, 52), 8.0),
        ("stage", (93, 138, 1032, 553), 8.0),
        ("level rows", (1152, 75, 276, 380), 6.0),
        ("goal panel", (1152, 462, 276, 230), 9.0),
    ] {
        let d = mean_diff(&img, &mock, Some(region));
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

/// Review images of the other boards (`SP_BUILDER_SHOTS=1`).
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
}
