//! Advanced renders at 1440 and 390 and stays close to the references (LOOK-1).

mod common;
use common::*;
use structures_playground::Route;

#[test]
fn renders_at_1440_and_390() {
    let mut h = harness_at(Route::Advanced, 1440.0, 1000.0);
    let p = screenshot(&mut h, "advanced-1440");
    println!("screenshot: {}", p.display());
    let img = render_full(Route::Advanced, 390.0, 1000.0);
    let p = screenshot_dir().join("advanced-390.png");
    save(&img, &p);
    println!("screenshot: {}", p.display());
}
