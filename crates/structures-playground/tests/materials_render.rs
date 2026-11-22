//! Materials (SCR-5) renders at 1440 and 390 and stays close to the mockup references
//! (LOOK-1): mean channel difference per column against `docs/design/screenshots/
//! Materials-1440.png`, and at 390 the columns stack in the mockup's order. The renders go to
//! the test's scratch dir (`screenshots/materials-*.png`); `scripts/compare-mockup.sh` makes the
//! side-by-side images.

mod common;

use common::*;
use eframe::egui::accesskit::Role;
use structures_playground::Route;

/// (name, region x, y, w, h, bound): the three columns under the chrome. Measured at the time
/// of writing: parts ≈ 1.9, stage ≈ 5.6, readouts ≈ 6.2; the bounds leave room for font
/// rasterisation differences, not for layout drift.
const REGIONS: [(&str, u32, u32, u32, u32, f64); 3] = [
    ("parts", 0, 106, 249, 794, 3.5),
    ("stage", 251, 106, 888, 580, 7.5),
    ("readouts", 1141, 106, 299, 794, 7.5),
];

#[test]
fn stays_close_to_the_1440_reference() {
    let mut h = harness(Route::Materials);
    settle(&mut h);
    let img = render(&mut h);
    save(&img, &screenshot_dir().join("materials-1440.png"));
    let reference = load(&mockup_png(Route::Materials, 1440));
    assert_eq!((img.width(), img.height()), (1440, 900));
    let mut report = Vec::new();
    for (name, x, y, w, hh, bound) in REGIONS {
        let d = mean_diff(&img, &reference, Some((x, y, w, hh)));
        report.push(format!("{name} {d:.2} (≤ {bound})"));
        assert!(d <= bound, "{name}: mean |Δ| {d:.2} > {bound}");
    }
    println!("materials-1440 vs mockup: {}", report.join(", "));
}

#[test]
fn renders_stacked_at_390() {
    let img = render_full(Route::Materials, PHONE_W, PHONE_H);
    save(&img, &screenshot_dir().join("materials-390.png"));
    assert_eq!(img.width(), PHONE_W as u32);
    // The reference is one long column too (structure, not pixels: LOOK-1).
    let reference = load(&mockup_png(Route::Materials, 390));
    assert!(img.height() >= reference.height() * 9 / 10);
    // Columns in the mockup's order: Parts, Stage, Readouts.
    let h = app().at(Route::Materials).phone().build();
    let ys: Vec<f32> = ["Parts", "Stage", "Readouts"]
        .iter()
        .map(|l| get(&h, l, Role::Region).rect().top())
        .collect();
    assert!(ys[0] < ys[1] && ys[1] < ys[2], "{ys:?}");
}

/// Renders the rig mid-test for review (`materials-necking.png`, `materials-fractured.png`,
/// `materials-hot-timber.png`): necking narrows the gauge, a fracture splits the sample with a
/// red crack, and the plot's done trace and marker follow.
#[test]
fn renders_pull_states() {
    use sp_engine::materials::Kind;
    let mut h = harness(Route::Materials);
    h.state_mut()
        .screens_mut()
        .materials
        .bench_mut()
        .set_strain(19.5);
    settle(&mut h);
    let v = h.state().screens().materials.view();
    assert_eq!(v.status_label, "Necking");
    assert!(v.rig.necking > 0.3);
    screenshot(&mut h, "materials-necking");
    let b = h.state_mut().screens_mut().materials.bench_mut();
    b.toggle_drive();
    while b.playing() {
        b.tick();
    }
    settle(&mut h);
    assert_eq!(
        h.state().screens().materials.view().status_label,
        "Fractured at 22%"
    );
    screenshot(&mut h, "materials-fractured");
    let m = &mut h.state_mut().screens_mut().materials;
    m.pick(Kind::Timber);
    m.set_temp(250.0);
    m.set_strain(0.2);
    settle(&mut h);
    screenshot(&mut h, "materials-hot-timber");
}
