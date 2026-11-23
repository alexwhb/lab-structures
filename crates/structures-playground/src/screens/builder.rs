//! SCR-1 the Playground: one builder for bridges and towers (SPEC "Builder", BLD-1…BLD-15;
//! `docs/design/v2/mockup/Builder.dc.html`).
//!
//! All logic, every number and every sentence live in [`sp_engine::builder`] ([`Game`], a port
//! of the mockup's `class Component`); this module draws [`Game::view`] and forwards clicks,
//! drags, wheel and keys:
//!
//! - the row (`Tools` rail 76 px · `Stage` · `Level, site and inspector` aside) on
//!   [`layout::columns`]: side by side at 1440, stacked in the mockup's order at 390;
//! - [`rail`] — the tool rail; [`hint`] — the hint bar; [`stage`] — the stage (camera, the SVG
//!   layer painted by [`scene`], joints and handles as AccessKit buttons, HUD, toast, run
//!   overlay, result banner); [`legend`] — legend, zoom controls, provenance (PROV-1);
//!   [`aside`] — levels, goal panel, Sandbox scenes, Conditions, inspector / Materials, "Open in
//!   Advanced", What's happening; [`bits`] / [`paint`] — controls and painting primitives;
//! - [`host`] — the fold-in hooks ([`host::FoldIn`]); the fold-in UIs are
//!   [`comfort`] / [`time`] (agent `time`) and [`lab`] / [`fire`] (agent `lab`);
//! - header hooks: "Run traffic" / "Run the test" / Pause, Reset, the clock, the document name
//!   (`river-level` … / `sandbox`), Share (the structure as a `.str` netlist); ⌘V loads a
//!   netlist into the Sandbox, ⌘Z, Delete, ⌘= / ⌘− / ⌘0, arrows pan, ⌥ arrows nudge, Esc;
//!   "Open in Advanced" / Advanced's "Open in Playground" hand-offs (NAV-4).
//!
//! Frame pacing (PLAT-3, BLD-15): ~30 frames a second while running or while a fold-in
//! animates, a single wake-up when a toast fades, nothing otherwise.

pub mod aside;
pub mod bits;
pub mod comfort;
pub mod fire;
pub mod hint;
pub mod host;
pub mod lab;
pub mod legend;
pub mod paint;
pub mod rail;
pub mod scene;
pub mod stage;
pub mod time;

use crate::ctx::{Carry, Command, Ctx, Ticker};
use crate::layout::{self, Column};
use crate::router::Route;
use crate::screens::{Drive, Screen, Share};
use eframe::egui::{Id, Ui};
use host::FoldIns;
use sp_engine::builder::{
    Foundation, Game, LevelKey, Phase, PieceType, Scene, Selection, Soil, Tool, TrafficSet, View,
};

/// Something the player did this frame (applied after drawing, so a frame draws one state).
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Pick a tool.
    Tool(Tool),
    /// Pick the piece type to build.
    Piece(PieceType),
    /// Pick the foundation kind (Found tool).
    Found(Foundation),
    /// Pick the crate weight, t.
    Weight(f64),
    /// "Finish line".
    Finish,
    /// "Show forces".
    ToggleForces,
    /// Undo.
    Undo,
    /// Clear ("Start this level again").
    Clear,
    /// Pick a level.
    Level(LevelKey),
    /// Pick the Sandbox scene.
    Scene(Scene),
    /// The goal panel's Run / Stop / Back to building.
    RunToggle,
    /// The result banner's "Back to building".
    BackToBuild,
    /// "Replay in slow motion".
    Replay,
    /// "Slow motion" / "Normal speed".
    ToggleSlow,
    /// Wind speed, m/s.
    Wind(f64),
    /// Wind direction (1 from the left, −1 from the right).
    Dir(f64),
    /// Snow, kg/m.
    Snow(f64),
    /// Earthquake, g.
    Quake(f64),
    /// Soil.
    Soil(Soil),
    /// Traffic.
    Traffic(TrafficSet),
    /// "Shake now".
    ShakeNow,
    /// A joint was clicked.
    ClickJoint(String),
    /// A piece's handle was clicked.
    ClickHandle(String),
    /// The grid was clicked at stage px.
    ClickSurface(f64, f64),
    /// Pointer down on a joint (drag start).
    JointDown(String),
    /// Pointer down on empty stage at stage px (pan start).
    SurfaceDown(f64, f64),
    /// The pointer moved over the stage, stage px.
    PointerMove(f64, f64),
    /// Pointer released.
    PointerUp,
    /// The pointer left the stage.
    ClearHover,

    /// "−".
    ZoomOut,
    /// "+".
    ZoomIn,
    /// "Fit".
    Fit,
    /// The wheel or a pinch on the stage: zoom by the factor about stage px.
    Pinch(f64, f64, f64),
    /// Inspector foundation chip.
    SetFoundation(String, Option<Foundation>),
    /// Inspector "Lighter crate".
    CrateLess(String),
    /// Inspector "Heavier crate".
    CrateMore(String),
    /// Inspector material chip.
    SetPieceType(String, PieceType),
    /// "Delete piece".
    DeletePiece(String),
    /// "Delete joint".
    DeleteJoint(String),
    /// A Materials card (opens the sample drawer, LAB-1).
    Sample(PieceType),
    /// "Open in Advanced".
    OpenInAdvanced,
}

/// The row: tool rail · stage · aside. The rail is 77 px, not its 76 px basis: Chrome's
/// automatic minimum size for a flex item is its min-content width, the 60 px tool buttons +
/// 2 × 8 px padding + the 1 px border (measured in `BridgeBuild-1440.png`).
const COLS: [Column; 3] = [
    Column::rail("Tools").min(77.0),
    Column::main("Stage"),
    Column::readouts("Level, site and inspector"),
];

/// The key of the best stars per level in egui's persisted memory (`Vec<(level key, stars)>`,
/// saved by eframe with the window, so they survive a restart; BLD-10, NAV-7).
pub const BEST_STARS_KEY: &str = "structures-playground/builder/best-stars";

fn best_id() -> Id {
    Id::unique(BEST_STARS_KEY)
}

/// What the fold-ins' [`host::FoldIn::reset`] watches: a new level or scene, or back to
/// building.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Mark {
    level: LevelKey,
    scene: Scene,
    phase: Phase,
}

/// SCR-1 state.
#[derive(Debug)]
pub struct BuilderScreen {
    /// The game (public for tests and MCP: `app.screens_mut().builder.game`).
    pub game: Game,
    /// The fold-ins (comfort, time, lab, fire).
    pub fold: FoldIns,
    ticker: Ticker,
    /// Water / wind dash offset, stage px (grows while running).
    flow: f32,
    /// Snowfall cycle 0…1 (moves while running).
    fall: f32,
    /// Quake shake cycle 0…1.
    shake: f32,
    /// The best stars last written to persisted memory.
    saved_best: Vec<(String, u8)>,
    mark: Mark,
    /// The last view drawn (the header's run label and clock read it).
    view: View,
}

impl Default for BuilderScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl BuilderScreen {
    /// The builder in build mode on level 1 (SPEC MB-10: the app starts in build mode).
    pub fn new() -> Self {
        let game = Game::new();
        let view = game.view();
        let mark = Mark {
            level: game.level(),
            scene: game.scene(),
            phase: game.phase(),
        };
        Self {
            game,
            fold: FoldIns::default(),
            ticker: Ticker::default(),
            flow: 0.0,
            fall: 0.0,
            shake: 0.0,
            saved_best: Vec::new(),
            mark,
            view,
        }
    }

    /// Whether a run is going (the header says "Pause").
    pub fn running(&self) -> bool {
        self.game.phase() == Phase::Run
    }

    /// The structure as a `.str` netlist with the level and conditions as comments (Share,
    /// "Open in Advanced").
    pub fn netlist_text(&self) -> String {
        self.game.share_text()
    }

    /// Loads a netlist into the Sandbox (⌘V, "Open in Playground"), toasting the outcome.
    fn load_netlist(&mut self, text: &str, ctx: &mut Ctx) {
        match self.game.paste_netlist(text) {
            Ok(()) => {
                let n = self.game.design().nodes.len();
                ctx.toast(format!("Loaded a {n}-joint structure into the Sandbox"));
            }
            Err(e) => ctx.toast(format!("Couldn't load that netlist: {e}")),
        }
        self.ticker.reset();
        self.reset_fold_ins();
    }

    /// Recomputes the cached view (after the state changed outside `ui`).
    fn refresh_view(&mut self) {
        self.view = self.adjusted_view();
    }

    /// The engine's view with the fold-ins' adjustments (what the stage, the header and the
    /// MCP summary show).
    pub fn adjusted_view(&self) -> View {
        let mut view = self.game.view();
        for f in self.fold.each_ref() {
            f.adjust_view(&self.game, &mut view);
        }
        view
    }

    fn reset_fold_ins(&mut self) {
        let g = &self.game;
        for f in self.fold.each() {
            f.reset(g);
        }
        self.mark = self.mark_now();
    }

    fn mark_now(&self) -> Mark {
        Mark {
            level: self.game.level(),
            scene: self.game.scene(),
            phase: self.game.phase(),
        }
    }

    /// Calls the fold-ins' `reset` after a level / scene change or a return to building.
    fn watch(&mut self) {
        let now = self.mark_now();
        let before = self.mark;
        if now.level != before.level
            || now.scene != before.scene
            || (now.phase == Phase::Build && before.phase != Phase::Build)
        {
            self.reset_fold_ins();
        } else {
            self.mark = now;
        }
    }

    /// The drive button / goal Run button: a fold-in may take it (TIME-3), else run / stop.
    fn drive_press(&mut self, ctx: &mut Ctx) {
        let g = &mut self.game;
        let mut taken = false;
        for f in self.fold.each() {
            if f.drive(g, ctx) {
                taken = true;
                break;
            }
        }
        if !taken {
            self.game.toggle_run();
        }
        self.ticker.reset();
    }

    fn apply(&mut self, a: Action, ctx: &mut Ctx) {
        let g = &mut self.game;
        match a {
            Action::Tool(t) => g.pick_tool(t),
            Action::Piece(k) => g.pick_type(k),
            Action::Found(f) => g.pick_found(f),
            Action::Weight(w) => g.pick_weight(w),
            Action::Finish => g.finish_line(),
            Action::ToggleForces => g.toggle_forces(),
            Action::Undo => g.undo(),
            Action::Clear => {
                g.clear();
                self.reset_fold_ins();
            }
            Action::Level(k) => g.pick_level(k),
            Action::Scene(s) => g.pick_scene(s),
            Action::RunToggle => self.drive_press(ctx),
            Action::BackToBuild => g.stop_run(),
            Action::Replay => {
                g.replay();
                self.ticker.reset();
            }
            Action::ToggleSlow => g.toggle_slow(),
            Action::Wind(v) => g.set_wind(v),
            Action::Dir(d) => g.set_dir(d),
            Action::Snow(v) => g.set_snow(v),
            Action::Quake(v) => g.set_quake(v),
            Action::Soil(s) => g.set_soil(s),
            Action::Traffic(t) => g.set_traffic(t),
            Action::ShakeNow => g.shake_now(),
            Action::ClickJoint(id) => g.click_node(&id),
            Action::ClickHandle(id) => g.pick_handle(&id),
            Action::ClickSurface(x, y) => g.click_surface(x, y),
            Action::JointDown(id) => g.joint_down(&id),
            Action::SurfaceDown(x, y) => g.surface_down(x, y),
            Action::PointerMove(x, y) => g.pointer_move(x, y),
            Action::PointerUp => g.pointer_up(),
            Action::ClearHover => g.clear_hover(),

            Action::ZoomOut => g.zoom_out(),
            Action::ZoomIn => g.zoom_in(),
            Action::Fit => g.fit(),
            Action::Pinch(f, x, y) => g.zoom_by(f, Some(x), Some(y)),
            Action::SetFoundation(id, f) => g.set_foundation(&id, f),
            Action::CrateLess(id) => g.crate_less(&id),
            Action::CrateMore(id) => g.crate_more(&id),
            Action::SetPieceType(id, k) => g.set_piece_type(&id, k),
            Action::DeletePiece(id) => g.delete_member(&id),
            Action::DeleteJoint(id) => g.delete_joint(&id),
            Action::Sample(k) => {
                for f in self.fold.each() {
                    if f.open_sample(k) {
                        break;
                    }
                }
            }
            Action::OpenInAdvanced => {
                let text = self.netlist_text();
                let name = self.game.level().slug();
                ctx.go_with(
                    Route::Advanced,
                    Carry::netlist(text, Some(&name), Route::Playground),
                );
            }
        }
    }

    fn best_list(&self) -> Vec<(String, u8)> {
        self.game
            .best()
            .iter()
            .map(|(k, s)| (k.key().to_string(), *s))
            .collect()
    }
}

impl Screen for BuilderScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let before = self.game.revision();
        let mut actions = Vec::new();
        let view = self.adjusted_view();
        let mut chips = Vec::new();
        for f in self.fold.each_ref() {
            f.hud(&self.game, &mut chips);
        }
        let anim = scene::Anim {
            flow: self.flow,
            fall: self.fall,
            shake: self.shake,
        };
        let game = &mut self.game;
        let fold = &mut self.fold;
        layout::columns(ui, ctx, &COLS, |ui, ctx, i| match i {
            0 => rail::show(ui, game.tool(), &mut actions),
            1 => stage::main_column(ui, game, fold, &view, &chips, anim, &mut actions),
            _ => aside::show(ui, ctx, game, fold, &view, &mut actions),
        });
        for a in actions {
            self.apply(a, ctx);
        }
        self.watch();
        if self.game.revision() != before {
            ui.ctx().request_repaint();
        }
        self.view = view;
        let best = self.best_list();
        if best != self.saved_best {
            ui.ctx()
                .data_mut(|d| d.insert_persisted(best_id(), best.clone()));
            self.saved_best = best;
        }
    }

    fn document(&self) -> Option<String> {
        Some(self.game.level().slug())
    }

    fn drive_button(&self) -> Option<Drive> {
        for f in self.fold.each_ref() {
            if let Some(d) = f.drive_button(&self.game) {
                return Some(d);
            }
        }
        Some(Drive::new(self.view.hdr_run).running(self.running()))
    }

    fn drive(&mut self, ctx: &mut Ctx) {
        self.drive_press(ctx);
        self.watch();
        self.refresh_view();
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        self.game.clear();
        self.ticker.reset();
        self.reset_fold_ins();
        self.refresh_view();
    }

    fn clock(&self) -> String {
        self.view.clock.clone()
    }

    fn share(&mut self, _ctx: &mut Ctx) -> Option<Share> {
        Some(Share::new(self.netlist_text()))
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        let dt = ctx.dt();
        if self.game.advance(dt as f64)
            && let Some(left) = self.game.toast_remaining()
        {
            ctx.wake_in(left as f32 + 0.05);
        }
        if let Some(left) = self.game.toast_remaining() {
            ctx.wake_in(left as f32 + 0.05);
        }
        let g = &mut self.game;
        for f in self.fold.each() {
            f.tick(g, ctx);
        }
        if self.game.animating() {
            let n = self.ticker.steps(dt, 30.0);
            for _ in 0..n {
                self.game.tick();
            }
            let running = self.game.phase() == Phase::Run;
            if running {
                // `@keyframes flow { to { stroke-dashoffset: -48 } }` over 1.6 s, snowfall over
                // 2.6 s, the quake shake over 0.2 s
                let k = if self.game.slow() { 0.3 } else { 1.0 };
                self.flow = (self.flow + 30.0 * dt * k) % 48.0;
                self.fall = (self.fall + dt * k / 2.6) % 1.0;
                self.shake = (self.shake + dt / 0.2) % 1.0;
            }
            // ~30 frames a second: the next frame when the next tick is due (PLAT-3)
            ctx.wake_in(self.ticker.due_in(30.0));
        } else {
            self.ticker.reset();
        }
        if self.fold.each_ref().iter().any(|f| f.busy()) {
            ctx.wake_in(1.0 / 30.0);
        }
        self.watch();
        self.refresh_view();
    }

    fn command(&mut self, cmd: Command, ctx: &mut Ctx) -> bool {
        match cmd {
            Command::Undo => self.game.undo(),
            Command::Delete => self.game.delete_selected(),
            Command::Paste(text) => self.load_netlist(&text, ctx),
            Command::ZoomIn => self.game.zoom_in(),
            Command::ZoomOut => self.game.zoom_out(),
            Command::ZoomFit => self.game.fit(),
            Command::Pan { dx, dy } => {
                // one arrow press pans 40 stage px (the view moves the other way)
                self.game.pan_by(-40.0 * dx as f64, 40.0 * dy as f64);
            }
            Command::Nudge { dx, dy } => self.game.nudge(dx as f64, dy as f64),
            Command::Escape => {
                let mut taken = false;
                for f in self.fold.each() {
                    if f.escape() {
                        taken = true;
                        break;
                    }
                }
                if !taken {
                    if self.game.pending().is_some() {
                        self.game.finish_line();
                    } else if self.game.selection().is_some() {
                        self.game.select(None::<Selection>);
                    } else {
                        return false;
                    }
                }
            }
        }
        self.watch();
        self.refresh_view();
        true
    }

    fn enter(&mut self, ctx: &mut Ctx) {
        // best stars from an earlier run (never lowers what this run has earned)
        if let Some(saved) = ctx
            .egui()
            .data_mut(|d| d.get_persisted::<Vec<(String, u8)>>(best_id()))
        {
            for (k, s) in &saved {
                if let Some(level) = LevelKey::from_key(k)
                    && *s > self.game.best_for(level)
                {
                    self.game.set_best(level, *s);
                }
            }
            self.saved_best = self.best_list();
        }
    }

    fn receive(&mut self, carry: Carry, ctx: &mut Ctx) {
        let Carry::Netlist { text, .. } = carry;
        self.load_netlist(&text, ctx);
        self.refresh_view();
    }

    fn describe(&self) -> String {
        let v = self.adjusted_view();
        let mut s = format!(
            "Playground, {}: {} {}",
            self.game.level().slug(),
            v.stage_label,
            v.clock
        );
        if let Some(r) = &v.result {
            s.push_str(&format!(" Result: {}", r.title));
            if !r.stars.is_empty() {
                s.push_str(&format!(" {}", r.stars));
            }
            s.push('.');
        }
        for f in self.fold.each_ref() {
            if let Some(more) = f.describe(&self.game) {
                s.push(' ');
                s.push_str(&more);
            }
        }
        s
    }
}
