//! ENG-4: the influence line (49 full solves of the Advanced truss) in under 5 ms, and a whole
//! screen update (parse + analyse + influence line) comfortably inside a frame.

use sp_engine::generate::{self, Params};
use sp_engine::{influence, netlist};
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
fn influence_line_under_5_ms() {
    for braced in [false, true] {
        let g = generate::generate(&Params {
            braced,
            ..Params::default()
        });
        let r = influence::analyse(&g.netlist).unwrap();
        let pick = influence::picks(&g.netlist, &r)[0].member;
        let t = best(15, || influence::influence_line(&g.netlist, pick));
        assert!(t < Duration::from_millis(5), "influence line took {t:?}");
    }
}

#[test]
fn a_full_update_fits_in_a_frame() {
    let g = generate::generate(&Params::default());
    let t = best(15, || {
        let net = netlist::parse(&g.text).unwrap();
        let r = influence::analyse(&net).unwrap();
        let m = influence::picks(&net, &r)[0].member;
        influence::influence_line(&net, m)
    });
    assert!(t < Duration::from_millis(8), "update took {t:?}");
}
