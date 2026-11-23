//! What the builder screen draws ([`Game::view`], the mockup's `renderVals()`): every shape in
//! **stage pixels** on the 1120 × 600 stage (the SVG `viewBox`; the screen maps it to points
//! with one scale and offset, aspect kept), every sentence, label and chip. Every number the
//! screen shows comes from here (SPEC DONE-2); the screen maps tones / bands / piece types to
//! `sp_theme` colours and owns nothing but pixels.
//!
//! Drawing order (the mockup's SVG, back to front): grid dots, water, flow lines, soil, grains,
//! riverbed, soil top, edge, foundations, wind streaks, wind arrows, back copies, ghost line,
//! façades, façade sides, window lines, deck tops, deck fronts, snow, road lines, material
//! lines (Concrete 7, Wood 6, Steel 4.5, Rope 2.4, Cable 2.2 px), force bands (2.5 px; Over
//! 3.5 px dashed 9 6), crate wires / sides / tops / bodies, vehicles, foundation triangles,
//! target line, splashes, sparks, red rings, hover dot, snowflakes; then the HTML layer:
//! piece handles, joints, crate tags, HUD chips, toast, run overlay, result banner.

use super::camera::{Camera, STAGE_H, STAGE_W};
use super::data::{
    DIRS, FIRES, Foundation, Group, LevelKey, PieceType, QUAKES, SNOWS, Scene, Soil, TrafficSet,
    VehicleKind, WEIGHTS, WINDS, levels, stars_text,
};
use super::game::{Game, Outcome, Phase, RunResult, Selection, Tool};
use super::setup::{Setup, spec};
use super::sim::{Sim, Splash as SimSplash, VehicleState, utilisation};
use crate::jsnum::{round, to_fixed, to_string};
use crate::truss::{ZERO_FORCE_N, hypot};

/// A point, stage px.
pub type Pt = [f64; 2];
/// A quadrilateral, stage px (in drawing order).
pub type Quad = [Pt; 4];

/// A force band (the overlay line over a piece): pulled / pushed at three brightness levels by
/// utilisation (< 35 %, < 70 %, ≥ 70 %), or over capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Band {
    /// Pulled, < 35 % (`t1`, blue at 0.5).
    T1,
    /// Pulled, < 70 % (`t2`, blue at 0.8).
    T2,
    /// Pulled, ≥ 70 % (`t3`, bright blue).
    T3,
    /// Pushed, < 35 % (`c1`, orange at 0.5).
    C1,
    /// Pushed, < 70 % (`c2`).
    C2,
    /// Pushed, ≥ 70 % (`c3`, bright orange).
    C3,
    /// Over capacity (`x`, red dashed): it will break.
    Over,
}

impl Band {
    /// The mockup's key: `t1` … `c3`, `x`.
    pub fn key(self) -> &'static str {
        match self {
            Band::T1 => "t1",
            Band::T2 => "t2",
            Band::T3 => "t3",
            Band::C1 => "c1",
            Band::C2 => "c2",
            Band::C3 => "c3",
            Band::Over => "x",
        }
    }
    /// The band of a force (`N`, over flag, utilisation); `None` when slack or below 500 N.
    pub fn of(n: f64, over: bool, slack: bool, ut: f64) -> Option<Band> {
        if over || ut > 1.0 {
            return Some(Band::Over);
        }
        if slack || n.abs() < ZERO_FORCE_N {
            return None;
        }
        let lvl = if ut < 0.35 {
            0
        } else if ut < 0.7 {
            1
        } else {
            2
        };
        Some(if n > 0.0 {
            [Band::T1, Band::T2, Band::T3][lvl]
        } else {
            [Band::C1, Band::C2, Band::C3][lvl]
        })
    }
}

/// A deck's 2.5D slab (Road, Floor).
#[derive(Debug, Clone, PartialEq)]
pub struct DeckView {
    /// The top face: a, b, b + offset, a + offset (`roadTopD`).
    pub top: Quad,
    /// The front face: a, b, b + thickness, a + thickness (`roadFrontD`).
    pub front: Quad,
    /// Road: the dashed centre line (`roadLineD`).
    pub line: Option<[Pt; 2]>,
    /// Floor: the façade up to 3.6 m above the slab (`facadeD`).
    pub facade: Option<Quad>,
    /// Floor: the façade's right side face (`facadeSideD`).
    pub side: Option<Quad>,
    /// Floor: the window line 2 m above the slab (`winD`).
    pub window: Option<[Pt; 2]>,
    /// Snow lying on it (`snowD`).
    pub snow: Option<Quad>,
}

/// A piece on the stage (broken pieces are not drawn).
#[derive(Debug, Clone, PartialEq)]
pub struct MemberView {
    /// Index into the design's pieces.
    pub index: usize,
    /// "M7".
    pub id: String,
    /// Its type (colour, stroke width).
    pub kind: PieceType,
    /// End a, px.
    pub a: Pt,
    /// End b, px.
    pub b: Pt,
    /// End a, world m (displaced while running).
    pub wa: Pt,
    /// End b, world m.
    pub wb: Pt,
    /// A slack Rope / Cable while running: the quadratic curve's control point, px (the line
    /// is `a Q sag b`; the force band follows the same curve).
    pub sag: Option<Pt>,
    /// The force band over it.
    pub band: Option<Band>,
    /// Decks: the slab. Other non-tension-only pieces get a back copy at `a + off`, `b + off`
    /// (`backD`, 3 px dark); tension-only pieces have none.
    pub deck: Option<DeckView>,
    /// The back copy (`backD`): `[a + off, b + off]`, px.
    pub back: Option<[Pt; 2]>,
    /// Fire hook: the piece's temperature, °C (20 when cold; the glow is the `lab` agent's).
    pub temp: f64,
}

/// A foundation on the stage (`foundD` + `supD`).
#[derive(Debug, Clone, PartialEq)]
pub struct FoundationView {
    /// Joint id.
    pub joint: String,
    /// Its kind.
    pub kind: Foundation,
    /// The joint, px.
    pub at: Pt,
    /// The pad: x, y, w, h, px.
    pub pad: [f64; 4],
    /// Pile: the shaft under the pad (14 m deep): x, y, w, h, px.
    pub pile: Option<[f64; 4]>,
    /// The small support triangle: (x, y − 1), (x − 6, y + 6), (x + 6, y + 6).
    pub tri: [Pt; 3],
}

/// A crate on the stage.
#[derive(Debug, Clone, PartialEq)]
pub struct CrateView {
    /// Joint id.
    pub joint: String,
    /// Tonnes.
    pub t: f64,
    /// The wire from the joint down to the crate, px.
    pub wire: [Pt; 2],
    /// The front: x, y, w, h, px.
    pub body: [f64; 4],
    /// The top face, px.
    pub top: Quad,
    /// The side face, px.
    pub side: Quad,
    /// "5 t".
    pub tag: String,
    /// The tag's position (centre, its top), px.
    pub tag_at: Pt,
}

/// A vehicle on the stage.
#[derive(Debug, Clone, PartialEq)]
pub struct VehicleView {
    /// Its kind (body colour: car `vCarD`, bus `vBusD`, truck box `vBoxD`).
    pub kind: VehicleKind,
    /// Its state.
    pub state: VehicleState,
    /// Centre x, world m.
    pub x: f64,
    /// Road height under it, world m.
    pub y: f64,
    /// Body angle, rad.
    pub ang: f64,
    /// Body rectangles (car: chassis + cabin; bus: body; truck: the box).
    pub body: Vec<Quad>,
    /// Truck: the cab (`vCabD`).
    pub cab: Option<Quad>,
    /// Windows (`vWinD`).
    pub windows: Vec<Quad>,
    /// Wheels: centre px and radius px.
    pub wheels: [(Pt, f64); 2],
}

/// A wind arrow at one height.
#[derive(Debug, Clone, PartialEq)]
pub struct WindArrow {
    /// The shaft start, px.
    pub from: Pt,
    /// The tip, px.
    pub to: Pt,
    /// The head: two barbs meeting at the tip, px.
    pub head: [Pt; 3],
    /// A streak crossing the sky: start, two cubic control points, end, px (dashed 20 28).
    pub streak: [Pt; 4],
}

/// A splash where a vehicle sank: two humps from `at.x − r` to `at.x + r`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplashView {
    /// The middle of the splash on the water line, px.
    pub at: Pt,
    /// Seconds since the vehicle sank (shown for 1.6 s).
    pub age: f64,
    /// Half-width, px (grows for 1.6 s).
    pub r: f64,
}

/// The terrain (drawn in world coordinates, so it zooms with everything else).
#[derive(Debug, Clone, PartialEq)]
pub struct Terrain {
    /// The scene.
    pub scene: Scene,
    /// The soil (`soilColor` / `soilTop`).
    pub soil: Soil,
    /// The riverbed's soil (`bedColor`).
    pub bed: Soil,
    /// The soil body, px.
    pub soil_poly: Vec<Pt>,
    /// The riverbed strip, px (river).
    pub bed_poly: Option<Vec<Pt>>,
    /// The water, px (river).
    pub water_poly: Option<Vec<Pt>>,
    /// The three flow lines, px (river; dashed 18 30, animated while running).
    pub flow: Vec<[Pt; 2]>,
    /// The 2.5D soil top faces, px.
    pub tops: Vec<Quad>,
    /// The ground's edge lines, px.
    pub edges: Vec<[Pt; 2]>,
    /// Soil grains: 2 × 2 px squares at these top-left corners.
    pub grains: Vec<Pt>,
}

/// The dotted grid (a pattern: dots of radius 1.3 at `origin + k·step`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridView {
    /// World (0, 0), px.
    pub origin: Pt,
    /// Dot spacing, px.
    pub step: f64,
}

/// A piece's handle (Select / Erase tools, or "Show forces").
#[derive(Debug, Clone, PartialEq)]
pub struct HandleView {
    /// Piece id.
    pub id: String,
    /// The piece's midpoint, px.
    pub at: Pt,
    /// "+2.4 t" / "−0.8 t" / "slack" with "Show forces", else empty.
    pub text: String,
    /// "Steel M7, 63% of its limit".
    pub label: String,
    /// "Show forces" is on (the handle is a pill with the text).
    pub num: bool,
    /// Selected.
    pub selected: bool,
    /// Over capacity.
    pub over: bool,
}

/// A joint button (building only).
#[derive(Debug, Clone, PartialEq)]
pub struct JointView {
    /// "J3".
    pub id: String,
    /// px.
    pub at: Pt,
    /// World m.
    pub pos: Pt,
    /// Its foundation.
    pub sup: Option<Foundation>,
    /// "Joint J3, on a pile, carrying a 5 t crate".
    pub label: String,
    /// It has a foundation (`j-lock`).
    pub founded: bool,
    /// A level's own joint (can't be dragged).
    pub locked: bool,
    /// The chain's glowing joint (`j-pend`).
    pub pending: bool,
    /// Selected (`j-sel`).
    pub selected: bool,
    /// Ringed red (`j-bad`): it sags, or its foundation is overloaded.
    pub bad: bool,
}

/// A HUD chip's colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HudTone {
    /// `#EAEAEA`.
    Plain,
    /// `#5FBF8A`: the height reached the target.
    Good,
    /// `#E8A39E`: over the comfort limit.
    Bad,
    /// `#8FD3EE`: wind.
    Wind,
    /// `#E6EEF3`: snow.
    Snow,
    /// `#E7A47C`: shaking.
    Shake,
}

/// A HUD chip (top right of the stage): "Height " + "40 m / 64".
#[derive(Debug, Clone, PartialEq)]
pub struct HudChip {
    /// The muted key, with its trailing space ("Height ").
    pub key: &'static str,
    /// The value.
    pub value: String,
    /// Its colour.
    pub tone: HudTone,
}

/// The status pill's colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusTone {
    /// `#A3A3A3`: nothing built.
    Muted,
    /// `#5FBF8A`: stands up.
    Ok,
    /// `#D8B86A`: strained.
    Warn,
    /// `#E5534B`: sags, breaks, a foundation overloaded.
    Danger,
    /// `#D6C7A1` (sand): running.
    Running,
}

/// The status pill.
#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    /// "Stands up", …
    pub label: String,
    /// Its colour.
    pub tone: StatusTone,
}

/// A chip (toggle button) with the value it sets.
#[derive(Debug, Clone, PartialEq)]
pub struct Chip<T> {
    /// What picking it sets.
    pub value: T,
    /// Its text.
    pub name: String,
    /// Pressed.
    pub on: bool,
    /// Dimmed (`chip-dim`: not allowed here / now).
    pub dim: bool,
}

/// A palette chip (Build tool).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeChip {
    /// The type.
    pub kind: PieceType,
    /// "Steel".
    pub name: &'static str,
    /// Tooltip.
    pub tip: &'static str,
    /// The swatch bar's height, px (16 px wide).
    pub swatch_h: f64,
    /// Pressed.
    pub on: bool,
}

/// A foundation chip (Found tool).
#[derive(Debug, Clone, PartialEq)]
pub struct FoundChip {
    /// The kind.
    pub kind: Foundation,
    /// "Footing · $0.3k".
    pub name: &'static str,
    /// Tooltip.
    pub tip: &'static str,
    /// Pressed.
    pub on: bool,
}

/// A rail tool button.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolView {
    /// The tool.
    pub tool: Tool,
    /// "Select".
    pub name: &'static str,
    /// Tooltip.
    pub tip: &'static str,
    /// The icon path (22 × 22).
    pub icon: &'static str,
    /// Pressed.
    pub on: bool,
}

/// A level row.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelRow {
    /// The level.
    pub key: LevelKey,
    /// "1 · First crossing".
    pub name: String,
    /// Best stars "★★☆" (empty for the Sandbox).
    pub best: String,
    /// The current level.
    pub on: bool,
}

/// A level group.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelGroup {
    /// The group.
    pub group: Group,
    /// "Bridges".
    pub name: &'static str,
    /// Its levels.
    pub items: Vec<LevelRow>,
}

/// The Conditions panel.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionsView {
    /// Wind, m/s.
    pub winds: Vec<Chip<f64>>,
    /// Direction (1 / −1).
    pub dirs: Vec<Chip<f64>>,
    /// Snow, kg/m.
    pub snows: Vec<Chip<f64>>,
    /// Earthquake, g.
    pub quakes: Vec<Chip<f64>>,
    /// Soil (swatch: the soil's top colour; dimmed while running).
    pub soils: Vec<Chip<Soil>>,
    /// Traffic (river scenes only; dimmed while running).
    pub traffic: Vec<Chip<TrafficSet>>,
    /// Show the Traffic row (`isRiver`).
    pub show_traffic: bool,
    /// Fire, °C (SPEC FIRE-1; the row is the `lab` agent's UI).
    pub fires: Vec<Chip<f64>>,
    /// "Shake now" (running with a quake set).
    pub can_shake: bool,
}

/// The utilisation bar's colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarTone {
    /// Red: over capacity.
    Over,
    /// Blue: pulled (or no force).
    Pulled,
    /// Orange: pushed.
    Pushed,
}

/// The piece inspector.
#[derive(Debug, Clone, PartialEq)]
pub struct PieceInspector {
    /// Piece id.
    pub id: String,
    /// Its type.
    pub kind: PieceType,
    /// "Steel · M7".
    pub title: String,
    /// "12.0 m".
    pub len: String,
    /// The bar's fill, 0…100 (`w`).
    pub fill: f64,
    /// "63%".
    pub ut: String,
    /// The bar's colour.
    pub tone: BarTone,
    /// The sentence.
    pub text: String,
    /// Material chips (this scene's types).
    pub types: Vec<Chip<PieceType>>,
    /// A level's own piece (can't be changed or deleted).
    pub locked: bool,
}

/// The joint inspector.
#[derive(Debug, Clone, PartialEq)]
pub struct JointInspector {
    /// Joint id.
    pub id: String,
    /// "Joint J3 · level foundation".
    pub title: String,
    /// "x 24 m · y 6 m".
    pub pos: String,
    /// None / Footing / Pile (dimmed where not allowed or locked).
    pub founds: Vec<Chip<Option<Foundation>>>,
    /// "5 t".
    pub load: String,
    /// The note under the chips.
    pub note: String,
    /// "Delete joint" shown.
    pub can_delete: bool,
}

/// A Materials card (nothing selected): opens the sample lab (SPEC LAB-1).
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialCard {
    /// The type.
    pub kind: PieceType,
    /// "Steel".
    pub name: &'static str,
    /// The pull / push glyph: an SVG path in a 56 × 30 box (`curveFor`), stroked 1.8 in the
    /// swatch colour over the axes `M28 2V28M2 15H54`.
    pub curve: String,
    /// The glyph's polylines (the same points as `curve`).
    pub curve_lines: Vec<Vec<Pt>>,
    /// "Strong and stiff both ways. Bends before it breaks. Max 14 m."
    pub blurb: String,
    /// "$260/m".
    pub cost: String,
}

/// The aside's inspector slot.
#[derive(Debug, Clone, PartialEq)]
pub enum Inspector {
    /// A piece is selected.
    Piece(PieceInspector),
    /// A joint is selected.
    Joint(JointInspector),
    /// Nothing selected: "Materials · how each one pulls and pushes".
    Materials(Vec<MaterialCard>),
}

/// The result banner.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultView {
    /// "All across!", "5 of 8 made it", "It stood!", "It came down", "Too short", "Too wobbly
    /// to live in", "…, but over budget".
    pub title: String,
    /// Passed (green) or not (red).
    pub pass: bool,
    /// "★★☆", "practice" or empty.
    pub stars: String,
}

/// The mouse cursor over the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cursor {
    /// Arrow.
    Default,
    /// Build / Found.
    Crosshair,
    /// Select.
    Grab,
}

/// Everything the builder screen draws.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    // ----- header
    /// `river-level` / `sandbox`.
    pub level_slug: String,
    /// `building` / `t = 3.1 s`.
    pub clock: String,
    /// Running (the header button shows "Pause").
    pub playing: bool,
    /// The header button while not running: "Run traffic" / "Run the test".
    pub hdr_run: &'static str,
    /// The phase.
    pub phase: Phase,
    // ----- rail
    /// The five tools.
    pub tools: Vec<ToolView>,
    /// Undo is available.
    pub can_undo: bool,
    // ----- hint bar
    /// The hint sentence.
    pub hint: &'static str,
    /// Build tool: the palette is shown.
    pub show_types: bool,
    /// The palette (this scene's types).
    pub types: Vec<TypeChip>,
    /// Found tool: the foundation chips are shown.
    pub show_founds: bool,
    /// Footing / Pile.
    pub founds: Vec<FoundChip>,
    /// Crate tool: the weight chips are shown.
    pub show_weights: bool,
    /// 1 / 5 / 10 / 25 t.
    pub weights: Vec<Chip<f64>>,
    /// "Finish line" is shown.
    pub has_pending: bool,
    /// "Show forces" is pressed.
    pub show_forces: bool,
    // ----- stage
    /// The camera (world m → stage px).
    pub camera: Camera,
    /// The grid dots.
    pub grid: GridView,
    /// The terrain.
    pub terrain: Terrain,
    /// The water's flow lines are animated (running).
    pub flowing: bool,
    /// Pieces (not broken), in the design's order.
    pub members: Vec<MemberView>,
    /// Snow depth on exposed decks, px.
    pub snow_px: f64,
    /// Wind now, m/s (easing toward the setting while running).
    pub wind_now: f64,
    /// Wind direction: 1 from the left, −1 from the right.
    pub wind_dir: f64,
    /// Snow now, kg/m.
    pub snow_now: f64,
    /// The quake envelope now, 0…1 (running).
    pub quake_env: f64,
    /// Simulation time, s (running / result).
    pub sim_t: Option<f64>,
    /// Foundations.
    pub foundations: Vec<FoundationView>,
    /// Crates, in first-hung order.
    pub crates: Vec<CrateView>,
    /// Vehicles (waiting at the left while building).
    pub vehicles: Vec<VehicleView>,
    /// Wind arrows and streaks (wind > 0.5 m/s).
    pub wind: Vec<WindArrow>,
    /// Snowflakes (snow set): dots of radius 1.6 px whose top is here (centre `y + 1.6`).
    pub flakes: Vec<Pt>,
    /// The target height's dashed line, y px (open ground levels).
    pub target_y: Option<f64>,
    /// Red ✕ sparks (12 px), px.
    pub sparks: Vec<Pt>,
    /// Splashes.
    pub splashes: Vec<SplashView>,
    /// Red dashed rings (radius 13 px): the sagging joint, overloaded foundations.
    pub rings: Vec<Pt>,
    /// Build tool: the dashed line from the chain's joint to the hover point, px.
    pub ghost: Option<[Pt; 2]>,
    /// Build / Found tools: the hover dot (radius 5 px).
    pub hover: Option<Pt>,
    /// Piece handles.
    pub handles: Vec<HandleView>,
    /// Joint buttons (building only).
    pub joints: Vec<JointView>,
    /// HUD chips.
    pub hud: Vec<HudChip>,
    /// The stage shakes (quake envelope > 0.2 while running).
    pub shaking: bool,
    /// The cursor.
    pub cursor: Cursor,
    /// "Bridge with 9 joints and 15 pieces. Stands up."
    pub stage_label: String,
    /// "Building grid".
    pub surface_label: &'static str,
    /// The toast.
    pub toast: Option<String>,
    /// The run overlay is shown.
    pub running: bool,
    /// "2 / 8 across", "t = 3.1 / 18 s", "t = 3 s · runs until you stop".
    pub run_info: String,
    /// "Slow motion" / "Normal speed".
    pub slow_label: &'static str,
    /// The result banner.
    pub result: Option<ResultView>,
    // ----- legend
    /// "100%".
    pub zoom_pct: String,
    // ----- aside
    /// Level groups.
    pub level_groups: Vec<LevelGroup>,
    /// The goal sentence.
    pub goal: String,
    /// Goal chips: the traffic ("Car · 1.5 t") or "Reach 40 m", and "Practice: easier than
    /// the level".
    pub chips_info: Vec<String>,
    /// The status pill.
    pub status: Status,
    /// "$12.4k / $22k".
    pub cost_text: String,
    /// Over budget (cost in `#E8A39E`).
    pub over_budget: bool,
    /// The goal panel's button: "Run traffic" / "Run the test" / "Stop" / "Back to building".
    pub run_label: &'static str,
    /// The Sandbox (scene chips shown).
    pub is_sandbox: bool,
    /// River scene.
    pub is_river: bool,
    /// The Sandbox scene chips.
    pub sb_scenes: Vec<Chip<Scene>>,
    /// Conditions.
    pub conditions: ConditionsView,
    /// The inspector slot.
    pub inspector: Inspector,
    /// "What's happening".
    pub explain: String,
}

/// Positions and forces to draw the structure at ([`Game::view_settled`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SettledView<'a> {
    /// Each joint's world position, m, in the design's joint order (missing: as designed).
    pub pos: &'a [[f64; 2]],
    /// Each piece's axial force N (+ tension), broken and slack flags, in the design's piece
    /// order (missing: no force).
    pub pieces: &'a [(f64, bool, bool)],
}

/// `fmtT`: "+2.4 t" / "−12 t" (one decimal under 10 t).
pub fn fmt_t(v: f64) -> String {
    let t = (v / 9810.0).abs();
    format!(
        "{}{} t",
        if v < 0.0 { "\u{2212}" } else { "+" },
        to_fixed(t, if t < 10.0 { 1 } else { 0 })
    )
}

/// The Materials card glyph (`curveFor(t)`): the SVG path.
pub fn curve_for(kind: PieceType) -> String {
    let t = kind.spec();
    let m = 4500e3;
    let ht = 13.0 * (t.area * t.f_t / m).sqrt();
    let hc = if t.t_only {
        0.0
    } else {
        13.0 * (t.area * t.f_c / m).sqrt()
    };
    let s = to_string;
    let pull = if t.ductile {
        format!("M28 15 L36 {} L50 {}", s(15.0 - ht), s(15.0 - ht * 1.08))
    } else {
        format!("M28 15 L38 {} L39 15", s(15.0 - ht))
    };
    let push = if t.t_only {
        " M28 15 L6 15".to_string()
    } else if t.ductile {
        format!(" M28 15 L20 {} L6 {}", s(15.0 + hc), s(15.0 + hc * 1.05))
    } else {
        format!(" M28 15 L18 {} L16 {}", s(15.0 + hc), s(15.0 + hc * 0.4))
    };
    pull + &push
}

/// The Materials card glyph as polylines (the points of [`curve_for`]).
pub fn curve_lines(kind: PieceType) -> Vec<Vec<Pt>> {
    let t = kind.spec();
    let m = 4500e3;
    let ht = 13.0 * (t.area * t.f_t / m).sqrt();
    let hc = if t.t_only {
        0.0
    } else {
        13.0 * (t.area * t.f_c / m).sqrt()
    };
    let pull = if t.ductile {
        vec![[28.0, 15.0], [36.0, 15.0 - ht], [50.0, 15.0 - ht * 1.08]]
    } else {
        vec![[28.0, 15.0], [38.0, 15.0 - ht], [39.0, 15.0]]
    };
    let push = if t.t_only {
        vec![[28.0, 15.0], [6.0, 15.0]]
    } else if t.ductile {
        vec![[28.0, 15.0], [20.0, 15.0 + hc], [6.0, 15.0 + hc * 1.05]]
    } else {
        vec![[28.0, 15.0], [18.0, 15.0 + hc], [16.0, 15.0 + hc * 0.4]]
    };
    vec![pull, push]
}

/// A piece's numbers as the view uses them (`cinfo[id]`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Info {
    n: f64,
    broken: bool,
    slack: bool,
    cap_t: f64,
    cap_c: f64,
    over: bool,
    len: f64,
    exposed: bool,
    temp: f64,
}

impl Info {
    fn ut(&self) -> f64 {
        utilisation(self.n, self.cap_t, self.cap_c)
    }
}

fn tip(kind: PieceType, mode: super::sim::BreakMode) -> &'static str {
    use super::fire::FireBreak;
    use super::sim::BreakMode::*;
    match mode {
        Softened => return super::fire::tip(FireBreak::Softened, kind),
        BurnedThrough => return super::fire::tip(FireBreak::BurnedThrough, kind),
        _ => {}
    }
    if kind == PieceType::Rope {
        return "Rope is cheap but weak: swap it for Cable where the pull is big.";
    }
    match mode {
        Cracked if kind == PieceType::Concrete => {
            "Concrete cracks when pulled. Use it only where things push: columns, arches, struts."
        }
        Buckled => {
            "Long pieces that get pushed bow sideways. Make them shorter, use Steel, or brace them halfway."
        }
        Snapped => "It was pulled apart. Steel or a Cable takes far more pull.",
        _ => "It was squeezed past its strength.",
    }
}

fn ftip(mode: super::sim::FoundMode) -> &'static str {
    use super::sim::FoundMode::*;
    match mode {
        Liquefied => {
            "The shaking turned the wet sand to soup and the footing sank. Piles reach down to firmer layers and keep most of their grip."
        }
        Sank => {
            "Too much weight for this soil, so the footing pushed down into it. Spread the load over more foundations, or use a pile."
        }
        RippedOut => {
            "It was pulled straight out of the ground. Footings barely resist pulling; a pile grips the soil along its whole length."
        }
    }
}

fn add(p: Pt, d: Pt) -> Pt {
    [p[0] + d[0], p[1] + d[1]]
}

impl Game {
    /// The render model (`renderVals()`).
    pub fn view(&self) -> View {
        self.view_with(None)
    }

    /// The render model with the structure drawn at given positions and forces instead of the
    /// preview's (SPEC TIME-1: the 30-year fast-forward's frames; agent `time`). Pieces get
    /// their bands from `pieces[i].n` against their capacities; broken ones are not drawn;
    /// joints, handles, foundations, crates and rings follow `pos`. Everything else is as
    /// [`Game::view`].
    pub fn view_settled(&self, s: &SettledView) -> View {
        self.view_with(Some(s))
    }

    fn view_with(&self, over: Option<&SettledView>) -> View {
        let setup = self.setup();
        let sp = spec(self.level(), self.sb_scene());
        let design = self.design();
        let cond = *self.cond();
        let cam = self.camera();
        let sc = cam.sc;
        let off = cam.off;
        let river = setup.scene == Scene::River;
        let phase = self.phase();
        let live = self.live();
        let sim: Option<&Sim> = if live { self.sim() } else { None };
        let pv = if live { None } else { self.preview() };
        let running = phase == Phase::Run;
        let building = phase == Phase::Build;

        // positions (world) and piece info
        let pos: Vec<[f64; 2]> = match (over, sim) {
            (Some(o), _) => design
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| o.pos.get(i).copied().unwrap_or([n.x, n.y]))
                .collect(),
            (None, Some(s)) => s.p.iter().map(|p| [p.x, p.y]).collect(),
            (None, None) => design.nodes.iter().map(|n| [n.x, n.y]).collect(),
        };
        let pos_of = |id: &str| design.node_index(id).and_then(|i| pos.get(i).copied());
        let infos: Vec<(String, Info)> = match (sim, pv) {
            (Some(s), _) => {
                s.c.iter()
                    .map(|c| {
                        (
                            c.id.clone(),
                            Info {
                                n: c.avg,
                                broken: c.broken,
                                slack: c.slack,
                                cap_t: c.cap_t_now(),
                                cap_c: c.cap_c_now(),
                                over: false,
                                len: c.len,
                                exposed: c.exposed,
                                temp: c.temp,
                            },
                        )
                    })
                    .collect()
            }
            (None, Some(p)) => p
                .pieces
                .iter()
                .map(|q| {
                    (
                        q.id.clone(),
                        Info {
                            n: q.n,
                            broken: false,
                            slack: q.slack,
                            cap_t: q.cap_t,
                            cap_c: q.cap_c,
                            over: q.over,
                            len: q.len,
                            exposed: q.exposed,
                            temp: 20.0,
                        },
                    )
                })
                .collect(),
            _ => vec![],
        };
        let infos = match over {
            None => infos,
            Some(o) => design
                .members
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let len = design.length(m);
                    let cap = super::sim::capacity(m.kind, len);
                    let base = infos.iter().find(|(k, _)| *k == m.id).map(|(_, q)| *q);
                    let (n, broken, slack) =
                        o.pieces.get(i).copied().unwrap_or((0.0, false, false));
                    (
                        m.id.clone(),
                        Info {
                            n,
                            broken,
                            slack,
                            cap_t: base.map_or(cap.cap_t, |q| q.cap_t),
                            cap_c: base.map_or(cap.cap_c, |q| q.cap_c),
                            over: false,
                            len,
                            exposed: base.is_some_and(|q| q.exposed),
                            temp: 20.0,
                        },
                    )
                })
                .collect(),
        };
        let info = |id: &str| infos.iter().find(|(k, _)| k == id).map(|(_, q)| *q);
        let px = |p: [f64; 2]| cam.px(p);

        let snow_now = sim.map_or(cond.snow, |s| s.snow_now);
        let snow_px = 10f64.min(snow_now / 800.0 * 3f64.max(0.6 * sc));

        // pieces
        let mut members = Vec::new();
        for (index, m) in design.members.iter().enumerate() {
            let q = info(&m.id);
            if q.is_some_and(|q| q.broken) {
                continue;
            }
            let (Some(a), Some(b)) = (pos_of(&m.a), pos_of(&m.b)) else {
                continue;
            };
            let t = m.kind.spec();
            let (pa, pb) = (px(a), px(b));
            let mut sag = None;
            let mut deck = None;
            let mut back = None;
            if t.t_only {
                let len = hypot(b[0] - a[0], b[1] - a[1]);
                let l0 = match q {
                    Some(q) if q.len != 0.0 && !q.len.is_nan() => q.len,
                    _ => design.length(m),
                };
                if live && len < l0 * 0.995 {
                    let s = 0f64.max(l0 * l0 - len * len).sqrt() / 2.0;
                    sag = Some(px([(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0 - s]));
                }
            } else if t.deck {
                let floor = m.kind == PieceType::Floor;
                let th = if floor {
                    3f64.max(0.5 * sc)
                } else {
                    4f64.max(0.33 * sc)
                };
                let fh = 3.6;
                let mut d = DeckView {
                    top: [pa, pb, add(pb, off), add(pa, off)],
                    front: [pa, pb, add(pb, [0.0, th]), add(pa, [0.0, th])],
                    line: None,
                    facade: None,
                    side: None,
                    window: None,
                    snow: None,
                };
                if m.kind == PieceType::Road {
                    let h = [off[0] / 2.0, off[1] / 2.0];
                    d.line = Some([add(pa, h), add(pb, h)]);
                }
                if floor {
                    let au = px([a[0], a[1] + fh]);
                    let bu = px([b[0], b[1] + fh]);
                    d.facade = Some([pa, pb, bu, au]);
                    let right = if a[0] > b[0] { a } else { b };
                    let r = px(right);
                    let ru = px([right[0], right[1] + fh]);
                    d.side = Some([r, add(r, off), add(ru, off), ru]);
                    d.window = Some([px([a[0], a[1] + 2.0]), px([b[0], b[1] + 2.0])]);
                }
                if snow_px > 0.4 && q.is_some_and(|q| q.exposed) {
                    let top = if floor { 3.6 } else { 0.0 };
                    let o = [off[0] * 0.2, off[1] * 0.2];
                    let o2 = [off[0] * 0.2, off[1] * 0.2 - snow_px];
                    let at = px([a[0], a[1] + top]);
                    let bt = px([b[0], b[1] + top]);
                    d.snow = Some([add(at, o), add(bt, o), add(bt, o2), add(at, o2)]);
                }
                deck = Some(d);
            } else {
                back = Some([add(pa, off), add(pb, off)]);
            }
            let band = q.and_then(|q| Band::of(q.n, q.over, q.slack, q.ut()));
            members.push(MemberView {
                index,
                id: m.id.clone(),
                kind: m.kind,
                a: pa,
                b: pb,
                wa: a,
                wb: b,
                sag,
                band,
                deck,
                back,
                temp: q.map_or(20.0, |q| q.temp),
            });
        }

        // terrain
        let terrain = terrain(&cam, &setup, off);

        // foundations
        let mut foundations = Vec::new();
        for (i, n) in design.nodes.iter().enumerate() {
            let sup = match sim {
                Some(s) if s.p.get(i).is_some_and(|p| p.ripped) => None,
                _ => n.sup,
            };
            let Some(kind) = sup else { continue };
            let p = px(pos[i]);
            let (x, y) = (p[0], p[1]);
            let pw = 12f64.max(4.0 * sc);
            let ph = 4f64.max(sc);
            let pile = (kind == Foundation::Pile).then(|| {
                let w = 4f64.max(0.8 * sc);
                [x - w / 2.0, y + ph, w, 14.0 * sc]
            });
            foundations.push(FoundationView {
                joint: n.id.clone(),
                kind,
                at: p,
                pad: [x - pw / 2.0, y, pw, ph],
                pile,
                tri: [[x, y - 1.0], [x - 6.0, y + 6.0], [x + 6.0, y + 6.0]],
            });
        }

        // crates
        let cs = 0.5f64.max(1.6f64.min(sc / 18.0));
        let mut crates = Vec::new();
        for (id, t) in &design.loads {
            let Some(p) = pos_of(id) else { continue };
            let p = px(p);
            let (x, y) = (p[0], p[1]);
            let w = (12.0 + t.sqrt() * 4.0) * cs;
            let h = w * 0.8;
            let top = y + 9.0 * cs;
            crates.push(CrateView {
                joint: id.clone(),
                t: *t,
                wire: [[x, y + 3.0], [x, top]],
                body: [x - w / 2.0, top, w, h],
                top: [
                    [x - w / 2.0, top],
                    [x - w / 2.0 + 6.0, top - 4.0],
                    [x + w / 2.0 + 6.0, top - 4.0],
                    [x + w / 2.0, top],
                ],
                side: [
                    [x + w / 2.0, top],
                    [x + w / 2.0 + 6.0, top - 4.0],
                    [x + w / 2.0 + 6.0, top - 4.0 + h],
                    [x + w / 2.0, top + h],
                ],
                tag: format!("{} t", to_string(*t)),
                tag_at: [x, top + h + 10.0],
            });
        }

        // vehicles
        let waiting;
        let vlist: &[super::sim::Vehicle] = if !river {
            &[]
        } else if let Some(s) = sim {
            &s.v
        } else {
            waiting = Sim::new(design, &setup, &cond, false).v;
            &waiting
        };
        let vehicles = vlist
            .iter()
            .filter(|v| !(v.state == VehicleState::Done && v.x > 64.0))
            .map(|v| vehicle_view(v, &cam))
            .collect();

        // weather
        let wind_now = sim.map_or(cond.wind, |s| s.wind_now);
        let dir = if cond.dir == 0.0 || cond.dir.is_nan() {
            1.0
        } else {
            cond.dir
        };
        let mut wind = Vec::new();
        if wind_now > 0.5 {
            let flat = setup.scene == Scene::Flat;
            let z_top = if flat {
                design
                    .nodes
                    .iter()
                    .map(|n| n.y)
                    .fold(setup.target.unwrap_or(0.0).max(24.0), f64::max)
            } else {
                14.0
            };
            let step = if flat { 12.0 } else { 4.0 };
            let mut z: f64 = if flat { 8.0 } else { 2.0 };
            let mut guard = 0;
            while z <= z_top + 4.0 && guard < 1000 {
                guard += 1;
                let vz = wind_now * super::jsmath::pow02(z.max(2.0) / 10.0);
                let len = 220f64.min(vz * vz / 6.0);
                let y = cam.y(z);
                if (0.0..=600.0).contains(&y) {
                    let x0 = if dir > 0.0 { 12.0 } else { 1108.0 };
                    let x1 = x0 + dir * len;
                    let streak = if dir > 0.0 {
                        [
                            [260.0 + (z * 37.0) % 120.0, y - 8.0],
                            [500.0, y - 30.0],
                            [800.0, y + 20.0],
                            [1120.0, y - 4.0],
                        ]
                    } else {
                        [
                            [860.0 - (z * 37.0) % 120.0, y - 8.0],
                            [620.0, y - 30.0],
                            [320.0, y + 20.0],
                            [0.0, y - 4.0],
                        ]
                    };
                    wind.push(WindArrow {
                        from: [x0, y],
                        to: [x1, y],
                        head: [
                            [x1 - dir * 7.0, y - 5.0],
                            [x1, y],
                            [x1 - dir * 7.0, y + 5.0],
                        ],
                        streak,
                    });
                }
                z += step;
            }
        }
        let mut flakes = Vec::new();
        if cond.snow > 0.0 {
            let nfl = if cond.snow >= 800.0 {
                220
            } else if cond.snow >= 400.0 {
                140
            } else {
                70
            };
            for i in 0..nfl {
                let i = i as f64;
                flakes.push([(i * 61.7) % 1120.0, ((i * 97.1) % 660.0) - 60.0]);
            }
        }
        let target_y = setup.target.filter(|&t| t != 0.0).map(|t| cam.y(t));
        let mut sparks = Vec::new();
        let mut splashes = Vec::new();
        if let Some(s) = sim {
            sparks = s.sparks.iter().map(|k| px(*k)).collect();
            for SimSplash { x, t } in &s.splashes {
                let age = s.t - t;
                if age > 1.6 {
                    continue;
                }
                splashes.push(SplashView {
                    at: [cam.x(*x), cam.y(-8.0)],
                    age,
                    r: (8.0 + age * 30.0) * cs,
                });
            }
        }
        let wobbly = pv.is_some_and(|p| p.moved > 1.5) && !design.members.is_empty();
        let found_over: Vec<String> = pv.map_or(vec![], |p| {
            p.found_over().into_iter().map(|f| f.id.clone()).collect()
        });
        let moved_id = pv.and_then(|p| p.moved_id.clone());
        let mut rings = Vec::new();
        if wobbly
            && let Some(id) = &moved_id
            && let Some(p) = pos_of(id)
        {
            rings.push(px(p));
        }
        for id in &found_over {
            if let Some(p) = pos_of(id) {
                rings.push(px(p));
            }
        }
        let tool = self.tool();
        let (mut ghost, mut hover) = (None, None);
        if building
            && let Some(h) = self.hover()
            && matches!(tool, Tool::Build | Tool::Found)
        {
            let hp = px(h);
            hover = Some(hp);
            if tool == Tool::Build
                && let Some(pid) = self.pending()
                && let Some(p) = pos_of(pid)
            {
                ghost = Some([px(p), hp]);
            }
        }

        // handles and joints
        let nums = self.show_forces();
        let show_handles = building && (matches!(tool, Tool::Select | Tool::Erase) || nums);
        let sel = self.selection();
        let mut handles = Vec::new();
        if show_handles {
            for m in &design.members {
                let (Some(a), Some(b)) = (pos_of(&m.a), pos_of(&m.b)) else {
                    continue;
                };
                let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
                if !cam.on_screen(mid) {
                    continue;
                }
                let q = info(&m.id);
                let ut = q.map_or(0.0, |q| q.ut());
                handles.push(HandleView {
                    id: m.id.clone(),
                    at: px(mid),
                    text: match q {
                        Some(q) if nums => {
                            if q.slack {
                                "slack".into()
                            } else {
                                fmt_t(q.n)
                            }
                        }
                        _ => String::new(),
                    },
                    label: format!(
                        "{} {}{}",
                        m.kind.name(),
                        m.id,
                        if q.is_some() {
                            format!(", {}% of its limit", to_string(round(ut * 100.0)))
                        } else {
                            String::new()
                        }
                    ),
                    num: nums,
                    selected: sel == Some(&Selection::Member(m.id.clone())),
                    over: q.is_some_and(|q| q.over || ut > 1.0),
                });
            }
        }
        let mut joints = Vec::new();
        if !live {
            for (i, n) in design.nodes.iter().enumerate() {
                let p = pos[i];
                if !cam.on_screen(p) {
                    continue;
                }
                let load = design.load(&n.id);
                joints.push(JointView {
                    id: n.id.clone(),
                    at: px(p),
                    pos: p,
                    sup: n.sup,
                    label: format!(
                        "Joint {}{}{}",
                        n.id,
                        n.sup
                            .map_or(String::new(), |f| format!(", on a {}", f.key())),
                        if load != 0.0 {
                            format!(", carrying a {} t crate", to_string(load))
                        } else {
                            String::new()
                        }
                    ),
                    founded: n.sup.is_some(),
                    locked: n.locked,
                    pending: self.pending() == Some(n.id.as_str()),
                    selected: sel == Some(&Selection::Node(n.id.clone())),
                    bad: (wobbly && moved_id.as_deref() == Some(n.id.as_str()))
                        || found_over.contains(&n.id),
                });
            }
        }

        // status
        let cost = self.cost();
        let mut worst_id: Option<&str> = None;
        let mut worst_ut = 0.0;
        for (id, q) in &infos {
            let u = q.ut();
            if u > worst_ut {
                worst_ut = u;
                worst_id = Some(id);
            }
        }
        let over_any = pv.is_some_and(|p| p.over_any());
        let design_top = design.top();
        let has_members = !design.members.is_empty();
        let mut status = Status {
            label: "Nothing built yet".into(),
            tone: StatusTone::Muted,
        };
        if has_members {
            status = if wobbly {
                Status {
                    label: "Sags and folds".into(),
                    tone: StatusTone::Danger,
                }
            } else if over_any {
                Status {
                    label: if cond.snow != 0.0 {
                        "Breaks under its weight and snow"
                    } else {
                        "Breaks under its own weight"
                    }
                    .into(),
                    tone: StatusTone::Danger,
                }
            } else if !found_over.is_empty() {
                Status {
                    label: "A foundation is overloaded".into(),
                    tone: StatusTone::Danger,
                }
            } else if worst_ut > 0.7 {
                Status {
                    label: "Stands, but strained".into(),
                    tone: StatusTone::Warn,
                }
            } else {
                Status {
                    label: "Stands up".into(),
                    tone: StatusTone::Ok,
                }
            };
        }
        if running {
            let s = sim;
            status = Status {
                label: if s.is_some_and(|s| s.env > 0.2) {
                    "The ground is shaking"
                } else if s.is_some_and(|s| s.wind_now > 20.0) {
                    "Riding out the wind"
                } else if river {
                    "Traffic is crossing"
                } else {
                    "Under test"
                }
                .into(),
                tone: StatusTone::Running,
            };
        }

        // explain
        let r = self.result();
        let explain = self.explain(
            &setup,
            sim,
            pv,
            wobbly,
            over_any,
            &found_over,
            design_top,
            worst_id,
            worst_ut,
        );

        // inspector
        let mut inspector = None;
        if !live {
            match sel {
                Some(Selection::Member(id)) => {
                    if let Some(m) = design.member(id) {
                        let q = info(&m.id);
                        let ut = q.map_or(0.0, |q| q.ut());
                        let text = match q {
                            None => String::new(),
                            Some(q) => {
                                let base = if q.slack {
                                    "Slack: it is being pushed, so it carries nothing. Ropes and cables only pull.".to_string()
                                } else if q.n.abs() < ZERO_FORCE_N {
                                    "Doing almost nothing. A candidate to delete.".to_string()
                                } else if q.n >= 0.0 {
                                    format!(
                                        "Pulled with {} t of force. It breaks at {} t.",
                                        to_fixed(q.n / 9810.0, 1),
                                        to_fixed(q.cap_t / 9810.0, 0)
                                    )
                                } else {
                                    format!(
                                        "Pushed with {} t of force. It gives way at {} t.",
                                        to_fixed(-q.n / 9810.0, 1),
                                        to_fixed(q.cap_c / 9810.0, 0)
                                    )
                                };
                                format!(
                                    "{}{} Running the test adds more.",
                                    base,
                                    if q.exposed && cond.snow != 0.0 {
                                        " Snow is piling on it."
                                    } else {
                                        ""
                                    }
                                )
                            }
                        };
                        inspector = Some(Inspector::Piece(PieceInspector {
                            id: m.id.clone(),
                            kind: m.kind,
                            title: format!("{} · {}", m.kind.name(), m.id),
                            len: match q {
                                Some(q) if q.len != 0.0 && !q.len.is_nan() => {
                                    format!("{} m", to_fixed(q.len, 1))
                                }
                                _ => String::new(),
                            },
                            fill: to_fixed(100f64.min(ut * 100.0), 0).parse().unwrap_or(0.0),
                            ut: format!("{}%", to_string(round(ut * 100.0))),
                            tone: if q.is_some_and(|q| q.over || ut > 1.0) {
                                BarTone::Over
                            } else if q.is_some_and(|q| q.n >= 0.0) {
                                BarTone::Pulled
                            } else {
                                BarTone::Pushed
                            },
                            text,
                            types: PieceType::for_scene(setup.scene)
                                .into_iter()
                                .map(|k| Chip {
                                    value: k,
                                    name: k.name().into(),
                                    on: m.kind == k,
                                    dim: false,
                                })
                                .collect(),
                            locked: m.locked,
                        }));
                    }
                }
                Some(Selection::Node(id)) => {
                    if let Some(n) = design.node(id) {
                        let can = setup.sup_ok(n.x, n.y);
                        let f = pv.and_then(|p| p.foundation(&n.id));
                        let soil_n = setup.soil_at(n);
                        let fs = soil_n.grip(n.sup.unwrap_or(Foundation::Footing));
                        let note = match n.sup {
                            Some(k) => format!(
                                "{} under a {}: holds up to {} t pushing down and {} t pulling up.{}",
                                soil_n.name(),
                                k.key(),
                                to_fixed(fs[0] / 9810.0, 0),
                                to_fixed(fs[1] / 9810.0, 0),
                                f.map_or(String::new(), |f| format!(
                                    " Carrying {} t right now.",
                                    to_fixed(f.r / 9810.0, 1)
                                ))
                            ),
                            None if can => "On the ground: you can put a foundation here.".into(),
                            None => "In the air.".into(),
                        };
                        let opts: [(Option<Foundation>, &str); 3] = [
                            (None, "None"),
                            (Some(Foundation::Footing), "Footing"),
                            (Some(Foundation::Pile), "Pile"),
                        ];
                        inspector = Some(Inspector::Joint(JointInspector {
                            id: n.id.clone(),
                            title: format!(
                                "Joint {}{}",
                                n.id,
                                if n.locked { " · level foundation" } else { "" }
                            ),
                            pos: format!("x {} m · y {} m", to_string(n.x), to_string(n.y)),
                            founds: opts
                                .iter()
                                .map(|&(v, name)| Chip {
                                    value: v,
                                    name: name.into(),
                                    on: n.sup == v,
                                    dim: (v.is_some() && !can) || n.locked,
                                })
                                .collect(),
                            load: format!("{} t", to_string(design.load(&n.id))),
                            note,
                            can_delete: !n.locked,
                        }));
                    }
                }
                None => {}
            }
        }
        let inspector = inspector.unwrap_or_else(|| {
            Inspector::Materials(
                PieceType::for_scene(setup.scene)
                    .into_iter()
                    .map(|k| MaterialCard {
                        kind: k,
                        name: k.name(),
                        curve: curve_for(k),
                        curve_lines: curve_lines(k),
                        blurb: format!("{} Max {} m.", k.spec().blurb, to_string(k.spec().max_len)),
                        cost: format!("${}/m", to_string(k.spec().cost)),
                    })
                    .collect(),
            )
        });

        // rail, hint bar, aside
        let tools = Tool::ALL
            .into_iter()
            .map(|t| ToolView {
                tool: t,
                name: t.name(),
                tip: t.tip(),
                icon: t.icon(),
                on: tool == t,
            })
            .collect();
        let hint = if building {
            tool.hint(self.pending().is_some())
        } else {
            "Building is paused while it runs. You can still change the conditions, zoom and pan."
        };
        let practice = self.practice();
        let mut chips_info: Vec<String> = Vec::new();
        if river {
            chips_info.extend(setup.traffic.iter().map(|k| k.spec().label.to_string()));
        } else if let Some(t) = setup.target.filter(|&t| t != 0.0) {
            chips_info.push(format!("Reach {} m", to_string(t)));
        }
        if practice {
            chips_info.push("Practice: easier than the level".into());
        }
        let mut hud = Vec::new();
        if setup.scene == Scene::Flat {
            let now_top = match sim {
                Some(s) => s.top_i.map_or(design_top, |i| s.p[i].y),
                None => design_top,
            };
            let target = setup.target.filter(|&t| t != 0.0);
            hud.push(HudChip {
                key: "Height ",
                value: format!(
                    "{} m{}",
                    to_fixed(now_top, 0),
                    target.map_or(String::new(), |t| format!(" / {}", to_string(t)))
                ),
                tone: if target.is_some_and(|t| design_top >= t) {
                    HudTone::Good
                } else {
                    HudTone::Plain
                },
            });
            if let Some(s) = sim {
                let mg = s.milli_g();
                hud.push(HudChip {
                    key: "Sway ",
                    value: format!("{} m", to_fixed(s.sway, 2)),
                    tone: HudTone::Plain,
                });
                hud.push(HudChip {
                    key: "Comfort ",
                    value: format!("{} mg", to_fixed(mg, 0)),
                    tone: if setup.comfort.is_some_and(|c| c != 0.0 && mg > c) {
                        HudTone::Bad
                    } else {
                        HudTone::Plain
                    },
                });
            }
        }
        if wind_now > 0.5 {
            hud.push(HudChip {
                key: "Wind ",
                value: format!(
                    "{} m/s {}",
                    to_fixed(wind_now, 0),
                    if dir > 0.0 { "→" } else { "←" }
                ),
                tone: HudTone::Wind,
            });
        }
        if snow_now > 1.0 {
            hud.push(HudChip {
                key: "Snow ",
                value: format!("{} kg/m", to_fixed(snow_now, 0)),
                tone: HudTone::Snow,
            });
        }
        if let Some(s) = sim {
            if s.env > 0.05 {
                hud.push(HudChip {
                    key: "Shaking ",
                    value: format!("{}%", to_string(round(s.env * 100.0))),
                    tone: HudTone::Shake,
                });
            }
            if river && s.sandbox {
                hud.push(HudChip {
                    key: "Crossings ",
                    value: s.laps().to_string(),
                    tone: HudTone::Plain,
                });
            }
        }
        let crossed_now = match sim {
            Some(s) if river => s.crossed(),
            _ => 0,
        };
        let run_name = if river { "Run traffic" } else { "Run the test" };
        let budget = setup.budget.filter(|&b| b != 0.0);
        let result = r.map(|r| ResultView {
            title: r.title(&setup),
            pass: r.pass,
            stars: r.stars_text(&setup),
        });
        let level_groups = Group::ALL
            .into_iter()
            .map(|g| LevelGroup {
                group: g,
                name: g.name(),
                items: levels()
                    .into_iter()
                    .filter(|l| l.group == g)
                    .map(|l| LevelRow {
                        key: l.key,
                        name: l.name.clone(),
                        best: if l.budget.is_some_and(|b| b != 0.0) {
                            stars_text(self.best_for(l.key))
                        } else {
                            String::new()
                        },
                        on: self.level() == l.key,
                    })
                    .collect(),
            })
            .collect();
        let chip_row = |list: &[(f64, &str)], cur: f64| -> Vec<Chip<f64>> {
            list.iter()
                .map(|&(v, name)| Chip {
                    value: v,
                    name: name.into(),
                    on: cur == v,
                    dim: false,
                })
                .collect()
        };
        let conditions = ConditionsView {
            winds: chip_row(&WINDS, cond.wind),
            dirs: chip_row(&DIRS, dir),
            snows: chip_row(&SNOWS, cond.snow),
            quakes: chip_row(&QUAKES, cond.quake),
            soils: Soil::ALL
                .into_iter()
                .map(|k| Chip {
                    value: k,
                    name: k.name().into(),
                    on: cond.soil == k,
                    dim: running,
                })
                .collect(),
            traffic: TrafficSet::ALL
                .into_iter()
                .map(|k| Chip {
                    value: k,
                    name: k.name().into(),
                    on: cond.traffic == k,
                    dim: running,
                })
                .collect(),
            show_traffic: river,
            fires: chip_row(&FIRES, cond.fire),
            can_shake: running && cond.quake > 0.0,
        };
        let sim_t = sim.map(|s| s.t);
        View {
            level_slug: self.level().slug(),
            clock: if building {
                "building".into()
            } else if let Some(t) = sim_t {
                format!("t = {} s", to_fixed(t, 1))
            } else {
                String::new()
            },
            playing: running,
            hdr_run: run_name,
            phase,
            tools,
            can_undo: building && self.undo_depth() > 0,
            hint,
            show_types: tool == Tool::Build,
            types: PieceType::for_scene(setup.scene)
                .into_iter()
                .map(|k| TypeChip {
                    kind: k,
                    name: k.name(),
                    tip: k.spec().tip,
                    swatch_h: 3f64.max(k.spec().swatch_h),
                    on: self.piece_type() == k,
                })
                .collect(),
            show_founds: tool == Tool::Found,
            founds: Foundation::ALL
                .into_iter()
                .map(|f| FoundChip {
                    kind: f,
                    name: f.chip(),
                    tip: f.tip(),
                    on: self.found_kind() == f,
                })
                .collect(),
            show_weights: tool == Tool::Crate,
            weights: WEIGHTS
                .iter()
                .map(|&w| Chip {
                    value: w,
                    name: format!("{} t", to_string(w)),
                    on: self.weight() == w,
                    dim: false,
                })
                .collect(),
            has_pending: self.pending().is_some() && building,
            show_forces: nums,
            camera: cam,
            grid: GridView {
                origin: [cam.x(0.0), cam.y(0.0)],
                step: cam.grid * sc,
            },
            terrain,
            flowing: running,
            members,
            snow_px,
            wind_now,
            wind_dir: dir,
            snow_now,
            quake_env: sim.map_or(0.0, |s| s.env),
            sim_t,
            foundations,
            crates,
            vehicles,
            wind,
            flakes,
            target_y,
            sparks,
            splashes,
            rings,
            ghost,
            hover,
            handles,
            joints,
            hud,
            shaking: running && sim.is_some_and(|s| s.env > 0.2),
            cursor: if !building {
                Cursor::Default
            } else {
                match tool {
                    Tool::Build | Tool::Found => Cursor::Crosshair,
                    Tool::Select => Cursor::Grab,
                    _ => Cursor::Default,
                }
            },
            stage_label: format!(
                "{} with {} joints and {} pieces. {}.",
                if river { "Bridge" } else { "Tower" },
                design.nodes.len(),
                design.members.len(),
                status.label
            ),
            surface_label: "Building grid",
            toast: self.toast().map(str::to_string),
            running,
            run_info: match sim {
                Some(s) if s.sandbox => {
                    format!("t = {} s · runs until you stop", to_fixed(s.t, 0))
                }
                _ if river => format!("{} / {} across", crossed_now, setup.traffic.len()),
                _ => format!(
                    "t = {} / 18 s",
                    sim_t.map_or("0".to_string(), |t| to_fixed(t, 1))
                ),
            },
            slow_label: if self.slow() {
                "Normal speed"
            } else {
                "Slow motion"
            },
            result,
            zoom_pct: format!("{}%", to_string(round(self.view_state().z * 100.0))),
            level_groups,
            goal: sp.goal.clone(),
            chips_info,
            cost_text: format!(
                "${}k{}",
                to_fixed(cost, 1),
                budget.map_or(String::new(), |b| format!(" / ${}k", to_string(b)))
            ),
            over_budget: budget.is_some_and(|b| cost > b),
            status,
            run_label: if building {
                run_name
            } else if running {
                "Stop"
            } else {
                "Back to building"
            },
            is_sandbox: self.level() == LevelKey::Sandbox,
            is_river: river,
            sb_scenes: [Scene::River, Scene::Flat]
                .into_iter()
                .map(|s| Chip {
                    value: s,
                    name: s.chip_name().into(),
                    on: self.sb_scene() == s,
                    dim: false,
                })
                .collect(),
            conditions,
            inspector,
            explain,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn explain(
        &self,
        setup: &Setup,
        sim: Option<&Sim>,
        pv: Option<&super::preview::Settled>,
        wobbly: bool,
        over_any: bool,
        found_over: &[String],
        design_top: f64,
        worst_id: Option<&str>,
        worst_ut: f64,
    ) -> String {
        let river = setup.scene == Scene::River;
        let cond = self.cond();
        let design = self.design();
        match self.phase() {
            Phase::Result => self.result().map_or(String::new(), |r| r.explain(setup)),
            Phase::Run => {
                let Some(s) = sim else { return String::new() };
                if let Some(ff) = &s.first_found {
                    format!("Foundation {} {}!", ff.id, ff.mode.text())
                } else if let Some(fb) = &s.first_break {
                    format!("{} just {}!", fb.id, fb.mode.text())
                } else {
                    format!(
                        "Change the weather while it runs: the wind builds up gradually, snow piles up on decks and roofs, and you can set off a quake at any time.{}",
                        if river && s.wind_now > 20.0 {
                            " Strong wind makes light road decks flutter."
                        } else {
                            ""
                        }
                    )
                }
            }
            Phase::Build => {
                let snowy = cond.snow != 0.0;
                if design.members.is_empty() {
                    if river {
                        "Pick Road and click from the left foundation across, joint by joint. Road alone hangs like a chain, so hold it up: Wood or Steel triangles, Concrete where it pushes, or hang it from Rope and Cable, which only pull. Zoom in for finer joints.".into()
                    } else {
                        "Build up from the foundations: Steel or Concrete for columns, Wood or Steel diagonals for bracing, and a Floor across every level you want to count. Reach the dashed line.".into()
                    }
                } else if let Some(p) = pv.filter(|_| wobbly) {
                    format!(
                        "Under its own weight this shape folds: joint {} drops {} m. Pin-jointed pieces need triangles, or something to hang from.",
                        p.moved_id.clone().unwrap_or_default(),
                        to_fixed(p.moved, 1)
                    )
                } else if over_any {
                    format!(
                        "Something breaks just holding itself up{} (red dashes). Make it shorter, or swap in Steel or Cable.",
                        if snowy {
                            " with this much snow on it"
                        } else {
                            ""
                        }
                    )
                } else if let Some(id) = found_over.first() {
                    let f = pv.and_then(|p| p.foundation(id));
                    format!(
                        "Foundation {} is {}. Try a Pile, or add more foundations to share the load.",
                        id,
                        if f.is_some_and(|f| f.over_t) {
                            "being pulled up harder than it can hold, so it would rip out"
                                .to_string()
                        } else {
                            format!(
                                "pushing on the soil harder than {} can take, so it would sink",
                                setup.soil.name().to_lowercase()
                            )
                        }
                    )
                } else if river && !design.has(PieceType::Road) {
                    "It stands, but there is no Road on it, so the vehicles have nothing to drive on.".into()
                } else if !river
                    && let Some(t) = setup.target.filter(|&t| t != 0.0)
                    && design_top < t
                {
                    format!(
                        "It holds its own weight. The top floor is at {} m; keep going to the {} m line.",
                        to_string(design_top),
                        to_string(t)
                    )
                } else {
                    format!(
                        "It holds its own weight{}. Busiest piece: {} at {}% of its limit. Now run it.",
                        if snowy { " and the snow" } else { "" },
                        worst_id.unwrap_or("null"),
                        to_string(round(worst_ut * 100.0))
                    )
                }
            }
        }
    }
}

fn fb_text(fb: &super::sim::FirstBreak) -> String {
    format!(
        "{} ({}) {} first. {}",
        fb.id,
        fb.kind.name().to_lowercase(),
        fb.mode.text(),
        tip(fb.kind, fb.mode)
    )
}

fn ff_text(ff: &super::sim::FirstFound) -> String {
    format!(
        "Foundation {} {} first. {}",
        ff.id,
        ff.mode.text(),
        ftip(ff.mode)
    )
}

impl RunResult {
    /// The result banner's title (`resultTitle`): "All across!", "5 of 8 made it", "It stood!",
    /// "It came down", "Too short", "Too wobbly to live in", "…, but over budget".
    pub fn title(&self, setup: &Setup) -> String {
        let over = setup
            .budget
            .filter(|&b| b != 0.0)
            .is_some_and(|b| self.cost > b);
        if self.years.is_some() {
            return if self.pass {
                if over {
                    "Straight enough, but over budget"
                } else {
                    "Straight enough!"
                }
            } else {
                "It still leans"
            }
            .to_string();
        }
        match &self.outcome {
            Outcome::River { crossed, total } => {
                if self.pass {
                    if over {
                        "All across, but over budget".to_string()
                    } else {
                        "All across!".to_string()
                    }
                } else {
                    format!("{crossed} of {total} made it")
                }
            }
            Outcome::Tower {
                standing, reached, ..
            } => if self.pass {
                if over {
                    "It stood, but over budget"
                } else {
                    "It stood!"
                }
            } else if !standing {
                "It came down"
            } else if !reached {
                "Too short"
            } else {
                "Too wobbly to live in"
            }
            .to_string(),
        }
    }

    /// The banner's star row: "★★☆" for a passing level run, "practice", or empty.
    pub fn stars_text(&self, setup: &Setup) -> String {
        if setup.budget.is_some_and(|b| b != 0.0) && self.pass && !self.practice {
            stars_text(self.stars)
        } else if self.practice {
            "practice".into()
        } else {
            String::new()
        }
    }

    /// "What's happening" after the run (`explain` in the result phase).
    pub fn explain(&self, setup: &Setup) -> String {
        let river = setup.scene == Scene::River;
        let budget = setup.budget.filter(|&b| b != 0.0);
        if let Some(y) = self.years {
            return format!(
                "After 30 years: tilt {}°, settled {} mm.",
                to_fixed(y.tilt, 1),
                to_fixed(y.settled * 1000.0, 0)
            );
        }
        let pr = if self.practice {
            " This was a practice run: the conditions were easier than the level asks, so no stars."
        } else {
            ""
        };
        if self.pass {
            let lead = if river {
                "Everyone made it. "
            } else {
                "It stood through the whole test. "
            };
            let mid = match budget {
                Some(b) if !self.practice => {
                    if self.cost <= b {
                        if self.stars < 3 {
                            "Under budget. Trim it for more stars: delete pieces that show almost no force."
                        } else {
                            "Three stars. Nicely done."
                        }
                    } else {
                        "Over budget though, so no stars."
                    }
                }
                _ => "",
            };
            return format!("{lead}{mid}{pr}");
        }
        let why = match (&self.first_found, &self.first) {
            (Some(ff), _) => ff_text(ff),
            (None, Some(fb)) => fb_text(fb),
            _ => String::new(),
        };
        match &self.outcome {
            Outcome::River { crossed, total } => format!(
                "{crossed} of {total} made it. {}{pr}",
                if why.is_empty() {
                    "Nothing broke: a vehicle ran out of road. Make sure the Road runs all the way across.".to_string()
                } else {
                    why
                }
            ),
            Outcome::Tower {
                standing,
                reached,
                mg,
                height,
                ..
            } => {
                if !reached {
                    format!(
                        "It stood, but the top floor is at {} m. The goal is {} m.{pr}",
                        to_fixed(*height, 0),
                        setup.target.map_or("null".into(), to_string)
                    )
                } else if !standing {
                    format!("It came down. {why}{pr}")
                } else {
                    format!(
                        "It stood, but the top floor swayed hard enough to feel {} milli-g (the limit is {}). A wider base, diagonal bracing or a heavy crate hung on a rope near the top (a pendulum damper) all calm it down.{pr}",
                        to_fixed(*mg, 0),
                        setup.comfort.map_or("null".into(), to_string)
                    )
                }
            }
        }
    }
}

fn terrain(cam: &Camera, setup: &Setup, off: Pt) -> Terrain {
    let px = |x: f64, y: f64| [cam.x(x), cam.y(y)];
    let poly = |pts: &[(f64, f64)]| pts.iter().map(|&(x, y)| px(x, y)).collect::<Vec<Pt>>();
    let y0 = cam.y(0.0);
    let river = setup.scene == Scene::River;
    let mut t = Terrain {
        scene: setup.scene,
        soil: setup.soil,
        bed: setup.bed,
        soil_poly: vec![],
        bed_poly: None,
        water_poly: None,
        flow: vec![],
        tops: vec![],
        edges: vec![],
        grains: vec![],
    };
    if river {
        t.soil_poly = poly(&[
            (-300.0, 0.0),
            (8.0, 0.0),
            (7.6, -12.0),
            (48.4, -12.0),
            (48.0, 0.0),
            (400.0, 0.0),
            (400.0, -90.0),
            (-300.0, -90.0),
        ]);
        t.bed_poly = Some(poly(&[
            (7.6, -12.0),
            (48.4, -12.0),
            (48.4, -14.0),
            (7.6, -14.0),
        ]));
        t.water_poly = Some(poly(&[
            (8.0, -8.0),
            (48.0, -8.0),
            (48.4, -12.0),
            (7.6, -12.0),
        ]));
        t.flow = vec![
            [px(10.0, -8.8), [cam.x(46.0), cam.y(-8.8)]],
            [px(12.0, -10.0), [cam.x(47.0), cam.y(-10.0)]],
            [px(9.0, -11.2), [cam.x(45.0), cam.y(-11.2)]],
        ];
        t.tops = vec![
            [
                [cam.x(-300.0), y0],
                [cam.x(8.0), y0],
                [cam.x(8.0) + off[0], y0 + off[1]],
                [cam.x(-300.0), y0 + off[1]],
            ],
            [
                [cam.x(48.0), y0],
                [cam.x(400.0), y0],
                [cam.x(400.0), y0 + off[1]],
                [cam.x(48.0) + off[0], y0 + off[1]],
            ],
        ];
        t.edges = vec![
            [[cam.x(-300.0), y0], [cam.x(8.0), y0]],
            [[cam.x(48.0), y0], [cam.x(400.0), y0]],
        ];
    } else {
        t.soil_poly = poly(&[(-400.0, 0.0), (600.0, 0.0), (600.0, -90.0), (-400.0, -90.0)]);
        t.tops = vec![[
            [cam.x(-400.0), y0],
            [cam.x(600.0), y0],
            [cam.x(600.0), y0 + off[1]],
            [cam.x(-400.0), y0 + off[1]],
        ]];
        t.edges = vec![[[cam.x(-400.0), y0], [cam.x(600.0), y0]]];
    }
    let span = if river { (-20.0, 76.0) } else { (-40.0, 240.0) };
    for i in 0..160 {
        let i = i as f64;
        let wx = span.0 + ((i * 37.3) % (span.1 - span.0));
        let wy = -1.0 - ((i * 13.7) % 30.0);
        if river && wx > 7.5 && wx < 48.5 && wy > -12.0 {
            continue;
        }
        t.grains.push(px(wx, wy));
    }
    t
}

fn vehicle_view(v: &super::sim::Vehicle, cam: &Camera) -> VehicleView {
    let (ca, sa) = (super::jsmath::cos(v.ang), super::jsmath::sin(v.ang));
    let p2 = |u: f64, w: f64| cam.px([v.x + u * ca - w * sa, v.y + 0.35 + u * sa + w * ca]);
    let rect = |u0: f64, w0: f64, u1: f64, w1: f64| -> Quad {
        [p2(u0, w0), p2(u1, w0), p2(u1, w1), p2(u0, w1)]
    };
    let hl = v.len / 2.0;
    let mut out = VehicleView {
        kind: v.kind,
        state: v.state,
        x: v.x,
        y: v.y,
        ang: v.ang,
        body: vec![],
        cab: None,
        windows: vec![],
        wheels: [
            (p2(-v.d, 0.35), 0.42 * cam.sc),
            (p2(v.d, 0.35), 0.42 * cam.sc),
        ],
    };
    match v.kind {
        VehicleKind::Car => {
            out.body.push(rect(-hl, 0.2, hl, 0.8));
            out.body.push(rect(-hl * 0.5, 0.8, hl * 0.45, v.ht));
            out.windows
                .push(rect(-hl * 0.4, 0.9, hl * 0.35, v.ht - 0.12));
        }
        VehicleKind::Bus => {
            out.body.push(rect(-hl, 0.25, hl, v.ht));
            for i in 0..6 {
                let u = -hl + 0.8 + i as f64 * (v.len - 1.6) / 6.0;
                out.windows.push(rect(u, 1.6, u + 1.2, 2.6));
            }
        }
        VehicleKind::Truck => {
            out.body.push(rect(-hl, 0.4, hl - 2.6, v.ht));
            out.cab = Some(rect(hl - 2.4, 0.3, hl, 2.6));
            out.windows.push(rect(hl - 1.3, 1.5, hl - 0.15, 2.4));
        }
    }
    out
}

/// The stage size, px (for the screen's mapping).
pub const STAGE: [f64; 2] = [STAGE_W, STAGE_H];
