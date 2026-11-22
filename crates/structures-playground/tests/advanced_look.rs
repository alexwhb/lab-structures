//! Advanced renders at 1440 and 390 and stays close to the references (LOOK-1): mean |Δ| per
//! stable region against `docs/design/screenshots/Advanced-1440.png`, and the 390 layout
//! stacks in the mockup's order.

mod common;
use common::*;
use eframe::egui::accesskit::Role;
use structures_playground::Route;

/// (name, x, y, w, h, bound): regions of the 1440 × 1000 reference and the largest mean |Δ|
/// (0–255 per channel) allowed. Measured, then a little headroom; text antialiasing differs
/// between egui and Chrome, so text-heavy regions sit higher than flat ones.
const REGIONS: [(&str, u32, u32, u32, u32, f64); 8] = [
    ("toolbar", 0, 61, 1440, 60, 4.5),            // measured 3.3
    ("netlist header", 16, 138, 362, 44, 7.0),    // 5.9
    ("netlist lines", 16, 182, 362, 818, 3.0),    // 1.5
    ("elevation", 395, 139, 673, 228, 7.0),       // 5.8
    ("influence line", 395, 384, 673, 340, 7.0),  // 5.9
    ("member results", 1085, 139, 339, 512, 9.0), // 8.0 (Chrome's auto table layout)
    ("checks", 1085, 667, 339, 333, 8.5),         // 7.4
    ("whole board", 0, 0, 1440, 1000, 5.0),       // 3.9
];

#[test]
fn stays_close_to_the_1440_reference() {
    let mut h = harness_at(Route::Advanced, 1440.0, 1000.0);
    let img = render(&mut h);
    let mock = load(&mockup_png(Route::Advanced, 1440));
    let p = screenshot_dir().join("advanced-1440.png");
    save(&img, &p);
    println!("screenshot: {}", p.display());
    let mut report = Vec::new();
    for (name, x, y, w, hh, bound) in REGIONS {
        let d = mean_diff(&img, &mock, Some((x, y, w, hh)));
        report.push(format!("{name}: {d:.2} (≤ {bound})"));
        assert!(
            d <= bound,
            "{name}: mean |Δ| {d:.2} > {bound}\n{}",
            report.join("\n")
        );
    }
    println!("{}", report.join("\n"));
}

#[test]
fn stacks_at_390_in_the_mockups_order() {
    let mut h = app().at(Route::Advanced).phone().build();
    settle(&mut h);
    // regions top to bottom: Netlist, Schematic, Influence line, Member results, Checks
    let tops: Vec<(String, f32)> = nodes_with_roles(&h, &[Role::Region])
        .into_iter()
        .filter(|(_, l, _)| {
            [
                "Netlist",
                "Schematic",
                "Influence line",
                "Member results",
                "Checks",
            ]
            .contains(&l.as_str())
        })
        .map(|(_, l, r)| (l, r.top()))
        .collect();
    let mut sorted = tops.clone();
    sorted.sort_by(|a, b| a.1.total_cmp(&b.1));
    let names: Vec<&str> = sorted.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(
        names,
        [
            "Netlist",
            "Schematic",
            "Influence line",
            "Member results",
            "Checks"
        ]
    );
    // nothing wider than the phone
    for (_, l, r) in nodes_with_roles(&h, &[Role::Region]) {
        assert!(r.width() <= 390.0 - 32.0 + 0.5, "{l} is {} wide", r.width());
    }
    let img = render_full(Route::Advanced, 390.0, 1000.0);
    let p = screenshot_dir().join("advanced-390.png");
    save(&img, &p);
    println!("screenshot: {}", p.display());
    // the full page is about as tall as the reference (3000 px), plus the editor's footer
    assert!(
        (2900..3300).contains(&img.height()),
        "page height {}",
        img.height()
    );
}
