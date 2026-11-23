//! ENG-4 / TIME-1: the whole 30-year fast-forward (61 settled states, each an exact
//! equilibrium with the P-Δ geometry) in under 300 ms (release). Prints the `PERF` lines
//! `scripts/test-all.sh` gates.

use sp_engine::builder::game::{Preset, PresetBuild};
use sp_engine::builder::longterm::{self, pisa};
use sp_engine::builder::{Conditions, Foundation, Game, LevelKey, PieceType, Scene, Setup, Soil};
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
fn thirty_years_under_300_ms() {
    let level = pisa::level();
    let cond = Conditions::for_level(&level);
    let setup = Setup::new(LevelKey::Pisa, Scene::Flat, &cond);
    let mut cases = vec![("Leaning tower".to_string(), pisa::prebuilt(), setup, cond)];
    for (h, bays) in [(64.0, 4), (80.0, 5)] {
        let mut g = Game::with_preset(&Preset::board("TowerStorm").unwrap());
        g.set_soil(Soil::Soft);
        g.build_preset(PresetBuild::Tower {
            h,
            bays,
            col: PieceType::Steel,
            brace: PieceType::Steel,
            found: Foundation::Footing,
        });
        cases.push((
            format!("{h} m tower on soft clay"),
            g.design().clone(),
            g.setup(),
            *g.cond(),
        ));
    }
    for (label, d, s, c) in &cases {
        let lt = longterm::fast_forward_design(d, s, c);
        assert!(!lt.frames.is_empty());
        let t = best(3, || longterm::fast_forward_design(d, s, c));
        let ms = t.as_secs_f64() * 1e3;
        println!(
            "PERF 30-year fast-forward, {label} ({} joints, {} frames): {ms:.1} ms (limit 300 ms)",
            d.nodes.len(),
            lt.frames.len()
        );
        assert!(ms < 300.0, "{label}: {ms:.1} ms");
    }
}
