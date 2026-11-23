//! ENG-4 / CMF-2: "Compare shapes" — four headless 18 s open-ground tests of the player's
//! tower — in under 400 ms (release). Prints the `PERF` line `scripts/test-all.sh` gates.

use sp_engine::builder::Game;
use sp_engine::builder::comfort;
use sp_engine::builder::game::{Preset, PresetBuild};
use sp_engine::builder::{LevelKey, PieceType};
use std::time::{Duration, Instant};

/// The best of `n` runs (robust against a busy machine).
fn best<T>(n: usize, mut f: impl FnMut() -> T) -> Duration {
    (0..n)
        .map(|_| {
            let t = Instant::now();
            std::hint::black_box(f());
            t.elapsed()
        })
        .min()
        .unwrap_or_default()
}

#[test]
fn compare_shapes_under_400_ms() {
    // the storm-tower board (64 m, four bays: 85 joints, 221 pieces) and a taller, wider one
    let mut boards = vec![(
        "64 m storm tower",
        Game::with_preset(&Preset::board("TowerStorm").unwrap()),
    )];
    let mut g = Game::with_preset(&Preset::board("TowerStorm").unwrap());
    g.pick_level(LevelKey::T2);
    g.build_preset(PresetBuild::Tower {
        h: 80.0,
        bays: 5,
        col: PieceType::Steel,
        brace: PieceType::Steel,
        found: sp_engine::builder::Foundation::Pile,
    });
    boards.push(("80 m five-bay tower", g));
    for (label, g) in &boards {
        let (d, s, c) = (g.design().clone(), g.setup(), *g.cond());
        let joints = d.nodes.len();
        let t = best(3, || comfort::compare(&d, &s, &c));
        let ms = t.as_secs_f64() * 1e3;
        println!("PERF compare shapes, {label} ({joints} joints): {ms:.1} ms (limit 400 ms)");
        assert!(ms < 400.0, "{label}: {ms:.1} ms");
    }
}
