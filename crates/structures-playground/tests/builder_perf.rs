//! BLD-15 on the screen side: a whole app frame (engine tick + view + layout + painting into
//! egui's shapes) of the biggest board, TowerStorm (a 64 m tower in a storm), while it runs,
//! stays cheap. Prints the `PERF` gate line `scripts/test-all.sh` collects.

mod common;

use common::*;
use sp_engine::builder::{Game, Phase, Preset};
use std::time::Instant;
use structures_playground::Route;

#[test]
fn a_running_frame_is_cheap() {
    let mut h = harness(Route::Playground);
    let mut p = Preset::board("TowerStorm").expect("the TowerStorm board");
    p.run = false;
    p.demo_loop = false;
    h.state_mut().screens_mut().builder.game = Game::with_preset(&p);
    settle(&mut h);
    h.state_mut().screens_mut().builder.game.start_run(false);
    h.run_steps(10);
    let n = 60;
    let t0 = Instant::now();
    for _ in 0..n {
        // one engine frame and one app frame, as at 30 fps
        h.state_mut().screens_mut().builder.game.tick();
        h.step();
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0 / n as f64;
    let limit = 16.0;
    println!("PERF builder app frame (TowerStorm running): {ms:.2} ms (limit {limit} ms)");
    assert_eq!(
        h.state().screens().builder.game.phase(),
        Phase::Run,
        "still running"
    );
    assert!(ms < limit, "{ms:.2} ms per frame");
}
