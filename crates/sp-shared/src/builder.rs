//! The v2 builder for MCP, on `sp_engine::builder::Game`: reading arguments (levels, scenes,
//! conditions — refused, never clamped), the reports both sides send ([`builder_state`],
//! [`run_report`]) and a headless run of a structure on a level ([`test_structure`], the engine
//! tool `builder_test`). The app's bridge methods call the same functions on the screen's
//! `Game`, so the tool and the app always agree.

use crate::proto::{ConditionsChange, MAX_RUN_FRAMES, SANDBOX_RUN_SECONDS};
use crate::report::{
    BuilderState, ConditionsInfo, FoundationFailure, PieceFailure, RunReport, SampleReport,
    TowerOutcome, YearsOutcome, short,
};
use sp_engine::builder::data::{
    FIRES, LevelKey, PieceType, QUAKES, SNOWS, Scene, Soil, TrafficSet, WINDS,
};
use sp_engine::builder::design::Design;
use sp_engine::builder::game::{Game, Outcome, Phase, Tool};
use sp_engine::builder::sample::{Sample, Test};
use sp_engine::builder::{comfort, longterm, netlist};

/// The level ids, in the aside's order.
pub fn level_ids() -> Vec<&'static str> {
    LevelKey::ALL.iter().map(|k| k.key()).collect()
}

/// A level id (`river`, `bus`, `haul`, `t1`, `t2`, `t3`, `pisa`, `sandbox`; any case).
pub fn level(s: &str) -> Result<LevelKey, String> {
    let k = s.trim().to_ascii_lowercase();
    LevelKey::from_key(&k)
        .ok_or_else(|| format!("unknown level `{s}`; levels are {}", level_ids().join(", ")))
}

/// A Sandbox scene: `river` (River crossing) or `flat` (Open ground; also `open`, `ground`).
pub fn scene(s: &str) -> Result<Scene, String> {
    let k = s.trim().to_ascii_lowercase();
    let k = match k.as_str() {
        "open" | "ground" | "open ground" => "flat",
        other => other,
    };
    Scene::from_key(k).ok_or_else(|| format!("unknown scene `{s}`; scenes are river, flat"))
}

fn soil(s: &str) -> Result<Soil, String> {
    let k = s.trim().to_ascii_lowercase();
    Soil::from_key(&k).ok_or_else(|| {
        format!(
            "unknown soil `{s}`; soils are {}",
            Soil::ALL.map(|s| s.key()).join(", ")
        )
    })
}

fn traffic(s: &str) -> Result<TrafficSet, String> {
    let k = s.trim().to_ascii_lowercase();
    TrafficSet::from_key(&k).ok_or_else(|| {
        format!(
            "unknown traffic `{s}`; traffic sets are {}",
            TrafficSet::ALL.map(|t| t.key()).join(", ")
        )
    })
}

fn in_range(name: &str, v: Option<f64>, lo: f64, hi: f64) -> Result<Option<f64>, String> {
    match v {
        Some(x) if !(x.is_finite() && (lo..=hi).contains(&x)) => Err(format!(
            "`{name}` must be {} to {}, not {x}",
            short(lo),
            short(hi)
        )),
        other => Ok(other),
    }
}

/// Checks a [`ConditionsChange`] and applies it to `game` as the Conditions chips would (soil
/// and traffic only while building, as the app refuses them mid-run). Nothing changes when
/// anything is wrong.
pub fn apply_conditions(game: &mut Game, c: &ConditionsChange) -> Result<(), String> {
    let wind = in_range("wind_ms", c.wind_ms, 0.0, 50.0)?;
    let snow = in_range("snow_kg_m", c.snow_kg_m, 0.0, 800.0)?;
    let quake = in_range("quake_m", c.quake_m, 0.0, 0.5)?;
    let fire = in_range("fire_c", c.fire_c, 0.0, 900.0)?;
    let dir = match c
        .wind_from
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
    {
        None => None,
        Some(s) if s == "left" => Some(1.0),
        Some(s) if s == "right" => Some(-1.0),
        Some(s) => return Err(format!("`wind_from` must be left or right, not `{s}`")),
    };
    let soil = c.soil.as_deref().map(soil).transpose()?;
    let traffic = c.traffic.as_deref().map(traffic).transpose()?;
    if (soil.is_some() || traffic.is_some()) && game.phase() == Phase::Run {
        return Err(
            "soil and traffic change only while building: stop the run first (builder_level \
             or a new builder_run)"
                .into(),
        );
    }
    if let Some(v) = wind {
        game.set_wind(v);
    }
    if let Some(v) = dir {
        game.set_dir(v);
    }
    if let Some(v) = snow {
        game.set_snow(v);
    }
    if let Some(v) = quake {
        game.set_quake(v);
    }
    if let Some(v) = fire {
        game.set_fire(v);
    }
    if let Some(v) = soil {
        game.set_soil(v);
    }
    if let Some(v) = traffic {
        game.set_traffic(v);
    }
    Ok(())
}

/// The chip whose value is `v` ("Breezy"), or "" between chips.
fn chip(table: &[(f64, &'static str)], v: f64) -> String {
    table
        .iter()
        .find(|(x, _)| (x - v).abs() < 1e-9)
        .map(|(_, n)| n.to_string())
        .unwrap_or_default()
}

fn conditions(game: &Game) -> ConditionsInfo {
    let c = game.cond();
    ConditionsInfo {
        wind_ms: c.wind,
        wind: chip(&WINDS, c.wind),
        wind_from: if c.dir < 0.0 { "right" } else { "left" }.into(),
        snow_kg_m: c.snow,
        snow: chip(&SNOWS, c.snow),
        quake_m: c.quake,
        quake: chip(&QUAKES, c.quake),
        soil: c.soil.key().into(),
        soil_name: c.soil.name().into(),
        traffic: c.traffic.key().into(),
        traffic_name: c.traffic.name().into(),
        fire_c: c.fire,
        fire: chip(&FIRES, c.fire),
    }
}

/// The builder as it stands.
pub fn builder_state(game: &Game) -> BuilderState {
    let view = game.view();
    let setup = game.setup();
    let level = game.level();
    let design = game.design();
    let result = (game.phase() == Phase::Result)
        .then(|| run_report(game).map(Box::new))
        .flatten();
    BuilderState {
        level: level.key().into(),
        level_name: level.level().name,
        scene: game.scene().key().into(),
        document: level.slug(),
        goal: view.goal.clone(),
        phase: game.phase().key().into(),
        conditions: conditions(game),
        joints: design.nodes.len(),
        pieces: design.members.len(),
        foundations: design.nodes.iter().filter(|n| n.sup.is_some()).count(),
        // (an empty sum is −0.0)
        crates_t: design.loads.iter().map(|(_, t)| *t).sum::<f64>() + 0.0,
        cost_k: game.cost(),
        budget_k: setup.budget.filter(|b| *b != 0.0),
        status: view.status.label.clone(),
        practice: game.practice(),
        best_stars: game.best_for(level),
        result,
        summary: view.stage_label.clone(),
    }
}

/// The result of the last run, as the banner shows it (`None` before a run ends).
pub fn run_report(game: &Game) -> Option<RunReport> {
    let r = game.result()?;
    let view = game.view();
    let setup = game.setup();
    let title = view
        .result
        .as_ref()
        .map(|v| v.title.clone())
        .unwrap_or_default();
    let (crossed, vehicles, tower) = match &r.outcome {
        Outcome::River { crossed, total } => (Some(*crossed), Some(*total), None),
        Outcome::Tower {
            standing,
            reached,
            mg,
            comfy,
            height,
            sway,
        } => (
            None,
            None,
            Some(TowerOutcome {
                standing: *standing,
                height_m: *height,
                target_m: setup.target,
                reached: *reached,
                sway_m: *sway,
                comfort_mg: *mg,
                comfort_limit_mg: setup.comfort,
                comfy: *comfy,
                comfort: comfort::Meter::new(*mg, setup.comfort)
                    .sentence()
                    .to_string(),
            }),
        ),
    };
    Some(RunReport {
        pass: r.pass,
        title,
        explain: view.explain.clone(),
        stars: r.stars,
        practice: r.practice,
        cost_k: r.cost,
        budget_k: setup.budget.filter(|b| *b != 0.0),
        first_failure: r.first.as_ref().map(|f| PieceFailure {
            piece: f.id.clone(),
            piece_type: f.kind.name().into(),
            mode: f.mode.text().into(),
            t_s: f.t,
        }),
        first_foundation: r.first_found.as_ref().map(|f| FoundationFailure {
            joint: f.id.clone(),
            mode: f.mode.text().into(),
        }),
        crossed,
        vehicles,
        tower,
        years: None,
        t_s: game.sim().map_or(0.0, |s| s.t),
    })
}

/// Runs the builder's structure to the end, headless (as pressing the drive button and
/// waiting): a level's run until its result; the Sandbox (which never ends) for
/// [`SANDBOX_RUN_SECONDS`], then stopped there with the numbers so far. The game is left
/// showing the result (or the Sandbox's last frame). `Err` when it can't run (the Leaning
/// tower's run is the 30-year fast-forward).
pub fn run_to_end(game: &mut Game) -> Result<RunReport, String> {
    if game.phase() != Phase::Build {
        game.stop_run();
    }
    if game.level() == LevelKey::Sandbox {
        game.start_run(false);
        let frames = (SANDBOX_RUN_SECONDS * 30.0) as usize;
        for _ in 0..frames {
            if game.phase() != Phase::Run {
                break;
            }
            game.tick();
        }
        return Ok(sandbox_report(game));
    }
    let finished = game.run_test(MAX_RUN_FRAMES).is_some();
    match finished {
        true => run_report(game).ok_or_else(|| "the run gave no result".into()),
        false if game.phase() == Phase::Build => Err(format!(
            "{} runs as a fast-forward through the years: use builder_fast_forward",
            game.level().level().name
        )),
        false => Err(format!(
            "the run didn't finish within {MAX_RUN_FRAMES} frames"
        )),
    }
}

/// A Sandbox run so far (it never ends on its own): what broke first, vehicles' laps, the
/// tower's height, sway and comfort.
fn sandbox_report(game: &Game) -> RunReport {
    let view = game.view();
    let sim = game.sim();
    let first = sim.and_then(|s| s.first_break.clone());
    let first_found = sim.and_then(|s| s.first_found.clone());
    let (crossed, vehicles, tower) = match (game.scene(), sim) {
        (Scene::River, Some(s)) => (Some(s.laps() as usize), Some(s.v.len()), None),
        (Scene::Flat, Some(s)) => {
            let mg = s.milli_g();
            let height = s.top_y0;
            let standing = s.top_i.is_some_and(|i| s.p[i].y > height * 0.8);
            (
                None,
                None,
                Some(TowerOutcome {
                    standing,
                    height_m: height,
                    target_m: None,
                    reached: true,
                    sway_m: s.sway,
                    comfort_mg: mg,
                    comfort_limit_mg: None,
                    comfy: true,
                    comfort: comfort::Meter::new(mg, None).sentence().to_string(),
                }),
            )
        }
        _ => (None, None, None),
    };
    let pass = first.is_none() && first_found.is_none();
    RunReport {
        pass,
        title: if pass {
            "Still standing".into()
        } else {
            "Something gave way".into()
        },
        explain: view.explain.clone(),
        stars: 0,
        practice: false,
        cost_k: game.cost(),
        budget_k: None,
        first_failure: first.map(|f| PieceFailure {
            piece: f.id,
            piece_type: f.kind.name().into(),
            mode: f.mode.text().into(),
            t_s: f.t,
        }),
        first_foundation: first_found.map(|f| FoundationFailure {
            joint: f.id,
            mode: f.mode.text().into(),
        }),
        crossed,
        vehicles,
        tower,
        years: None,
        t_s: sim.map_or(0.0, |s| s.t),
    }
}

/// Builds `design` on the game's current level through the editing tools, as a player would:
/// joints at their spots (the level's own anchors reused), pieces joint to joint with their
/// types, foundations, crates. Edits the level refuses (a foundation off the banks, a piece too
/// long, underground) are skipped and returned as the game's toasts.
pub fn place_design(game: &mut Game, design: &Design) -> Vec<String> {
    let mut refused = Vec::new();
    let note = |game: &mut Game, refused: &mut Vec<String>| {
        if let Some(t) = game.toast() {
            let t = t.to_string();
            if !refused.contains(&t) {
                refused.push(t);
            }
        }
        game.advance(60.0); // the toast fades
    };
    game.advance(60.0);
    // joints: reuse a joint already at the spot (the level's anchors), else drop one
    let mut ids = std::collections::HashMap::new();
    game.pick_tool(Tool::Build);
    for j in &design.nodes {
        let existing = game
            .design()
            .nodes
            .iter()
            .find(|n| n.x == j.x && n.y == j.y)
            .map(|n| n.id.clone());
        let id = match existing {
            Some(id) => Some(id),
            None => {
                game.finish_line();
                game.click_at([j.x, j.y]);
                let id = game.pending().map(str::to_string);
                game.finish_line();
                id
            }
        };
        note(game, &mut refused);
        if let Some(id) = id {
            ids.insert(j.id.clone(), id);
        }
    }
    // pieces (one already there, as the level's own, of the same type, is left as it is)
    for m in &design.members {
        let (Some(a), Some(b)) = (ids.get(&m.a), ids.get(&m.b)) else {
            continue;
        };
        let there =
            game.design().members.iter().any(|p| {
                p.kind == m.kind && ((p.a == *a && p.b == *b) || (p.a == *b && p.b == *a))
            });
        if there {
            continue;
        }
        game.pick_tool(Tool::Build);
        game.pick_type(m.kind);
        game.finish_line();
        game.click_node(a);
        game.click_node(b);
        game.finish_line();
        note(game, &mut refused);
    }
    // foundations (only where they differ: the level's anchors are already founded)
    for j in &design.nodes {
        let Some(id) = ids.get(&j.id) else { continue };
        let now = game.design().node(id).and_then(|n| n.sup);
        if now != j.sup {
            game.set_foundation(id, j.sup);
            note(game, &mut refused);
        }
    }
    // crates: set to the design's (a level's own crate is already there; add the difference)
    for (jid, t) in &design.loads {
        if let Some(id) = ids.get(jid) {
            let more = *t - game.design().load(id);
            if more.abs() > 1e-9 {
                game.add_crate(id, more);
                note(game, &mut refused);
            }
        }
    }
    game.pick_tool(Tool::Select);
    refused
}

/// Checks a number of years (0–30).
pub fn years(y: Option<f64>) -> Result<f64, String> {
    let y = y.unwrap_or(longterm::YEARS);
    if y.is_finite() && (0.0..=longterm::YEARS).contains(&y) {
        Ok(y)
    } else {
        Err(format!(
            "`years` must be 0 to {}, not {y}",
            short(longterm::YEARS)
        ))
    }
}

/// Fast-forwards the game's structure `years` (TIME-1) headless and reports it: on the Leaning
/// tower the level's own result (pass under 0.5°, stars), which is judged after 30 years only
/// (fewer years are refused); elsewhere the settlement at `years`, passing when it still
/// stands then. The game itself is not changed.
pub fn fast_forward_report(game: &Game, years: f64) -> Result<RunReport, String> {
    let setup = game.setup();
    let lt = longterm::fast_forward_design(game.design(), &setup, game.cond());
    let snap = lt.at(years);
    let budget = setup.budget.filter(|b| *b != 0.0);
    let practice = game.practice();
    let cost = game.cost();
    let full = years >= longterm::YEARS - 1e-9;
    let (pass, stars, title, explain, sentence) = if game.level() == LevelKey::Pisa {
        if !full {
            return Err(format!(
                "the Leaning tower is judged after {} years: leave out `years` (or give {})",
                short(longterm::YEARS),
                short(longterm::YEARS)
            ));
        }
        let r = longterm::pisa::result(&lt, cost, practice, budget.unwrap_or(0.0));
        let sentence = format!("{} {}", lt.result_line(), lt.sentence());
        (r.pass, r.stars, r.title, r.explain, sentence)
    } else {
        let ended = matches!(lt.ending, longterm::Ending::Nothing);
        let stood = !snap.fallen && !ended;
        if full || snap.fallen || ended {
            // the 30-year line (or how it came down, or that nothing settles): the app's text
            let line = lt.result_line();
            let sentence = format!("{line} {}", lt.sentence());
            (stood, 0, line, sentence.clone(), sentence)
        } else {
            // part of the way: the numbers at that year, in the result line's words
            let numbers = if lt.is_tower() {
                format!(
                    "tilt {}°, settled {}",
                    sp_engine::jsnum::to_fixed(snap.tilt.abs(), 1),
                    longterm::mm(snap.max_settle)
                )
            } else {
                format!(
                    "differential settlement {}, settled {}",
                    longterm::mm(snap.differential),
                    longterm::mm(snap.max_settle)
                )
            };
            let line = format!("After {} years: {numbers}.", short(snap.year));
            let explain = format!(
                "{line} By year {}: {}",
                short(longterm::YEARS),
                lt.result_line()
            );
            (stood, 0, line.clone(), explain, line)
        }
    };
    Ok(RunReport {
        pass,
        title,
        explain,
        stars,
        practice,
        cost_k: cost,
        budget_k: budget,
        first_failure: None,
        first_foundation: None,
        crossed: None,
        vehicles: None,
        tower: None,
        years: Some(YearsOutcome {
            years: snap.year,
            tilt_deg: snap.tilt,
            settled_mm: snap.max_settle * 1000.0,
            differential_mm: snap.differential * 1000.0,
            failed: snap.fallen,
            sentence,
        }),
        t_s: 0.0,
    })
}

/// A piece type id (`road`, `floor`, `wood`, `steel`, `concrete`, `rope`, `cable`; any case).
pub fn piece_type(s: &str) -> Result<PieceType, String> {
    let k = s.trim().to_ascii_lowercase();
    PieceType::from_key(&k).ok_or_else(|| {
        format!(
            "unknown piece type `{s}`; piece types are {}",
            PieceType::ALL.map(|p| p.key()).join(", ")
        )
    })
}

/// Checks `sample_test`'s arguments: the piece type, `pull` / `push` (Rope and Cable only
/// pull), the temperature (−40…900 °C).
pub fn sample_args(
    piece: &str,
    mode: Option<&str>,
    temperature_c: Option<f64>,
) -> Result<(PieceType, Test, f64), String> {
    let kind = piece_type(piece)?;
    let test = match mode.map(|m| m.trim().to_ascii_lowercase()).as_deref() {
        None | Some("pull") => Test::Pull,
        Some("push") => Test::Push,
        Some(other) => return Err(format!("`mode` must be pull or push, not `{other}`")),
    };
    if test == Test::Push && kind.spec().t_only {
        return Err(format!(
            "{} only pulls (it goes slack when pushed): test it with mode pull",
            kind.name()
        ));
    }
    let t = temperature_c.unwrap_or(20.0);
    if !(t.is_finite() && (-40.0..=900.0).contains(&t)) {
        return Err(format!("`temperature_c` must be -40 to 900, not {t}"));
    }
    Ok((kind, test, t))
}

/// Runs a sample test to its end on `sample` (the drawer's bench: fresh, at `temp`, `test`)
/// and reports it.
pub fn run_sample(sample: &mut Sample, kind: PieceType, test: Test, temp: f64) -> SampleReport {
    sample.open(kind);
    sample.set_temp(temp);
    sample.fresh_sample();
    match test {
        Test::Pull => sample.pull(),
        Test::Push => sample.push(),
    }
    let broken = sample.run_to_end();
    sample_report(sample, broken)
}

/// The report for a sample bench as it stands.
pub fn sample_report(sample: &Sample, broken: bool) -> SampleReport {
    let kind = sample.kind();
    let test = sample.test();
    let curve = sample.curve();
    let points: Vec<[f64; 2]> = curve.map(|c| c.points.clone()).unwrap_or_default();
    let peak_n = curve.map_or(0.0, |c| c.peak_mpa() * 1e6 * c.area);
    let status = sample.status().label(kind);
    let capacity = sample.capacity_n();
    let t = |n: f64| n / (sp_engine::truss::G * 1000.0);
    let summary = format!(
        "{} {} at {} °C: {} — peak {} t ({} kN); a piece this size breaks at {} t in the builder",
        kind.name(),
        test.name().to_lowercase(),
        short(sample.temp()),
        status,
        sp_engine::jsnum::to_fixed(t(peak_n), 1),
        sp_engine::jsnum::to_fixed(peak_n / 1000.0, 0),
        sp_engine::jsnum::to_fixed(t(capacity), 1),
    );
    SampleReport {
        piece: kind.key().into(),
        mode: test.name().to_lowercase(),
        temperature_c: sample.temp(),
        broken,
        status,
        peak_t: t(peak_n),
        peak_kn: peak_n / 1000.0,
        piece_breaks_at_t: t(capacity),
        curve: points,
        summary,
    }
}

/// What [`test_structure`] ran and found.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StructureTest {
    /// The run's result.
    pub result: RunReport,
    /// The builder it ran in (level, conditions, joints, pieces, cost, the live status).
    pub state: BuilderState,
    /// Edits the level refused while building the structure (its rules' toasts).
    pub refused: Vec<String>,
}

/// The engine tool `builder_test`: on a fresh game, picks `level` (and the Sandbox's `scene`),
/// builds `netlist`'s structure there (in the Sandbox as pasted; on a level through the
/// editing tools, so the level's rules apply), applies the netlist's `* cond` line and then
/// `change`, and runs it to the end headless.
pub fn test_structure(
    level_id: Option<&str>,
    scene_id: Option<&str>,
    netlist_text: Option<&str>,
    change: &ConditionsChange,
    years_after: Option<f64>,
) -> Result<StructureTest, String> {
    let years_after = years_after.map(|y| years(Some(y))).transpose()?;
    let imported = netlist_text
        .map(|t| {
            netlist::read(t).map_err(|e| {
                format!(
                    "{e}. The builder takes netlists whose nodes sit on whole metres in a scene: \
                     the river scene (x −20…76 m, nothing below the banks at y = 0 for x ≤ 8 \
                     and x ≥ 48 or the riverbed at y = −12) or else open ground (y ≥ 0), like \
                     the one its Share copies"
                )
            })
        })
        .transpose()?;
    let level = match level_id {
        Some(l) => level(l)?,
        None => imported
            .as_ref()
            .and_then(|i| i.level)
            .unwrap_or(if imported.is_some() {
                LevelKey::Sandbox
            } else {
                LevelKey::River
            }),
    };
    let scene = match scene_id {
        Some(s) => {
            if level != LevelKey::Sandbox {
                return Err("`scene` is for the Sandbox only (levels have their own)".into());
            }
            Some(scene(s)?)
        }
        None => None,
    };
    let mut game = Game::new();
    let mut refused = Vec::new();
    if level == LevelKey::Sandbox {
        match &imported {
            Some(imp) => {
                if let Some(s) = scene
                    && s != imp.scene
                {
                    return Err(format!(
                        "the netlist's joints fit the {} scene, not `{}`",
                        imp.scene.key(),
                        s.key()
                    ));
                }
                game.open_in_sandbox(imp.scene, imp.design.clone());
            }
            None => {
                game.pick_level(LevelKey::Sandbox);
                if let Some(s) = scene {
                    game.pick_scene(s);
                }
            }
        }
    } else {
        game.pick_level(level);
        if let Some(imp) = &imported {
            let here = game.scene();
            if imp.scene != here {
                let what = |s: Scene| match s {
                    Scene::River => "a river crossing",
                    Scene::Flat => "open ground",
                };
                return Err(format!(
                    "this netlist is a structure for {} (its joints fit that scene), but level \
                     `{}` is {}: pick a level of that kind, or the Sandbox",
                    what(imp.scene),
                    level.key(),
                    what(here)
                ));
            }
            refused = place_design(&mut game, &imp.design);
        }
    }
    if let Some(c) = imported.as_ref().and_then(|i| i.cond) {
        let keep_level_soil = c.soil;
        game.set_wind(c.wind);
        game.set_dir(c.dir);
        game.set_snow(c.snow);
        game.set_quake(c.quake);
        game.set_fire(c.fire);
        game.set_soil(keep_level_soil);
        game.set_traffic(c.traffic);
    }
    apply_conditions(&mut game, change)?;
    // the Leaning tower's run is the fast-forward
    let years_after = match (years_after, game.level()) {
        (None, LevelKey::Pisa) => Some(longterm::YEARS),
        (y, _) => y,
    };
    let result = if game.level() == LevelKey::Pisa {
        fast_forward_report(&game, years_after.unwrap_or(longterm::YEARS))?
    } else {
        let mut r = run_to_end(&mut game)?;
        if let Some(y) = years_after {
            // TIME-1: after a run that stood, the years
            r.years = fast_forward_report(&game, y)?.years;
        }
        r
    };
    Ok(StructureTest {
        result,
        state: builder_state(&game),
        refused,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_engine::builder::data::Foundation;
    use sp_engine::builder::game::PresetBuild;

    fn none() -> ConditionsChange {
        ConditionsChange::default()
    }

    #[test]
    fn arguments_are_refused_not_clamped() {
        assert_eq!(level("Bus").unwrap(), LevelKey::Bus);
        assert!(level("moon").unwrap_err().contains("river, bus, haul"));
        assert_eq!(scene("open").unwrap(), Scene::Flat);
        let mut g = Game::new();
        let before = builder_state(&g);
        for bad in [
            ConditionsChange {
                wind_ms: Some(51.0),
                ..none()
            },
            ConditionsChange {
                quake_m: Some(f64::NAN),
                ..none()
            },
            ConditionsChange {
                soil: Some("mud".into()),
                wind_ms: Some(10.0),
                ..none()
            },
            ConditionsChange {
                wind_from: Some("up".into()),
                ..none()
            },
        ] {
            assert!(apply_conditions(&mut g, &bad).is_err(), "{bad:?}");
        }
        assert_eq!(builder_state(&g), before, "nothing changed");
        apply_conditions(
            &mut g,
            &ConditionsChange {
                wind_ms: Some(35.0),
                wind_from: Some("right".into()),
                soil: Some("soft".into()),
                ..none()
            },
        )
        .unwrap();
        let s = builder_state(&g);
        assert_eq!(s.conditions.wind, "Storm");
        assert_eq!(s.conditions.wind_from, "right");
        assert_eq!(s.conditions.soil_name, "Soft clay");
    }

    #[test]
    fn an_empty_level_fails_and_says_so() {
        let t = test_structure(Some("river"), None, None, &none(), None).unwrap();
        assert!(!t.result.pass);
        assert_eq!(t.result.vehicles, Some(3));
        assert_eq!(t.result.crossed, Some(0));
        assert!(!t.result.title.is_empty() && !t.result.explain.is_empty());
        assert_eq!(t.state.level, "river");
        assert_eq!(t.state.phase, "result");
    }

    /// A structure handed over as the builder's own Share runs exactly as the game that built
    /// it: placing it through the editing tools rebuilds the same design.
    #[test]
    fn a_shared_structure_runs_as_the_game_that_built_it() {
        for (level, build) in [
            (
                LevelKey::River,
                PresetBuild::Truss {
                    chord: PieceType::Steel,
                },
            ),
            (
                LevelKey::T1,
                PresetBuild::Tower {
                    h: 40.0,
                    bays: 2,
                    col: PieceType::Steel,
                    brace: PieceType::Wood,
                    found: Foundation::Pile,
                },
            ),
        ] {
            let mut g = Game::new();
            g.pick_level(level);
            g.build_preset(build);
            let text = g.share_text();
            let design = g.design().clone();
            g.run_test(MAX_RUN_FRAMES);
            let direct = run_report(&g).expect("a result");
            let t = test_structure(Some(level.key()), None, Some(&text), &none(), None).unwrap();
            assert!(t.refused.is_empty(), "{level:?}: {:?}", t.refused);
            assert_eq!(t.state.joints, design.nodes.len(), "{level:?}");
            assert_eq!(t.state.pieces, design.members.len(), "{level:?}");
            assert_eq!(t.result.pass, direct.pass, "{level:?}");
            assert_eq!(t.result.title, direct.title, "{level:?}");
            assert_eq!(t.result.stars, direct.stars, "{level:?}");
            assert!((t.result.cost_k - direct.cost_k).abs() < 1e-9, "{level:?}");
        }
    }

    /// The sample lab breaks a steel sample at the builder's own capacity scale and refuses
    /// pushing a rope.
    #[test]
    fn samples_break_and_bad_samples_are_refused() {
        let (k, t, temp) = sample_args("Steel", None, None).unwrap();
        let mut s = Sample::new(k);
        let r = run_sample(&mut s, k, t, temp);
        assert!(r.broken, "{r:?}");
        assert_eq!((r.piece.as_str(), r.mode.as_str()), ("steel", "pull"));
        assert!(r.peak_t >= r.piece_breaks_at_t * 0.99, "{r:?}");
        assert!(r.curve.len() > 3);
        assert!(
            sample_args("rope", Some("push"), None)
                .unwrap_err()
                .contains("only pulls")
        );
        assert!(sample_args("gold", None, None).is_err());
        assert!(sample_args("wood", None, Some(1000.0)).is_err());
        let (k, t, temp) = sample_args("concrete", Some("push"), Some(700.0)).unwrap();
        let hot = run_sample(&mut s, k, t, temp);
        let (k, t, temp) = sample_args("concrete", Some("push"), None).unwrap();
        let cold = run_sample(&mut s, k, t, temp);
        assert!(
            hot.piece_breaks_at_t < cold.piece_breaks_at_t,
            "{hot:?} {cold:?}"
        );
    }

    /// TIME-1 / TIME-3: the Leaning tower left alone leans past its goal over 30 years; a
    /// tower on soft clay reports its settlement after a run; years are checked.
    #[test]
    fn years_of_settlement() {
        let t = test_structure(Some("pisa"), None, None, &none(), None).unwrap();
        let y = t.result.years.as_ref().expect("the years");
        assert_eq!(y.years, 30.0);
        assert!(!t.result.title.is_empty() && !t.result.explain.is_empty());
        // (until the engine lists the Leaning tower, `pisa` is a placeholder level)
        if sp_engine::builder::data::pisa_available() {
            assert!(!t.result.pass, "untouched it keeps leaning: {:?}", t.result);
            assert!(y.tilt_deg.abs() > 0.5, "{y:?}");
        }
        let mut g = Game::new();
        g.pick_level(LevelKey::T1);
        g.build_preset(PresetBuild::Tower {
            h: 40.0,
            bays: 2,
            col: PieceType::Steel,
            brace: PieceType::Wood,
            found: Foundation::Footing,
        });
        let soft = ConditionsChange {
            soil: Some("soft".into()),
            ..none()
        };
        let t = test_structure(Some("t1"), None, Some(&g.share_text()), &soft, Some(12.0)).unwrap();
        let y = t.result.years.expect("the years after the run");
        assert_eq!(y.years, 12.0);
        assert!(y.settled_mm > 0.0, "{y:?}");
        assert!(years(Some(31.0)).is_err() && years(Some(f64::NAN)).is_err());
        assert_eq!(years(None).unwrap(), 30.0);
    }

    /// Every level's own Share, built back on that level through the editing tools, is the
    /// same structure — joints, pieces, crates (a level's own crates are set, not doubled) —
    /// with nothing refused and the same result (review finding: the Leaning tower's bells).
    #[test]
    fn every_levels_share_round_trips() {
        for k in LevelKey::ALL {
            if k == LevelKey::Sandbox {
                continue;
            }
            let mut g = Game::new();
            g.pick_level(k);
            let text = g.share_text();
            let crates: f64 = g.design().loads.iter().map(|(_, t)| t).sum();
            let t = test_structure(None, None, Some(&text), &none(), None).unwrap();
            assert!(t.refused.is_empty(), "{k:?}: {:?}", t.refused);
            assert_eq!(t.state.level, k.key(), "{k:?}");
            assert_eq!(t.state.joints, g.design().nodes.len(), "{k:?}");
            assert_eq!(t.state.pieces, g.design().members.len(), "{k:?}");
            assert!(
                (t.state.crates_t - crates).abs() < 1e-9,
                "{k:?}: {} vs {crates}",
                t.state.crates_t
            );
            let direct = if k == LevelKey::Pisa {
                fast_forward_report(&g, longterm::YEARS).unwrap()
            } else {
                run_to_end(&mut g).unwrap()
            };
            assert_eq!(t.result.title, direct.title, "{k:?}");
            assert_eq!(t.result.pass, direct.pass, "{k:?}");
            assert_eq!(t.result.stars, direct.stars, "{k:?}");
        }
    }

    /// Part of the way through the years, the line is the snapshot's (review finding), and the
    /// Leaning tower is judged after 30 years only.
    #[test]
    fn partial_years_say_what_they_show() {
        let mut g = Game::new();
        g.pick_level(LevelKey::Sandbox);
        g.pick_scene(Scene::Flat);
        g.build_preset(PresetBuild::Tower {
            h: 32.0,
            bays: 2,
            col: PieceType::Steel,
            brace: PieceType::Wood,
            found: Foundation::Footing,
        });
        g.set_soil(Soil::Soft);
        let five = fast_forward_report(&g, 5.0).unwrap();
        let thirty = fast_forward_report(&g, 30.0).unwrap();
        let y5 = five.years.as_ref().unwrap();
        assert_eq!(y5.years, 5.0);
        assert!(
            five.title.starts_with("After 5 years: tilt "),
            "{}",
            five.title
        );
        assert!(
            five.title.contains(&longterm::mm(y5.settled_mm / 1000.0)),
            "{} vs {y5:?}",
            five.title
        );
        assert!(
            thirty.title.starts_with("After 30 years"),
            "{}",
            thirty.title
        );
        assert_ne!(five.title, thirty.title);
        assert!(y5.settled_mm < thirty.years.as_ref().unwrap().settled_mm);
        let mut p = Game::new();
        p.pick_level(LevelKey::Pisa);
        assert!(
            fast_forward_report(&p, 12.0)
                .unwrap_err()
                .contains("after 30 years")
        );
        assert!(fast_forward_report(&p, 30.0).is_ok());
    }

    #[test]
    fn the_sandbox_takes_a_netlist_and_runs_for_a_while() {
        let mut g = Game::new();
        g.build_preset(PresetBuild::Truss {
            chord: PieceType::Steel,
        });
        let text = g.share_text().replace("* level river", "* level sandbox");
        let t = test_structure(Some("sandbox"), None, Some(&text), &none(), None).unwrap();
        assert_eq!(t.state.level, "sandbox");
        assert_eq!(t.result.stars, 0);
        assert!(t.result.t_s > SANDBOX_RUN_SECONDS - 1.0, "{}", t.result.t_s);
        let err =
            test_structure(Some("sandbox"), Some("flat"), Some(&text), &none(), None).unwrap_err();
        assert!(err.contains("river scene"), "{err}");
        let err = test_structure(Some("river"), Some("flat"), None, &none(), None).unwrap_err();
        assert!(err.contains("Sandbox only"), "{err}");
        // an open-ground tower isn't built on a river level (review finding)
        let mut t = Game::new();
        t.pick_level(LevelKey::T1);
        let tower = t.share_text();
        let err = test_structure(Some("river"), None, Some(&tower), &none(), None).unwrap_err();
        assert!(
            err.contains("open ground") && err.contains("river crossing"),
            "{err}"
        );
        let err = test_structure(None, None, Some(".node A 1.5 0"), &none(), None).unwrap_err();
        assert!(err.contains("grid"), "{err}");
    }
}
