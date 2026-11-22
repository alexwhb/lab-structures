//! `sp_engine::materials` reproduces the Materials mockup (SPEC MT-2, ENG-5): every scenario in
//! `materials_goldens.json` (produced by `materials_goldens.mjs`, which runs the mockup's own
//! `class Component` in Node) is replayed on [`Bench`] and every displayed value compared —
//! strings to the character, SVG geometry to the mockup's 0.1 px rounding, state bit for bit.

use serde_json::{Map, Value};
use sp_engine::materials::view::Pt;
use sp_engine::materials::{Bench, Kind, Tone, View, Zoom};
use std::collections::HashMap;
use std::path::PathBuf;

fn goldens_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/materials_goldens.json")
}

fn load() -> Value {
    let text = std::fs::read_to_string(goldens_path()).expect("read materials_goldens.json");
    serde_json::from_str(&text).expect("parse materials_goldens.json")
}

fn kind(key: &str) -> Kind {
    Kind::parse(key).unwrap_or_else(|| panic!("unknown material {key}"))
}

fn zoom_of(v: f64) -> Zoom {
    if v == 25.0 { Zoom::Wide } else { Zoom::Close }
}

fn num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().expect("number"),
        Value::String(s) => s.parse().unwrap_or_else(|_| panic!("not a number: {s}")),
        other => panic!("not a number: {other}"),
    }
}

fn apply(bench: &mut Bench, step: &[Value]) {
    let op = step[0].as_str().expect("op");
    let arg = step.get(1);
    match op {
        "pick" => bench.pick(kind(arg.and_then(Value::as_str).expect("key"))),
        "temp" => bench.set_temp(num(arg.expect("temp"))),
        "strain" => bench.set_strain(num(arg.expect("strain"))),
        "zoom" => bench.set_zoom(zoom_of(num(arg.expect("zoom")))),
        "toggle" => bench.toggle_drive(),
        "fresh" => bench.fresh_sample(),
        "reset" => bench.reset(),
        "tick" => {
            for _ in 0..num(arg.expect("ticks")) as usize {
                bench.tick();
            }
        }
        "snap" => {}
        other => panic!("unknown step {other}"),
    }
}

/// Parses the SVG path subset the mockup builds (M, L, H, V, Z; absolute) into subpaths.
fn parse_path(d: &str) -> Vec<Vec<Pt>> {
    let mut spaced = String::new();
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() {
            spaced.push(' ');
            spaced.push(ch);
            spaced.push(' ');
        } else {
            spaced.push(ch);
        }
    }
    let toks: Vec<&str> = spaced.split_whitespace().collect();
    let mut out: Vec<Vec<Pt>> = Vec::new();
    let mut cur: Pt = [0.0, 0.0];
    let mut i = 0;
    let f = |s: &str| {
        s.parse::<f64>()
            .unwrap_or_else(|_| panic!("bad number {s} in {d}"))
    };
    while i < toks.len() {
        match toks[i] {
            "M" => {
                cur = [f(toks[i + 1]), f(toks[i + 2])];
                out.push(vec![cur]);
                i += 3;
            }
            "L" => {
                cur = [f(toks[i + 1]), f(toks[i + 2])];
                out.last_mut().expect("L before M").push(cur);
                i += 3;
            }
            "H" => {
                cur = [f(toks[i + 1]), cur[1]];
                out.last_mut().expect("H before M").push(cur);
                i += 2;
            }
            "V" => {
                cur = [cur[0], f(toks[i + 1])];
                out.last_mut().expect("V before M").push(cur);
                i += 2;
            }
            "Z" => i += 1,
            t => panic!("unexpected token {t} in {d}"),
        }
    }
    out
}

const TOL: f64 = 0.05 + 1e-9;

fn same_paths(what: &str, want: &str, got: &[Vec<Pt>], ctx: &str) {
    let want = parse_path(want);
    assert_eq!(
        want.len(),
        got.len(),
        "{ctx}: {what}: subpath count {want:?} vs {got:?}"
    );
    for (w, g) in want.iter().zip(got) {
        assert_eq!(w.len(), g.len(), "{ctx}: {what}: points {w:?} vs {g:?}");
        for (a, b) in w.iter().zip(g) {
            assert!(
                (a[0] - b[0]).abs() <= TOL && (a[1] - b[1]).abs() <= TOL,
                "{ctx}: {what}: {a:?} vs {b:?}\n  want {w:?}\n  got  {g:?}"
            );
        }
    }
}

fn near(what: &str, want: f64, got: f64, ctx: &str) {
    assert!((want - got).abs() <= TOL, "{ctx}: {what}: {want} vs {got}");
}

fn tone_of(hex: &str) -> Tone {
    match hex {
        "#A3A3A3" => Tone::Muted,
        "#5FBF8A" => Tone::Ok,
        "#D6C7A1" => Tone::Accent,
        "#E7A47C" => Tone::Caution,
        "#E5534B" => Tone::Danger,
        other => panic!("unknown colour {other}"),
    }
}

fn rect(r: [f64; 4]) -> Vec<Pt> {
    vec![[r[0], r[1]], [r[2], r[1]], [r[2], r[3]], [r[0], r[3]]]
}

fn check_state(bench: &Bench, state: &Value, ctx: &str) {
    assert_eq!(
        bench.kind(),
        kind(state["mat"].as_str().expect("mat")),
        "{ctx}: mat"
    );
    assert_eq!(
        bench.strain().to_bits(),
        num(&state["e"]).to_bits(),
        "{ctx}: e {} vs {}",
        bench.strain(),
        num(&state["e"])
    );
    assert_eq!(bench.temp(), num(&state["temp"]), "{ctx}: temp");
    assert_eq!(bench.zoom().max(), num(&state["zoom"]), "{ctx}: zoom");
    assert_eq!(
        bench.playing(),
        state["playing"].as_bool().expect("playing"),
        "{ctx}: playing"
    );
    let broke = state["broke"].as_object().expect("broke");
    for k in Kind::ALL {
        let want = broke.get(k.mockup_key()).and_then(Value::as_bool) == Some(true);
        assert_eq!(bench.broke(k), want, "{ctx}: broke {k:?}");
    }
}

fn check_view(bench: &Bench, v: &View, w: &Map<String, Value>, ctx: &str) {
    let s = |k: &str| {
        w.get(k)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("{ctx}: missing {k}"))
    };
    let arr = |k: &str| {
        w.get(k)
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("{ctx}: missing {k}"))
    };

    // Plot geometry.
    let ghosts: Vec<Vec<Pt>> = v.plot.ghosts.iter().map(|(_, p)| p.clone()).collect();
    same_paths("ghostD", s("ghostD"), &ghosts, ctx);
    same_paths(
        "curveD",
        s("curveD"),
        std::slice::from_ref(&v.plot.curve),
        ctx,
    );
    let done: Vec<Vec<Pt>> = if v.plot.done.is_empty() {
        vec![]
    } else {
        vec![v.plot.done.clone()]
    };
    same_paths("doneD", s("doneD"), &done, ctx);
    let grid: Vec<Vec<Pt>> = v.plot.grid.iter().map(|l| l.to_vec()).collect();
    same_paths("gridD", s("gridD"), &grid, ctx);
    near("mx", num(&w["mx"]), v.plot.marker[0], ctx);
    near("my", num(&w["my"]), v.plot.marker[1], ctx);
    let ty = arr("ticksY");
    assert_eq!(ty.len(), v.plot.ticks_y.len(), "{ctx}: ticksY");
    for (t, g) in ty.iter().zip(&v.plot.ticks_y) {
        assert_eq!(t["v"].to_string(), g.label, "{ctx}: ticksY label");
        near("ticksY.y", num(&t["y"]), g.pos / 400.0 * 100.0, ctx);
    }
    let tx = arr("ticksX");
    assert_eq!(tx.len(), v.plot.ticks_x.len(), "{ctx}: ticksX");
    for (t, g) in tx.iter().zip(&v.plot.ticks_x) {
        assert_eq!(t["v"].to_string(), g.label, "{ctx}: ticksX label");
        near("ticksX.x", num(&t["x"]), g.pos / 560.0 * 100.0, ctx);
    }
    let cl = arr("curveLabels");
    assert_eq!(cl.len(), v.plot.labels.len(), "{ctx}: curveLabels");
    for (c, g) in cl.iter().zip(&v.plot.labels) {
        assert_eq!(c["name"], g.name, "{ctx}: label name");
        near("label.x", num(&c["x"]), g.at[0] / 560.0 * 100.0, ctx);
        near("label.y", num(&c["y"]), g.at[1] / 400.0 * 100.0, ctx);
        assert_eq!(c["c"] == "#F5F5F5", g.selected, "{ctx}: label colour");
        assert_eq!(c["fw"] == "600", g.selected, "{ctx}: label weight");
    }

    // Rig geometry.
    same_paths("specD", s("specD"), &v.rig.specimen, ctx);
    let crack: Vec<Vec<Pt>> = v.rig.crack.iter().map(|c| c.to_vec()).collect();
    same_paths("crackD", s("crackD"), &crack, ctx);
    let gauge: Vec<Vec<Pt>> = v.rig.gauge.iter().map(|l| l.to_vec()).collect();
    same_paths("gaugeD", s("gaugeD"), &gauge, ctx);
    let head: Vec<Vec<Pt>> = v.rig.head.iter().map(|r| rect(*r)).collect();
    same_paths("headD", s("headD"), &head, ctx);
    same_paths(
        "headTopD",
        s("headTopD"),
        std::slice::from_ref(&v.rig.head_top.to_vec()),
        ctx,
    );

    // Strings, verbatim.
    for (key, got) in [
        ("rigLabel", &v.rig_label),
        ("plotLabel", &v.plot_label),
        ("force", &v.force_label),
        ("statusLabel", &v.status_label),
        ("explain", &v.explain),
        ("tempLabel", &v.temp_label),
        ("tempNote", &v.temp_note),
        ("strainLabel", &v.strain_label),
        ("clock", &v.clock),
    ] {
        assert_eq!(s(key), got, "{ctx}: {key}");
    }
    assert_eq!(s("note"), v.note, "{ctx}: note");
    assert_eq!(
        tone_of(s("statusColor")),
        v.status_tone,
        "{ctx}: statusColor"
    );
    assert_eq!(tone_of(s("tempColor")), v.temp_tone, "{ctx}: tempColor");

    // Numbers the controls show.
    assert_eq!(num(&w["temp"]), bench.temp(), "{ctx}: temp");
    assert_eq!(num(&w["e"]).to_bits(), bench.strain().to_bits(), "{ctx}: e");
    assert_eq!(num(&w["zoom"]), bench.zoom().max(), "{ctx}: zoom");
    assert_eq!(num(&w["step"]), bench.zoom().slider_step(), "{ctx}: step");
    assert_eq!(
        w["playing"].as_bool(),
        Some(bench.playing()),
        "{ctx}: playing"
    );

    // Tiles, bars, chips, pickers.
    let tiles = arr("tiles");
    assert_eq!(tiles.len(), v.tiles.len(), "{ctx}: tiles");
    for (t, g) in tiles.iter().zip(&v.tiles) {
        assert_eq!(t["k"], g.key, "{ctx}: tile key");
        assert_eq!(t["v"], g.value.as_str(), "{ctx}: tile {}", g.key);
        assert_eq!(t["n"], g.note, "{ctx}: tile note {}", g.key);
    }
    let spec = arr("spec");
    assert_eq!(spec.len(), v.spec.len(), "{ctx}: spec");
    for (c, g) in spec.iter().zip(&v.spec) {
        assert_eq!(c["name"], g.name, "{ctx}: spec name");
        assert_eq!(c["v"], g.label.as_str(), "{ctx}: spec value {}", g.name);
        assert_eq!(
            c["w"].as_str().map(|x| x.parse::<f64>().unwrap_or(-1.0)),
            Some((g.fill * 100.0).round()),
            "{ctx}: spec width {}",
            g.name
        );
        assert_eq!(c["color"] == "#D6C7A1", g.selected, "{ctx}: spec colour");
        assert_eq!(c["tc"] == "#F5F5F5", g.selected, "{ctx}: spec text colour");
        assert_eq!(c["fw"] == "600", g.selected, "{ctx}: spec weight");
    }
    let broken = arr("broken");
    assert_eq!(broken.len(), v.challenge.len(), "{ctx}: broken");
    for (b, g) in broken.iter().zip(&v.challenge) {
        assert_eq!(b["name"], g.name, "{ctx}: chip name");
        assert_eq!(b["c"] == "#5FBF8A", g.lit, "{ctx}: chip {}", g.name);
    }
    let mats = arr("mats");
    for (m, k) in mats.iter().zip(Kind::ALL) {
        assert_eq!(m.as_bool(), Some(k == v.kind), "{ctx}: sample tile {k:?}");
    }
    let zooms = arr("zooms");
    for (z, k) in zooms.iter().zip(Zoom::ALL) {
        assert_eq!(z.as_bool(), Some(k == bench.zoom()), "{ctx}: zoom {k:?}");
    }
}

#[test]
fn every_mockup_scenario_replays_exactly() {
    let g = load();
    let scenarios = g["scenarios"].as_array().expect("scenarios");
    let snaps = g["snaps"].as_array().expect("snaps");
    let mut by_step: HashMap<(usize, usize), &Value> = HashMap::new();
    for s in snaps {
        let key = (
            s["scenario"].as_u64().expect("scenario") as usize,
            s["step"].as_u64().expect("step") as usize,
        );
        by_step.insert(key, s);
    }
    let mut compared = 0;
    for (si, sc) in scenarios.iter().enumerate() {
        let name = sc["name"].as_str().expect("name");
        let mut bench = Bench::default();
        let mut view_acc: Map<String, Value> = Map::new();
        for (i, step) in sc["steps"].as_array().expect("steps").iter().enumerate() {
            let step = step.as_array().expect("step");
            apply(&mut bench, step);
            if let Some(snap) = by_step.get(&(si, i)) {
                let ctx = format!("scenario {si} '{name}' step {i}");
                check_state(&bench, &snap["state"], &ctx);
                for (k, val) in snap["view"].as_object().expect("view") {
                    view_acc.insert(k.clone(), val.clone());
                }
                check_view(&bench, &bench.view(), &view_acc, &ctx);
                compared += 1;
            }
        }
    }
    assert_eq!(compared, snaps.len(), "every snapshot compared");
    assert!(compared > 200, "goldens cover many states ({compared})");
}

/// When Node is available, the checked-in goldens must be what the mockup produces today.
#[test]
fn goldens_are_fresh_when_node_is_available() {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/materials_goldens.mjs");
    let Ok(out) = std::process::Command::new("node")
        .arg(&script)
        .arg("--stdout")
        .output()
    else {
        eprintln!("node not found: skipping the freshness check");
        return;
    };
    if !out.status.success() {
        panic!(
            "materials_goldens.mjs failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let fresh = String::from_utf8(out.stdout).expect("utf-8");
    let checked_in = std::fs::read_to_string(goldens_path()).expect("read goldens");
    assert!(
        fresh == checked_in,
        "materials_goldens.json is stale: run `node crates/sp-engine/tests/materials_goldens.mjs`"
    );
}
