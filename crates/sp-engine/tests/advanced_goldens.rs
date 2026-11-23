//! Advanced screen goldens (SPEC ADV-10, ENG-5): for every toolbar state recorded by
//! `scripts/mockup-goldens.mjs` (the mockup's own `class Component`, run in Node), the generator,
//! solver, report and influence line must reproduce what the mockup shows — the netlist line for
//! line, the forces and displacements bit for bit, and every string of the member table, checks,
//! status pill and influence-line tiles. Regenerate with `node scripts/mockup-goldens.mjs`.

use serde_json::Value;
use sp_engine::generate::{self, MaterialKind, Params, Style};
use sp_engine::influence::{self, Level, Report, Tone, UseTone};
use sp_engine::jsnum::to_fixed;
use sp_engine::netlist;

fn goldens() -> Vec<Value> {
    let text = include_str!("golden/advanced.json");
    let v: Value = serde_json::from_str(text).expect("golden json");
    v["cases"].as_array().expect("cases").clone()
}

fn params(s: &Value) -> Params {
    Params {
        style: match s["style"].as_str().unwrap() {
            "pratt" => Style::Pratt,
            "howe" => Style::Howe,
            _ => Style::Warren,
        },
        braced: s["braced"].as_bool().unwrap(),
        material: MaterialKind::from_key(s["mat"].as_str().unwrap()).unwrap(),
        size: s["size"].as_u64().unwrap() as u32,
        truck_t: s["truckT"].as_f64().unwrap(),
        x: s["xt"].as_f64().unwrap(),
    }
}

fn tone_css(t: Tone) -> &'static str {
    match t {
        Tone::Zero => "#A3A3A3",
        Tone::Tension => "#7FB0E0",
        Tone::Compression => "#E7A47C",
        Tone::Failed => "#E8A39E",
    }
}

fn level_css(l: Level) -> &'static str {
    match l {
        Level::Good => "#5FBF8A",
        Level::Accent => "#D6C7A1",
        Level::Bad => "#E5534B",
        Level::Neutral => "#A3A3A3",
    }
}

fn use_css(u: UseTone) -> &'static str {
    match u {
        UseTone::Normal => "#EAEAEA",
        UseTone::Near => "#D6C7A1",
        UseTone::Over => "#E8A39E",
    }
}

/// A raw double from the golden (written as a string; see the script).
fn num(v: &Value) -> f64 {
    v.as_str().unwrap().parse().unwrap()
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

/// Everything the screen prints for a report, in the golden's shapes.
fn rows_of(r: &Report) -> Vec<Vec<String>> {
    r.rows
        .iter()
        .map(|row| {
            vec![
                row.id.clone(),
                row.nodes.clone(),
                row.force.clone(),
                row.stress.clone(),
                row.limit.clone(),
                row.utilisation.clone(),
                tone_css(row.tone).into(),
                use_css(row.use_tone).into(),
            ]
        })
        .collect()
}

/// The mockup's checks for a state, with the one deliberate change: SPEC MB-8 names the weight
/// check after the material ("Aluminum weight"); the mockup says "Steel weight" for every one.
fn want_checks(case: &Value) -> Vec<Vec<String>> {
    let mut c: Vec<Vec<String>> = case["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(strs)
        .collect();
    let mat = MaterialKind::from_key(case["state"]["mat"].as_str().unwrap()).unwrap();
    assert_eq!(c[4][0], "Steel weight", "the mockup's label");
    c[4][0] = format!("{} weight", mat.name());
    c
}

fn checks_of(r: &Report) -> Vec<Vec<String>> {
    r.checks
        .iter()
        .map(|c| {
            vec![
                c.label.clone(),
                c.value.clone(),
                c.note.clone(),
                level_css(c.level).into(),
            ]
        })
        .collect()
}

#[test]
fn generator_writes_the_mockups_netlist_for_every_state() {
    for case in goldens() {
        let p = params(&case["state"]);
        let g = generate::generate(&p);
        let want: Vec<(String, String, String)> = case["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| {
                let a = strs(l);
                (a[0].clone(), a[1].clone(), a[2].clone())
            })
            .collect();
        assert_eq!(g.lines.len(), want.len(), "{p:?}");
        for (k, (line, (dir, args, cmt))) in g.lines.iter().zip(&want).enumerate() {
            let cmt_want = cmt.strip_prefix("  ; ").unwrap_or(cmt);
            assert_eq!(
                (&line.dir, &line.args, line.comment.as_str()),
                (dir, args, cmt_want),
                "{p:?} line {}",
                k + 1
            );
            // and the text form is what the mockup renders (white-space: pre), trimmed
            let rendered = format!("{dir} {args}{cmt}");
            assert_eq!(line.text(), rendered.trim_end());
        }
        let text_lines: Vec<&str> = g.text.lines().collect();
        assert_eq!(text_lines.len(), want.len());
    }
}

#[test]
fn solve_matches_the_mockup_bit_for_bit() {
    for case in goldens() {
        let p = params(&case["state"]);
        let g = generate::generate(&p);
        let sol = g.netlist.solve_at(p.x).unwrap();
        assert_eq!(sol.stable, case["ok"].as_bool().unwrap());
        let u: Vec<f64> = case["u"].as_array().unwrap().iter().map(num).collect();
        for (i, d) in sol.displacements.iter().enumerate() {
            assert_eq!(d[0], u[2 * i], "{p:?} u[{}]", 2 * i);
            assert_eq!(d[1], u[2 * i + 1], "{p:?} u[{}]", 2 * i + 1);
        }
        for (i, (m, n)) in sol
            .members
            .iter()
            .zip(case["N"].as_array().unwrap())
            .enumerate()
        {
            let m = m.unwrap();
            assert_eq!(m.force, num(n), "{p:?} N of M{}", i + 1);
            let ut = num(&case["ut"][i]);
            assert_eq!(m.utilisation, ut, "{p:?} ut of M{}", i + 1);
        }
    }
}

#[test]
fn report_prints_what_the_mockup_prints() {
    for case in goldens() {
        let p = params(&case["state"]);
        let g = generate::generate(&p);
        let r = influence::analyse(&g.netlist).unwrap();
        let want_rows: Vec<Vec<String>> =
            case["rows"].as_array().unwrap().iter().map(strs).collect();
        assert_eq!(rows_of(&r), want_rows, "{p:?}");
        let want_checks = want_checks(&case);
        assert_eq!(checks_of(&r), want_checks, "{p:?}");
        assert_eq!(r.status.label(), case["statusLabel"].as_str().unwrap());
        assert_eq!(
            level_css(r.status.level()),
            case["statusColor"].as_str().unwrap()
        );
        let labels: Vec<Vec<String>> = case["memLabels"]
            .as_array()
            .unwrap()
            .iter()
            .map(strs)
            .collect();
        for (k, l) in labels.iter().enumerate() {
            assert_eq!(r.truss.members[k].id, l[0]);
            assert_eq!(tone_css(r.tones[k]), l[1], "{p:?} {}", l[0]);
        }
        // the mockup's `ok` + worst > 1 → "Over its limit." in the elevation label
        let schem = case["schemLabel"].as_str().unwrap();
        assert_eq!(schem.ends_with("Over its limit."), r.failed(), "{schem}");
        // also the mass, which the mockup computes as 2·vol·ρ
        assert_eq!(r.mass, num(&case["mass"]), "{p:?}");
    }
}

#[test]
fn influence_line_matches_the_mockup() {
    for case in goldens() {
        let p = params(&case["state"]);
        let g = generate::generate(&p);
        let r = influence::analyse(&g.netlist).unwrap();
        let picks = influence::picks(&g.netlist, &r);
        let names: Vec<String> = picks.iter().map(|k| k.name.clone()).collect();
        assert_eq!(names, strs(&case["ilPicks"]));
        let key = case["state"]["il"].as_str().unwrap();
        let pick = picks.iter().find(|k| k.key == key).unwrap();
        let il = influence::influence_line(&g.netlist, pick.member).unwrap();
        assert_eq!(il.id, case["ilId"].as_str().unwrap());
        assert_eq!(il.peak_text(), case["ilPeak"].as_str().unwrap(), "{p:?}");
        assert_eq!(
            il.peak_at_text(),
            case["ilPeakAt"].as_str().unwrap(),
            "{p:?}"
        );
        assert_eq!(il.limit_text(), case["ilLim"].as_str().unwrap(), "{p:?}");
        assert_eq!(il.label(), case["ilLabel"].as_str().unwrap(), "{p:?}");
        assert_eq!(il.range_text(), "x 0 → 48 m");
        let color = if il.tone() == Tone::Tension {
            "#7FB0E0"
        } else {
            "#E0905E"
        };
        assert_eq!(color, case["ilColor"].as_str().unwrap());
        let now = influence::force_kn(&r, pick.member);
        assert_eq!(influence::now_text(now), case["ilNow"].as_str().unwrap());
        // the plotted path, in the mockup's 640 × 220 viewBox
        let gx = |x: f64| 40.0 + (x - il.from) / (il.to - il.from) * 580.0;
        let gy = |v: f64| 110.0 - il.level(v) * 95.0;
        let d: Vec<String> = il
            .points
            .iter()
            .enumerate()
            .map(|(i, pt)| {
                format!(
                    "{}{} {}",
                    if i == 0 { "M" } else { "L" },
                    to_fixed(gx(pt[0]), 1),
                    to_fixed(gy(pt[1]), 1)
                )
            })
            .collect();
        assert_eq!(d.join(" "), case["ilD"].as_str().unwrap(), "{p:?}");
        let mut cap = String::new();
        if il.shows_tension_limit() {
            cap += &format!("M40 {} H620 ", to_fixed(gy(il.limit_tension), 1));
        }
        if il.shows_compression_limit() {
            cap += &format!("M40 {} H620", to_fixed(gy(-il.limit_compression), 1));
        }
        assert_eq!(cap, case["ilCap"].as_str().unwrap(), "{p:?}");
        assert_eq!(to_fixed(gx(p.x), 1), case["ilCx"].as_str().unwrap());
        assert_eq!(to_fixed(gy(now), 1), case["ilCy"].as_str().unwrap());
        assert_eq!(
            format!("x = {} m", to_fixed(p.x, 1)),
            case["clock"].as_str().unwrap()
        );
    }
}

/// Parsing the generated text (as the screen does once the user edits it) gives the same
/// screen: the section areas may differ from the mockup's `0.0025 × size` by one ulp, which
/// must not change anything printed — except where the mockup itself prints round-off: rows
/// are compared as sets (an ulp can reorder exact ties) and a zero-force member's sign
/// ("−0 crush" vs "0 yield") is noise either way.
#[test]
fn parsing_the_generated_text_shows_the_same_numbers() {
    for case in goldens() {
        let p = params(&case["state"]);
        let g = generate::generate(&p);
        let net = netlist::parse(&g.text).unwrap_or_else(|e| panic!("{p:?}: {e:?}"));
        assert!(net.warnings.is_empty(), "{:?}", net.warnings);
        assert_eq!(net.nodes, g.netlist.nodes);
        assert_eq!(net.members, g.netlist.members);
        assert_eq!(net.materials, g.netlist.materials);
        assert_eq!(net.loads, g.netlist.loads);
        assert_eq!(net.truck, g.netlist.truck);
        assert_eq!(net.plot, g.netlist.plot);
        assert_eq!(net.title, g.netlist.title);
        assert_eq!(net.notes, g.netlist.notes);
        for (a, b) in net.sections.iter().zip(&g.netlist.sections) {
            assert_eq!(a.name, b.name);
            assert!((a.area - b.area).abs() <= f64::EPSILON * b.area, "{p:?}");
            assert!(a.is_box());
        }
        let r = influence::analyse(&net).unwrap();
        let zero = |mut rows: Vec<Vec<String>>| {
            for row in &mut rows {
                if row[2] == "0" || row[2] == "\u{2212}0" {
                    row[2] = "0".into();
                    row[3] = row[3].replace('\u{2212}', "");
                    row[4] = "~".into();
                }
            }
            rows.sort();
            rows
        };
        let got = zero(rows_of(&r));
        let want = zero(case["rows"].as_array().unwrap().iter().map(strs).collect());
        assert_eq!(got, want, "{p:?}");
        // the worst member of an exact tie (M8 / M9 under a symmetric load) is round-off too
        let tail = |mut c: Vec<Vec<String>>| {
            if let Some(w) = c.get_mut(3) {
                w[1] = w[1]
                    .split_once(" at ")
                    .map(|x| x.1.to_string())
                    .unwrap_or_default();
            }
            c
        };
        let want_checks = want_checks(&case);
        assert_eq!(tail(checks_of(&r)), tail(want_checks), "{p:?}");
        let status = |s: &str| s.replace(|c: char| c.is_ascii_digit(), "#");
        assert_eq!(
            status(&r.status.label()),
            status(case["statusLabel"].as_str().unwrap())
        );
    }
}

/// The Truck and x sliders patch the netlist text in place; for the generated netlist that is
/// exactly what the generator writes for the new truck and position.
#[test]
fn truck_patch_equals_regeneration() {
    for case in goldens().iter().step_by(5) {
        let p = params(&case["state"]);
        let g = generate::generate(&p);
        for (t, x) in [(0.0, 0.0), (120.0, 48.0), (37.0, 12.5), (40.0, 24.0)] {
            let patched = netlist::set_truck(&g.text, Some(t), Some(x)).unwrap();
            let regen = generate::generate(&Params { truck_t: t, x, ..p });
            assert_eq!(patched, regen.text);
        }
    }
}
