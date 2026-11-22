//! `sp_engine::materials` speed (ENG-4): the screen recomputes the whole [`View`] every frame
//! and a Pull step every 50 ms, so both must cost next to nothing. Prints `PERF` lines for
//! `scripts/test-all.sh` (release gates).

use sp_engine::materials::{Bench, Kind};
use std::time::Instant;

#[test]
fn view_and_pull_are_cheap() {
    let mut b = Bench::default();
    b.set_strain(17.3);
    let n = 2000;
    let t = Instant::now();
    let mut pts = 0;
    for _ in 0..n {
        pts += b.view().plot.curve.len();
    }
    let view_ms = t.elapsed().as_secs_f64() * 1e3 / n as f64;
    assert!(pts > 0);
    println!("PERF materials view(): {view_ms:.4} ms (limit 0.2 ms)");
    assert!(view_ms < 0.2, "view() took {view_ms:.4} ms");

    let t = Instant::now();
    let runs = 100;
    let mut total = 0.0;
    for i in 0..runs {
        let mut b = Bench::default();
        b.pick(Kind::ALL[i % 5]);
        b.toggle_drive();
        while b.playing() {
            b.tick();
        }
        total += std::hint::black_box(b.strain());
    }
    assert!(total > 0.0);
    let pull_ms = t.elapsed().as_secs_f64() * 1e3 / runs as f64;
    println!("PERF materials whole pull to fracture: {pull_ms:.4} ms (limit 1 ms)");
    assert!(pull_ms < 1.0, "a whole pull took {pull_ms:.4} ms");
}
