//! The engine reports `structures-mcp` returns, built from `sp-engine` here so the engine tools
//! (headless) and the app tools (over the bridge) send exactly the same thing: a solved truss
//! ([`TrussReport`]), a member's influence line ([`InfluenceReport`]), a tower in the wind
//! ([`TowerReport`]), a building on its ground ([`GroundReport`]), a tensile test
//! ([`TensileReport`]), and the v2 builder: its state ([`BuilderState`]), a run's result
//! ([`RunReport`]) and a sample test ([`SampleReport`]).
//!
//! Units are in the field names (`_kn`, `_mpa`, `_m`, `_mm`, `_pct`); forces are positive in
//! tension. Every number comes from the engine (SPEC DONE-2); text fields are the strings the
//! screens print.

use serde::{Deserialize, Serialize};
use sp_engine::ground::{self, GroundInput};
use sp_engine::influence;
use sp_engine::materials::bench::Bench;
use sp_engine::materials::curves::Kind;
use sp_engine::netlist::{Diagnostic, Netlist, Severity};
use sp_engine::tower::{self, TowerInput};
use sp_engine::truss::{FailureMode, SolveError};

// ---- shared bits ------------------------------------------------------------------------------

/// A netlist problem with its place (1-based line and column, in characters).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    /// Line, from 1.
    pub line: usize,
    /// Column, from 1.
    pub column: usize,
    /// `error` or `warning`.
    pub severity: String,
    /// What's wrong ("unknown node B9").
    pub message: String,
}

impl From<&Diagnostic> for DiagnosticInfo {
    fn from(d: &Diagnostic) -> Self {
        Self {
            line: d.line,
            column: d.column,
            severity: match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            }
            .into(),
            message: d.message.clone(),
        }
    }
}

impl DiagnosticInfo {
    /// `line 3, column 9: unknown node B9`.
    pub fn text(&self) -> String {
        format!(
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

/// A readout tile as the screen shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TileInfo {
    /// The eyebrow ("Top sway").
    pub label: String,
    /// The value ("928 mm").
    pub value: String,
    /// The note under it ("drift H/259 · aim H/500").
    pub note: String,
}

impl From<&tower::Tile> for TileInfo {
    fn from(t: &tower::Tile) -> Self {
        Self {
            label: t.key.into(),
            value: t.value.clone(),
            note: t.note.clone(),
        }
    }
}

/// A challenge goal and whether it is met now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalInfo {
    /// The goal.
    pub text: String,
    /// Met.
    pub done: bool,
}

impl From<&tower::Goal> for GoalInfo {
    fn from(g: &tower::Goal) -> Self {
        Self {
            text: g.text.into(),
            done: g.done,
        }
    }
}

/// `yield`, `buckling`, `crushing`.
pub fn mode_name(m: FailureMode) -> &'static str {
    match m {
        FailureMode::Yield => "yield",
        FailureMode::Buckling => "buckling",
        FailureMode::Crushing => "crushing",
    }
}

fn kn(n: f64) -> f64 {
    n / 1000.0
}

/// A number for a summary: at most two decimals, printed as JavaScript prints it (`22.08`,
/// `24`, `0.25`), so float noise like `22.07999999999988` never shows.
pub fn short(v: f64) -> String {
    sp_engine::jsnum::to_string((v * 100.0).round() / 100.0)
}

// ---- truss --------------------------------------------------------------------------------------

/// One member's result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberInfo {
    /// "M7".
    pub id: String,
    /// First node id.
    pub a: String,
    /// Second node id.
    pub b: String,
    /// Length, m.
    pub length_m: f64,
    /// Axial force, kN (positive in tension; 0 without a solution).
    pub force_kn: f64,
    /// `tension`, `compression` or `zero` (|N| < 1 kN, as the Advanced table greys it).
    pub state: String,
    /// Axial stress, MPa.
    pub stress_mpa: f64,
    /// The limit for the current sign of the force, kN (A·fy, or min(A·fy, Euler) squeezed).
    pub limit_kn: f64,
    /// Euler's critical load, kN.
    pub euler_kn: f64,
    /// |N| / limit (over 1: past its limit).
    pub utilisation: f64,
    /// What governs: `yield` (stretched), `buckling` or `crushing` (squeezed).
    pub mode: String,
}

/// A support reaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactionInfo {
    /// The node.
    pub node: String,
    /// `xy` (pin), `y` (roller) or `x`.
    pub fix: String,
    /// Horizontal reaction, kN.
    pub rx_kn: f64,
    /// Vertical reaction, kN (up is positive).
    pub ry_kn: f64,
}

/// The moving load where it stands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TruckInfo {
    /// Weight, t.
    pub tonnes: f64,
    /// Position along the lane, m.
    pub x_m: f64,
    /// Share of the truck one truss carries (0.5: two trusses).
    pub share: f64,
}

/// A check from the Advanced screen ("Determinacy", "Midspan deflection", …).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckInfo {
    /// "Determinacy".
    pub label: String,
    /// "m + r − 2j = 21 + 3 − 24 = 0".
    pub value: String,
    /// The sentence under it.
    pub note: String,
}

/// A netlist solved: what `solve_truss` returns (the same solve the Advanced screen shows).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrussReport {
    /// The netlist's title (`river-crossing.str` from its first `*` line), if any.
    pub title: Option<String>,
    /// A solution exists (the stiffness matrix is regular).
    pub stable: bool,
    /// The node free to move when it isn't (a mechanism).
    pub mechanism: Option<String>,
    /// The status pill: "Holding · worst 40%", "M9 over its limit", "Mechanism".
    pub status: String,
    /// Joints.
    pub nodes: usize,
    /// Members.
    pub members: usize,
    /// Restrained directions (r).
    pub reactions_count: usize,
    /// `m + r − 2j`: 0 statically determinate, > 0 indeterminate (redundant members), < 0 a
    /// mechanism.
    pub determinacy: i64,
    /// Mass of the members of every truss carrying the deck, kg.
    pub mass_kg: f64,
    /// Identical trusses carrying the deck (`1 / share`).
    pub copies: u32,
    /// The moving load, if the netlist has one.
    pub truck: Option<TruckInfo>,
    /// Support reactions (supported nodes, in node order).
    pub reactions: Vec<ReactionInfo>,
    /// Every member's result, in netlist order.
    pub member_results: Vec<MemberInfo>,
    /// The most used member.
    pub worst: Option<MemberInfo>,
    /// The Advanced screen's checks.
    pub checks: Vec<CheckInfo>,
    /// Parser warnings (directives for other worlds, filled-in values).
    pub warnings: Vec<DiagnosticInfo>,
}

impl TrussReport {
    /// A one-line summary: "river-crossing.str: Holding · worst 40% — 21 members, worst M9 at
    /// 40% (buckling), determinacy 0, 7.6 t".
    pub fn summary(&self) -> String {
        let name = self.title.as_deref().unwrap_or("the netlist");
        let worst = match &self.worst {
            Some(w) if self.stable => format!(
                ", worst {} at {}% ({})",
                w.id,
                (w.utilisation * 100.0).round(),
                w.mode
            ),
            _ => String::new(),
        };
        format!(
            "{name}: {} — {} nodes, {} members{worst}, determinacy {}, {:.1} t",
            self.status,
            self.nodes,
            self.members,
            self.determinacy,
            self.mass_kg / 1000.0
        )
    }
}

/// Solves `net` with the truck at `x` (its written position when `None`), the way the
/// Advanced screen does.
pub fn truss_report(net: &Netlist, x: Option<f64>) -> Result<TrussReport, SolveError> {
    let x = x.unwrap_or_else(|| net.truck_x());
    let r = influence::analyse_at(net, x)?;
    let truss = &r.truss;
    let node_id = |i: usize| truss.nodes.get(i).map(|n| n.id.clone()).unwrap_or_default();
    let member_results: Vec<MemberInfo> = truss
        .members
        .iter()
        .enumerate()
        .filter_map(|(i, m)| {
            let res = r.solution.members.get(i)?.as_ref()?;
            let state = if !r.solution.stable || res.force.abs() < influence::NEAR_ZERO_N {
                "zero"
            } else if res.force > 0.0 {
                "tension"
            } else {
                "compression"
            };
            Some(MemberInfo {
                id: m.id.clone(),
                a: node_id(m.a),
                b: node_id(m.b),
                length_m: res.length,
                force_kn: kn(res.force),
                state: state.into(),
                stress_mpa: res.stress / 1e6,
                limit_kn: kn(res.limit),
                euler_kn: kn(res.euler),
                utilisation: res.utilisation,
                mode: mode_name(res.mode).into(),
            })
        })
        .collect();
    let worst = r
        .solution
        .worst
        .and_then(|w| truss.members.get(w))
        .and_then(|m| member_results.iter().find(|x| x.id == m.id).cloned());
    let reactions = truss
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.fix.x || n.fix.y)
        .map(|(i, n)| {
            let [rx, ry] = r.solution.reactions.get(i).copied().unwrap_or([0.0, 0.0]);
            ReactionInfo {
                node: n.id.clone(),
                fix: sp_engine::netlist::fix_text(n.fix).unwrap_or("").into(),
                rx_kn: kn(rx),
                ry_kn: kn(ry),
            }
        })
        .collect::<Vec<_>>();
    let reactions_count = truss
        .nodes
        .iter()
        .map(|n| n.fix.x as usize + n.fix.y as usize)
        .sum();
    Ok(TrussReport {
        title: net.title.clone(),
        stable: r.solution.stable,
        mechanism: r.solution.mechanism.map(node_id),
        status: r.status.label(),
        nodes: truss.nodes.len(),
        members: truss.members.len(),
        reactions_count,
        determinacy: truss.determinacy(),
        mass_kg: r.mass,
        copies: r.copies,
        truck: net.truck.as_ref().map(|t| TruckInfo {
            tonnes: t.tonnes.unwrap_or(t.weight / 9810.0),
            x_m: x,
            share: t.share,
        }),
        reactions,
        member_results,
        worst,
        checks: r
            .checks
            .iter()
            .map(|c| CheckInfo {
                label: c.label.clone(),
                value: c.value.clone(),
                note: c.note.clone(),
            })
            .collect(),
        warnings: net.warnings.iter().map(DiagnosticInfo::from).collect(),
    })
}

/// A member's influence line: the force in it as the truck crosses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InfluenceReport {
    /// The member ("M8").
    pub member: String,
    /// Its nodes ("T2–T3").
    pub nodes: String,
    /// The truck's weight, t.
    pub truck_tonnes: f64,
    /// `[truck x m, N kN]`, positive in tension (0 where there is no solution).
    pub points: Vec<[f64; 2]>,
    /// The largest |N| over the crossing, signed, kN.
    pub peak_kn: f64,
    /// Where the truck stands for the peak, m.
    pub peak_at_m: f64,
    /// Tension limit A·fy, kN.
    pub limit_tension_kn: f64,
    /// Compression limit min(A·fy, Euler), kN (positive).
    pub limit_compression_kn: f64,
    /// The peak is past the limit for its sign.
    pub over_limit: bool,
    /// The sweep: first and last truck positions, m.
    pub from_m: f64,
    /// The last truck position, m.
    pub to_m: f64,
}

impl InfluenceReport {
    /// "M8 (T2–T3): peak −1004 kN with the truck at x = 24 m, limit −2500 kN; 49 points".
    pub fn summary(&self) -> String {
        let limit = if self.peak_kn >= 0.0 {
            self.limit_tension_kn
        } else {
            -self.limit_compression_kn
        };
        format!(
            "{} ({}): peak {:.0} kN with the truck at x = {} m, limit {:.0} kN{}; {} points",
            self.member,
            self.nodes,
            self.peak_kn,
            short(self.peak_at_m),
            limit,
            if self.over_limit { " — over it" } else { "" },
            self.points.len()
        )
    }
}

/// The influence line of member `id` (its netlist id) in `net`. `Err` names the problem: no
/// such member, or no moving load to sweep.
pub fn influence_report(net: &Netlist, id: &str) -> Result<InfluenceReport, String> {
    let Some(index) = net.members.iter().position(|m| m.id == id) else {
        let ids: Vec<&str> = net.members.iter().take(40).map(|m| m.id.as_str()).collect();
        return Err(format!(
            "no member `{id}` in the netlist; its members are {}{}",
            ids.join(", "),
            if net.members.len() > 40 { ", …" } else { "" }
        ));
    };
    let Some(truck) = net.truck.as_ref() else {
        return Err("the netlist has no moving load to sweep: add a line like \
                    `.load truck 40t x=24 lane=B0..B6 share=0.5`"
            .into());
    };
    let il = influence::influence_line(net, index).ok_or_else(|| {
        "the influence line can't be swept: the truck's lane has no length, or the truss \
         doesn't validate"
            .to_string()
    })?;
    let m = &net.members[index];
    let over_limit = if il.peak >= 0.0 {
        il.peak > il.limit_tension
    } else {
        -il.peak > il.limit_compression
    };
    let node = |i: usize| net.nodes.get(i).map(|n| n.id.clone()).unwrap_or_default();
    Ok(InfluenceReport {
        member: il.id.clone(),
        nodes: format!("{}–{}", node(m.a), node(m.b)),
        truck_tonnes: truck.tonnes.unwrap_or(truck.weight / 9810.0),
        points: il.points.clone(),
        peak_kn: il.peak,
        peak_at_m: il.peak_at,
        limit_tension_kn: il.limit_tension,
        limit_compression_kn: il.limit_compression,
        over_limit,
        from_m: il.from,
        to_m: il.to,
    })
}

// ---- tower --------------------------------------------------------------------------------------

/// A tower's settings with string keys (what the tools take and return).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TowerSettings {
    /// Floors (5–120; 4 m each).
    pub floors: u32,
    /// `core`, `tube`, `outrigger`, `braced`.
    pub system: String,
    /// `straight`, `tapered`, `twisted`, `hollow`.
    pub shape: String,
    /// Wind at 10 m, m/s (5–50).
    pub wind_ms: f64,
    /// Tuned mass damper at the top.
    pub damper: bool,
}

impl From<&TowerInput> for TowerSettings {
    fn from(i: &TowerInput) -> Self {
        Self {
            floors: i.floors,
            system: i.system.def().key.into(),
            shape: i.shape.def().key.into(),
            wind_ms: i.v10,
            damper: i.damper,
        }
    }
}

/// A tower in the wind (the Tower screen's model, a simplified cantilever).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TowerReport {
    /// What was modelled.
    pub settings: TowerSettings,
    /// Height, m.
    pub height_m: f64,
    /// Static top sway, mm.
    pub top_sway_mm: f64,
    /// Drift ratio x of H/x (the aim is H/500 or better: larger is stiffer).
    pub drift_ratio: f64,
    /// First-mode sway period, s.
    pub period_s: f64,
    /// First natural frequency, Hz.
    pub frequency_hz: f64,
    /// Damping ratio used, % (1.5, or 5 with the damper).
    pub damping_pct: f64,
    /// Base shear, MN.
    pub base_shear_mn: f64,
    /// Overturning moment at the base, GN·m.
    pub overturning_gnm: f64,
    /// Peak acceleration at the top, milli-g (comfort bands 5 / 10 / 20).
    pub peak_accel_mg: f64,
    /// Wind at the top, m/s.
    pub wind_top_ms: f64,
    /// Vortex shedding frequency over the sway frequency.
    pub vortex_ratio: f64,
    /// Inside the lock-in band (0.75–1.25).
    pub lock_in: bool,
    /// Game cost, $M.
    pub cost_musd: f64,
    /// The status pill.
    pub status: String,
    /// The occupant comfort sentence.
    pub comfort: String,
    /// "What's happening".
    pub explain: String,
    /// The six readout tiles.
    pub tiles: Vec<TileInfo>,
    /// "Challenge · 100 floors".
    pub goals: Vec<GoalInfo>,
    /// Share's `.story … .wind …` line.
    pub share: String,
}

impl TowerReport {
    /// "60 floors (240 m), core, straight, wind 30 m/s: top sway 928 mm (H/259), period 9.9 s,
    /// 9.5 milli-g — Too much sway".
    pub fn summary(&self) -> String {
        format!(
            "{} floors ({:.0} m), {}, {}{}, wind {} m/s: top sway {:.0} mm (H/{:.0}), period {:.1} s, \
             {:.1} milli-g — {}",
            self.settings.floors,
            self.height_m,
            self.settings.system,
            self.settings.shape,
            if self.settings.damper { ", damper" } else { "" },
            short(self.settings.wind_ms),
            self.top_sway_mm,
            self.drift_ratio,
            self.period_s,
            self.peak_accel_mg,
            self.status
        )
    }
}

/// Models `input` (clamped to the screen's ranges).
pub fn tower_report(input: &TowerInput) -> TowerReport {
    let r = tower::calc(input);
    TowerReport {
        settings: input.into(),
        height_m: r.height,
        top_sway_mm: r.delta * 1000.0,
        drift_ratio: r.drift_ratio(),
        period_s: r.period(),
        frequency_hz: r.f1,
        damping_pct: r.zeta * 100.0,
        base_shear_mn: r.base_shear / 1e6,
        overturning_gnm: r.overturning / 1e9,
        peak_accel_mg: r.mg,
        wind_top_ms: r.v_top,
        vortex_ratio: r.ratio,
        lock_in: r.lock,
        cost_musd: r.cost,
        status: tower::Status::of(&r).label().into(),
        comfort: tower::Comfort::of(r.mg).sentence().into(),
        explain: tower::explain(input, &r),
        tiles: tower::tiles(&r).iter().map(TileInfo::from).collect(),
        goals: tower::goals(input, &r).iter().map(GoalInfo::from).collect(),
        share: tower::share_line(input, &r),
    }
}

// ---- ground -------------------------------------------------------------------------------------

/// A building's ground with string keys (what the tools take and return).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundSettings {
    /// `dense`, `stiff`, `soft`, `loose`.
    pub soil: String,
    /// `footing`, `mat`, `piles`, `caisson`.
    pub foundation: String,
    /// Building floors (2–80).
    pub floors: u32,
    /// Pile length, m (8–40; piles only).
    pub pile_length_m: f64,
    /// Earthquake shaking.
    pub quake: bool,
    /// Years after construction (0–30).
    pub years: f64,
}

impl From<&GroundInput> for GroundSettings {
    fn from(i: &GroundInput) -> Self {
        Self {
            soil: i.soil.def().key.into(),
            foundation: i.foundation.def().key.into(),
            floors: i.floors,
            pile_length_m: i.pile_len,
            quake: i.quake,
            years: i.years,
        }
    }
}

/// A building on its ground (the Ground screen's model, simplified).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundReport {
    /// What was modelled.
    pub settings: GroundSettings,
    /// Building load over the footprint, kPa.
    pub load_kpa: f64,
    /// Pressure on the bearing area, kPa.
    pub pressure_kpa: f64,
    /// Capacity, kPa.
    pub capacity_kpa: f64,
    /// Capacity / pressure.
    pub safety_factor: f64,
    /// The factor aimed for: 2 for deep foundations, 3 for shallow ones.
    pub target_safety_factor: f64,
    /// Building load, MN.
    pub building_load_mn: f64,
    /// Settlement at `years`, mm.
    pub settlement_mm: f64,
    /// Final settlement, mm.
    pub settlement_final_mm: f64,
    /// Tilt at `years`, degrees.
    pub tilt_deg: f64,
    /// Final tilt, degrees.
    pub tilt_final_deg: f64,
    /// Consolidation time constant, years.
    pub consolidation_years: f64,
    /// Piles or caissons.
    pub deep: bool,
    /// Founded on bedrock.
    pub on_rock: bool,
    /// The sand has liquefied.
    pub liquefied: bool,
    /// The status pill.
    pub status: String,
    /// "What's happening".
    pub explain: String,
    /// The six readout tiles.
    pub tiles: Vec<TileInfo>,
    /// "Challenge · The lean of Pisa".
    pub goals: Vec<GoalInfo>,
    /// Share's `.soil … .foundation …` line.
    pub share: String,
}

impl GroundReport {
    /// "6 floors on a mat slab in soft clay, year 10: safety factor 0.83 (target 3), tilt
    /// 2.37°, sank 268 mm — Bearing failure".
    pub fn summary(&self) -> String {
        format!(
            "{} floors, {} in {} soil{}, year {}: safety factor {:.2} (target {}), tilt {:.2}°, \
             sank {:.0} mm — {}",
            self.settings.floors,
            self.settings.foundation,
            self.settings.soil,
            if self.settings.quake { ", shaking" } else { "" },
            short(self.settings.years),
            self.safety_factor,
            short(self.target_safety_factor),
            self.tilt_deg,
            self.settlement_mm,
            self.status
        )
    }
}

/// Models `input` (clamped to the screen's ranges).
pub fn ground_report(input: &GroundInput) -> GroundReport {
    let r = ground::calc(input);
    GroundReport {
        settings: input.into(),
        load_kpa: r.load,
        pressure_kpa: r.pressure,
        capacity_kpa: r.capacity,
        safety_factor: r.fs,
        target_safety_factor: r.target(),
        building_load_mn: r.building_load_mn(),
        settlement_mm: r.settle,
        settlement_final_mm: r.settle_final,
        tilt_deg: r.tilt,
        tilt_final_deg: r.tilt_final,
        consolidation_years: r.tau,
        deep: r.deep,
        on_rock: r.to_rock,
        liquefied: r.liquefied,
        status: ground::Status::of(&r).label().into(),
        explain: ground::explain(input, &r).into(),
        tiles: ground::tiles(input, &r)
            .iter()
            .map(TileInfo::from)
            .collect(),
        goals: ground::goals(input).iter().map(GoalInfo::from).collect(),
        share: ground::share_line(input, &r),
    }
}

// ---- materials ----------------------------------------------------------------------------------

/// A tensile test at one stretch (and optionally the whole curve).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TensileReport {
    /// `steel`, `aluminum`, `concrete`, `timber`, `carbon-fiber`.
    pub material: String,
    /// "Carbon fiber".
    pub name: String,
    /// Temperature, °C.
    pub temperature_c: f64,
    /// Stretch, %.
    pub strain_pct: f64,
    /// Stress at that stretch, MPa (0 once fractured).
    pub stress_mpa: f64,
    /// The load cell force on the 50 mm² sample, kN.
    pub force_kn: f64,
    /// Fractured at (or before) this stretch.
    pub broken: bool,
    /// The stretch at fracture, %.
    pub break_strain_pct: f64,
    /// The status pill: Ready / Elastic: springs back / Plastic: permanently stretched /
    /// Necking / Fractured at 22%.
    pub status: String,
    /// Stiffness E, GPa.
    pub e_gpa: f64,
    /// Yield strength at this temperature, MPa (the brittle ones have none).
    pub yield_mpa: Option<f64>,
    /// Peak strength at this temperature, MPa.
    pub peak_mpa: f64,
    /// Toughness (area under the curve at this temperature), MJ/m³.
    pub toughness_mj_m3: f64,
    /// Strength per kilogram at this temperature, kN·m/kg.
    pub specific_strength: f64,
    /// Density, kg/m³.
    pub density_kg_m3: f64,
    /// `[strain %, stress MPa]` samples from 0 to fracture, when asked for.
    pub curve: Option<Vec<[f64; 2]>>,
}

impl TensileReport {
    /// "Steel at 20 °C, 12.00 %: 421 MPa (21.1 kN) — Plastic: permanently stretched; breaks at 22 %"
    /// (the stretch with two decimals, like the Materials clock).
    pub fn summary(&self) -> String {
        format!(
            "{} at {} °C, {} %: {:.0} MPa ({:.1} kN) — {}; breaks at {} %",
            self.name,
            short(self.temperature_c),
            sp_engine::jsnum::to_fixed(self.strain_pct, 2),
            self.stress_mpa,
            self.force_kn,
            self.status,
            short(self.break_strain_pct)
        )
    }
}

/// The test of `kind` at `t_c` °C stretched to `strain` % (on the wide 0–25 % axis), with the
/// curve sampled at `curve_points` points when given (2–2001).
pub fn tensile_report(
    kind: Kind,
    t_c: f64,
    strain: f64,
    curve_points: Option<usize>,
) -> TensileReport {
    let mut bench = Bench::default();
    bench.pick(kind);
    bench.set_zoom(sp_engine::materials::bench::Zoom::Wide);
    bench.set_temp(t_c);
    bench.set_strain(strain);
    tensile_from_bench(&bench, curve_points)
}

/// The report for a bench as it stands.
pub fn tensile_from_bench(bench: &Bench, curve_points: Option<usize>) -> TensileReport {
    let kind = bench.kind();
    let c = kind.curve();
    let t = bench.temp();
    let now = bench.now();
    let curve = curve_points.map(|n| {
        let n = n.clamp(2, 2001);
        let end = c.break_strain();
        (0..n)
            .map(|i| {
                let e = end * i as f64 / (n - 1) as f64;
                // the last corner itself is the moment of fracture: report its stress
                let s = c.stress_at(e.min(end * (1.0 - 1e-12)), t);
                [e, s.mpa]
            })
            .collect()
    });
    TensileReport {
        material: kind.key().into(),
        name: kind.name().into(),
        temperature_c: t,
        strain_pct: bench.strain(),
        stress_mpa: now.mpa,
        force_kn: bench.force_kn(),
        broken: now.broken,
        break_strain_pct: now.break_strain,
        status: bench.status().label(),
        e_gpa: c.e_gpa,
        yield_mpa: c.yield_at(t),
        peak_mpa: c.peak_mpa(t),
        toughness_mj_m3: c.toughness_at(t),
        specific_strength: c.specific_strength(t),
        density_kg_m3: c.rho,
        curve,
    }
}

// ---- the builder (v2) ---------------------------------------------------------------------------

/// The conditions as the builder shows them (BLD-12).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionsInfo {
    /// Wind at 10 m, m/s.
    pub wind_ms: f64,
    /// The wind chip ("Breezy"; "" between chips).
    pub wind: String,
    /// `left` (blowing →) or `right` (←).
    pub wind_from: String,
    /// Snow, kg/m.
    pub snow_kg_m: f64,
    /// The snow chip ("Heavy").
    pub snow: String,
    /// Earthquake peak ground motion, g (0: off).
    pub quake_g: f64,
    /// The earthquake chip ("Strong").
    pub quake: String,
    /// `rock`, `dense`, `stiff`, `soft`, `loose`.
    pub soil: String,
    /// "Soft clay".
    pub soil_name: String,
    /// `level`, `rush`, `buses`, `trucks`, `convoy` (river scenes).
    pub traffic: String,
    /// "Rush hour".
    pub traffic_name: String,
    /// Fire, °C (0: off).
    pub fire_c: f64,
    /// The fire chip ("Big").
    pub fire: String,
}

/// The builder as it stands (`builder.get` and every builder change).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuilderState {
    /// `river`, `bus`, `haul`, `t1`, `t2`, `t3`, `pisa`, `sandbox`.
    pub level: String,
    /// "1 · First crossing" / "Sandbox".
    pub level_name: String,
    /// `river` (a river crossing) or `flat` (open ground).
    pub scene: String,
    /// The header's document name (`river-level`, `sandbox`).
    pub document: String,
    /// The level's goal sentence.
    pub goal: String,
    /// `build`, `run` or `result`.
    pub phase: String,
    /// The conditions.
    pub conditions: ConditionsInfo,
    /// Joints.
    pub joints: usize,
    /// Pieces.
    pub pieces: usize,
    /// Joints with a footing or a pile.
    pub foundations: usize,
    /// Crates hung, t in all.
    pub crates_t: f64,
    /// Cost, $k.
    pub cost_k: f64,
    /// The level's budget, $k (`None`: the Sandbox).
    pub budget_k: Option<f64>,
    /// The live preview's status pill ("Stands up", "Sags and folds", …).
    pub status: String,
    /// Easier conditions than the level's: a practice run, no stars.
    pub practice: bool,
    /// Best stars earned on this level so far (0–3).
    pub best_stars: u8,
    /// The last run's result, while it shows.
    pub result: Option<Box<RunReport>>,
    /// One line: the stage's summary ("Bridge with 9 joints and 15 pieces. Stands up.").
    pub summary: String,
}

/// The first piece that broke in a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PieceFailure {
    /// The piece ("M7").
    pub piece: String,
    /// Its type ("Wood").
    pub piece_type: String,
    /// "snapped", "cracked", "buckled", "crushed", "softened", "burned through".
    pub mode: String,
    /// When, s into the run.
    pub t_s: f64,
}

/// The first foundation that gave way in a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundationFailure {
    /// The joint ("J3").
    pub joint: String,
    /// "sank", "ripped out", "liquefied".
    pub mode: String,
}

/// A run on open ground: height, sway and comfort (CMF-1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TowerOutcome {
    /// The top floor stayed above 80 % of its height.
    pub standing: bool,
    /// The top floor's height, m.
    pub height_m: f64,
    /// The level's target height, m.
    pub target_m: Option<f64>,
    /// It reached the target (or there is none).
    pub reached: bool,
    /// Peak sway at the top, m.
    pub sway_m: f64,
    /// Peak top-floor acceleration, milli-g.
    pub comfort_mg: f64,
    /// The level's comfort limit, milli-g.
    pub comfort_limit_mg: Option<f64>,
    /// Under the limit (or there is none).
    pub comfy: bool,
    /// The comfort meter's sentence ("Some people on the top floors notice.").
    pub comfort: String,
}

/// Settlement over the years (TIME-1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YearsOutcome {
    /// Years fast-forwarded.
    pub years: f64,
    /// Tilt at the end, degrees (towers; TIME-2).
    pub tilt_deg: f64,
    /// The most any foundation settled, mm.
    pub settled_mm: f64,
    /// The largest settlement difference between foundations, mm (bridges; TIME-2).
    pub differential_mm: f64,
    /// It toppled (a tower leaning past 5°) or a piece broke from the settlement.
    pub failed: bool,
    /// The result's sentence ("After 30 years: tilt 0.3°, settled 41 mm. …").
    pub sentence: String,
}

/// The result of a run (`builder.run`, `builder_test`, `builder.fast_forward`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunReport {
    /// It passed: every vehicle across, or the tower stood, reached and stayed comfortable.
    pub pass: bool,
    /// The banner ("All across!", "5 of 8 made it", "It stood!", "It came down", "Too short",
    /// "Too wobbly to live in", "…, but over budget").
    pub title: String,
    /// The banner's sentence: what happened first, with a tip.
    pub explain: String,
    /// Stars (1–3 within budget: ≤ budget, ≤ 80 %, ≤ 60 %; 0 for a fail, over budget, a
    /// practice run or the Sandbox).
    pub stars: u8,
    /// Easier conditions than the level's.
    pub practice: bool,
    /// Cost, $k.
    pub cost_k: f64,
    /// The level's budget, $k.
    pub budget_k: Option<f64>,
    /// The first piece that broke.
    pub first_failure: Option<PieceFailure>,
    /// The first foundation that gave way.
    pub first_foundation: Option<FoundationFailure>,
    /// River scenes: vehicles across.
    pub crossed: Option<usize>,
    /// River scenes: vehicles in the run.
    pub vehicles: Option<usize>,
    /// Open ground: height, sway, comfort.
    pub tower: Option<TowerOutcome>,
    /// After a fast-forward: the settlement.
    pub years: Option<YearsOutcome>,
    /// Seconds simulated.
    pub t_s: f64,
}

impl RunReport {
    /// "All across! ★★☆ ($17.2k of $22k)" / "It came down — M7 (Wood) buckled at 3.1 s".
    pub fn summary(&self) -> String {
        let mut s = self.title.clone();
        if self.stars > 0 {
            s.push_str(&format!(
                " {}{}",
                "★".repeat(self.stars as usize),
                "☆".repeat(3usize.saturating_sub(self.stars as usize))
            ));
        }
        if self.practice {
            s.push_str(" (practice: easier than the level)");
        }
        match self.budget_k {
            Some(b) => s.push_str(&format!(" — ${}k of ${}k", short(self.cost_k), short(b))),
            None => s.push_str(&format!(" — ${}k", short(self.cost_k))),
        }
        if let Some(f) = &self.first_failure {
            s.push_str(&format!(
                "; first: {} ({}) {} at {} s",
                f.piece,
                f.piece_type,
                f.mode,
                sp_engine::jsnum::to_fixed(f.t_s, 1)
            ));
        } else if let Some(f) = &self.first_foundation {
            s.push_str(&format!(
                "; first: the foundation at {} {}",
                f.joint, f.mode
            ));
        }
        if let Some(t) = &self.tower {
            s.push_str(&format!(
                "; {} m tall, {} mg",
                short(t.height_m),
                sp_engine::jsnum::to_fixed(t.comfort_mg, 0)
            ));
        }
        if let Some(y) = &self.years {
            s.push_str(&format!("; {}", y.sentence));
        }
        s
    }
}

/// A sample test in the sample lab (LAB-1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SampleReport {
    /// The piece type (`steel`).
    pub piece: String,
    /// `pull` or `push`.
    pub mode: String,
    /// Temperature, °C.
    pub temperature_c: f64,
    /// It broke (snapped, crushed or buckled).
    pub broken: bool,
    /// The status ("Snapped at 0.4 %").
    pub status: String,
    /// The peak force the sample took, t.
    pub peak_t: f64,
    /// The peak force, kN.
    pub peak_kn: f64,
    /// What a piece this size breaks at in the builder, t (the engine's capacity).
    pub piece_breaks_at_t: f64,
    /// `[strain %, stress MPa]` along the test.
    pub curve: Vec<[f64; 2]>,
    /// One line.
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Summaries print clean numbers: no float noise, no Debug floats.
    #[test]
    fn summaries_have_no_float_noise() {
        assert_eq!(short(22.07999999999988), "22.08");
        assert_eq!(short(24.000000000001), "24");
        assert_eq!(short(0.25), "0.25");
        let mut bench = Bench::default();
        bench.pick(Kind::Steel);
        bench.set_strain(22.07999999999988);
        let t = tensile_from_bench(&bench, None);
        let s = t.summary();
        assert!(s.contains("20 °C, 22.08 %"), "{s}");
        assert!(!s.contains("999"), "{s}");
        let g = ground_report(&GroundInput {
            years: 10.000000000002,
            ..GroundInput::default()
        });
        assert!(g.summary().contains("year 10:"), "{}", g.summary());
        let tw = tower_report(&TowerInput {
            v10: 30.000000000004,
            ..TowerInput::default()
        });
        assert!(tw.summary().contains("wind 30 m/s"), "{}", tw.summary());
    }
}
