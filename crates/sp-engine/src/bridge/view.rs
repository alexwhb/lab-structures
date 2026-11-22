//! What the Bridge screen draws, derived from the [`Game`] (`renderVals()` in
//! `Playground.dc.html`): stage geometry in grid metres, member bands, handles and joints with
//! their accessible labels, the status pill, the explain / inspector sentences, the hint bar,
//! the overlays and the header hooks. Every number and sentence the screen shows comes from
//! here (SPEC DONE-2); the screen maps metres to pixels and tones to colours.

use super::data::{LevelKey, MaterialKey, SizeKey, Support};
use super::design::{Analysis, Band};
use super::fmt::{num, round_str, stars, to_fixed};
use super::game::{Game, Phase, Selection, Tool};
use crate::truss::{FailureMode, ZERO_FORCE_N};

/// Displacements are drawn × 40 with "Exaggerate sag" on.
pub const SAG_FACTOR: f64 = 40.0;

/// A colour role (the screen maps it to `sp_theme`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tone {
    /// Grey (`#A3A3A3`): nothing built yet.
    Muted,
    /// Green (`#5FBF8A`): holding, it held.
    Ok,
    /// Amber (`#D8B86A`): near the limit.
    Warn,
    /// Red (`#E5534B`): unstable, will break, collapse.
    Danger,
}

/// The status pill (`statusLabel` / `statusColor`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// "Nothing built yet", "Wobbly: not stable", "Will break at 63% load", "Holding, near the
    /// limit", "Holding".
    pub label: String,
    /// Its colour.
    pub tone: Tone,
}

/// A member line on the stage.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberView {
    /// Index into the design's members.
    pub index: usize,
    /// Its id.
    pub id: String,
    /// End positions, grid m (displaced / simulated).
    pub a: [f64; 2],
    /// End position.
    pub b: [f64; 2],
    /// Its size (stroke width).
    pub size: SizeKey,
    /// Its colour band.
    pub band: Band,
}

/// A support triangle (drawn at the joint's design position, not displaced).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportView {
    /// Joint index.
    pub joint: usize,
    /// Position, grid m.
    pub at: [f64; 2],
    /// Pin or roller (roller adds two wheels).
    pub kind: Support,
}

/// A hung crate with its cable and tag.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadView {
    /// Joint index.
    pub joint: usize,
    /// The joint's drawn position, grid m.
    pub at: [f64; 2],
    /// Tonnes (crate width `12 + √t · 4` px).
    pub tonnes: f64,
    /// The tag under the crate ("20 t").
    pub tag: String,
}

/// A member's handle (dot at its middle; with "Show forces", its force in kN).
#[derive(Debug, Clone, PartialEq)]
pub struct HandleView {
    /// Index into the design's members.
    pub member: usize,
    /// Member id.
    pub id: String,
    /// Middle of the drawn member, grid m.
    pub at: [f64; 2],
    /// The force text ("−120", "+86") with "Show forces", else empty.
    pub text: String,
    /// Shown as a number tag (`mh-num`, 20 px tall) rather than a 14 px dot.
    pub numeric: bool,
    /// Selected (`mh-sel`).
    pub selected: bool,
    /// Over its limit (`mh-over`).
    pub over: bool,
    /// Accessible label ("Member M7, 63% of its limit").
    pub label: String,
}

/// A joint button.
#[derive(Debug, Clone, PartialEq)]
pub struct JointView {
    /// Joint index.
    pub index: usize,
    /// Its id.
    pub id: String,
    /// Drawn position, grid m.
    pub at: [f64; 2],
    /// A level joint (square dot, sand ring).
    pub locked: bool,
    /// The member being drawn starts here (filled, glowing).
    pub pending: bool,
    /// Selected (halo).
    pub selected: bool,
    /// The mechanism's free joint (red ring).
    pub bad: bool,
    /// Accessible label ("Joint J3, pin support, carrying 20 t").
    pub label: String,
}

/// The Test's loading bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Loading {
    /// Bar fill 0–1 (rounded to the shown percent).
    pub frac: f64,
    /// "63%".
    pub text: String,
}

/// The result banner.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultView {
    /// "It held!", "It held, but over budget", "Collapse: M7 buckled at 63%", "Collapse: it
    /// folded".
    pub title: String,
    /// It held (green) rather than collapsed (red).
    pub held: bool,
    /// The star row ("★★☆") when it held in a level with a budget, else empty.
    pub stars: String,
    /// Offer "Replay in slow motion".
    pub replay: bool,
}

/// A level row.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelRow {
    /// Which level.
    pub key: LevelKey,
    /// "1 · River crossing".
    pub name: &'static str,
    /// Best stars ("★☆☆"), empty for the Sandbox.
    pub best: String,
    /// The current level.
    pub on: bool,
}

/// The member inspector's colour for the utilisation bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarTone {
    /// No forces (grey `#5E5E5E`).
    Zero,
    /// Tension blue.
    Tension,
    /// Compression orange.
    Compression,
    /// Over its limit (red).
    Over,
}

/// The inspector for a selected member (`mi`).
#[derive(Debug, Clone, PartialEq)]
pub struct MemberInspector {
    /// Member id.
    pub id: String,
    /// "M7 · J3–J5".
    pub title: String,
    /// "20.9 m" (empty while it's broken).
    pub length: String,
    /// Utilisation bar fill 0–1.
    pub fill: f64,
    /// "63%" or "—".
    pub ut: String,
    /// Bar colour.
    pub tone: BarTone,
    /// The sentence ("Stretched with 120 kN. It will snap at 2000 kN.").
    pub text: String,
    /// Its size (chips).
    pub size: SizeKey,
}

/// The inspector for a selected joint (`ni`).
#[derive(Debug, Clone, PartialEq)]
pub struct JointInspector {
    /// Joint id.
    pub id: String,
    /// "Joint J3 · level".
    pub title: String,
    /// "x 28 m · y 0 m".
    pub pos: String,
    /// Its support (which chip is on).
    pub support: Option<Support>,
    /// Supports are allowed here (Pin / Roller chips not dimmed).
    pub can_support: bool,
    /// A level joint (every support chip dimmed, no Delete).
    pub locked: bool,
    /// "20 t".
    pub load: String,
    /// The support note ("Pinned: held in place, free to rotate.").
    pub note: &'static str,
    /// "Delete joint" shows.
    pub can_delete: bool,
}

impl JointInspector {
    /// Is the support chip for `s` dimmed (`chip-dim`)?
    pub fn chip_dim(&self, s: Option<Support>) -> bool {
        (s.is_some() && !self.can_support) || self.locked
    }
}

/// "Built from" (nothing selected).
#[derive(Debug, Clone, PartialEq)]
pub struct BuiltFrom {
    /// The material in use.
    pub material: MaterialKey,
    /// Joints tile ("3").
    pub joints: String,
    /// Members tile ("0").
    pub members: String,
    /// Weight tile ("0.0 t").
    pub weight: String,
    /// "Drop in a starter truss" shows.
    pub can_starter: bool,
}

/// The aside's inspector slot.
#[derive(Debug, Clone, PartialEq)]
pub enum Inspector {
    /// A member is selected.
    Member(MemberInspector),
    /// A joint is selected.
    Joint(JointInspector),
    /// Nothing selected: "Built from".
    Nothing(BuiltFrom),
}

/// Everything the Bridge screen shows.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    // ---- stage
    /// The collapse simulation is drawn (no joints, grey members, sparks).
    pub in_sim: bool,
    /// Drawn position of every joint, grid m.
    pub positions: Vec<[f64; 2]>,
    /// Members still standing, in order.
    pub members: Vec<MemberView>,
    /// Supports.
    pub supports: Vec<SupportView>,
    /// Hung crates.
    pub loads: Vec<LoadView>,
    /// Sparks (red ×) where members broke, grid m.
    pub sparks: Vec<[f64; 2]>,
    /// The mechanism's free joint (dashed red ring), drawn position.
    pub bad: Option<[f64; 2]>,
    /// The hover dot, grid m.
    pub hover: Option<[f64; 2]>,
    /// The ghost member from the glowing joint to the hover dot.
    pub ghost: Option<([f64; 2], [f64; 2])>,
    /// Member handles (Select / Erase, or Show forces).
    pub handles: Vec<HandleView>,
    /// Joint buttons (none during the collapse).
    pub joints: Vec<JointView>,
    /// The river's dashes flow (testing / collapsing).
    pub water_flowing: bool,
    /// Crosshair cursor over the grid.
    pub crosshair: bool,
    /// The stage's summary ("Structure with 5 joints and 7 members. Holding.").
    pub stage_label: String,
    /// The grid surface button's label.
    pub surface_label: &'static str,
    /// The legend line.
    pub legend: [&'static str; 4],
    // ---- overlays
    /// The toast.
    pub toast: Option<String>,
    /// The loading bar (testing).
    pub loading: Option<Loading>,
    /// The result banner (held / collapsed).
    pub result: Option<ResultView>,
    // ---- hint bar
    /// The hint.
    pub hint: &'static str,
    /// Size chips show (Member tool).
    pub show_sizes: bool,
    /// Weight chips show (Weight tool).
    pub show_weights: bool,
    /// "Finish line" shows.
    pub has_pending: bool,
    // ---- aside
    /// The level rows.
    pub levels: Vec<LevelRow>,
    /// The goal sentence.
    pub goal: &'static str,
    /// The status pill.
    pub status: Status,
    /// "$0.8k / $30k".
    pub cost_text: String,
    /// Over budget (cost in red).
    pub over_budget: bool,
    /// "Test it: add the load" / "Back to building".
    pub test_label: &'static str,
    /// Inspector / Built from.
    pub inspector: Inspector,
    /// What's happening.
    pub explain: String,
    // ---- header
    /// The drive button reads "Pause".
    pub playing: bool,
    /// The document name ("river-level").
    pub slug: &'static str,
    /// The clock ("building", "load 63%", "held", "collapse").
    pub clock: String,
}

/// The status pill for an analysis (`statusLabel`).
pub fn status_of(r: &Analysis) -> Status {
    if r.n_act == 0 {
        return Status {
            label: "Nothing built yet".into(),
            tone: Tone::Muted,
        };
    }
    if !r.ok {
        return Status {
            label: "Wobbly: not stable".into(),
            tone: Tone::Danger,
        };
    }
    match r.worst_state() {
        Some((_, w)) if w.utilisation > 1.0 => Status {
            label: format!("Will break at {}% load", round_str(100.0 / w.utilisation)),
            tone: Tone::Danger,
        },
        Some((_, w)) if w.utilisation > 0.8 => Status {
            label: "Holding, near the limit".into(),
            tone: Tone::Warn,
        },
        _ => Status {
            label: "Holding".into(),
            tone: Tone::Ok,
        },
    }
}

/// `fmtK`: "+86" / "−120" (kN, typographic minus).
pub fn fmt_kn(v: f64) -> String {
    format!(
        "{}{}",
        if v < 0.0 { "−" } else { "+" },
        to_fixed((v / 1000.0).abs(), 0)
    )
}

/// The explanation of a collapse by the first failure mode.
fn mode_lesson(m: FailureMode) -> &'static str {
    match m {
        FailureMode::Buckling => {
            "Long, thin members in compression bow sideways long before the material itself gives out. Make it thicker, shorter, or brace it halfway."
        }
        FailureMode::Yield => {
            "It was stretched past what the material can take. A thicker member spreads the same pull over more steel."
        }
        FailureMode::Crushing => "It was squeezed past its strength.",
    }
}

impl Game {
    /// What the screen draws now.
    pub fn view(&self) -> View {
        let d = self.design();
        let r = self.analysis();
        let level = self.level().level();
        let phase = self.phase();
        let building = phase == Phase::Build;
        let in_sim = self.in_sim();
        let sim = self.sim().filter(|_| in_sim);
        let ex = if self.sag() { SAG_FACTOR } else { 1.0 };

        // positions
        let positions: Vec<[f64; 2]> = d
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                if let Some(p) = sim.and_then(|s| s.p.get(i)) {
                    return [p.x, p.y];
                }
                let u = if r.ok {
                    r.displacement.get(i).copied().unwrap_or([0.0; 2])
                } else {
                    [0.0; 2]
                };
                [n.x as f64 + u[0] * ex, n.y as f64 + u[1] * ex]
            })
            .collect();
        let pos = |id: &str| d.joint_index(id).and_then(|i| positions.get(i).copied());

        // members still standing
        let alive: Vec<usize> = (0..d.members.len())
            .filter(|&i| {
                let id = &d.members[i].id;
                !self.broken().contains(id) && sim.is_none_or(|s| s.holds(id))
            })
            .collect();
        let members: Vec<MemberView> = alive
            .iter()
            .filter_map(|&i| {
                let m = &d.members[i];
                let band = match r.members.get(i).and_then(|q| q.as_ref()) {
                    Some(q) if !in_sim && r.ok => Band::of(q),
                    _ => Band::None,
                };
                Some(MemberView {
                    index: i,
                    id: m.id.clone(),
                    a: pos(&m.a)?,
                    b: pos(&m.b)?,
                    size: m.size,
                    band,
                })
            })
            .collect();

        let supports = d
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                Some(SupportView {
                    joint: i,
                    at: [n.x as f64, n.y as f64],
                    kind: n.sup?,
                })
            })
            .collect();

        let loads = d
            .loads
            .iter()
            .filter_map(|(id, t)| {
                let i = d.joint_index(id)?;
                Some(LoadView {
                    joint: i,
                    at: positions[i],
                    tonnes: *t,
                    tag: format!("{} t", num(*t)),
                })
            })
            .collect();

        let sparks = sim.map(|s| s.sparks.clone()).unwrap_or_default();
        let bad = if !in_sim && !r.ok {
            r.bad.and_then(|i| positions.get(i).copied())
        } else {
            None
        };
        let (hover, ghost) = match self.hover() {
            Some((hx, hy)) if building && self.tool().places_joints() => {
                let h = [hx as f64, hy as f64];
                let g = if self.tool() == Tool::Member {
                    self.pending().and_then(pos).map(|p| (p, h))
                } else {
                    None
                };
                (Some(h), g)
            }
            _ => (None, None),
        };

        let sel_member = match self.selection() {
            Some(Selection::Member(id)) => Some(id.as_str()),
            _ => None,
        };
        let sel_joint = match self.selection() {
            Some(Selection::Joint(id)) => Some(id.as_str()),
            _ => None,
        };
        let nums = self.show_forces();
        let show_handles = (building
            && (matches!(self.tool(), Tool::Select | Tool::Erase) || nums))
            || (nums && !building && !in_sim);
        let handles = if !show_handles {
            Vec::new()
        } else {
            alive
                .iter()
                .filter_map(|&i| {
                    let m = &d.members[i];
                    let (a, b) = (pos(&m.a)?, pos(&m.b)?);
                    let q = r.members.get(i).and_then(|q| q.as_ref());
                    Some(HandleView {
                        member: i,
                        id: m.id.clone(),
                        at: [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0],
                        text: match q {
                            Some(q) if nums && r.ok => fmt_kn(q.force),
                            _ => String::new(),
                        },
                        numeric: nums && r.ok,
                        selected: sel_member == Some(m.id.as_str()),
                        over: q.is_some_and(|q| q.utilisation > 1.0),
                        label: match q {
                            Some(q) if r.ok => format!(
                                "Member {}, {}% of its limit",
                                m.id,
                                round_str(q.utilisation * 100.0)
                            ),
                            _ => format!("Member {}", m.id),
                        },
                    })
                })
                .collect()
        };

        let joints = if in_sim {
            Vec::new()
        } else {
            d.nodes
                .iter()
                .enumerate()
                .map(|(i, n)| {
                    let load = d.load(&n.id);
                    let mut label = format!("Joint {}", n.id);
                    if let Some(s) = n.sup {
                        label.push_str(&format!(", {} support", s.word()));
                    }
                    if load != 0.0 {
                        label.push_str(&format!(", carrying {} t", num(load)));
                    }
                    JointView {
                        index: i,
                        id: n.id.clone(),
                        at: positions[i],
                        locked: n.locked,
                        pending: self.pending() == Some(n.id.as_str()),
                        selected: sel_joint == Some(n.id.as_str()),
                        bad: !r.ok && r.bad == Some(i),
                        label,
                    }
                })
                .collect()
        };

        let status = status_of(r);
        let explain = self.explain(r);

        // inspector
        let inspector = self.inspector(r, sel_member, sel_joint);

        let first = self.log().iter().find(|l| l.member.is_some());
        let over_budget = level.budget.is_some_and(|b| r.cost > b);
        let result = match phase {
            Phase::Held => Some(ResultView {
                title: if over_budget {
                    "It held, but over budget".into()
                } else {
                    "It held!".into()
                },
                held: true,
                stars: if level.budget.is_some() {
                    stars(self.stars())
                } else {
                    String::new()
                },
                replay: false,
            }),
            Phase::Collapsed => Some(ResultView {
                title: match first {
                    Some(f) => format!(
                        "Collapse: {} {} at {}%",
                        f.member.as_deref().unwrap_or(""),
                        f.mode.map(FailureMode::past_tense).unwrap_or(""),
                        round_str(f.lam * 100.0)
                    ),
                    None => "Collapse: it folded".into(),
                },
                held: false,
                stars: String::new(),
                replay: true,
            }),
            _ => None,
        };
        let lam_pct = round_str(self.lam() * 100.0);
        let loading = (phase == Phase::Testing).then(|| Loading {
            frac: lam_pct.parse::<f64>().unwrap_or(0.0).clamp(0.0, 100.0) / 100.0,
            text: format!("{lam_pct}%"),
        });

        let cost_text = format!(
            "${}k{}",
            to_fixed(r.cost, 1),
            level
                .budget
                .map(|b| format!(" / ${}k", num(b)))
                .unwrap_or_default()
        );

        let levels = LevelKey::ALL
            .into_iter()
            .map(|k| LevelRow {
                key: k,
                name: k.level().name,
                best: if k.level().budget.is_some() {
                    stars(self.best_for(k))
                } else {
                    String::new()
                },
                on: k == self.level(),
            })
            .collect();

        View {
            in_sim,
            stage_label: format!(
                "Structure with {} joints and {} members. {}.",
                d.nodes.len(),
                d.members.len(),
                status.label
            ),
            positions,
            members,
            supports,
            loads,
            sparks,
            bad,
            hover,
            ghost,
            handles,
            joints,
            water_flowing: self.animating(),
            crosshair: building && self.tool().places_joints(),
            surface_label: if self.tool().places_joints() {
                "Building grid: click to place a joint"
            } else {
                "Building grid"
            },
            legend: [
                "Stretched (tension)",
                "Squeezed (compression)",
                "Will break",
                "brighter = closer to its limit",
            ],
            toast: self.toast().map(str::to_string),
            loading,
            result,
            hint: match phase {
                Phase::Build => self.tool().hint(self.pending().is_some()),
                Phase::Testing => "Testing… building is paused.",
                _ => "Building is paused while you look at the result.",
            },
            show_sizes: self.tool() == Tool::Member,
            show_weights: self.tool() == Tool::Weight,
            has_pending: self.pending().is_some() && building,
            levels,
            goal: level.goal,
            status,
            cost_text,
            over_budget,
            test_label: if building {
                "Test it: add the load"
            } else {
                "Back to building"
            },
            inspector,
            explain,
            playing: self.playing(),
            slug: self.level().slug(),
            clock: self.clock(),
        }
    }

    /// The header clock: `building`, `load 63%`, `held`, `collapse`.
    pub fn clock(&self) -> String {
        match self.phase() {
            Phase::Build => "building".into(),
            Phase::Testing => format!("load {}%", round_str(self.lam() * 100.0)),
            Phase::Held => "held".into(),
            _ => "collapse".into(),
        }
    }

    /// A one-line summary (MCP `describe_screen`): level, counts, status, cost, phase.
    pub fn summary(&self) -> String {
        let r = self.analysis();
        let level = self.level().level();
        format!(
            "Bridge · {} · {} joints, {} members · {} · ${}k{} · {}",
            level.name,
            self.design().nodes.len(),
            self.design().members.len(),
            status_of(r).label,
            to_fixed(r.cost, 1),
            level
                .budget
                .map(|b| format!(" / ${}k", num(b)))
                .unwrap_or_default(),
            self.clock()
        )
    }

    fn inspector(
        &self,
        r: &Analysis,
        sel_member: Option<&str>,
        sel_joint: Option<&str>,
    ) -> Inspector {
        let d = self.design();
        if let Some(i) = sel_member.and_then(|id| d.member_index(id)) {
            let m = &d.members[i];
            let q = r.members.get(i).and_then(|q| q.as_ref());
            let solved = q.filter(|_| r.ok);
            return Inspector::Member(MemberInspector {
                id: m.id.clone(),
                title: format!("{} · {}–{}", m.id, m.a, m.b),
                length: q
                    .map(|q| format!("{} m", to_fixed(q.length, 1)))
                    .unwrap_or_default(),
                fill: solved
                    .map(|q| {
                        to_fixed((q.utilisation * 100.0).min(100.0), 0)
                            .parse::<f64>()
                            .unwrap_or(0.0)
                            .clamp(0.0, 100.0)
                            / 100.0
                    })
                    .unwrap_or(0.0),
                ut: solved
                    .map(|q| format!("{}%", round_str(q.utilisation * 100.0)))
                    .unwrap_or_else(|| "—".into()),
                tone: match solved {
                    None => BarTone::Zero,
                    Some(q) if q.utilisation > 1.0 => BarTone::Over,
                    Some(q) if q.force >= 0.0 => BarTone::Tension,
                    Some(_) => BarTone::Compression,
                },
                text: match solved {
                    None => "No forces until the structure is stable.".into(),
                    Some(q) if q.force.abs() < ZERO_FORCE_N => {
                        "Carrying almost nothing. A candidate to delete.".into()
                    }
                    Some(q) if q.force >= 0.0 => format!(
                        "Stretched with {} kN. It will snap at {} kN.",
                        to_fixed(q.force / 1000.0, 0),
                        to_fixed(q.limit / 1000.0, 0)
                    ),
                    Some(q) => format!(
                        "Squeezed with {} kN. It {} at {} kN.",
                        to_fixed(-q.force / 1000.0, 0),
                        if q.mode == FailureMode::Buckling {
                            "buckles"
                        } else {
                            "crushes"
                        },
                        to_fixed(q.limit / 1000.0, 0)
                    ),
                },
                size: m.size,
            });
        }
        if let Some(n) = sel_joint.and_then(|id| d.joint(id)) {
            let can = self.level().level().sup.allows(n.x, n.y);
            return Inspector::Joint(JointInspector {
                id: n.id.clone(),
                title: format!("Joint {}{}", n.id, if n.locked { " · level" } else { "" }),
                pos: format!("x {} m · y {} m", n.x, n.y),
                support: n.sup,
                can_support: can,
                locked: n.locked,
                load: format!("{} t", num(d.load(&n.id))),
                note: match n.sup {
                    Some(Support::Pin) => "Pinned: held in place, free to rotate.",
                    Some(Support::Roller) => {
                        "Roller: held up, free to slide sideways, so the structure can expand."
                    }
                    None if can => "On the ground: you can make this a support.",
                    None => "In the air. Supports must sit on the ground.",
                },
                can_delete: !n.locked,
            });
        }
        Inspector::Nothing(BuiltFrom {
            material: self.material(),
            joints: d.nodes.len().to_string(),
            members: d.members.len().to_string(),
            weight: format!("{} t", to_fixed(r.mass / 1000.0, 1)),
            can_starter: self.can_starter(),
        })
    }

    /// The "What's happening" sentence (`explain`).
    fn explain(&self, r: &Analysis) -> String {
        let d = self.design();
        let level = self.level().level();
        let log = self.log();
        let first = log.iter().find(|l| l.member.is_some());
        match self.phase() {
            Phase::Collapse | Phase::Collapsed => {
                if let Some(f) = first {
                    let mode = f.mode.unwrap_or(FailureMode::Yield);
                    format!(
                        "{} {} at {}% of the load. {}{}",
                        f.member.as_deref().unwrap_or(""),
                        mode.past_tense(),
                        round_str(f.lam * 100.0),
                        mode_lesson(mode),
                        if log.len() > 1 {
                            " Once it went, the load jumped onto its neighbours and the failure spread."
                        } else {
                            ""
                        }
                    )
                } else if let Some(l) = log.first().filter(|l| l.loose) {
                    format!(
                        "The weight on {} was not hanging on anything, so it simply fell. Join that joint to the structure with members.",
                        l.bad.as_deref().unwrap_or("")
                    )
                } else {
                    format!(
                        "It was never stable: joint {} could swing freely, so as soon as weight arrived the shape folded. Close every open four-sided panel with a diagonal.",
                        log.first().and_then(|l| l.bad.as_deref()).unwrap_or("")
                    )
                }
            }
            Phase::Held => {
                let mut s = String::from("It carried the full load");
                if !self.broken().is_empty() {
                    s.push_str(&format!(
                        ", even after losing {} along the way: that is redundancy at work",
                        self.broken().join(", ")
                    ));
                }
                s.push_str(". ");
                s.push_str(match level.budget {
                    Some(b) if r.cost <= b => "Under budget. Can you do it cheaper for more stars?",
                    Some(_) => {
                        "But it is over budget, so no stars. Trim members or go thinner where forces are small."
                    }
                    None => "Now add more weight and find where it gives.",
                });
                s
            }
            Phase::Testing => {
                "Loading it up gradually. Watch the colours brighten as members approach their limits."
                    .into()
            }
            Phase::Build => {
                if r.n_act == 0 {
                    return "Pick Member and click from joint to joint, or click empty space to make new joints as you go. Triangles are the secret: a triangle cannot change shape without a member stretching or squeezing.".into();
                }
                if !r.ok {
                    let bad = r
                        .bad
                        .and_then(|i| d.nodes.get(i))
                        .map(|n| n.id.as_str())
                        .unwrap_or("");
                    return format!(
                        "Joint {bad} can still move without stretching any member, so this would fold under load. Add a diagonal to make a triangle, or a support."
                    );
                }
                let locked_loads_ok = level.loads.iter().all(|(i, _)| {
                    let id = format!("J{}", i + 1);
                    d.joint_index(&id)
                        .is_some_and(|j| r.used.get(j).copied().unwrap_or(false))
                        && d.has_member_at(&id)
                });
                if !locked_loads_ok {
                    return "Connect the level’s loaded joint to your structure so the weight has a path to the banks.".into();
                }
                match r.worst_state() {
                    Some((w, q)) => format!(
                        "Busiest member: {}, {} with {} kN, {}% of its limit{}. Press Test to load it for real.",
                        d.members[w].id,
                        if q.force >= 0.0 { "stretched" } else { "squeezed" },
                        to_fixed((q.force / 1000.0).abs(), 0),
                        round_str(q.utilisation * 100.0),
                        if q.force < 0.0 && q.mode == FailureMode::Buckling {
                            " (buckling)"
                        } else {
                            ""
                        }
                    ),
                    None => String::new(),
                }
            }
        }
    }
}
