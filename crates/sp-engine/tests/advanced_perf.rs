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

/// A Bridge-sized netlist (450 nodes, the whole sandbox grid) with a truck: parse, solve and
/// the (coarsened) influence line stay interactive.
#[test]
fn a_bridge_sized_netlist_stays_interactive() {
    let panels = 224;
    let mut s = String::from(".material steel\n.section s A=50cm2\n");
    for i in 0..=panels {
        let fix = match i {
            0 => "  fix=xy",
            p if p == panels => "  fix=y",
            _ => "",
        };
        s.push_str(&format!(".node B{i}  {}  0{fix}\n", i * 2));
        s.push_str(&format!(".node T{i}  {}  2\n", i * 2));
    }
    let mut m = 0;
    let mut member = |a: String, b: String, s: &mut String| {
        m += 1;
        s.push_str(&format!(".member M{m}  {a}  {b}  s\n"));
    };
    for i in 0..panels {
        member(format!("B{i}"), format!("B{}", i + 1), &mut s);
        member(format!("T{i}"), format!("T{}", i + 1), &mut s);
        member(format!("B{i}"), format!("T{i}"), &mut s);
        member(format!("B{i}"), format!("T{}", i + 1), &mut s);
    }
    member(format!("B{panels}"), format!("T{panels}"), &mut s);
    s.push_str(&format!(
        ".load truck 40t x=100 lane=B0..B{panels}\n.load self +self\n"
    ));
    let t = Instant::now();
    let net = netlist::parse(&s).unwrap();
    assert_eq!(net.nodes.len(), 450);
    let r = influence::analyse(&net).unwrap();
    assert!(r.solution.stable);
    let solve = t.elapsed();
    let il = influence::influence_line(&net, 0).unwrap();
    assert!(il.points.len() <= influence::BIG_SWEEP + 1);
    let all = t.elapsed();
    // `PERF <label>: <ms> ms (limit <ms> ms)` — the line format scripts/test-all.sh gates on.
    println!(
        "PERF advanced 450-node parse + solve + influence line: {:.3} ms (limit 1000 ms)",
        all.as_secs_f64() * 1e3
    );
    println!(
        "  (parse + solve alone: {:.3} ms)",
        solve.as_secs_f64() * 1e3
    );
    assert!(all < Duration::from_secs(1), "{all:?}");
}
