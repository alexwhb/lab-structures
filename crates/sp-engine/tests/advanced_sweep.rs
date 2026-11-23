//! Big netlists (review finding, worlds → advanced): the influence line by superposition
//! matches the full solves, the left-to-right joint numbering gives the same solution as the
//! declared order, and dragging Truck on a Bridge-sized netlist doesn't re-solve.

use sp_engine::influence::{self, EXACT_SWEEP_NODES};
use sp_engine::netlist;
use sp_engine::{SolveOptions, Truss};
use std::time::{Duration, Instant};

/// A Warren-ish truss of `panels` 2 m panels, 2 m deep, bottom chord declared first, then the
/// top chord (the worst order for a dense elimination), with a truck along the bottom.
fn long_truss(panels: usize, truck_t: f64) -> String {
    let mut s = String::from(".material steel\n.section s A=50cm2\n");
    for i in 0..=panels {
        let fix = match i {
            0 => "  fix=xy",
            p if p == panels => "  fix=y",
            _ => "",
        };
        s.push_str(&format!(".node B{i}  {}  0{fix}\n", i * 2));
    }
    for i in 0..=panels {
        s.push_str(&format!(".node T{i}  {}  2\n", i * 2));
    }
    let mut m = 0;
    let mut add = |a: String, b: String, s: &mut String| {
        m += 1;
        s.push_str(&format!(".member M{m}  {a}  {b}  s\n"));
    };
    for i in 0..panels {
        add(format!("B{i}"), format!("B{}", i + 1), &mut s);
        add(format!("T{i}"), format!("T{}", i + 1), &mut s);
        add(format!("B{i}"), format!("T{i}"), &mut s);
        add(format!("B{i}"), format!("T{}", i + 1), &mut s);
    }
    add(format!("B{panels}"), format!("T{panels}"), &mut s);
    s.push_str(&format!(
        ".load dead B1..B{} Fy=-5 +self\n.load truck {truck_t}t x=7 lane=B0..B{panels} share=0.5\n",
        panels - 1
    ));
    s
}

#[test]
fn superposition_matches_full_solves() {
    let net = netlist::parse(&long_truss(30, 40.0)).unwrap();
    assert!(net.nodes.len() > EXACT_SWEEP_NODES);
    let sweep = influence::unit_sweep(&net).unwrap();
    for member in [0, 1, 3, 57, 120] {
        let exact = influence::influence_line_exact(&net, member).unwrap();
        let sup = influence::line_from_sweep(&net, &sweep, member).unwrap();
        assert_eq!(exact.points.len(), sup.points.len());
        for (a, b) in exact.points.iter().zip(&sup.points) {
            assert_eq!(a[0], b[0]);
            assert!(
                (a[1] - b[1]).abs() <= 1e-6 * a[1].abs().max(1.0),
                "M{}: {a:?} vs {b:?}",
                member + 1
            );
        }
        assert!((exact.peak - sup.peak).abs() <= 1e-6 * exact.peak.abs().max(1.0));
        // a symmetric line has two equal peaks; round-off may pick either: the exact line is as
        // big at the position superposition chose
        let at = exact.points.iter().find(|p| p[0] == sup.peak_at).unwrap();
        assert!((at[1].abs() - exact.peak.abs()).abs() <= 1e-6 * exact.peak.abs().max(1.0));
    }
    // a heavier truck is the same sweep, scaled
    let heavy = netlist::parse(&long_truss(30, 90.0)).unwrap();
    let a = influence::line_from_sweep(&heavy, &sweep, 3).unwrap();
    let b = influence::influence_line_exact(&heavy, 3).unwrap();
    for (p, q) in a.points.iter().zip(&b.points) {
        assert!((p[1] - q[1]).abs() <= 1e-6 * q[1].abs().max(1.0));
    }
}

#[test]
fn left_to_right_numbering_gives_the_same_solution() {
    let net = netlist::parse(&long_truss(30, 40.0)).unwrap();
    let truss: Truss = net.to_truss();
    let loads = net.loads_at(&truss, 7.0);
    let opts = SolveOptions::default();
    let plain = truss.solve(&loads, &opts).unwrap();
    let ordered = influence::solve_ordered(&truss, &loads, &opts).unwrap();
    assert!(plain.stable && ordered.stable);
    for (a, b) in plain.members.iter().zip(&ordered.members) {
        let (a, b) = (a.unwrap().force, b.unwrap().force);
        assert!((a - b).abs() <= 1e-6 * a.abs().max(1.0), "{a} vs {b}");
    }
    for (a, b) in plain.displacements.iter().zip(&ordered.displacements) {
        assert!((a[1] - b[1]).abs() <= 1e-9 + 1e-6 * a[1].abs());
    }
    for (a, b) in plain.reactions.iter().zip(&ordered.reactions) {
        assert!((a[1] - b[1]).abs() <= 1e-6 * a[1].abs().max(1.0));
    }
    assert_eq!(plain.active, ordered.active);
    // a mechanism names the same joint (mapped back to the declared order)
    let mut loose = truss.clone();
    loose.members.retain(|m| m.id != "M3" && m.id != "M4");
    let loads = net.loads_at(&loose, 7.0);
    let p = loose.solve(&loads, &opts).unwrap();
    let o = influence::solve_ordered(&loose, &loads, &opts).unwrap();
    assert!(!p.stable && !o.stable);
    let j = o.mechanism.unwrap();
    assert!(j < loose.nodes.len());
}

#[test]
fn dragging_truck_on_a_bridge_sized_netlist_does_not_solve() {
    // 450 nodes, declared bottom chord then top chord
    let net = netlist::parse(&long_truss(224, 40.0)).unwrap();
    assert_eq!(net.nodes.len(), 450);
    let t = Instant::now();
    let r = influence::analyse(&net).unwrap();
    assert!(r.solution.stable);
    let sweep = influence::unit_sweep(&net).unwrap();
    let model = t.elapsed();
    let heavier = netlist::parse(&long_truss(224, 80.0)).unwrap();
    let t = Instant::now();
    for member in 0..20 {
        influence::line_from_sweep(&heavier, &sweep, member).unwrap();
    }
    let per_step = t.elapsed() / 20;
    println!(
        "PERF advanced 450-node (worst order): solve + sweep {model:?}, truck/pick step {per_step:?}"
    );
    assert!(model < Duration::from_secs(1), "{model:?}");
    assert!(per_step < Duration::from_millis(1), "{per_step:?}");
}
