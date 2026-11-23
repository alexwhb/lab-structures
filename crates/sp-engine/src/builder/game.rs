//! The builder's state machine (`class Component` without the drawing): the level, the
//! conditions, the camera, the design and its 40-step undo, the tools and every editing rule
//! with its toast, the live preview (cached by design), the run (`startRun` / `tick` /
//! `stopRun`), results, stars and best stars.
//!
//! Undo (as the v1 Bridge, lead decision): an edit leaves an undo step only when it changed the
//! design (the mockup pushes first and checks later, so a refused edit there leaves a no-op
//! step; the golden harness drops those the same way).

use super::camera::{self, Camera, ViewState, WHEEL_STEP, ZOOM_STEP};
use super::data::{Foundation, LevelKey, PieceType, RunKind, Scene, Soil, TrafficSet, stars_for};
use super::design::{Design, Joint, Piece};
use super::preview::{Settled, preview};
use super::setup::{Conditions, Setup, practice, spec};
use super::sim::{FirstBreak, FirstFound, Sim};
use crate::truss::hypot;
use std::collections::BTreeMap;

/// Undo depth.
pub const UNDO_DEPTH: usize = 40;
/// Toasts fade after this long, s.
pub const TOAST_SECONDS: f64 = 2.6;
/// A demo board (`presetDef().loop`) restarts its run this many frames after the result.
pub const DEMO_LOOP_FRAMES: u32 = 110;

/// A rail tool (`toolDefs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    /// Select pieces, drag joints, drag empty space to pan.
    Select,
    /// Build pieces joint to joint.
    Build,
    /// Put foundations in the ground (`anchor`).
    Found,
    /// Hang a crate on a joint (`weight`).
    Crate,
    /// Delete joints and pieces.
    Erase,
}

impl Tool {
    /// In the rail's order.
    pub const ALL: [Tool; 5] = [
        Tool::Select,
        Tool::Build,
        Tool::Found,
        Tool::Crate,
        Tool::Erase,
    ];
    /// The mockup's key: `select`, `build`, `anchor`, `weight`, `erase`.
    pub fn key(self) -> &'static str {
        match self {
            Tool::Select => "select",
            Tool::Build => "build",
            Tool::Found => "anchor",
            Tool::Crate => "weight",
            Tool::Erase => "erase",
        }
    }
    /// From the key.
    pub fn from_key(k: &str) -> Option<Tool> {
        Tool::ALL.into_iter().find(|t| t.key() == k)
    }
    /// "Select", "Build", "Found", "Crate", "Erase".
    pub fn name(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Build => "Build",
            Tool::Found => "Found",
            Tool::Crate => "Crate",
            Tool::Erase => "Erase",
        }
    }
    /// The tooltip.
    pub fn tip(self) -> &'static str {
        match self {
            Tool::Select => "Select pieces, drag joints, drag empty space to pan",
            Tool::Build => "Build pieces joint to joint",
            Tool::Found => "Put foundations in the ground",
            Tool::Crate => "Hang a crate on a joint",
            Tool::Erase => "Delete joints and pieces",
        }
    }
    /// The icon (an SVG path in a 22 × 22 box, stroked 1.6).
    pub fn icon(self) -> &'static str {
        match self {
            Tool::Select => "M5 3l12 7-5 1.5L9.5 17z",
            Tool::Build => "M4 18L18 4M4 18a1.6 1.6 0 1 0 0.01 0M18 4a1.6 1.6 0 1 0 0.01 0",
            Tool::Found => "M11 5L4 13H18Z M8 13V20M14 13V20M3 13H19",
            Tool::Crate => "M5 9h12v9H5ZM5 9l2-3h12l-2 3M17 18l2-3V6M11 2v4",
            Tool::Erase => "M6 6l10 10M16 6L6 16",
        }
    }
    /// The hint bar sentence while building (`hints`).
    pub fn hint(self, pending: bool) -> &'static str {
        match self {
            Tool::Select => {
                "Click a piece or joint to inspect it. Drag joints to reshape, drag empty space to pan."
            }
            Tool::Build if pending => {
                "Click the next joint, or empty grid to place one. Click the glowing joint to stop."
            }
            Tool::Build => "Click a joint or the grid to start. Zoom in for a finer grid.",
            Tool::Found => {
                "Click the ground to place a foundation. Footings are cheap; piles grip deeper and resist pulling."
            }
            Tool::Crate => {
                "Click a joint to hang a crate. A crate on a rope near the top of a tower acts as a damper."
            }
            Tool::Erase => "Click a joint or a piece\u{2019}s dot to delete it.",
        }
    }
}

/// Building, running, or showing the result of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Building (the live preview shows the settled forces).
    Build,
    /// Running.
    Run,
    /// The run is over; the stage shows its last frame and the result.
    Result,
}

impl Phase {
    /// `build`, `run`, `result`.
    pub fn key(self) -> &'static str {
        match self {
            Phase::Build => "build",
            Phase::Run => "run",
            Phase::Result => "result",
        }
    }
}

/// What is selected (`state.sel`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Selection {
    /// A joint.
    Node(String),
    /// A piece.
    Member(String),
}

/// How a run went, per scene.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// River: vehicles across of the total.
    River {
        /// Across.
        crossed: usize,
        /// Vehicles.
        total: usize,
    },
    /// Open ground.
    Tower {
        /// The top floor stayed above 80 % of its height.
        standing: bool,
        /// The top floor reached the target (or there is none).
        reached: bool,
        /// Peak top-floor acceleration, milli-g.
        mg: f64,
        /// Within the comfort limit (or there is none).
        comfy: bool,
        /// The top floor's design height, m.
        height: f64,
        /// Peak sway, m.
        sway: f64,
    },
}

/// The Leaning tower's 30-year result (SPEC TIME-3; filled by `longterm`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Years {
    /// Tilt after 30 years, degrees.
    pub tilt: f64,
    /// Largest settlement, m.
    pub settled: f64,
}

/// The result of a run (`state.result`).
#[derive(Debug, Clone, PartialEq)]
pub struct RunResult {
    /// Passed.
    pub pass: bool,
    /// Stars (0 for a practice run, over budget, a fail or the Sandbox).
    pub stars: u8,
    /// Cost, $k.
    pub cost: f64,
    /// A practice run.
    pub practice: bool,
    /// The first piece that broke.
    pub first: Option<FirstBreak>,
    /// The first foundation that failed.
    pub first_found: Option<FirstFound>,
    /// Per scene.
    pub outcome: Outcome,
    /// A years run (Leaning tower): its tilt and settlement (`outcome` then holds the tower's
    /// standing / height).
    pub years: Option<Years>,
}

impl RunResult {
    /// A years run's result (SPEC TIME-3, for `longterm`): `outcome` is a tower that reached
    /// its height, `standing` as given.
    #[allow(clippy::too_many_arguments)]
    pub fn of_years(
        pass: bool,
        stars: u8,
        cost: f64,
        practice: bool,
        standing: bool,
        height: f64,
        years: Years,
        first: Option<FirstBreak>,
    ) -> RunResult {
        RunResult {
            pass,
            stars,
            cost,
            practice,
            first,
            first_found: None,
            outcome: Outcome::Tower {
                standing,
                reached: true,
                mg: 0.0,
                comfy: true,
                height,
                sway: 0.0,
            },
            years: Some(years),
        }
    }
}

/// The result of a finished simulation (`tick()`'s result block).
pub fn result_of(sim: &Sim, cost: f64, practice: bool) -> RunResult {
    let setup = &sim.setup;
    let (pass, outcome) = if setup.scene == Scene::River {
        let crossed = sim.crossed();
        let total = sim.v.len();
        (
            crossed == total && total > 0,
            Outcome::River { crossed, total },
        )
    } else {
        let top = sim.top_i.map(|i| &sim.p[i]);
        let standing = top.is_some_and(|p| p.y > sim.top_y0 * 0.8);
        let reached = setup.target.is_none_or(|t| sim.top_y0 >= t - 0.01);
        let mg = sim.milli_g();
        let comfy = setup.comfort.is_none_or(|c| mg <= c);
        (
            standing && reached && comfy && sim.top_i.is_some(),
            Outcome::Tower {
                standing,
                reached,
                mg,
                comfy,
                height: sim.top_y0,
                sway: sim.sway,
            },
        )
    };
    let stars = match setup.budget {
        Some(b) if pass && !practice && b != 0.0 => stars_for(cost, b),
        _ => 0,
    };
    RunResult {
        pass,
        stars,
        cost,
        practice,
        first: sim.first_break.clone(),
        first_found: sim.first_found.clone(),
        outcome,
        years: None,
    }
}

/// Runs a design headless to the end (open ground: 18 s; river: until the traffic is done or
/// 45 s), for MCP and the fold-ins' comparisons. `tweak` may set the sim's hooks (drag,
/// shedding, …) before the first frame. Returns the result and the final simulation; the
/// Sandbox (which never ends) and runs past `max_frames` give `None`.
pub fn run_headless(
    design: &Design,
    setup: &Setup,
    cond: &Conditions,
    practice: bool,
    max_frames: usize,
    tweak: impl FnOnce(&mut Sim),
) -> (Option<RunResult>, Sim) {
    let mut sim = Sim::new(design, setup, cond, true);
    tweak(&mut sim);
    for _ in 0..max_frames {
        sim.frame(false);
        if sim.check_done() {
            let r = result_of(&sim, design.cost(), practice);
            return (Some(r), sim);
        }
    }
    (None, sim)
}

/// A preset build (`buildPreset`; SPEC MB-10: for tests and demos).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PresetBuild {
    /// A Road deck J1–J2 in 8 m panels with a 6 m-deep truss over it in `chord`.
    Truss {
        /// The truss pieces' type.
        chord: PieceType,
    },
    /// A cable-stayed deck: piles at 0 and 56 m, Steel pylons over the banks, six Cables.
    Cable,
    /// A tower of `h` m (4 m storeys) and `bays` 8 m bays centred on x = 100, columns `col`,
    /// one diagonal `brace` per bay and storey, a Floor across every level, every base joint
    /// on a `found` (the level's anchors are removed).
    Tower {
        /// Height, m.
        h: f64,
        /// Bays.
        bays: u32,
        /// Columns.
        col: PieceType,
        /// Diagonals.
        brace: PieceType,
        /// Base foundations.
        found: Foundation,
    },
}

/// A board's starting state (`presetDef()`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Preset {
    /// The level (`river` when `None`).
    pub level: Option<LevelKey>,
    /// The Sandbox scene.
    pub scene: Option<Scene>,
    /// Condition overrides.
    pub wind: Option<f64>,
    /// Condition overrides.
    pub snow: Option<f64>,
    /// Condition overrides.
    pub quake: Option<f64>,
    /// Condition overrides.
    pub traffic: Option<TrafficSet>,
    /// Condition overrides.
    pub soil: Option<Soil>,
    /// A preset build.
    pub build: Option<PresetBuild>,
    /// The tool.
    pub tool: Option<Tool>,
    /// "Show forces" on.
    pub nums: bool,
    /// Demo: the board starts its run on its own (MB-10: not the app's behaviour).
    pub run: bool,
    /// Demo: the board restarts its run 110 frames after each result.
    pub demo_loop: bool,
}

impl Preset {
    /// The seven v2 boards' presets by board name (`BridgeBuild`, `BridgeStorm`,
    /// `BridgeCollapse`, `BridgeCable`, `TowerBuild`, `TowerStorm`, `TowerQuake`).
    pub fn board(name: &str) -> Option<Preset> {
        use Foundation::*;
        use PieceType::*;
        let tower = |h: f64, bays: u32, found: Foundation| PresetBuild::Tower {
            h,
            bays,
            col: Steel,
            brace: Wood,
            found,
        };
        Some(match name {
            "BridgeBuild" => Preset::default(),
            "BridgeStorm" => Preset {
                level: Some(LevelKey::River),
                build: Some(PresetBuild::Truss { chord: Wood }),
                wind: Some(35.0),
                snow: Some(150.0),
                traffic: Some(TrafficSet::Rush),
                run: true,
                demo_loop: true,
                ..Preset::default()
            },
            "BridgeCollapse" => Preset {
                level: Some(LevelKey::Haul),
                build: Some(PresetBuild::Truss { chord: Wood }),
                run: true,
                demo_loop: true,
                ..Preset::default()
            },
            "BridgeCable" => Preset {
                level: Some(LevelKey::Sandbox),
                scene: Some(Scene::River),
                build: Some(PresetBuild::Cable),
                run: true,
                ..Preset::default()
            },
            "TowerBuild" => Preset {
                level: Some(LevelKey::T2),
                build: Some(tower(40.0, 4, Pile)),
                tool: Some(Tool::Select),
                nums: true,
                ..Preset::default()
            },
            "TowerStorm" => Preset {
                level: Some(LevelKey::T2),
                build: Some(tower(64.0, 4, Pile)),
                run: true,
                demo_loop: true,
                ..Preset::default()
            },
            "TowerQuake" => Preset {
                level: Some(LevelKey::T3),
                build: Some(tower(32.0, 2, Footing)),
                run: true,
                demo_loop: true,
                ..Preset::default()
            },
            _ => return None,
        })
    }
    /// The seven board names.
    pub const BOARDS: [&'static str; 7] = [
        "BridgeBuild",
        "BridgeStorm",
        "BridgeCollapse",
        "BridgeCable",
        "TowerBuild",
        "TowerStorm",
        "TowerQuake",
    ];
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Pan {
    px: f64,
    py: f64,
    cx: f64,
    cy: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct PreviewKey {
    design: Design,
    level: LevelKey,
    sb_scene: Scene,
    soil: Soil,
    snow: f64,
}

/// The builder game.
#[derive(Debug, Clone)]
pub struct Game {
    level: LevelKey,
    sb_scene: Scene,
    cond: Conditions,
    view: ViewState,
    design: Design,
    pending: Option<String>,
    sel: Option<Selection>,
    phase: Phase,
    hover: Option<[f64; 2]>,
    drag: Option<String>,
    result: Option<RunResult>,
    kind: PieceType,
    tool: Tool,
    found: Foundation,
    weight: f64,
    nums: bool,
    best: BTreeMap<LevelKey, u8>,
    toast: Option<String>,
    toast_age: f64,
    slow: bool,
    pan: Option<Pan>,
    pan_moved: bool,
    hist: Vec<Design>,
    sim: Option<Sim>,
    pv: Option<(PreviewKey, Settled)>,
    demo_loop: bool,
    after: u32,
    revision: u64,
}

impl Default for Game {
    fn default() -> Game {
        Game::new()
    }
}

impl Game {
    /// A fresh game: level 1 (First crossing), the Build tool, Road, building (SPEC MB-10).
    pub fn new() -> Game {
        Game::with_preset(&Preset::default())
    }

    /// A board's starting state (`constructor` with `presetDef()`); the demo `run` flag is
    /// not acted on here — call [`Game::start_run`] (the board starts 0.7 s in).
    pub fn with_preset(p: &Preset) -> Game {
        let lvl = p.level.unwrap_or(LevelKey::River);
        let sc = p.scene.unwrap_or(Scene::River);
        let mut g = Game {
            level: lvl,
            sb_scene: sc,
            cond: Conditions::for_level(&lvl.level()),
            view: ViewState::default_for(Scene::River),
            design: Design::default(),
            pending: None,
            sel: None,
            phase: Phase::Build,
            hover: None,
            drag: None,
            result: None,
            kind: PieceType::Road,
            tool: Tool::Build,
            found: Foundation::Footing,
            weight: 5.0,
            nums: false,
            best: BTreeMap::new(),
            toast: None,
            toast_age: 0.0,
            slow: false,
            pan: None,
            pan_moved: false,
            hist: vec![],
            sim: None,
            pv: None,
            demo_loop: p.demo_loop,
            after: 0,
            revision: 0,
        };
        g.level_state(lvl, Some(sc));
        if let Some(w) = p.wind {
            g.cond.wind = w;
        }
        if let Some(s) = p.snow {
            g.cond.snow = s;
        }
        if let Some(q) = p.quake {
            g.cond.quake = q;
        }
        if let Some(t) = p.traffic {
            g.cond.traffic = t;
        }
        if let Some(s) = p.soil {
            g.cond.soil = s;
        }
        if let Some(b) = p.build {
            g.build_preset(b);
        }
        if let Some(t) = p.tool {
            g.tool = t;
        }
        if p.nums {
            g.nums = true;
        }
        g.refresh();
        g
    }

    // ---------- reading ----------

    /// The level.
    pub fn level(&self) -> LevelKey {
        self.level
    }
    /// The Sandbox scene chip (kept while on other levels).
    pub fn sb_scene(&self) -> Scene {
        self.sb_scene
    }
    /// The scene now.
    pub fn scene(&self) -> Scene {
        spec(self.level, self.sb_scene)
            .scene
            .unwrap_or(self.sb_scene)
    }
    /// The conditions.
    pub fn cond(&self) -> &Conditions {
        &self.cond
    }
    /// The level as it runs with the conditions (`L()`).
    pub fn setup(&self) -> Setup {
        Setup::new(self.level, self.sb_scene, &self.cond)
    }
    /// The camera's view.
    pub fn view_state(&self) -> ViewState {
        self.view
    }
    /// The camera.
    pub fn camera(&self) -> Camera {
        Camera::new(self.scene(), self.view)
    }
    /// The design.
    pub fn design(&self) -> &Design {
        &self.design
    }
    /// The joint a member chain starts from (Build tool).
    pub fn pending(&self) -> Option<&str> {
        self.pending.as_deref()
    }
    /// The selection.
    pub fn selection(&self) -> Option<&Selection> {
        self.sel.as_ref()
    }
    /// The phase.
    pub fn phase(&self) -> Phase {
        self.phase
    }
    /// Building (`building()`): editing is allowed.
    pub fn building(&self) -> bool {
        self.phase == Phase::Build
    }
    /// The stage shows a simulation (running or its result: `live`).
    pub fn live(&self) -> bool {
        matches!(self.phase, Phase::Run | Phase::Result) && self.sim.is_some()
    }
    /// The hovered grid point (Build / Found tools), m.
    pub fn hover(&self) -> Option<[f64; 2]> {
        self.hover
    }
    /// The joint being dragged.
    pub fn dragging(&self) -> Option<&str> {
        self.drag.as_deref()
    }
    /// The last run's result.
    pub fn result(&self) -> Option<&RunResult> {
        self.result.as_ref()
    }
    /// The piece type the Build tool places.
    pub fn piece_type(&self) -> PieceType {
        self.kind
    }
    /// The tool.
    pub fn tool(&self) -> Tool {
        self.tool
    }
    /// The foundation kind the Found tool places.
    pub fn found_kind(&self) -> Foundation {
        self.found
    }
    /// The crate step, t.
    pub fn weight(&self) -> f64 {
        self.weight
    }
    /// "Show forces".
    pub fn show_forces(&self) -> bool {
        self.nums
    }
    /// Slow motion.
    pub fn slow(&self) -> bool {
        self.slow
    }
    /// Best stars of a level.
    pub fn best_for(&self, k: LevelKey) -> u8 {
        self.best.get(&k).copied().unwrap_or(0)
    }
    /// Best stars of every level that has any.
    pub fn best(&self) -> &BTreeMap<LevelKey, u8> {
        &self.best
    }
    /// The toast showing, if any.
    pub fn toast(&self) -> Option<&str> {
        self.toast.as_deref()
    }
    /// The simulation (running, or the finished run on the result).
    pub fn sim(&self) -> Option<&Sim> {
        self.sim.as_ref()
    }
    /// The simulation, mutably (fold-ins' hooks; the Game copies the conditions in each frame).
    pub fn sim_mut(&mut self) -> Option<&mut Sim> {
        self.sim.as_mut()
    }
    /// The live preview of the design (`preview(st)`); `None` while running.
    pub fn preview(&self) -> Option<&Settled> {
        if self.live() {
            None
        } else {
            self.pv.as_ref().map(|(_, s)| s)
        }
    }
    /// Undo steps available.
    pub fn undo_depth(&self) -> usize {
        self.hist.len()
    }
    /// Bumped on every change of the design (for caches).
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// The cost, $k.
    pub fn cost(&self) -> f64 {
        self.design.cost()
    }
    /// A practice run (`practice()`).
    pub fn practice(&self) -> bool {
        practice(self.level, self.sb_scene, &self.cond)
    }
    /// Something is moving (a run): the screen should repaint every frame.
    pub fn animating(&self) -> bool {
        self.phase == Phase::Run || (self.phase == Phase::Result && self.demo_loop)
    }

    // ---------- toasts, time ----------

    /// Shows a toast (`say`).
    pub fn say(&mut self, text: impl Into<String>) {
        self.toast = Some(text.into());
        self.toast_age = 0.0;
    }
    /// Wall-clock time passes (toasts fade after 2.6 s). Returns whether something changed.
    pub fn advance(&mut self, dt: f64) -> bool {
        if self.toast.is_some() {
            self.toast_age += if dt.is_finite() { dt.max(0.0) } else { 0.0 };
            if self.toast_age > TOAST_SECONDS {
                self.toast = None;
                return true;
            }
        }
        false
    }
    /// Seconds until the toast fades (for repaint scheduling).
    pub fn toast_remaining(&self) -> Option<f64> {
        self.toast
            .as_ref()
            .map(|_| (TOAST_SECONDS - self.toast_age).max(0.0))
    }

    // ---------- levels, scene ----------

    fn level_state(&mut self, key: LevelKey, sb: Option<Scene>) {
        let sb = sb.unwrap_or(self.sb_scene);
        let sp = spec(key, sb);
        let scene = sp.scene.unwrap_or(sb);
        self.level = key;
        self.sb_scene = sb;
        self.cond = Conditions::for_level(&sp);
        self.view = ViewState::default_for(scene);
        self.design = match &sp.prebuilt {
            Some(d) => d.clone(),
            None => {
                let nodes: Vec<Joint> = sp
                    .anchors
                    .iter()
                    .enumerate()
                    .map(|(i, a)| Joint {
                        id: format!("J{}", i + 1),
                        x: a.x,
                        y: a.y,
                        sup: Some(a.found),
                        locked: scene != Scene::Flat,
                    })
                    .collect();
                Design {
                    n_n: nodes.len() as u32,
                    nodes,
                    ..Design::default()
                }
            }
        };
        self.pending = None;
        self.sel = None;
        self.phase = Phase::Build;
        self.hover = None;
        self.drag = None;
        self.result = None;
        self.kind = if scene == Scene::Flat {
            PieceType::Floor
        } else {
            PieceType::Road
        };
        self.revision += 1;
    }

    /// Picks a level (resets to its anchors and conditions; clears undo; stops a run).
    pub fn pick_level(&mut self, k: LevelKey) {
        self.sim = None;
        self.hist.clear();
        self.level_state(k, None);
        self.refresh();
    }
    /// The Sandbox scene chips: switching resets the Sandbox to that scene.
    pub fn pick_scene(&mut self, scene: Scene) {
        if self.sb_scene != scene {
            self.sim = None;
            self.hist.clear();
            self.level_state(LevelKey::Sandbox, Some(scene));
            self.refresh();
        }
    }
    /// Clear / Reset: the level again from the start.
    pub fn clear(&mut self) {
        self.sim = None;
        self.hist.clear();
        self.level_state(self.level, None);
        self.refresh();
    }
    /// Restores saved best stars (the screen persists them).
    pub fn set_best(&mut self, k: LevelKey, stars: u8) {
        if stars == 0 {
            self.best.remove(&k);
        } else {
            self.best.insert(k, stars.min(3));
        }
    }
    /// Loads a design into the Sandbox (⌘V, "Open in Playground"): the scene, then the design
    /// as given (no undo step).
    pub fn open_in_sandbox(&mut self, scene: Scene, design: Design) {
        self.sim = None;
        self.hist.clear();
        self.level_state(LevelKey::Sandbox, Some(scene));
        self.design = design;
        self.revision += 1;
        self.refresh();
    }

    /// Puts a design on the current level as it is (MCP `builder_test`): building, the level
    /// and conditions kept, undo and any run cleared.
    pub fn load_design(&mut self, design: Design) {
        self.sim = None;
        self.hist.clear();
        self.design = design;
        self.phase = Phase::Build;
        self.pending = None;
        self.sel = None;
        self.hover = None;
        self.drag = None;
        self.result = None;
        self.revision += 1;
        self.refresh();
    }

    /// The level's own crate on a joint, t (a pre-built level's crates can't go lower).
    pub fn level_crate(&self, id: &str) -> f64 {
        spec(self.level, self.sb_scene)
            .prebuilt
            .map_or(0.0, |d| d.load(id))
    }

    /// The Leaning tower's run starts (SPEC TIME-3, driven by `longterm`): running with no
    /// simulation — editing paused, the selection and chain cleared.
    pub fn start_years(&mut self) {
        self.sim = None;
        self.phase = Phase::Run;
        self.sel = None;
        self.pending = None;
        self.hover = None;
        self.result = None;
        self.drag = None;
    }

    /// The Leaning tower's run is over: the result shows and best stars update (as `tick`).
    pub fn finish_years(&mut self, r: RunResult) {
        if r.stars > self.best_for(self.level) {
            self.best.insert(self.level, r.stars.min(3));
        }
        self.result = Some(r);
        self.phase = Phase::Result;
    }

    // ---------- tools and chips ----------

    /// Picks a rail tool (ends a member chain).
    pub fn pick_tool(&mut self, t: Tool) {
        self.tool = t;
        self.pending = None;
        self.hover = None;
    }
    /// Picks the piece type to build.
    pub fn pick_type(&mut self, k: PieceType) {
        self.kind = k;
    }
    /// Picks the foundation kind to place.
    pub fn pick_found(&mut self, f: Foundation) {
        self.found = f;
    }
    /// Picks the crate step, t.
    pub fn pick_weight(&mut self, t: f64) {
        if t.is_finite() {
            self.weight = t;
        }
    }
    /// "Show forces".
    pub fn toggle_forces(&mut self) {
        self.nums = !self.nums;
    }
    /// "Finish line": ends the member chain.
    pub fn finish_line(&mut self) {
        self.pending = None;
    }

    // ---------- conditions ----------

    /// Wind, m/s (any time).
    pub fn set_wind(&mut self, v: f64) {
        if v.is_finite() {
            self.cond.wind = v;
        }
    }
    /// Wind direction: `1` from the left, `-1` from the right (any time).
    pub fn set_dir(&mut self, d: f64) {
        if d == 1.0 || d == -1.0 {
            self.cond.dir = d;
        }
    }
    /// Snow, kg/m (any time).
    pub fn set_snow(&mut self, v: f64) {
        if v.is_finite() {
            self.cond.snow = v;
            self.refresh();
        }
    }
    /// Earthquake, g (any time).
    pub fn set_quake(&mut self, v: f64) {
        if v.is_finite() {
            self.cond.quake = v;
        }
    }
    /// Fire, °C, 0 = off (any time; SPEC FIRE-1).
    pub fn set_fire(&mut self, v: f64) {
        if v.is_finite() {
            self.cond.fire = v;
        }
    }
    /// Soil (refused while running: "Soil can only change while building.").
    pub fn set_soil(&mut self, s: Soil) {
        if self.phase == Phase::Run {
            self.say("Soil can only change while building.");
            return;
        }
        self.cond.soil = s;
        self.refresh();
    }
    /// Traffic (refused while running: "Traffic changes on the next run.").
    pub fn set_traffic(&mut self, t: TrafficSet) {
        if self.phase == Phase::Run {
            self.say("Traffic changes on the next run.");
            return;
        }
        self.cond.traffic = t;
    }
    /// "Shake now" (while running): the quake envelope starts in 0.2 s.
    pub fn shake_now(&mut self) {
        if let Some(s) = &mut self.sim {
            s.quake_start = Some(s.t + 0.2);
        }
    }
    /// "Light it now" (while running; SPEC FIRE-1).
    pub fn light_fire_now(&mut self) {
        if let Some(s) = &mut self.sim {
            s.light_fire();
        }
    }

    // ---------- camera ----------

    /// Zooms by `f` about a stage point (the centre by default).
    pub fn zoom_by(&mut self, f: f64, px: Option<f64>, py: Option<f64>) {
        if !f.is_finite() || f <= 0.0 {
            return;
        }
        let v = self.camera().zoomed(f, px, py);
        if v.cx.is_finite() && v.cy.is_finite() && v.z.is_finite() {
            self.view = v;
        }
    }
    /// The + button (× 1.3).
    pub fn zoom_in(&mut self) {
        self.zoom_by(ZOOM_STEP, None, None);
    }
    /// The − button (÷ 1.3).
    pub fn zoom_out(&mut self) {
        self.zoom_by(1.0 / ZOOM_STEP, None, None);
    }
    /// The mouse wheel at a stage point: in (× 1.15) for `delta_y < 0`, else out.
    pub fn wheel(&mut self, delta_y: f64, px: f64, py: f64) {
        let f = if delta_y < 0.0 {
            WHEEL_STEP
        } else {
            1.0 / WHEEL_STEP
        };
        self.zoom_by(f, Some(px), Some(py));
    }
    /// "Fit".
    pub fn fit(&mut self) {
        let sp = spec(self.level, self.sb_scene);
        let scene = sp.scene.unwrap_or(self.sb_scene);
        let pts: Vec<[f64; 2]> = self.design.nodes.iter().map(|n| [n.x, n.y]).collect();
        let v = camera::fit(scene, &pts, sp.target);
        if v.cx.is_finite() && v.cy.is_finite() && v.z.is_finite() {
            self.view = v;
        }
    }
    /// Pans the view by stage pixels (arrow keys; a drag of empty space pans with the
    /// pointer).
    pub fn pan_by(&mut self, dx_px: f64, dy_px: f64) {
        if !(dx_px.is_finite() && dy_px.is_finite()) {
            return;
        }
        let sc = self.camera().sc;
        self.view.cx -= dx_px / sc;
        self.view.cy += dy_px / sc;
    }
    /// Sets the view (restoring a saved camera); ignored unless finite, zoom clamped.
    pub fn set_view(&mut self, v: ViewState) {
        if v.cx.is_finite() && v.cy.is_finite() && v.z.is_finite() {
            self.view = ViewState {
                z: v.z.clamp(camera::ZOOM_MIN, camera::ZOOM_MAX),
                ..v
            };
        }
    }

    // ---------- editing ----------

    fn push(&mut self) {
        self.hist.push(self.design.clone());
        if self.hist.len() > UNDO_DEPTH {
            self.hist.remove(0);
        }
    }
    /// Keeps the undo step pushed for an edit only if the design changed.
    fn settle_step(&mut self, depth: usize, before: &Design) {
        if self.design == *before {
            self.hist.truncate(depth);
        } else {
            self.revision += 1;
        }
    }
    fn edit(&mut self, f: impl FnOnce(&mut Game)) {
        let before = self.design.clone();
        let depth = self.hist.len();
        f(self);
        if self.hist.len() > depth {
            self.settle_step(depth, &before);
        } else if self.design != before {
            self.revision += 1;
        }
        self.refresh();
    }

    fn add_node(&mut self, x: f64, y: f64) -> Option<String> {
        if !(x.is_finite() && y.is_finite()) {
            return None;
        }
        let scene = self.scene();
        if y < scene.ground(x) - 0.01 {
            self.say("That spot is underground.");
            return None;
        }
        if let Some(n) = self.design.nodes.iter().find(|n| n.x == x && n.y == y) {
            return Some(n.id.clone());
        }
        let id = format!("J{}", self.design.n_n + 1);
        self.design.nodes.push(Joint {
            id: id.clone(),
            x,
            y,
            sup: None,
            locked: false,
        });
        self.design.n_n += 1;
        Some(id)
    }

    fn add_member(&mut self, a: &str, b: &str) -> bool {
        let t = self.kind;
        if a == b {
            return false;
        }
        let (Some(na), Some(nb)) = (self.design.node(a), self.design.node(b)) else {
            return false;
        };
        let len = hypot(nb.x - na.x, nb.y - na.y);
        if len > t.spec().max_len + 0.01 {
            self.say(t.too_long_add_joint());
            return false;
        }
        if let Some(m) = self
            .design
            .members
            .iter_mut()
            .find(|m| (m.a == a && m.b == b) || (m.a == b && m.b == a))
        {
            if !m.locked {
                m.kind = t;
            }
            return true;
        }
        self.design.n_m += 1;
        self.design.members.push(Piece {
            id: format!("M{}", self.design.n_m),
            a: a.to_string(),
            b: b.to_string(),
            kind: t,
            locked: false,
        });
        true
    }

    fn del_node(&mut self, id: &str) {
        let Some(n) = self.design.node(id) else {
            self.say("Level foundations stay put.");
            return;
        };
        if n.locked {
            self.say("Level foundations stay put.");
            return;
        }
        if self
            .design
            .members
            .iter()
            .any(|m| m.locked && (m.a == id || m.b == id))
        {
            self.say(LOCKED_PIECE);
            return;
        }
        self.push();
        self.design.set_load(id, 0.0);
        self.design.nodes.retain(|q| q.id != id);
        self.design.members.retain(|m| m.a != id && m.b != id);
        self.sel = None;
        if self.pending.as_deref() == Some(id) {
            self.pending = None;
        }
    }

    fn del_member(&mut self, id: &str) {
        if self.design.member(id).is_some_and(|m| m.locked) {
            self.say(LOCKED_PIECE);
            return;
        }
        self.push();
        self.design.members.retain(|m| m.id != id);
        self.sel = None;
    }

    fn set_found(&mut self, id: &str, f: Option<Foundation>) {
        let Some(n) = self.design.node(id) else {
            return;
        };
        if n.locked {
            self.say("Level foundations stay put.");
            return;
        }
        let setup = self.setup();
        if f.is_some() && !setup.sup_ok(n.x, n.y) {
            self.say(setup.sup_refusal());
            return;
        }
        self.push();
        if let Some(q) = self.design.nodes.iter_mut().find(|q| q.id == id) {
            q.sup = f;
        }
    }

    fn add_load(&mut self, id: &str, t: f64) {
        if !t.is_finite() || self.design.node(id).is_none() {
            return;
        }
        let nv = 0f64.max(super::data::MAX_CRATE_T.min(self.design.load(id) + t));
        let floor = self.level_crate(id);
        if nv < floor {
            self.say(LEVEL_CRATES);
            return;
        }
        self.push();
        self.design.set_load(id, nv);
    }

    fn click_node_inner(&mut self, id: &str) {
        if !self.building() {
            return;
        }
        let Some(n) = self.design.node(id).cloned() else {
            return;
        };
        match self.tool {
            Tool::Select => self.sel = Some(Selection::Node(n.id)),
            Tool::Build => match self.pending.clone() {
                None => self.pending = Some(n.id),
                Some(p) if p == n.id => self.pending = None,
                Some(p) => {
                    self.push();
                    let ok = self.add_member(&p, &n.id);
                    self.pending = Some(if ok { n.id } else { p });
                }
            },
            Tool::Found => {
                let f = if n.sup == Some(self.found) {
                    None
                } else {
                    Some(self.found)
                };
                self.set_found(&n.id, f);
            }
            Tool::Crate => {
                let w = self.weight;
                self.add_load(&n.id, w);
            }
            Tool::Erase => self.del_node(&n.id),
        }
    }

    /// Clicks a joint (`clickNode`): Select inspects it; Build starts / extends / ends a chain;
    /// Found toggles the foundation kind; Crate hangs a crate; Erase deletes it.
    pub fn click_node(&mut self, id: &str) {
        let id = id.to_string();
        self.edit(|g| g.click_node_inner(&id));
    }

    fn click_at_inner(&mut self, p: [f64; 2]) {
        if !self.building() {
            return;
        }
        match self.tool {
            Tool::Build => {
                if let Some(pid) = &self.pending
                    && let Some(a) = self.design.node(pid)
                {
                    let t = self.kind;
                    if hypot(p[0] - a.x, p[1] - a.y) > t.spec().max_len + 0.01 {
                        self.say(t.too_long());
                        return;
                    }
                }
                self.push();
                let Some(n) = self.add_node(p[0], p[1]) else {
                    return;
                };
                if let Some(pid) = self.pending.clone() {
                    self.add_member(&pid, &n);
                }
                self.pending = Some(n);
            }
            Tool::Found => {
                if let Some(n0) = self
                    .design
                    .nodes
                    .iter()
                    .find(|n| n.x == p[0] && n.y == p[1])
                {
                    let id = n0.id.clone();
                    self.click_node_inner(&id);
                    return;
                }
                if !self.setup().sup_ok(p[0], p[1]) {
                    self.say("Foundations go on the ground.");
                    return;
                }
                self.push();
                if let Some(id) = self.add_node(p[0], p[1]) {
                    let f = self.found;
                    if let Some(q) = self.design.nodes.iter_mut().find(|q| q.id == id) {
                        q.sup = Some(f);
                    }
                }
            }
            Tool::Select => self.sel = None,
            _ => {}
        }
    }

    /// Clicks empty grid at a world grid point (`clickSurface` after `snap`): Build drops a
    /// joint (and chains a piece to it); Found places a foundation there; Select deselects.
    pub fn click_at(&mut self, p: [f64; 2]) {
        self.edit(|g| g.click_at_inner(p));
    }

    /// Clicks the stage surface at stage px (`onSurface`): a click that ended a pan does
    /// nothing; else [`Game::click_at`] the snapped point.
    pub fn click_surface(&mut self, px: f64, py: f64) {
        if self.pan_moved {
            self.pan_moved = false;
            return;
        }
        if !(px.is_finite() && py.is_finite()) {
            return;
        }
        let p = self.camera().snap(px, py);
        self.click_at(p);
    }

    /// Picks a piece's handle (`memHandles.pick`): Erase deletes it, otherwise it is selected
    /// and the tool becomes Select.
    pub fn pick_handle(&mut self, id: &str) {
        if !self.building() || self.design.member(id).is_none() {
            return;
        }
        if self.tool == Tool::Erase {
            let id = id.to_string();
            self.edit(|g| g.del_member(&id));
        } else {
            self.sel = Some(Selection::Member(id.to_string()));
            self.tool = Tool::Select;
        }
    }

    /// Selects a joint or piece directly (keyboard / MCP), or nothing.
    pub fn select(&mut self, sel: Option<Selection>) {
        if !self.building() {
            return;
        }
        self.sel = sel.filter(|s| match s {
            Selection::Node(id) => self.design.node(id).is_some(),
            Selection::Member(id) => self.design.member(id).is_some(),
        });
    }

    /// The inspector's foundation chips (None / Footing / Pile).
    pub fn set_foundation(&mut self, id: &str, f: Option<Foundation>) {
        if !self.building() {
            return;
        }
        let id = id.to_string();
        self.edit(|g| g.set_found(&id, f));
    }
    /// Adds `t` tonnes to a joint's crate (0…100 t).
    pub fn add_crate(&mut self, id: &str, t: f64) {
        if !self.building() {
            return;
        }
        let id = id.to_string();
        self.edit(|g| g.add_load(&id, t));
    }
    /// The inspector's "−" (lighter by the crate step).
    pub fn crate_less(&mut self, id: &str) {
        self.add_crate(id, -self.weight);
    }
    /// The inspector's "+" (heavier by the crate step).
    pub fn crate_more(&mut self, id: &str) {
        self.add_crate(id, self.weight);
    }
    /// The inspector's material chips: changes a piece's type (refused when too long:
    /// "Steel pieces can be at most 14 m.").
    pub fn set_piece_type(&mut self, id: &str, k: PieceType) {
        if !self.building() {
            return;
        }
        let Some(m) = self.design.member(id).cloned() else {
            return;
        };
        if m.locked {
            self.say(LOCKED_PIECE);
            return;
        }
        if self.design.length(&m) > k.spec().max_len + 0.01 {
            self.say(k.too_long());
            return;
        }
        self.edit(|g| {
            g.push();
            if let Some(x) = g.design.members.iter_mut().find(|x| x.id == m.id) {
                x.kind = k;
            }
        });
    }
    /// "Delete piece".
    pub fn delete_member(&mut self, id: &str) {
        if !self.building() {
            return;
        }
        let id = id.to_string();
        self.edit(|g| g.del_member(&id));
    }
    /// "Delete joint" (its pieces and crate go too; level foundations stay put).
    pub fn delete_joint(&mut self, id: &str) {
        if !self.building() {
            return;
        }
        let id = id.to_string();
        self.edit(|g| g.del_node(&id));
    }
    /// Delete / Backspace: deletes the selection.
    pub fn delete_selected(&mut self) {
        match self.sel.clone() {
            Some(Selection::Member(id)) => self.delete_member(&id),
            Some(Selection::Node(id)) => self.delete_joint(&id),
            None => {}
        }
    }
    /// Undo (building only).
    pub fn undo(&mut self) {
        if !self.building() {
            return;
        }
        let Some(d) = self.hist.pop() else { return };
        self.design = d;
        self.sel = None;
        self.pending = None;
        self.revision += 1;
        self.refresh();
    }

    // ---------- pointer ----------

    /// Pointer down on a joint (`j.down`): with Select while building, starts dragging it.
    pub fn joint_down(&mut self, id: &str) {
        if !(self.building() && self.tool == Tool::Select) {
            return;
        }
        let Some(n) = self.design.node(id) else {
            return;
        };
        if n.locked || self.design.holds_locked(id) {
            return;
        }
        self.push();
        self.drag = Some(id.to_string());
        self.sel = Some(Selection::Node(id.to_string()));
    }

    /// Pointer down on empty stage (`onSurfaceDown`): with Select, or while running, starts a
    /// pan.
    pub fn surface_down(&mut self, px: f64, py: f64) {
        if !(px.is_finite() && py.is_finite()) {
            return;
        }
        if self.tool == Tool::Select || self.phase != Phase::Build {
            self.pan = Some(Pan {
                px,
                py,
                cx: self.view.cx,
                cy: self.view.cy,
            });
            self.pan_moved = false;
        }
    }

    /// The pointer moves over the stage, stage px (`onMove`): pans, drags a joint (refusing
    /// underground and occupied spots and moves that make a piece too long; a foundation
    /// dragged off the ground is dropped) or updates the hover dot.
    pub fn pointer_move(&mut self, px: f64, py: f64) {
        if !(px.is_finite() && py.is_finite()) {
            return;
        }
        if let Some(pan) = self.pan {
            let sc = self.camera().sc;
            let dx = px - pan.px;
            let dy = py - pan.py;
            if dx.abs() + dy.abs() > 3.0 {
                self.view.cx = pan.cx - dx / sc;
                self.view.cy = pan.cy + dy / sc;
                self.pan_moved = true;
            }
            return;
        }
        if self.phase != Phase::Build {
            return;
        }
        let p = self.camera().snap(px, py);
        if let Some(id) = self.drag.clone() {
            self.drag_to(&id, p);
            return;
        }
        if matches!(self.tool, Tool::Build | Tool::Found) && self.hover != Some(p) {
            self.hover = Some(p);
        }
    }

    /// Moves a joint to a grid point under the drag rules; returns whether it moved.
    fn drag_to(&mut self, id: &str, p: [f64; 2]) -> bool {
        let scene = self.scene();
        if p[1] < scene.ground(p[0])
            || self
                .design
                .nodes
                .iter()
                .any(|n| n.id != id && n.x == p[0] && n.y == p[1])
        {
            return false;
        }
        let setup = self.setup();
        let mut d = self.design.clone();
        let Some(n) = d.nodes.iter_mut().find(|n| n.id == id) else {
            return false;
        };
        n.x = p[0];
        n.y = p[1];
        if n.sup.is_some() && !setup.sup_ok(p[0], p[1]) {
            n.sup = None;
        }
        if d.members
            .iter()
            .any(|m| (m.a == id || m.b == id) && d.length(m) > m.kind.spec().max_len + 0.01)
        {
            return false;
        }
        let changed = d != self.design;
        self.design = d;
        self.hover = None;
        if changed {
            self.revision += 1;
        }
        self.refresh();
        true
    }

    /// Pointer up / leave (`onUp`): ends a drag (dropping its undo step if the joint didn't
    /// move) or a pan.
    pub fn pointer_up(&mut self) {
        if self.drag.is_some() {
            if self.hist.last() == Some(&self.design) {
                self.hist.pop();
            }
            self.drag = None;
        }
        self.pan = None;
    }

    /// The pointer left the stage: no hover dot.
    pub fn clear_hover(&mut self) {
        self.hover = None;
    }

    /// ⌥ + arrow: moves the selected joint one grid step under the drag rules (one undo step).
    pub fn nudge(&mut self, dx: f64, dy: f64) {
        if !self.building() {
            return;
        }
        let Some(Selection::Node(id)) = self.sel.clone() else {
            return;
        };
        let Some(n) = self.design.node(&id) else {
            return;
        };
        if n.locked || self.design.holds_locked(&id) {
            return;
        }
        if !(dx.is_finite() && dy.is_finite()) {
            return;
        }
        let cam = self.camera();
        let step = |d: f64| {
            if d > 0.0 {
                cam.grid
            } else if d < 0.0 {
                -cam.grid
            } else {
                0.0
            }
        };
        let p = [
            cam.xmin.max(cam.xmax.min(n.x + step(dx))),
            cam.ymin.max(cam.ymax.min(n.y + step(dy))),
        ];
        let before = self.design.clone();
        self.push();
        if !self.drag_to(&id, p) || self.design == before {
            self.hist.pop();
        }
    }

    // ---------- presets ----------

    /// `buildPreset(b)` (no undo steps).
    pub fn build_preset(&mut self, b: PresetBuild) {
        let add = |g: &mut Game, x: f64, y: f64| g.add_node(x, y).unwrap_or_default();
        fn mem(g: &mut Game, t: PieceType, a: &str, c: &str) {
            g.kind = t;
            g.add_member(a, c);
        }
        match b {
            PresetBuild::Truss { chord } => {
                let mut bot = vec!["J1".to_string()];
                let mut x = 16.0;
                while x <= 40.0 {
                    bot.push(add(self, x, 0.0));
                    x += 8.0;
                }
                bot.push("J2".into());
                for i in 0..bot.len() - 1 {
                    mem(self, PieceType::Road, &bot[i], &bot[i + 1]);
                }
                let mut top = vec![];
                let mut x = 16.0;
                while x <= 40.0 {
                    top.push(add(self, x, 6.0));
                    x += 8.0;
                }
                for i in 0..top.len() - 1 {
                    mem(self, chord, &top[i], &top[i + 1]);
                }
                mem(self, chord, "J1", &top[0]);
                mem(self, chord, &top[3], "J2");
                for i in 0..4 {
                    mem(self, chord, &bot[i + 1], &top[i]);
                }
                mem(self, chord, &top[0], &bot[2]);
                mem(self, chord, &top[1], &bot[3]);
                mem(self, chord, &top[3], &bot[3]);
                self.kind = PieceType::Road;
            }
            PresetBuild::Cable => {
                let pl = add(self, 0.0, 0.0);
                let pr = add(self, 56.0, 0.0);
                for n in &mut self.design.nodes {
                    if n.id == pl || n.id == pr {
                        n.sup = Some(Foundation::Pile);
                    }
                }
                let d: Vec<String> = [16.0, 24.0, 32.0, 40.0]
                    .iter()
                    .map(|&x| add(self, x, 0.0))
                    .collect();
                let mut deck = vec!["J1".to_string()];
                deck.extend(d.iter().cloned());
                deck.push("J2".into());
                for i in 0..deck.len() - 1 {
                    mem(self, PieceType::Road, &deck[i], &deck[i + 1]);
                }
                let tl = add(self, 8.0, 10.0);
                let tr = add(self, 48.0, 10.0);
                mem(self, PieceType::Steel, "J1", &tl);
                mem(self, PieceType::Steel, "J2", &tr);
                mem(self, PieceType::Cable, &tl, &pl);
                mem(self, PieceType::Cable, &tl, &d[0]);
                mem(self, PieceType::Cable, &tl, &d[1]);
                mem(self, PieceType::Cable, &tr, &pr);
                mem(self, PieceType::Cable, &tr, &d[3]);
                mem(self, PieceType::Cable, &tr, &d[2]);
                self.kind = PieceType::Cable;
            }
            PresetBuild::Tower {
                h,
                bays,
                col,
                brace,
                found,
            } => {
                self.design = Design::default();
                let bays = bays.min(60);
                let xs: Vec<f64> = (0..=bays)
                    .map(|i| 100.0 - bays as f64 * 4.0 + i as f64 * 8.0)
                    .collect();
                let mut below: Vec<String> = xs.iter().map(|&x| add(self, x, 0.0)).collect();
                for n in &mut self.design.nodes {
                    n.sup = Some(found);
                }
                let h = if h.is_finite() { h.min(400.0) } else { 0.0 };
                let mut y = 4.0;
                while y <= h {
                    let cur: Vec<String> = xs.iter().map(|&x| add(self, x, y)).collect();
                    for i in 0..xs.len() {
                        mem(self, col, &below[i], &cur[i]);
                    }
                    for i in 0..bays as usize {
                        mem(self, PieceType::Floor, &cur[i], &cur[i + 1]);
                    }
                    for i in 0..bays as usize {
                        mem(self, brace, &below[i], &cur[i + 1]);
                    }
                    below = cur;
                    y += 4.0;
                }
                self.kind = PieceType::Floor;
            }
        }
        self.revision += 1;
        self.refresh();
    }

    // ---------- running ----------

    /// "Run traffic" / "Run the test" (`startRun`): warns when there is no Road / Floor yet,
    /// then runs (deselects, ends the chain).
    pub fn start_run(&mut self, slow: bool) {
        let setup = self.setup();
        if setup.scene == Scene::River && !self.design.has(PieceType::Road) {
            self.say("Nothing to drive on yet: build some Road first.");
        }
        if setup.scene == Scene::Flat && !self.design.has(PieceType::Floor) {
            self.say("No Floors yet, so there is no height to measure.");
        }
        self.sim = Some(Sim::new(&self.design, &setup, &self.cond, true));
        self.phase = Phase::Run;
        self.slow = slow;
        self.sel = None;
        self.pending = None;
        self.hover = None;
        self.result = None;
        self.drag = None;
        self.after = 0;
    }
    /// "Stop" / "Back to building" (`stopRun`).
    pub fn stop_run(&mut self) {
        self.sim = None;
        self.phase = Phase::Build;
        self.result = None;
        self.refresh();
    }
    /// The run button (`toggleRun` / header `toggleDrive`): runs while building, else stops.
    pub fn toggle_run(&mut self) {
        if self.phase == Phase::Build {
            self.start_run(false);
        } else {
            self.stop_run();
        }
    }
    /// "Watch again, slow": the same run in slow motion.
    pub fn replay(&mut self) {
        self.start_run(true);
    }
    /// "Slow motion" / "Normal speed".
    pub fn toggle_slow(&mut self) {
        self.slow = !self.slow;
    }

    /// One 33 ms frame of the run (`tick()`): 20 substeps, sway, and the end of the run with
    /// its result, stars and best stars. Returns whether anything changed.
    pub fn tick(&mut self) -> bool {
        if self.phase == Phase::Result && self.demo_loop {
            self.after += 1;
            if self.after > DEMO_LOOP_FRAMES {
                self.after = 0;
                self.start_run(false);
            }
            return true;
        }
        if self.phase != Phase::Run {
            return false;
        }
        let cost = self.design.cost();
        let practice = self.practice();
        let Some(sim) = self.sim.as_mut() else {
            return false;
        };
        sim.cond = self.cond;
        sim.frame(self.slow);
        if sim.check_done() {
            let r = result_of(sim, cost, practice);
            if r.stars > self.best_for(self.level) {
                self.best.insert(self.level, r.stars);
            }
            self.result = Some(r);
            self.phase = Phase::Result;
        }
        true
    }

    /// Headless "run to the end" (MCP, tests): starts a run if building, ticks until the
    /// result or `max_frames`, and returns the result (`None` in the Sandbox or past the
    /// limit). Open ground takes 541 frames, river runs up to 1351.
    pub fn run_test(&mut self, max_frames: usize) -> Option<&RunResult> {
        if self.setup().run == RunKind::Years {
            return None;
        }
        if self.phase != Phase::Run {
            self.start_run(false);
        }
        for _ in 0..max_frames {
            if self.phase != Phase::Run {
                break;
            }
            self.tick();
        }
        self.result.as_ref()
    }

    // ---------- netlists (NAV-4, NAV-6) ----------

    /// Share / "Open in Advanced": the design as a `.str` netlist named after the level
    /// (`river-level`, `sandbox`), with the level and conditions as comments.
    pub fn share_text(&self) -> String {
        super::netlist::write(
            &self.design,
            self.level,
            self.scene(),
            &self.cond,
            &self.level.slug(),
        )
    }

    /// ⌘V / "Open in Playground": loads a netlist into the Sandbox (its scene from where the
    /// joints sit; the conditions from its `* cond` line, if any). On error nothing changes.
    pub fn paste_netlist(&mut self, text: &str) -> Result<(), super::netlist::ReadError> {
        let imp = super::netlist::read(text)?;
        self.open_in_sandbox(imp.scene, imp.design);
        if let Some(c) = imp.cond {
            self.cond = c;
            self.refresh();
        }
        Ok(())
    }

    // ---------- the preview cache ----------

    fn refresh(&mut self) {
        if self.live() {
            return;
        }
        let key = PreviewKey {
            design: self.design.clone(),
            level: self.level,
            sb_scene: self.sb_scene,
            soil: self.cond.soil,
            snow: self.cond.snow,
        };
        if self.pv.as_ref().is_some_and(|(k, _)| *k == key) {
            return;
        }
        let s = preview(&self.design, &self.setup(), &self.cond);
        self.pv = Some((key, s));
    }
}

/// The toast for editing a level's own (locked) piece (Leaning tower; not in the mockup).
pub const LOCKED_PIECE: &str = "Level pieces stay put.";
/// The toast for taking a level's own crate below its weight (the Leaning tower's bells).
pub const LEVEL_CRATES: &str = "The bells stay: fix the lean another way.";
