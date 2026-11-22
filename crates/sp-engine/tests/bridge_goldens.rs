//! Golden tests: `sp_engine::bridge` against the Playground mockup's own JavaScript
//! (SPEC ENG-2, ENG-5, BR-4…BR-10, DONE-2).
//!
//! `bridge_goldens.json` is written by `node crates/sp-engine/tests/bridge_goldens.mjs`, which
//! runs the mockup's `class Component` verbatim through scripted scenarios (build level 1 by
//! clicks, test it, break a thin timber truss and replay it, a mechanism, a loose weight, the
//! Sandbox starter truss, every placement rule and toast, drag, undo, the Overhang and Heavy
//! haul levels). This test replays the same actions on [`Game`] and checks, at every snapshot,
//! the state, the solve (forces, utilisation, modes, displacements, mass, cost), every string
//! the screen shows, the drawn geometry (member bands and positions, supports, crates,
//! mechanism ring, sparks) and the collapse simulation.

use serde_json::Value;
use sp_engine::bridge::data::{px, py, snap};
use sp_engine::bridge::design::Band;
use sp_engine::bridge::fmt::to_fixed;
use sp_engine::bridge::view::{BarTone, Inspector, Tone, View};
use sp_engine::bridge::{Game, LevelKey, MaterialKey, Phase, Selection, SizeKey, Support, Tool};

const GOLDENS: &str = include_str!("bridge_goldens.json");

fn close(a: f64, b: f64, rel: f64, abs: f64) -> bool {
    a == b || (a - b).abs() <= abs.max(rel * a.abs().max(b.abs()))
}

/// Equal to within 2 ulps: serde_json's default float parser (no `float_roundtrip`) can be off
/// by one ulp, so "bit-exact" values from the mockup are compared this way.
fn exact(a: f64, b: f64) -> bool {
    close(a, b, 2.5 * f64::EPSILON, 0.0)
}

fn s(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(t) => t.clone(),
        other => other.to_string(),
    }
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

fn level(k: &str) -> LevelKey {
    LevelKey::from_key(k).unwrap()
}

fn tool(k: &str) -> Tool {
    match k {
        "select" => Tool::Select,
        "member" => Tool::Member,
        "joint" => Tool::Joint,
        "support" => Tool::Support,
        "weight" => Tool::Weight,
        "erase" => Tool::Erase,
        _ => panic!("tool {k}"),
    }
}

fn mat(k: &str) -> MaterialKey {
    match k {
        "steel" => MaterialKey::Steel,
        "timber" => MaterialKey::Timber,
        "alu" => MaterialKey::Aluminum,
        "cf" => MaterialKey::CarbonFiber,
        _ => panic!("mat {k}"),
    }
}

fn size(k: &str) -> SizeKey {
    match k {
        "S" => SizeKey::Thin,
        "M" => SizeKey::Medium,
        "L" => SizeKey::Thick,
        _ => panic!("size {k}"),
    }
}

fn size_letter(z: SizeKey) -> &'static str {
    match z {
        SizeKey::Thin => "S",
        SizeKey::Medium => "M",
        SizeKey::Thick => "L",
    }
}

fn phase_key(p: Phase) -> &'static str {
    match p {
        Phase::Build => "build",
        Phase::Testing => "testing",
        Phase::Collapse => "collapse",
        Phase::Collapsed => "collapsed",
        Phase::Held => "held",
    }
}

fn tone_hex(t: Tone) -> &'static str {
    match t {
        Tone::Muted => "#A3A3A3",
        Tone::Ok => "#5FBF8A",
        Tone::Warn => "#D8B86A",
        Tone::Danger => "#E5534B",
    }
}

fn mat_key(m: MaterialKey) -> &'static str {
    match m {
        MaterialKey::Steel => "steel",
        MaterialKey::Timber => "timber",
        MaterialKey::Aluminum => "alu",
        MaterialKey::CarbonFiber => "cf",
    }
}

fn grid(x: f64, y: f64) -> (i32, i32) {
    snap(px(x), py(y))
}

/// The numbers in an SVG path string, in order.
fn nums(d: &str) -> Vec<f64> {
    d.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|t| !t.is_empty() && *t != "-")
        .filter_map(|t| t.parse().ok())
        .collect()
}

fn assert_nums(ctx: &str, what: &str, js: &str, ours: &[f64], tol: f64) {
    let a = nums(js);
    assert_eq!(
        a.len(),
        ours.len(),
        "{ctx}: {what}: count\n js: {js}\nours: {ours:?}"
    );
    for (i, (x, y)) in a.iter().zip(ours).enumerate() {
        assert!(
            (x - y).abs() <= tol,
            "{ctx}: {what}[{i}]: js {x} vs ours {y}\n js: {js}\nours: {ours:?}"
        );
    }
}

fn apply(g: &mut Game, a: &[Value]) {
    let op = a[0].as_str().unwrap();
    let p = a.get(1);
    let q = a.get(2);
    let ps = || p.and_then(Value::as_str).unwrap_or("");
    match op {
        "snap" => {}
        "tool" => g.pick_tool(tool(ps())),
        "size" => g.pick_size(size(ps())),
        "weight" => g.pick_weight(p.and_then(Value::as_u64).unwrap() as u32),
        "mat" => g.pick_material(mat(ps())),
        "level" => g.pick_level(level(ps())),
        "joint" => g.click_joint(ps()),
        "grid" => {
            let (x, y) = grid(f(p.unwrap()), f(q.unwrap()));
            g.click_grid(x, y)
        }
        "finish" => g.finish_line(),
        "member" => {
            if g.view().handles.iter().any(|h| h.id == ps()) {
                g.click_member(ps())
            }
        }
        "msize" => {
            if let Inspector::Member(mi) = g.view().inspector {
                g.set_member_size(&mi.id, size(q.and_then(Value::as_str).unwrap()))
            }
        }
        "less" | "more" => {
            if let Inspector::Joint(ni) = g.view().inspector {
                let w = g.weight() as f64;
                g.add_load(&ni.id, if op == "less" { -w } else { w })
            }
        }
        "down" => {
            if g.view().joints.iter().any(|j| j.id == ps()) {
                g.joint_down(ps())
            }
        }
        "move" => {
            let (x, y) = grid(f(p.unwrap()), f(q.unwrap()));
            g.pointer_move(x, y)
        }
        "up" => g.pointer_up(),
        "undo" => g.undo(),
        "clear" => g.clear(),
        "starter" => g.starter(),
        "nums" => g.toggle_forces(),
        "sag" => g.toggle_sag(),
        "test" => g.start_or_stop(),
        "back" => g.back_to_build(),
        "replay" => g.replay(),
        "tick" => {
            for _ in 0..p.and_then(Value::as_u64).unwrap() {
                g.tick()
            }
        }
        "run" => {
            for _ in 0..2000 {
                if !g.animating() {
                    break;
                }
                g.tick()
            }
        }
        other => panic!("unknown action {other}"),
    }
}

/// The mockup's `j.cls` for a joint view.
fn joint_cls(j: &sp_engine::bridge::view::JointView) -> String {
    let mut c = String::from("joint");
    if j.locked {
        c += " j-lock";
    }
    if j.pending {
        c += " j-pend";
    }
    if j.selected {
        c += " j-sel";
    }
    if j.bad {
        c += " j-bad";
    }
    c
}

fn handle_cls(h: &sp_engine::bridge::view::HandleView) -> String {
    let mut c = String::from("mh");
    if h.numeric {
        c += " mh-num";
    }
    if h.selected {
        c += " mh-sel";
    }
    if h.over {
        c += " mh-over";
    }
    c
}

fn pct_x(x: f64) -> f64 {
    px(x) / 1120.0 * 100.0
}
fn pct_y(y: f64) -> f64 {
    py(y) / 600.0 * 100.0
}

fn check_view(ctx: &str, g: &Game, v: &View, j: &Value) {
    let eq = |what: &str, js: &Value, ours: String| {
        assert_eq!(s(js), ours, "{ctx}: {what}");
    };
    eq("statusLabel", &j["statusLabel"], v.status.label.clone());
    eq(
        "statusColor",
        &j["statusColor"],
        tone_hex(v.status.tone).into(),
    );
    eq("costText", &j["costText"], v.cost_text.clone());
    eq(
        "costColor",
        &j["costColor"],
        if v.over_budget { "#E8A39E" } else { "#EAEAEA" }.into(),
    );
    eq("explain", &j["explain"], v.explain.clone());
    eq("stageLabel", &j["stageLabel"], v.stage_label.clone());
    eq("surfaceLabel", &j["surfaceLabel"], v.surface_label.into());
    eq("hint", &j["hint"], v.hint.into());
    eq("clock", &j["clock"], v.clock.clone());
    eq("testLabel", &j["testLabel"], v.test_label.into());
    eq("levelSlug", &j["levelSlug"], v.slug.into());
    assert_eq!(
        j["showResult"].as_bool(),
        Some(v.result.is_some()),
        "{ctx}: showResult"
    );
    if let Some(r) = &v.result {
        eq("resultTitle", &j["resultTitle"], r.title.clone());
        eq(
            "resultColor",
            &j["resultColor"],
            if r.held { "#5FBF8A" } else { "#E5534B" }.into(),
        );
        eq("stars", &j["stars"], r.stars.clone());
        assert_eq!(j["collapsed"].as_bool(), Some(r.replay), "{ctx}: collapsed");
    } else {
        assert_eq!(s(&j["stars"]), "", "{ctx}: stars");
    }
    assert_eq!(
        j["testing"].as_bool(),
        Some(v.loading.is_some()),
        "{ctx}: testing"
    );
    if let Some(l) = &v.loading {
        eq("lamPct", &j["lamPct"], l.text.trim_end_matches('%').into());
    }
    assert_eq!(j["playing"].as_bool(), Some(v.playing), "{ctx}: playing");
    assert_eq!(
        j["hasPending"].as_bool(),
        Some(v.has_pending),
        "{ctx}: hasPending"
    );
    assert_eq!(
        j["showSizes"].as_bool(),
        Some(v.show_sizes),
        "{ctx}: showSizes"
    );
    assert_eq!(
        j["showWeights"].as_bool(),
        Some(v.show_weights),
        "{ctx}: showWeights"
    );
    assert_eq!(
        j["toastOn"].as_bool(),
        Some(v.toast.is_some()),
        "{ctx}: toastOn"
    );
    assert_eq!(s(&j["cursor"]) == "crosshair", v.crosshair, "{ctx}: cursor");
    // water: the mockup animates in every phase but build; we animate while something moves
    // (SPEC BR-1 / PLAT-3: only while testing), so only check build ⇒ still.
    if s(&j["flowCls"]).contains("still") {
        assert!(!v.water_flowing, "{ctx}: water");
    }
    // levels, materials, tools
    let levels = j["levels"].as_array().unwrap();
    assert_eq!(levels.len(), v.levels.len());
    for (lj, lr) in levels.iter().zip(&v.levels) {
        assert_eq!(s(&lj[0]), lr.name, "{ctx}: level name");
        assert_eq!(s(&lj[1]), lr.best, "{ctx}: level best");
        assert_eq!(s(&lj[2]) == "lvl lvl-on", lr.on, "{ctx}: level on");
    }
    for (mj, m) in j["mats"].as_array().unwrap().iter().zip(MaterialKey::ALL) {
        assert_eq!(
            s(mj) == "chip chip-on",
            g.material() == m,
            "{ctx}: material chip"
        );
    }
    for (tj, t) in j["tools"].as_array().unwrap().iter().zip(Tool::ALL) {
        assert_eq!(s(tj) == "tool tool-on", g.tool() == t, "{ctx}: tool");
    }
    // joints
    let joints = j["joints"].as_array().unwrap();
    assert_eq!(joints.len(), v.joints.len(), "{ctx}: joint count");
    for (jj, jv) in joints.iter().zip(&v.joints) {
        assert_eq!(s(&jj[0]), jv.label, "{ctx}: joint label");
        assert_eq!(s(&jj[1]), joint_cls(jv), "{ctx}: joint cls {}", jv.id);
        let (x, y) = (
            s(&jj[2]).parse::<f64>().unwrap(),
            s(&jj[3]).parse::<f64>().unwrap(),
        );
        assert!(
            (x - pct_x(jv.at[0])).abs() < 0.0051 && (y - pct_y(jv.at[1])).abs() < 0.0051,
            "{ctx}: joint {} at {x},{y} vs {:?}",
            jv.id,
            jv.at
        );
    }
    // handles
    let handles = j["handles"].as_array().unwrap();
    assert_eq!(handles.len(), v.handles.len(), "{ctx}: handle count");
    for (hj, hv) in handles.iter().zip(&v.handles) {
        assert_eq!(s(&hj[0]), hv.label, "{ctx}: handle label");
        assert_eq!(s(&hj[1]), hv.text, "{ctx}: handle text");
        assert_eq!(s(&hj[2]), handle_cls(hv), "{ctx}: handle cls");
        let (x, y) = (
            s(&hj[3]).parse::<f64>().unwrap(),
            s(&hj[4]).parse::<f64>().unwrap(),
        );
        assert!(
            (x - pct_x(hv.at[0])).abs() < 0.0051 && (y - pct_y(hv.at[1])).abs() < 0.0051,
            "{ctx}: handle {} position",
            hv.id
        );
    }
    // load tags
    let tags = j["loadTags"].as_array().unwrap();
    assert_eq!(tags.len(), v.loads.len(), "{ctx}: load count");
    for (tj, lv) in tags.iter().zip(&v.loads) {
        assert_eq!(s(&tj[0]), lv.tag, "{ctx}: load tag");
    }
    // inspector
    match (&j["mi"], &j["ni"], &v.inspector) {
        (Value::Object(mi), Value::Null, Inspector::Member(m)) => {
            assert_eq!(s(&mi["title"]), m.title, "{ctx}: mi.title");
            assert_eq!(s(&mi["len"]), m.length, "{ctx}: mi.len");
            assert_eq!(
                s(&mi["w"]).parse::<f64>().unwrap() / 100.0,
                m.fill,
                "{ctx}: mi.w"
            );
            assert_eq!(s(&mi["ut"]), m.ut, "{ctx}: mi.ut");
            let c = match m.tone {
                BarTone::Zero => "#5E5E5E",
                BarTone::Over => "#E5534B",
                BarTone::Tension => "#7FB0E0",
                BarTone::Compression => "#E0905E",
            };
            assert_eq!(s(&mi["color"]), c, "{ctx}: mi.color");
            assert_eq!(s(&mi["text"]), m.text, "{ctx}: mi.text");
            for (cj, z) in mi["sizes"].as_array().unwrap().iter().zip(SizeKey::ALL) {
                assert_eq!(s(cj) == "chip chip-on", m.size == z, "{ctx}: mi size chip");
            }
        }
        (Value::Null, Value::Object(ni), Inspector::Joint(n)) => {
            assert_eq!(s(&ni["title"]), n.title, "{ctx}: ni.title");
            assert_eq!(s(&ni["pos"]), n.pos, "{ctx}: ni.pos");
            assert_eq!(s(&ni["load"]), n.load, "{ctx}: ni.load");
            assert_eq!(s(&ni["note"]), n.note, "{ctx}: ni.note");
            assert_eq!(
                ni["canDel"].as_bool(),
                Some(n.can_delete),
                "{ctx}: ni.canDel"
            );
            for (cj, sup) in ni["sups"].as_array().unwrap().iter().zip([
                None,
                Some(Support::Pin),
                Some(Support::Roller),
            ]) {
                let mut c = String::from("chip");
                if n.support == sup {
                    c += " chip-on";
                }
                if n.chip_dim(sup) {
                    c += " chip-dim";
                }
                assert_eq!(s(cj), c, "{ctx}: ni support chip");
            }
        }
        (Value::Null, Value::Null, Inspector::Nothing(b)) => {
            assert_eq!(s(&j["massT"]), b.weight, "{ctx}: massT");
            assert_eq!(s(&j["nJ"]), b.joints, "{ctx}: nJ");
            assert_eq!(s(&j["nM"]), b.members, "{ctx}: nM");
            assert_eq!(
                j["canStarter"].as_bool(),
                Some(b.can_starter),
                "{ctx}: canStarter"
            );
        }
        (mi, ni, ours) => panic!("{ctx}: inspector mismatch: js mi={mi} ni={ni}, ours {ours:?}"),
    }
    // drawn geometry: members by band and size, the back face, supports, crates, ring, sparks
    let p = |at: [f64; 2]| [px(at[0]), py(at[1])];
    let bands = j["bands"].as_object().unwrap();
    for b in [
        Band::T1,
        Band::T2,
        Band::T3,
        Band::C1,
        Band::C2,
        Band::C3,
        Band::Zero,
        Band::Over,
        Band::None,
    ] {
        for z in SizeKey::ALL {
            let key = format!("{}{}", b.key(), size_letter(z));
            let ours: Vec<f64> = v
                .members
                .iter()
                .filter(|m| m.band == b && m.size == z)
                .flat_map(|m| {
                    let (a, c) = (p(m.a), p(m.b));
                    [a[0], a[1], c[0], c[1]]
                })
                .collect();
            // Mockup bug: `renderVals()` returns `nM` as both the member count and the
            // "no solution, Medium" path, so the mockup never draws unstable Medium members
            // (SPEC BR-2 says they are grey-blue): skip that one key.
            if key == "nM" {
                continue;
            }
            let js = bands.get(&key).map(s).unwrap_or_default();
            assert_nums(ctx, &key, &js, &ours, 0.051);
        }
    }
    let back: Vec<f64> = v
        .members
        .iter()
        .flat_map(|m| {
            let (a, c) = (p(m.a), p(m.b));
            [a[0] + 18.0, a[1] - 12.0, c[0] + 18.0, c[1] - 12.0]
        })
        .collect();
    assert_nums(ctx, "backD", &s(&j["backD"]), &back, 0.051);
    // supports: count triangles (each "M x y L … Z") and their apex
    let sup_js = nums(&s(&j["supD"]));
    let mut sup_ours = Vec::new();
    for sv in &v.supports {
        let (x, y) = (px(sv.at[0]), py(sv.at[1]));
        sup_ours.extend([x, y, x - 9.0, y + 13.0, x + 9.0]);
        if sv.kind == Support::Roller {
            sup_ours.extend([x - 5.0, y + 17.0, 3.0, 3.0, 0.0, 1.0, 0.0, 0.01, 0.0]);
            sup_ours.extend([x + 5.0, y + 17.0, 3.0, 3.0, 0.0, 1.0, 0.0, 0.01, 0.0]);
        }
    }
    assert_eq!(sup_js.len(), sup_ours.len(), "{ctx}: supD");
    for (a, b) in sup_js.iter().zip(&sup_ours) {
        assert!((a - b).abs() < 1e-6, "{ctx}: supD {a} vs {b}");
    }
    // crates: the cable starts 3 px under the drawn joint
    let cable = nums(&s(&j["cableD"]));
    let ours_cable: Vec<f64> = v
        .loads
        .iter()
        .flat_map(|l| {
            let (x, y) = (px(l.at[0]), py(l.at[1]));
            [x, y + 3.0, y + 9.0]
        })
        .collect();
    assert_eq!(cable.len(), ours_cable.len(), "{ctx}: cableD");
    for (a, b) in cable.iter().zip(&ours_cable) {
        assert!((a - b).abs() < 1e-6, "{ctx}: cableD {a} vs {b}");
    }
    assert_eq!(
        s(&j["badD"]).is_empty(),
        v.bad.is_none(),
        "{ctx}: badD {:?}",
        v.bad
    );
    if let Some(b) = v.bad {
        let n = nums(&s(&j["badD"]));
        assert!(
            (n[0] - (px(b[0]) + 13.0)).abs() < 1e-6 && (n[1] - py(b[1])).abs() < 1e-6,
            "{ctx}: badD position"
        );
    }
    assert_eq!(
        nums(&s(&j["sparkD"])).len(),
        v.sparks.len() * 8,
        "{ctx}: sparks"
    );
    assert_eq!(
        s(&j["hoverD"]).is_empty(),
        v.hover.is_none(),
        "{ctx}: hoverD"
    );
    assert_eq!(
        s(&j["ghostD"]).is_empty(),
        v.ghost.is_none(),
        "{ctx}: ghostD"
    );
}

fn check(ctx: &str, g: &Game, j: &Value) {
    assert_eq!(s(&j["phase"]), phase_key(g.phase()), "{ctx}: phase");
    assert!(
        exact(f(&j["lam"]), g.lam()),
        "{ctx}: lam {} vs {}",
        j["lam"],
        g.lam()
    );
    assert_eq!(s(&j["level"]), g.level().key(), "{ctx}: level");
    assert_eq!(s(&j["mat"]), mat_key(g.material()), "{ctx}: mat");
    let d = g.design();
    let nodes = j["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), d.nodes.len(), "{ctx}: node count");
    for (nj, n) in nodes.iter().zip(&d.nodes) {
        assert_eq!(s(&nj[0]), n.id, "{ctx}: node id");
        assert_eq!(f(&nj[1]) as i32, n.x, "{ctx}: {} x", n.id);
        assert_eq!(f(&nj[2]) as i32, n.y, "{ctx}: {} y", n.id);
        assert_eq!(
            s(&nj[3]),
            n.sup.map(|x| x.word()).unwrap_or(""),
            "{ctx}: {} sup",
            n.id
        );
        assert_eq!(nj[4].as_bool(), Some(n.locked), "{ctx}: {} locked", n.id);
    }
    let members = j["members"].as_array().unwrap();
    assert_eq!(members.len(), d.members.len(), "{ctx}: member count");
    for (mj, m) in members.iter().zip(&d.members) {
        assert_eq!(
            (s(&mj[0]), s(&mj[1]), s(&mj[2]), s(&mj[3])),
            (
                m.id.clone(),
                m.a.clone(),
                m.b.clone(),
                size_letter(m.size).to_string()
            ),
            "{ctx}: member"
        );
    }
    let loads: Vec<(String, f64)> = j["loads"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| (s(&l[0]), f(&l[1])))
        .collect();
    assert_eq!(loads, d.loads, "{ctx}: loads");
    assert_eq!(f(&j["nN"]) as u32, d.n_n, "{ctx}: nN");
    assert_eq!(f(&j["nM"]) as u32, d.n_m, "{ctx}: nM");
    let broken: Vec<String> = j["broken"].as_array().unwrap().iter().map(s).collect();
    assert_eq!(broken, g.broken(), "{ctx}: broken");
    let log = j["log"].as_array().unwrap();
    assert_eq!(log.len(), g.log().len(), "{ctx}: log length");
    for (lj, l) in log.iter().zip(g.log()) {
        assert_eq!(
            s(&lj[0]),
            l.member.clone().unwrap_or_default(),
            "{ctx}: log member"
        );
        assert!(exact(f(&lj[1]), l.lam), "{ctx}: log lam");
        assert_eq!(
            s(&lj[2]),
            l.mode.map(|m| m.past_tense()).unwrap_or(""),
            "{ctx}: log mode"
        );
        assert_eq!(
            s(&lj[3]),
            l.bad.clone().unwrap_or_default(),
            "{ctx}: log bad"
        );
        assert_eq!(lj[4].as_bool(), Some(l.loose), "{ctx}: log loose");
    }
    if g.phase() == Phase::Held {
        assert_eq!(f(&j["stars"]) as u8, g.stars(), "{ctx}: stars");
    }
    let best: Vec<u8> = j["best"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| f(b) as u8)
        .collect();
    assert_eq!(best, g.best().to_vec(), "{ctx}: best");
    assert_eq!(s(&j["toast"]), g.toast().unwrap_or(""), "{ctx}: toast");
    assert_eq!(
        s(&j["pending"]),
        g.pending().unwrap_or(""),
        "{ctx}: pending"
    );
    let sel = match g.selection() {
        None => Value::Null,
        Some(Selection::Joint(id)) => serde_json::json!(["node", id]),
        Some(Selection::Member(id)) => serde_json::json!(["member", id]),
    };
    assert_eq!(j["sel"], sel, "{ctx}: sel");
    assert_eq!(f(&j["undo"]) as usize, g.undo_depth(), "{ctx}: undo depth");

    // the solve
    let r = g.analysis();
    assert_eq!(j["ok"].as_bool(), Some(r.ok), "{ctx}: ok");
    assert_eq!(
        s(&j["bad"]),
        r.bad.map(|i| d.nodes[i].id.clone()).unwrap_or_default(),
        "{ctx}: bad"
    );
    assert!(exact(f(&j["mass"]), r.mass), "{ctx}: mass");
    assert!(exact(f(&j["cost"]), r.cost), "{ctx}: cost");
    assert_eq!(f(&j["nAct"]) as usize, r.n_act, "{ctx}: nAct");
    assert_eq!(
        s(&j["worst"]),
        r.worst.map(|w| d.members[w].id.clone()).unwrap_or_default(),
        "{ctx}: worst"
    );
    let ids: Vec<String> = j["ids"].as_array().unwrap().iter().map(s).collect();
    let used: Vec<String> = d
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| r.used[*i])
        .map(|(_, n)| n.id.clone())
        .collect();
    assert_eq!(ids, used, "{ctx}: used joints");
    let res = j["res"].as_array().unwrap();
    for q in res {
        let id = s(&q[0]);
        let i = d.member_index(&id).unwrap();
        let m = r.members[i].unwrap_or_else(|| panic!("{ctx}: {id} missing"));
        assert!(
            exact(f(&q[1]), m.force),
            "{ctx}: N({id}) js {} vs {} (bit-exact)",
            q[1],
            m.force
        );
        assert!(exact(f(&q[2]), m.utilisation), "{ctx}: ut({id})");
        assert_eq!(s(&q[3]), m.mode.past_tense(), "{ctx}: mode({id})");
        assert!(close(f(&q[4]), m.limit, 1e-12, 1e-9), "{ctx}: lim({id})");
        assert!(close(f(&q[5]), m.length, 1e-12, 1e-12), "{ctx}: L({id})");
    }
    assert_eq!(res.len(), r.members.iter().filter(|m| m.is_some()).count());
    for e in j["u"].as_array().unwrap() {
        let i = d.joint_index(&s(&e[0])).unwrap();
        for k in 0..2 {
            assert!(
                exact(f(&e[1][k]), r.displacement[i][k]),
                "{ctx}: u({})[{k}]",
                s(&e[0])
            );
        }
    }

    // the simulation
    match (&j["sim"], g.sim()) {
        (Value::Null, None) => {}
        (Value::Object(js), Some(sim)) => {
            assert_eq!(js["slow"].as_bool(), Some(sim.slow), "{ctx}: sim slow");
            assert!(
                exact(f(&js["t"]), sim.t),
                "{ctx}: sim t js {} vs {}",
                js["t"],
                sim.t
            );
            let lost: Vec<String> = js["lost"].as_array().unwrap().iter().map(s).collect();
            assert_eq!(lost, sim.lost, "{ctx}: sim lost");
            assert_eq!(f(&js["cons"]) as usize, sim.cons.len(), "{ctx}: sim cons");
            assert_eq!(
                js["sparks"].as_array().unwrap().len(),
                sim.sparks.len(),
                "{ctx}: sim sparks"
            );
            for (pj, p) in js["P"].as_array().unwrap().iter().zip(&sim.p) {
                assert!(
                    exact(f(&pj[0]), p.x) && exact(f(&pj[1]), p.y),
                    "{ctx}: sim particle js {pj} vs ({}, {})",
                    p.x,
                    p.y
                );
            }
        }
        (js, ours) => panic!(
            "{ctx}: sim mismatch js {} vs ours {}",
            js.is_null(),
            ours.is_none()
        ),
    }

    check_view(ctx, g, &g.view(), &j["view"]);
    // sanity on the shared formatter the inspector uses
    let _ = to_fixed(r.cost, 1);
}

#[test]
fn bridge_reproduces_the_mockup() {
    let all: Value = serde_json::from_str(GOLDENS).expect("bridge_goldens.json");
    let mut snaps = 0;
    for (name, sc) in all.as_object().unwrap() {
        let mut g = Game::new();
        let mut k = 0;
        for (i, a) in sc["actions"].as_array().unwrap().iter().enumerate() {
            let a = a.as_array().unwrap();
            apply(&mut g, a);
            if a[0] == "snap" {
                let ctx = format!("{name} snap {k} (action {i})");
                check(&ctx, &g, &sc["snaps"][k]);
                k += 1;
                snaps += 1;
            }
        }
        assert_eq!(
            k,
            sc["snaps"].as_array().unwrap().len(),
            "{name}: snapshots"
        );
    }
    assert!(snaps >= 60, "only {snaps} snapshots");
}
