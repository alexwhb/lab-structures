//! Golden tests: `sp_engine::builder` against the v2 builder mockup's own JavaScript (SPEC ENG-5,
//! BLD-*, DONE-2).
//!
//! `builder_goldens.json` is written by `node crates/sp-engine/tests/builder_goldens.mjs`, which
//! runs `docs/design/v2/mockup/Builder.dc.html`'s `class Component` verbatim through scripted
//! scenarios: every level's start, the seven boards' presets, edit sequences with every refusal
//! and toast, drag / undo / erase, previews under snow and soil changes, full runs (a steel truss
//! across First crossing, the wood truss collapsing under Heavy haul's truck, a cable-stayed
//! Bus route, the cable-stayed Sandbox and a looping river Sandbox, the 40 / 64 m storm towers,
//! the 32 m tower liquefying in a quake), slow motion, condition changes mid-run, zoom / pan /
//! fit / snap. This test replays the same actions on [`Game`] and checks at every snapshot the
//! state, the preview, the whole simulation (every joint, piece and vehicle), the result, every
//! string and chip, and the drawn geometry (each SVG path rebuilt from [`View`] with the
//! mockup's own number formatting, compared as text).
//!
//! The engine is bit-exact with the mockup (V8's `Math.sin` / `cos` / `atan2` / `pow` are
//! reproduced in `builder::jsmath`), so numbers are compared to 2.5 ulps (serde_json's float
//! parser may be one ulp off) — chaotic collapses included.

use serde_json::Value;
use sp_engine::builder::data::{Foundation, LevelKey, PieceType, Scene, Soil, TrafficSet};
use sp_engine::builder::game::{Outcome, Phase, Preset, PresetBuild, Selection, Tool};
use sp_engine::builder::jsmath::POW02_TABLE;
use sp_engine::builder::view::{BarTone, Chip, HudTone, Inspector, Pt, Quad, StatusTone, View};
use sp_engine::builder::{BreakMode, FoundMode, Game};
use sp_engine::jsnum::{to_fixed, to_string};

const GOLDENS: &str = include_str!("builder_goldens.json");

fn exact(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 2.5 * f64::EPSILON * a.abs().max(b.abs())
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

fn opt_s(v: &Value) -> Option<String> {
    v.as_str().map(str::to_string)
}

macro_rules! num {
    ($ctx:expr, $what:expr, $js:expr, $ours:expr) => {{
        let (a, b): (f64, f64) = ($js, $ours);
        assert!(
            exact(a, b),
            "{}: {}: js {:?} vs ours {:?}",
            $ctx,
            $what,
            a,
            b
        );
    }};
}

fn piece(k: &str) -> PieceType {
    PieceType::from_key(k).unwrap_or_else(|| panic!("piece {k}"))
}

fn preset_of(j: &Value) -> Preset {
    if j.is_null() {
        return Preset::default();
    }
    let build = j.get("build").map(|b| match b["kind"].as_str().unwrap() {
        "truss" => PresetBuild::Truss {
            chord: piece(b["chord"].as_str().unwrap()),
        },
        "cable" => PresetBuild::Cable,
        "tower" => PresetBuild::Tower {
            h: f(&b["H"]),
            bays: b["bays"].as_u64().unwrap() as u32,
            col: piece(b["col"].as_str().unwrap()),
            brace: piece(b["brace"].as_str().unwrap()),
            found: Foundation::from_key(b["found"].as_str().unwrap()).unwrap(),
        },
        k => panic!("build {k}"),
    });
    let cond = &j["cond"];
    Preset {
        level: j["level"].as_str().and_then(LevelKey::from_key),
        scene: j["scene"].as_str().and_then(Scene::from_key),
        wind: cond["wind"].as_f64(),
        snow: cond["snow"].as_f64(),
        quake: cond["quake"].as_f64(),
        traffic: cond["traffic"].as_str().and_then(TrafficSet::from_key),
        soil: cond["soil"].as_str().and_then(Soil::from_key),
        build,
        tool: j["tool"].as_str().and_then(Tool::from_key),
        nums: j["nums"].as_bool().unwrap_or(false),
        run: j["run"].as_bool().unwrap_or(false),
        demo_loop: j["loop"].as_bool().unwrap_or(false),
    }
}

/// The harness's pointer events go through `(clientX - left) / width * 1120`.
fn ev(p: f64) -> f64 {
    p / 1120.0 * 1120.0
}
fn evy(p: f64) -> f64 {
    p / 600.0 * 600.0
}

fn act(g: &mut Game, a: &[Value]) {
    let op = a[0].as_str().unwrap();
    let p = a.get(1).cloned().unwrap_or(Value::Null);
    let q = a.get(2).cloned().unwrap_or(Value::Null);
    let r = a.get(3).cloned().unwrap_or(Value::Null);
    match op {
        "tool" => g.pick_tool(Tool::from_key(p.as_str().unwrap()).unwrap()),
        "type" => g.pick_type(piece(p.as_str().unwrap())),
        "found" => g.pick_found(Foundation::from_key(p.as_str().unwrap()).unwrap()),
        "weight" => g.pick_weight(f(&p)),
        "node" => g.click_node(p.as_str().unwrap()),
        "at" => {
            let c = g.camera();
            g.click_surface(ev(c.x(f(&p))), evy(c.y(f(&q))));
        }
        "finish" => g.finish_line(),
        "handle" => {
            let id = p.as_str().unwrap();
            if g.view().handles.iter().any(|h| h.id == id) {
                g.pick_handle(id);
            }
        }
        "mtype" => {
            if let Inspector::Piece(pi) = g.view().inspector {
                g.set_piece_type(&pi.id, piece(q.as_str().unwrap()));
            }
        }
        "setfound" => {
            if let Inspector::Joint(ji) = g.view().inspector {
                g.set_foundation(&ji.id, q.as_str().and_then(Foundation::from_key));
            }
        }
        "less" | "more" => {
            if let Inspector::Joint(ji) = g.view().inspector {
                if op == "less" {
                    g.crate_less(&ji.id)
                } else {
                    g.crate_more(&ji.id)
                }
            }
        }
        "delsel" => match g.view().inspector {
            Inspector::Piece(pi) => g.delete_member(&pi.id),
            Inspector::Joint(ji) if ji.can_delete => g.delete_joint(&ji.id),
            _ => {}
        },
        "down" => {
            let id = p.as_str().unwrap();
            if g.view().joints.iter().any(|j| j.id == id) {
                g.joint_down(id);
            }
        }
        "move" => {
            let c = g.camera();
            g.pointer_move(ev(c.x(f(&p))), evy(c.y(f(&q))));
        }
        "movepx" => g.pointer_move(ev(f(&p)), evy(f(&q))),
        "up" => g.pointer_up(),
        "sdown" => g.surface_down(ev(f(&p)), evy(f(&q))),
        "sclick" => g.click_surface(ev(f(&p)), evy(f(&q))),
        "wheel" => g.wheel(f(&p), ev(f(&q)), evy(f(&r))),
        "zoomin" => g.zoom_in(),
        "zoomout" => g.zoom_out(),
        "fit" => g.fit(),
        "undo" => g.undo(),
        "clear" => g.clear(),
        "level" => g.pick_level(LevelKey::from_key(p.as_str().unwrap()).unwrap()),
        "scene" => g.pick_scene(Scene::from_key(p.as_str().unwrap()).unwrap()),
        "wind" => g.set_wind(f(&p)),
        "dir" => g.set_dir(f(&p)),
        "snow" => g.set_snow(f(&p)),
        "quake" => g.set_quake(f(&p)),
        "soil" => g.set_soil(Soil::from_key(p.as_str().unwrap()).unwrap()),
        "traffic" => g.set_traffic(TrafficSet::from_key(p.as_str().unwrap()).unwrap()),
        "run" => g.toggle_run(),
        "slow" => g.toggle_slow(),
        "replay" => g.replay(),
        "back" => g.stop_run(),
        "shake" => g.shake_now(),
        "nums" => g.toggle_forces(),
        "tick" => {
            for _ in 0..p.as_u64().unwrap() {
                g.tick();
            }
        }
        "toend" => {
            for _ in 0..p.as_u64().unwrap() {
                if g.phase() != Phase::Run {
                    break;
                }
                g.tick();
            }
        }
        "wait" => {
            g.advance(f(&p));
        }
        other => panic!("unknown action {other}"),
    }
}

// ---------- the mockup's path strings, rebuilt from the View ----------

fn f1(x: f64) -> String {
    to_fixed(x, 1)
}
fn pt(p: Pt) -> String {
    format!("{} {}", f1(p[0]), f1(p[1]))
}
fn quad(q: &Quad) -> String {
    format!("M{} L{} L{} L{} Z ", pt(q[0]), pt(q[1]), pt(q[2]), pt(q[3]))
}
fn line(a: Pt, b: Pt) -> String {
    format!("M{} L{} ", pt(a), pt(b))
}
fn poly(pts: &[Pt]) -> String {
    let parts: Vec<String> = pts.iter().map(|p| pt(*p)).collect();
    format!("M{} Z ", parts.join(" L"))
}

fn paths(v: &View) -> Vec<(&'static str, String)> {
    let mut o: std::collections::BTreeMap<&'static str, String> = Default::default();
    let mut add = |k: &'static str, t: String| o.entry(k).or_default().push_str(&t);
    for m in &v.members {
        let ln = match m.sag {
            Some(c) => format!("M{} Q{} {} ", pt(m.a), pt(c), pt(m.b)),
            None => line(m.a, m.b),
        };
        let t = m.kind.spec();
        if t.t_only {
            add(mat_key(m.kind), ln.clone());
        } else if let Some(d) = &m.deck {
            add("roadTopD", quad(&d.top));
            add("roadFrontD", quad(&d.front));
            if let Some(l) = d.line {
                add("roadLineD", line(l[0], l[1]));
            }
            if let Some(q) = &d.facade {
                add("facadeD", quad(q));
            }
            if let Some(q) = &d.side {
                add("facadeSideD", quad(q));
            }
            if let Some(l) = d.window {
                add("winD", line(l[0], l[1]));
            }
            if let Some(q) = &d.snow {
                add("snowD", quad(q));
            }
        } else {
            add(mat_key(m.kind), ln.clone());
            if let Some(b) = m.back {
                add("backD", line(b[0], b[1]));
            }
        }
        if let Some(b) = m.band {
            add(band_key(b.key()), ln);
        }
    }
    for fv in &v.foundations {
        let p = fv.pad;
        add(
            "foundD",
            format!(
                "M{} {} h{} v{} h{} Z ",
                f1(p[0]),
                f1(p[1]),
                f1(p[2]),
                f1(p[3]),
                f1(-p[2])
            ),
        );
        if let Some(p) = fv.pile {
            add(
                "foundD",
                format!(
                    "M{} {} h{} v{} h{} Z ",
                    f1(p[0]),
                    f1(p[1]),
                    f1(p[2]),
                    f1(p[3]),
                    f1(-p[2])
                ),
            );
        }
        let t = fv.tri;
        add(
            "supD",
            format!(
                "M{} {} L{} {} H{} Z ",
                f1(t[0][0]),
                f1(t[0][1]),
                f1(t[1][0]),
                f1(t[1][1]),
                f1(t[2][0])
            ),
        );
    }
    for c in &v.crates {
        add(
            "crateWireD",
            format!(
                "M{} {} V{} ",
                f1(c.wire[0][0]),
                f1(c.wire[0][1]),
                f1(c.wire[1][1])
            ),
        );
        let b = c.body;
        add(
            "crateD",
            format!(
                "M{} {} h{} v{} h{} Z ",
                f1(b[0]),
                f1(b[1]),
                f1(b[2]),
                f1(b[3]),
                f1(-b[2])
            ),
        );
        add(
            "crateTopD",
            format!("M{} {} l6 -4 h{} l-6 4 Z ", f1(b[0]), f1(b[1]), f1(b[2])),
        );
        add(
            "crateSideD",
            format!(
                "M{} {} l6 -4 v{} l-6 4 Z ",
                f1(c.side[0][0]),
                f1(c.side[0][1]),
                f1(b[3])
            ),
        );
    }
    for veh in &v.vehicles {
        let key = match veh.kind {
            sp_engine::builder::VehicleKind::Car => "vCarD",
            sp_engine::builder::VehicleKind::Bus => "vBusD",
            sp_engine::builder::VehicleKind::Truck => "vBoxD",
        };
        for q in &veh.body {
            add(key, quad(q));
        }
        if let Some(q) = &veh.cab {
            add("vCabD", quad(q));
        }
        for q in &veh.windows {
            add("vWinD", quad(q));
        }
        for (c, r) in veh.wheels {
            let c0: f64 = f1(c[0]).parse().unwrap();
            let c1: f64 = f1(c[1]).parse().unwrap();
            add(
                "vWheelD",
                format!(
                    "M{} {} a{} {} 0 1 0 {} 0 a{} {} 0 1 0 {} 0 Z ",
                    f1(c0 + r),
                    f1(c1),
                    f1(r),
                    f1(r),
                    f1(-2.0 * r),
                    f1(r),
                    f1(r),
                    f1(2.0 * r)
                ),
            );
        }
    }
    for w in &v.wind {
        let (x0, y, x1) = (w.from[0], w.from[1], w.to[0]);
        add(
            "windArrowD",
            format!(
                "M{} {} H{} M{} {} L{} {} L{} {} ",
                to_string(x0),
                f1(y),
                f1(x1),
                f1(w.head[0][0]),
                f1(w.head[0][1]),
                f1(w.head[1][0]),
                f1(w.head[1][1]),
                f1(w.head[2][0]),
                f1(w.head[2][1])
            ),
        );
        let st = w.streak;
        add(
            "windD",
            format!(
                "M{} {} C {} {} {} {} {} {} ",
                to_string(st[0][0]),
                f1(st[0][1]),
                to_string(st[1][0]),
                f1(st[1][1]),
                to_string(st[2][0]),
                f1(st[2][1]),
                to_string(st[3][0]),
                f1(st[3][1])
            ),
        );
    }
    for fl in &v.flakes {
        add("flakeD", format!("M{} a1.6 1.6 0 1 0 0.01 0 ", pt(*fl)));
    }
    if let Some(y) = v.target_y {
        add("targetD", format!("M0 {} H1120", f1(y)));
    }
    for k in &v.sparks {
        let (x, y) = (k[0], k[1]);
        add(
            "sparkD",
            format!(
                "M{} {} L{} {} M{} {} L{} {} ",
                f1(x - 6.0),
                f1(y - 6.0),
                f1(x + 6.0),
                f1(y + 6.0),
                f1(x + 6.0),
                f1(y - 6.0),
                f1(x - 6.0),
                f1(y + 6.0)
            ),
        );
    }
    for sp in &v.splashes {
        let (x, y, r) = (sp.at[0], sp.at[1], sp.r);
        add(
            "splashD",
            format!(
                "M{} {} q{} {} {} 0 q{} {} {} 0 ",
                f1(x - r),
                f1(y),
                f1(r / 2.0),
                f1(-r * 0.9),
                f1(r),
                f1(r / 2.0),
                f1(-r * 0.9),
                f1(r)
            ),
        );
    }
    for c in &v.rings {
        add(
            "badD",
            format!(
                "M{} {} a13 13 0 1 0 -26 0 a13 13 0 1 0 26 0 Z ",
                f1(c[0] + 13.0),
                f1(c[1])
            ),
        );
    }
    if let Some(h) = v.hover {
        add(
            "hoverD",
            format!(
                "M{} {} a5 5 0 1 0 -10 0 a5 5 0 1 0 10 0 Z",
                f1(h[0] + 5.0),
                f1(h[1])
            ),
        );
    }
    if let Some(gl) = v.ghost {
        add(
            "ghostD",
            format!("M{} L{} {}", pt(gl[0]), f1(gl[1][0]), f1(gl[1][1])),
        );
    }
    let t = &v.terrain;
    add("soilD", poly(&t.soil_poly));
    if let Some(b) = &t.bed_poly {
        add("bedD", poly(b));
    }
    if let Some(w) = &t.water_poly {
        add("waterD", poly(w));
    }
    if !t.flow.is_empty() {
        let parts: Vec<String> = t
            .flow
            .iter()
            .map(|l| format!("M{} {} H{}", f1(l[0][0]), f1(l[0][1]), f1(l[1][0])))
            .collect();
        add("flowD", parts.join(" "));
    }
    let off = v.camera.off;
    if t.scene == Scene::River {
        let (a, b) = (&t.tops[0], &t.tops[1]);
        add(
            "topD",
            format!(
                "M{} {} H{} l{} {} H{} Z M{} {} H{} v{} H{} Z",
                f1(a[0][0]),
                f1(a[0][1]),
                f1(a[1][0]),
                to_string(off[0]),
                to_string(off[1]),
                f1(a[3][0]),
                f1(b[0][0]),
                f1(b[0][1]),
                f1(b[1][0]),
                to_string(off[1]),
                f1(b[3][0])
            ),
        );
        let (e0, e1) = (t.edges[0], t.edges[1]);
        add(
            "edgeD",
            format!(
                "M{} {} H{} M{} {} H{}",
                f1(e0[0][0]),
                f1(e0[0][1]),
                f1(e0[1][0]),
                f1(e1[0][0]),
                f1(e1[0][1]),
                f1(e1[1][0])
            ),
        );
    } else {
        let a = &t.tops[0];
        add(
            "topD",
            format!(
                "M{} {} H{} v{} H{} Z",
                f1(a[0][0]),
                f1(a[0][1]),
                f1(a[1][0]),
                to_string(off[1]),
                f1(a[3][0])
            ),
        );
        let e = t.edges[0];
        add(
            "edgeD",
            format!("M{} {} H{}", f1(e[0][0]), f1(e[0][1]), f1(e[1][0])),
        );
    }
    for gr in &t.grains {
        add("grainD", format!("M{} {}h2v2h-2Z", f1(gr[0]), f1(gr[1])));
    }
    o.into_iter().collect()
}

fn mat_key(k: PieceType) -> &'static str {
    match k {
        PieceType::Wood => "woodD",
        PieceType::Steel => "steelD",
        PieceType::Concrete => "concreteD",
        PieceType::Rope => "ropeD",
        PieceType::Cable => "cableD",
        _ => "deckD",
    }
}
fn band_key(k: &str) -> &'static str {
    match k {
        "t1" => "t1",
        "t2" => "t2",
        "t3" => "t3",
        "c1" => "c1",
        "c2" => "c2",
        "c3" => "c3",
        _ => "x",
    }
}

fn hud_hex(t: HudTone) -> &'static str {
    match t {
        HudTone::Plain => "#EAEAEA",
        HudTone::Good => "#5FBF8A",
        HudTone::Bad => "#E8A39E",
        HudTone::Wind => "#8FD3EE",
        HudTone::Snow => "#E6EEF3",
        HudTone::Shake => "#E7A47C",
    }
}
fn status_hex(t: StatusTone) -> &'static str {
    match t {
        StatusTone::Muted => "#A3A3A3",
        StatusTone::Ok => "#5FBF8A",
        StatusTone::Warn => "#D8B86A",
        StatusTone::Danger => "#E5534B",
        StatusTone::Running => "#D6C7A1",
    }
}
fn soil_hex(s: Soil) -> &'static str {
    match s {
        Soil::Rock => "#45474B",
        Soil::Dense => "#7E6E4C",
        Soil::Stiff => "#6E5141",
        Soil::Soft => "#544C3F",
        Soil::Loose => "#8A7C58",
    }
}
fn chip_cls<T>(c: &Chip<T>) -> String {
    format!(
        "chip{}{}",
        if c.on { " chip-on" } else { "" },
        if c.dim { " chip-dim" } else { "" }
    )
}
fn pct(x: f64, of: f64) -> String {
    to_fixed(x / of * 100.0, 2)
}

fn check_chips<T>(ctx: &str, what: &str, js: &Value, ours: &[Chip<T>]) {
    let js = js.as_array().unwrap();
    assert_eq!(js.len(), ours.len(), "{ctx}: {what}: count");
    for (j, c) in js.iter().zip(ours) {
        assert_eq!(s(&j[0]), c.name, "{ctx}: {what}: name");
        assert_eq!(s(&j[1]), chip_cls(c), "{ctx}: {what}: {} cls", c.name);
    }
}

fn check_view(
    ctx: &str,
    j: &Value,
    v: &View,
    last: &mut std::collections::BTreeMap<String, String>,
) {
    let st = |k: &str| s(&j[k]);
    assert_eq!(st("statusLabel"), v.status.label, "{ctx}: status");
    assert_eq!(
        st("statusColor"),
        status_hex(v.status.tone),
        "{ctx}: status colour"
    );
    assert_eq!(st("explain"), v.explain, "{ctx}: explain");
    assert_eq!(st("stageLabel"), v.stage_label, "{ctx}: stage label");
    assert_eq!(st("surfaceLabel"), v.surface_label, "{ctx}: surface label");
    assert_eq!(st("hint"), v.hint, "{ctx}: hint");
    assert_eq!(st("clock"), v.clock, "{ctx}: clock");
    assert_eq!(st("runInfo"), v.run_info, "{ctx}: run info");
    assert_eq!(st("runLabel"), v.run_label, "{ctx}: run label");
    assert_eq!(st("hdrRun"), v.hdr_run, "{ctx}: header run");
    assert_eq!(j["playing"].as_bool(), Some(v.playing), "{ctx}: playing");
    assert_eq!(j["running"].as_bool(), Some(v.running), "{ctx}: running");
    assert_eq!(st("slowLabel"), v.slow_label, "{ctx}: slow label");
    assert_eq!(
        j["showResult"].as_bool(),
        Some(v.result.is_some()),
        "{ctx}: result shown"
    );
    match &v.result {
        Some(r) => {
            assert_eq!(st("resultTitle"), r.title, "{ctx}: result title");
            assert_eq!(st("stars"), r.stars, "{ctx}: stars");
            assert_eq!(
                st("resultColor"),
                if r.pass { "#5FBF8A" } else { "#E5534B" },
                "{ctx}: result colour"
            );
        }
        None => assert_eq!(st("stars"), "", "{ctx}: stars"),
    }
    assert_eq!(st("costText"), v.cost_text, "{ctx}: cost");
    assert_eq!(
        st("costColor"),
        if v.over_budget { "#E8A39E" } else { "#EAEAEA" },
        "{ctx}: cost colour"
    );
    assert_eq!(st("levelSlug"), v.level_slug, "{ctx}: slug");
    assert_eq!(st("zoomPct"), v.zoom_pct, "{ctx}: zoom");
    assert_eq!(st("goal"), v.goal, "{ctx}: goal");
    let chips: Vec<String> = j["chipsInfo"].as_array().unwrap().iter().map(s).collect();
    assert_eq!(chips, v.chips_info, "{ctx}: goal chips");
    assert_eq!(
        j["toastOn"].as_bool(),
        Some(v.toast.is_some()),
        "{ctx}: toast on"
    );
    assert_eq!(
        st("toast"),
        v.toast.clone().unwrap_or_default(),
        "{ctx}: toast"
    );
    assert_eq!(
        st("stageCls"),
        if v.shaking { "shake" } else { "" },
        "{ctx}: shake"
    );
    assert_eq!(
        st("flowCls"),
        if v.flowing { "flow" } else { "flow still-flow" },
        "{ctx}: flow"
    );
    let cursor = match v.cursor {
        sp_engine::builder::view::Cursor::Default => "default",
        sp_engine::builder::view::Cursor::Crosshair => "crosshair",
        sp_engine::builder::view::Cursor::Grab => "grab",
    };
    assert_eq!(st("cursor"), cursor, "{ctx}: cursor");
    assert_eq!(
        j["hasPending"].as_bool(),
        Some(v.has_pending),
        "{ctx}: pending"
    );
    assert_eq!(j["showTypes"].as_bool(), Some(v.show_types), "{ctx}: types");
    assert_eq!(
        j["showFounds"].as_bool(),
        Some(v.show_founds),
        "{ctx}: founds"
    );
    assert_eq!(
        j["showWeights"].as_bool(),
        Some(v.show_weights),
        "{ctx}: weights"
    );
    assert_eq!(
        st("numCls"),
        if v.show_forces {
            "chip chip-on"
        } else {
            "chip"
        },
        "{ctx}: show forces"
    );
    assert_eq!(
        j["isSandbox"].as_bool(),
        Some(v.is_sandbox),
        "{ctx}: sandbox"
    );
    assert_eq!(j["isRiver"].as_bool(), Some(v.is_river), "{ctx}: river");
    assert_eq!(
        j["canShake"].as_bool(),
        Some(v.conditions.can_shake),
        "{ctx}: can shake"
    );
    // HUD
    let hud = j["hud"].as_array().unwrap();
    assert_eq!(
        hud.len(),
        v.hud.len(),
        "{ctx}: hud count {hud:?} vs {:?}",
        v.hud
    );
    for (h, c) in hud.iter().zip(&v.hud) {
        assert_eq!(s(&h[0]), c.key, "{ctx}: hud key");
        assert_eq!(s(&h[1]), c.value, "{ctx}: hud {}", c.key);
        assert_eq!(s(&h[2]), hud_hex(c.tone), "{ctx}: hud {} colour", c.key);
    }
    // joints and handles
    let jj = j["joints"].as_array().unwrap();
    assert_eq!(jj.len(), v.joints.len(), "{ctx}: joints count");
    for (a, b) in jj.iter().zip(&v.joints) {
        assert_eq!(s(&a[0]), b.label, "{ctx}: joint label");
        let cls = format!(
            "joint{}{}{}{}",
            if b.founded { " j-lock" } else { "" },
            if b.pending { " j-pend" } else { "" },
            if b.selected { " j-sel" } else { "" },
            if b.bad { " j-bad" } else { "" }
        );
        assert_eq!(s(&a[1]), cls, "{ctx}: {} cls", b.id);
        assert_eq!(s(&a[2]), pct(b.at[0], 1120.0), "{ctx}: {} x", b.id);
        assert_eq!(s(&a[3]), pct(b.at[1], 600.0), "{ctx}: {} y", b.id);
    }
    let hh = j["handles"].as_array().unwrap();
    assert_eq!(hh.len(), v.handles.len(), "{ctx}: handles count");
    for (a, b) in hh.iter().zip(&v.handles) {
        assert_eq!(s(&a[0]), b.label, "{ctx}: handle label");
        assert_eq!(s(&a[1]), b.text, "{ctx}: {} text", b.id);
        let cls = format!(
            "mh{}{}{}",
            if b.num { " mh-num" } else { "" },
            if b.selected { " mh-sel" } else { "" },
            if b.over { " mh-over" } else { "" }
        );
        assert_eq!(s(&a[2]), cls, "{ctx}: {} cls", b.id);
        assert_eq!(s(&a[3]), pct(b.at[0], 1120.0), "{ctx}: {} x", b.id);
        assert_eq!(s(&a[4]), pct(b.at[1], 600.0), "{ctx}: {} y", b.id);
    }
    let tags = j["tags"].as_array().unwrap();
    assert_eq!(tags.len(), v.crates.len(), "{ctx}: tags");
    for (a, c) in tags.iter().zip(&v.crates) {
        assert_eq!(s(&a[0]), c.tag, "{ctx}: tag");
        assert_eq!(s(&a[1]), pct(c.tag_at[0], 1120.0), "{ctx}: tag x");
        assert_eq!(s(&a[2]), pct(c.tag_at[1], 600.0), "{ctx}: tag y");
    }
    // rail, chips, levels
    let tools: Vec<String> = v
        .tools
        .iter()
        .map(|t| format!("tool{}", if t.on { " tool-on" } else { "" }))
        .collect();
    let jt: Vec<String> = j["tools"].as_array().unwrap().iter().map(s).collect();
    assert_eq!(jt, tools, "{ctx}: tools");
    let types: Vec<Chip<()>> = v
        .types
        .iter()
        .map(|t| Chip {
            value: (),
            name: t.name.into(),
            on: t.on,
            dim: false,
        })
        .collect();
    check_chips(ctx, "palette", &j["types"], &types);
    let founds: Vec<Chip<()>> = v
        .founds
        .iter()
        .map(|t| Chip {
            value: (),
            name: t.name.into(),
            on: t.on,
            dim: false,
        })
        .collect();
    check_chips(ctx, "founds", &j["founds"], &founds);
    check_chips(ctx, "weights", &j["weights"], &v.weights);
    check_chips(ctx, "scenes", &j["sbScenes"], &v.sb_scenes);
    check_chips(ctx, "winds", &j["cWinds"], &v.conditions.winds);
    check_chips(ctx, "dirs", &j["cDirs"], &v.conditions.dirs);
    check_chips(ctx, "snows", &j["cSnows"], &v.conditions.snows);
    check_chips(ctx, "quakes", &j["cQuakes"], &v.conditions.quakes);
    check_chips(ctx, "soils", &j["cSoils"], &v.conditions.soils);
    check_chips(ctx, "traffic", &j["cTraffic"], &v.conditions.traffic);
    let lg = j["levels"].as_array().unwrap();
    assert_eq!(lg.len(), v.level_groups.len(), "{ctx}: groups");
    for (a, g2) in lg.iter().zip(&v.level_groups) {
        assert_eq!(s(&a[0]), g2.name, "{ctx}: group");
        let items = a[1].as_array().unwrap();
        // SPEC TIME-3 adds "7 · Leaning tower", which the mockup doesn't have
        let ours: Vec<_> = g2
            .items
            .iter()
            .filter(|r| r.key != LevelKey::Pisa)
            .collect();
        assert_eq!(items.len(), ours.len(), "{ctx}: {} levels", g2.name);
        for (i, r) in items.iter().zip(ours) {
            assert_eq!(s(&i[0]), r.name, "{ctx}: level name");
            assert_eq!(s(&i[1]), r.best, "{ctx}: {} best", r.name);
            assert_eq!(
                s(&i[2]),
                format!("lvl{}", if r.on { " lvl-on" } else { "" }),
                "{ctx}: {} cls",
                r.name
            );
        }
    }
    // inspector
    match &v.inspector {
        Inspector::Piece(mi) => {
            let a = &j["mi"];
            assert!(!a.is_null(), "{ctx}: piece inspector missing in js");
            assert_eq!(s(&a["title"]), mi.title, "{ctx}: mi title");
            assert_eq!(s(&a["len"]), mi.len, "{ctx}: mi len");
            assert_eq!(s(&a["w"]), to_string(mi.fill), "{ctx}: mi w");
            assert_eq!(s(&a["ut"]), mi.ut, "{ctx}: mi ut");
            let col = match mi.tone {
                BarTone::Over => "#E5534B",
                BarTone::Pulled => "#7FB0E0",
                BarTone::Pushed => "#E0905E",
            };
            assert_eq!(s(&a["color"]), col, "{ctx}: mi colour");
            assert_eq!(s(&a["text"]), mi.text, "{ctx}: mi text");
            check_chips(ctx, "mi types", &a["types"], &mi.types);
            assert!(
                j["ni"].is_null() && j["typeInfo"].is_null(),
                "{ctx}: one inspector"
            );
        }
        Inspector::Joint(ni) => {
            let a = &j["ni"];
            assert!(!a.is_null(), "{ctx}: joint inspector missing in js");
            assert_eq!(s(&a["title"]), ni.title, "{ctx}: ni title");
            assert_eq!(s(&a["pos"]), ni.pos, "{ctx}: ni pos");
            check_chips(ctx, "ni founds", &a["founds"], &ni.founds);
            assert_eq!(s(&a["load"]), ni.load, "{ctx}: ni load");
            assert_eq!(s(&a["note"]), ni.note, "{ctx}: ni note");
            assert_eq!(a["canDel"].as_bool(), Some(ni.can_delete), "{ctx}: ni del");
        }
        Inspector::Materials(cards) => {
            assert!(
                j["mi"].is_null() && j["ni"].is_null(),
                "{ctx}: no inspector"
            );
            let ti = j["typeInfo"].as_array().unwrap();
            assert_eq!(ti.len(), cards.len(), "{ctx}: cards");
            for (a, c) in ti.iter().zip(cards) {
                assert_eq!(s(&a[0]), c.name, "{ctx}: card name");
                assert_eq!(s(&a[1]), c.curve, "{ctx}: {} curve", c.name);
                assert_eq!(s(&a[2]), c.blurb, "{ctx}: {} blurb", c.name);
                assert_eq!(s(&a[3]), c.cost, "{ctx}: {} cost", c.name);
            }
        }
    }
    // grid, terrain colours, paths
    let gr = j["grid"].as_array().unwrap();
    assert_eq!(s(&gr[0]), f1(v.grid.origin[0]), "{ctx}: grid x");
    assert_eq!(s(&gr[1]), f1(v.grid.origin[1]), "{ctx}: grid y");
    assert_eq!(s(&gr[2]), f1(v.grid.step), "{ctx}: grid step");
    assert_eq!(st("soilColor"), soil_hex(v.terrain.soil), "{ctx}: soil");
    assert_eq!(st("bedColor"), soil_hex(v.terrain.bed), "{ctx}: bed");
    let ours = paths(v);
    let jp = j["paths"].as_object().unwrap();
    for (k, js) in jp {
        let mine = ours
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, t)| t.as_str())
            .unwrap_or("");
        if let Some(t) = js.as_str() {
            last.insert(k.clone(), t.to_string());
        }
        let js = last.get(k).map(String::as_str).unwrap_or("");
        if js != mine {
            let pos = js
                .chars()
                .zip(mine.chars())
                .position(|(a, b)| a != b)
                .unwrap_or(js.len().min(mine.len()));
            let from = pos.saturating_sub(40);
            panic!(
                "{ctx}: path {k} differs at {pos}\n  js: …{}\nours: …{}",
                &js[from.min(js.len())..(pos + 60).min(js.len())],
                &mine[from.min(mine.len())..(pos + 60).min(mine.len())]
            );
        }
    }
}

fn check_state(ctx: &str, j: &Value, g: &Game) {
    assert_eq!(s(&j["level"]), g.level().key(), "{ctx}: level");
    assert_eq!(s(&j["sbScene"]), g.sb_scene().key(), "{ctx}: sb scene");
    let c = &j["cond"];
    let ours = g.cond();
    num!(ctx, "wind", f(&c["wind"]), ours.wind);
    num!(ctx, "dir", f(&c["dir"]), ours.dir);
    num!(ctx, "snow", f(&c["snow"]), ours.snow);
    num!(ctx, "quake", f(&c["quake"]), ours.quake);
    assert_eq!(s(&c["soil"]), ours.soil.key(), "{ctx}: soil");
    assert_eq!(s(&c["traffic"]), ours.traffic.key(), "{ctx}: traffic");
    let vw = j["view"].as_array().unwrap();
    let v = g.view_state();
    num!(ctx, "view cx", f(&vw[0]), v.cx);
    num!(ctx, "view cy", f(&vw[1]), v.cy);
    num!(ctx, "view z", f(&vw[2]), v.z);
    let d = g.design();
    let nodes = j["nodes"].as_array().unwrap();
    assert_eq!(
        nodes.len(),
        d.nodes.len(),
        "{ctx}: joints {nodes:?} vs {:?}",
        d.nodes
    );
    for (a, n) in nodes.iter().zip(&d.nodes) {
        assert_eq!(s(&a[0]), n.id, "{ctx}: joint id");
        num!(ctx, format!("{} x", n.id), f(&a[1]), n.x);
        num!(ctx, format!("{} y", n.id), f(&a[2]), n.y);
        assert_eq!(
            opt_s(&a[3]).as_deref(),
            n.sup.map(|f| f.key()),
            "{ctx}: {} sup",
            n.id
        );
        assert_eq!(a[4].as_bool(), Some(n.locked), "{ctx}: {} locked", n.id);
    }
    let members = j["members"].as_array().unwrap();
    assert_eq!(members.len(), d.members.len(), "{ctx}: pieces");
    for (a, m) in members.iter().zip(&d.members) {
        assert_eq!(
            (s(&a[0]), s(&a[1]), s(&a[2]), s(&a[3])),
            (
                m.id.clone(),
                m.a.clone(),
                m.b.clone(),
                m.kind.key().to_string()
            ),
            "{ctx}: piece"
        );
    }
    let loads = j["loads"].as_array().unwrap();
    assert_eq!(loads.len(), d.loads.len(), "{ctx}: crates");
    for (a, (id, t)) in loads.iter().zip(&d.loads) {
        assert_eq!(&s(&a[0]), id, "{ctx}: crate joint");
        num!(ctx, "crate", f(&a[1]), *t);
    }
    assert_eq!(j["nN"].as_u64(), Some(d.n_n as u64), "{ctx}: nN");
    assert_eq!(j["nM"].as_u64(), Some(d.n_m as u64), "{ctx}: nM");
    assert_eq!(
        opt_s(&j["pending"]).as_deref(),
        g.pending(),
        "{ctx}: pending"
    );
    let sel = match g.selection() {
        Some(Selection::Node(id)) => Some(("node".to_string(), id.clone())),
        Some(Selection::Member(id)) => Some(("member".to_string(), id.clone())),
        None => None,
    };
    let jsel = j["sel"].as_array().map(|a| (s(&a[0]), s(&a[1])));
    assert_eq!(jsel, sel, "{ctx}: selection");
    assert_eq!(s(&j["phase"]), g.phase().key(), "{ctx}: phase");
    assert_eq!(s(&j["tool"]), g.tool().key(), "{ctx}: tool");
    assert_eq!(s(&j["type"]), g.piece_type().key(), "{ctx}: type");
    assert_eq!(s(&j["found"]), g.found_kind().key(), "{ctx}: found");
    num!(ctx, "weight", f(&j["weight"]), g.weight());
    assert_eq!(j["nums"].as_bool(), Some(g.show_forces()), "{ctx}: nums");
    assert_eq!(s(&j["toast"]), g.toast().unwrap_or(""), "{ctx}: toast");
    assert_eq!(j["slow"].as_bool(), Some(g.slow()), "{ctx}: slow");
    for k in LevelKey::ALL {
        let js = j["best"][k.key()].as_u64().unwrap_or(0) as u8;
        assert_eq!(js, g.best_for(k), "{ctx}: best {}", k.key());
    }
    assert_eq!(
        j["hist"].as_u64(),
        Some(g.undo_depth() as u64),
        "{ctx}: undo depth"
    );
    let hv = j["hover"].as_array().map(|a| [f(&a[0]), f(&a[1])]);
    assert_eq!(hv, g.hover(), "{ctx}: hover");
    assert_eq!(opt_s(&j["drag"]).as_deref(), g.dragging(), "{ctx}: drag");
}

fn check_preview(ctx: &str, j: &Value, g: &Game) {
    let Some(pv) = g.preview() else {
        assert!(j.is_null(), "{ctx}: preview expected");
        return;
    };
    assert!(!j.is_null(), "{ctx}: no preview in js");
    let res = j["res"].as_array().unwrap();
    assert_eq!(res.len(), pv.pieces.len(), "{ctx}: preview pieces");
    for (a, q) in res.iter().zip(&pv.pieces) {
        assert_eq!(s(&a[0]), q.id, "{ctx}: preview id");
        num!(ctx, format!("N({})", q.id), f(&a[1]), q.n);
        assert_eq!(a[2].as_bool(), Some(q.over), "{ctx}: {} over", q.id);
        assert_eq!(a[3].as_bool(), Some(q.slack), "{ctx}: {} slack", q.id);
        num!(ctx, format!("capT({})", q.id), f(&a[4]), q.cap_t);
        if a[5].is_null() {
            assert!(q.cap_c.is_infinite(), "{ctx}: {} capC", q.id);
        } else {
            num!(ctx, format!("capC({})", q.id), f(&a[5]), q.cap_c);
        }
        num!(ctx, format!("L({})", q.id), f(&a[6]), q.len);
        assert_eq!(a[7].as_bool(), Some(q.exposed), "{ctx}: {} exposed", q.id);
    }
    let found = j["found"].as_array().unwrap();
    let ours: Vec<_> = pv.found.iter().flatten().collect();
    assert_eq!(found.len(), ours.len(), "{ctx}: foundations");
    for (a, q) in found.iter().zip(ours) {
        assert_eq!(s(&a[0]), q.id, "{ctx}: foundation id");
        num!(ctx, format!("R({})", q.id), f(&a[1]), q.r);
        num!(ctx, "Qc", f(&a[2]), q.qc);
        num!(ctx, "Qt", f(&a[3]), q.qt);
        assert_eq!(a[4].as_bool(), Some(q.over_c), "{ctx}: {} overC", q.id);
        assert_eq!(a[5].as_bool(), Some(q.over_t), "{ctx}: {} overT", q.id);
    }
    num!(ctx, "moved", f(&j["moved"]), pv.moved);
    assert_eq!(opt_s(&j["movedId"]), pv.moved_id, "{ctx}: moved id");
}

fn check_sim(ctx: &str, j: &Value, g: &Game) {
    let sim = if g.live() { g.sim() } else { None };
    let Some(sm) = sim else {
        assert!(j.is_null(), "{ctx}: js has a sim");
        return;
    };
    assert!(!j.is_null(), "{ctx}: js has no sim");
    num!(ctx, "t", f(&j["t"]), sm.t);
    let pp = j["P"].as_array().unwrap();
    assert_eq!(pp.len(), sm.p.len(), "{ctx}: particles");
    for (a, p) in pp.iter().zip(&sm.p) {
        let w = |k: &str| format!("{} {k}", p.id);
        num!(ctx, w("x"), f(&a[0]), p.x);
        num!(ctx, w("y"), f(&a[1]), p.y);
        num!(ctx, w("vx"), f(&a[2]), p.vx);
        num!(ctx, w("vy"), f(&a[3]), p.vy);
        num!(ctx, w("ys"), f(&a[4]), p.ys);
        num!(ctx, w("R"), f(&a[5]), p.r);
        assert_eq!(
            opt_s(&a[6]).as_deref(),
            p.sup.map(|f| f.key()),
            "{ctx}: {} sup",
            p.id
        );
        assert_eq!(
            s(&a[7]),
            if p.ripped { "ripped" } else { "ok" },
            "{ctx}: {} fstate",
            p.id
        );
    }
    let cc = j["C"].as_array().unwrap();
    assert_eq!(cc.len(), sm.c.len(), "{ctx}: constraints");
    for (a, c) in cc.iter().zip(&sm.c) {
        assert_eq!(s(&a[0]), c.id, "{ctx}: constraint id");
        num!(ctx, format!("{} N", c.id), f(&a[1]), c.n);
        num!(ctx, format!("{} avg", c.id), f(&a[2]), c.avg);
        assert_eq!(a[3].as_bool(), Some(c.broken), "{ctx}: {} broken", c.id);
        assert_eq!(a[4].as_bool(), Some(c.slack), "{ctx}: {} slack", c.id);
    }
    let vv = j["V"].as_array().unwrap();
    assert_eq!(vv.len(), sm.v.len(), "{ctx}: vehicles");
    for (i, (a, v)) in vv.iter().zip(&sm.v).enumerate() {
        assert_eq!(s(&a[0]), v.kind.key(), "{ctx}: vehicle {i}");
        num!(ctx, format!("vehicle {i} x"), f(&a[1]), v.x);
        num!(ctx, format!("vehicle {i} y"), f(&a[2]), v.y);
        num!(ctx, format!("vehicle {i} ang"), f(&a[3]), v.ang);
        assert_eq!(s(&a[4]), v.state.key(), "{ctx}: vehicle {i} state");
        assert_eq!(
            a[5].as_u64(),
            Some(v.laps as u64),
            "{ctx}: vehicle {i} laps"
        );
    }
    check_first(
        ctx,
        &j["firstBreak"],
        &j["firstFound"],
        &sm.first_break,
        &sm.first_found,
    );
    match sm.end_at {
        Some(e) => num!(ctx, "endAt", f(&j["endAt"]), e),
        None => assert!(j["endAt"].is_null(), "{ctx}: endAt"),
    }
    num!(ctx, "sway", f(&j["sway"]), sm.sway);
    num!(ctx, "peakA", f(&j["peakA"]), sm.peak_a);
    num!(ctx, "aS", f(&j["aS"]), sm.a_s);
    num!(ctx, "windNow", f(&j["windNow"]), sm.wind_now);
    num!(ctx, "snowNow", f(&j["snowNow"]), sm.snow_now);
    num!(ctx, "env", f(&j["env"]), sm.env);
    match sm.quake_start {
        Some(q) => num!(ctx, "quakeStart", f(&j["quakeStart"]), q),
        None => assert!(j["quakeStart"].is_null(), "{ctx}: quakeStart"),
    }
    let sp = j["sparks"].as_array().unwrap();
    assert_eq!(sp.len(), sm.sparks.len(), "{ctx}: sparks");
    for (a, k) in sp.iter().zip(&sm.sparks) {
        num!(ctx, "spark x", f(&a[0]), k[0]);
        num!(ctx, "spark y", f(&a[1]), k[1]);
    }
    let sl = j["splashes"].as_array().unwrap();
    assert_eq!(sl.len(), sm.splashes.len(), "{ctx}: splashes");
    let ti = j["topI"].as_i64().unwrap();
    assert_eq!(ti, sm.top_i.map_or(-1, |i| i as i64), "{ctx}: topI");
    num!(ctx, "topY0", f(&j["topY0"]), sm.top_y0);
    num!(ctx, "width", f(&j["width"]), sm.width);
}

fn mode_key(m: BreakMode) -> &'static str {
    m.text()
}
fn fmode_key(m: FoundMode) -> &'static str {
    m.text()
}

fn check_first(
    ctx: &str,
    fb: &Value,
    ff: &Value,
    ofb: &Option<sp_engine::builder::FirstBreak>,
    off: &Option<sp_engine::builder::FirstFound>,
) {
    match ofb {
        Some(b) => {
            assert_eq!(s(&fb["id"]), b.id, "{ctx}: first break id");
            assert_eq!(s(&fb["type"]), b.kind.key(), "{ctx}: first break type");
            assert_eq!(s(&fb["mode"]), mode_key(b.mode), "{ctx}: first break mode");
            num!(ctx, "first break t", f(&fb["t"]), b.t);
        }
        None => assert!(fb.is_null(), "{ctx}: js first break {fb}"),
    }
    match off {
        Some(b) => {
            assert_eq!(s(&ff["id"]), b.id, "{ctx}: first foundation id");
            assert_eq!(
                s(&ff["mode"]),
                fmode_key(b.mode),
                "{ctx}: first foundation mode"
            );
        }
        None => assert!(ff.is_null(), "{ctx}: js first foundation {ff}"),
    }
}

fn check_result(ctx: &str, j: &Value, g: &Game) {
    let Some(r) = g.result() else {
        assert!(j.is_null(), "{ctx}: js result {j}");
        return;
    };
    assert!(!j.is_null(), "{ctx}: no js result");
    assert_eq!(j["pass"].as_bool(), Some(r.pass), "{ctx}: pass");
    assert_eq!(j["stars"].as_u64(), Some(r.stars as u64), "{ctx}: stars");
    num!(ctx, "cost", f(&j["cost"]), r.cost);
    assert_eq!(j["practice"].as_bool(), Some(r.practice), "{ctx}: practice");
    check_first(ctx, &j["first"], &j["firstFound"], &r.first, &r.first_found);
    match &r.outcome {
        Outcome::River { crossed, total } => {
            assert_eq!(
                j["crossed"].as_u64(),
                Some(*crossed as u64),
                "{ctx}: crossed"
            );
            assert_eq!(j["total"].as_u64(), Some(*total as u64), "{ctx}: total");
        }
        Outcome::Tower {
            standing,
            reached,
            mg,
            comfy,
            height,
            sway,
        } => {
            assert_eq!(
                j["standing"].as_bool().unwrap_or(false),
                *standing,
                "{ctx}: standing"
            );
            assert_eq!(j["reached"].as_bool(), Some(*reached), "{ctx}: reached");
            assert_eq!(j["comfy"].as_bool(), Some(*comfy), "{ctx}: comfy");
            num!(ctx, "mg", f(&j["mg"]), *mg);
            num!(ctx, "height", f(&j["height"]), *height);
            num!(ctx, "sway", f(&j["sway"]), *sway);
        }
    }
}

fn goldens() -> Value {
    serde_json::from_str(GOLDENS).expect("builder_goldens.json parses")
}

#[test]
fn pow_table_is_v8s() {
    let g = goldens();
    let t = g["pow02"].as_array().unwrap();
    assert_eq!(t.len(), POW02_TABLE.len());
    for (k, (a, b)) in t.iter().zip(POW02_TABLE).enumerate() {
        assert_eq!(
            u64::from_str_radix(a.as_str().unwrap(), 16).unwrap(),
            b,
            "k = {k}"
        );
    }
}

fn replay(name: &str) {
    let g0 = goldens();
    let sc = &g0["scenarios"][name];
    assert!(!sc.is_null(), "no scenario {name}");
    let mut g = Game::with_preset(&preset_of(&sc["preset"]));
    let snaps = sc["snaps"].as_array().unwrap();
    let mut k = 0;
    let mut last = std::collections::BTreeMap::new();
    for (i, a) in sc["actions"].as_array().unwrap().iter().enumerate() {
        let a = a.as_array().unwrap();
        if a[0] == "snap" {
            let ctx = format!("{name} #{k} (action {i})");
            let j = &snaps[k];
            check_state(&ctx, &j["state"], &g);
            check_preview(&ctx, &j["pv"], &g);
            check_sim(&ctx, &j["sim"], &g);
            check_result(&ctx, &j["result"], &g);
            check_view(&ctx, &j["view"], &g.view(), &mut last);
            k += 1;
        } else {
            act(&mut g, a);
        }
    }
    assert_eq!(k, snaps.len(), "{name}: snapshots");
}

macro_rules! scenarios {
    ($($f:ident => $n:expr),* $(,)?) => { $( #[test] fn $f() { replay($n); } )* };
}

scenarios! {
    levels => "levels",
    board_bridge_build => "board-BridgeBuild",
    board_tower_build => "board-TowerBuild",
    board_bridge_storm => "board-BridgeStorm",
    board_bridge_collapse => "board-BridgeCollapse",
    board_bridge_cable => "board-BridgeCable",
    board_tower_storm => "board-TowerStorm",
    board_tower_quake => "board-TowerQuake",
    river_steel_truss => "river-steel-truss",
    river_wood_truss_rush => "river-wood-truss-rush",
    haul_steel_piers => "haul-steel-piers",
    bus_cable => "bus-cable",
    edits_and_refusals => "edits-and-refusals",
    preview_snow_soil => "preview-snow-soil",
    mid_run_changes => "mid-run-changes",
    slow_motion => "slow-motion",
    towers => "towers",
    shaky_ground => "shaky-ground",
    camera => "camera",
    sandbox_flat_build => "sandbox-flat-build",
    sandbox_river_loops => "sandbox-river-loops",
}

#[test]
fn every_scenario_has_a_test() {
    let g = goldens();
    assert_eq!(g["scenarios"].as_object().unwrap().len(), 21);
}

/// The committed JSON is what the mockup gives today (run when Node is installed).
#[test]
fn goldens_are_current() {
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/builder_goldens.mjs");
    let Ok(out) = std::process::Command::new("node")
        .arg(script)
        .arg("--check")
        .output()
    else {
        eprintln!("node not found: skipping the re-run of builder_goldens.mjs");
        return;
    };
    assert!(
        out.status.success(),
        "builder_goldens.mjs failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout) == GOLDENS,
        "builder_goldens.json is stale: run `node crates/sp-engine/tests/builder_goldens.mjs`"
    );
}
