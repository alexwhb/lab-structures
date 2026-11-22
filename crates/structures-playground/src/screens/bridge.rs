//! SCR-1 Bridge (SPEC BR-1…BR-11) — mockup `docs/design/mockup/Playground.dc.html`: build a
//! truss over the river, hang the level's weights on it, press Test and watch it hold or fall.
//!
//! All logic and every number and sentence live in [`sp_engine::bridge`] ([`Game`], bit-exact
//! with the mockup's `class Component`); this module only draws [`Game::view`] and forwards
//! clicks, drags and keys:
//!
//! - the row (`Tools` rail 76 px · `Stage` · `Level and inspector` aside) on
//!   [`layout::columns`]: side by side at 1440, stacked in the mockup's order at 390;
//! - [`rail`] — the tools rail; [`stage`] — the stage painter and its buttons and overlays;
//!   [`aside`] — levels, goal, inspector / Built from, What's happening; [`bits`] — chips,
//!   pill, level rows, tool buttons, tiles, stars;
//! - the hint bar (hint, Size / Weight chips, "Finish line", "Exaggerate sag", "Show forces");
//! - header hooks: Test / Pause, Reset, the clock, the document name, Share (a `.str`
//!   netlist), the provenance badges; ⌘Z, Delete, ⌘V (a netlist into the Sandbox), arrow keys
//!   nudge the selected joint 2 m; "Open in Advanced" / "Open in Playground" hand-offs (NAV-4).

mod aside;
mod bits;
mod rail;
pub mod stage;

use super::{Badge, Drive, Screen, Share};
use crate::ctx::{Carry, Command, Ctx, Ticker};
use crate::layout::{self, Column};
use crate::router::Route;
use crate::widgets::text::{galley, lbl_galley, text_top, wrapped};
use crate::widgets::{ghost, set_bounds};
use bits::{CHIP_GAP, ChipLook, ChipSpec, chips_layout, chips_show, label_node};
use eframe::egui::{
    CornerRadius, Id, Key, Modifiers, Rect, Stroke, StrokeKind, Ui, UiBuilder, accesskit::Role,
    pos2, vec2,
};
use sp_engine::bridge::game::TICK_SECONDS;
use sp_engine::bridge::{
    Game, LevelKey, MaterialKey, Selection, SizeKey, Support, Tool, View, netlist,
};
use sp_theme as t;

/// Something the player did this frame (applied after drawing, so the frame draws one view).
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Pick a tool.
    Tool(Tool),
    /// Pick the size for new members.
    Size(SizeKey),
    /// Pick the weight step.
    Weight(u32),
    /// Toggle "Exaggerate sag".
    ToggleSag,
    /// Toggle "Show forces".
    ToggleForces,
    /// "Finish line".
    Finish,
    /// Pick the material.
    Material(MaterialKey),
    /// Pick a level.
    Level(LevelKey),
    /// Undo.
    Undo,
    /// Clear ("Start this level again").
    Clear,
    /// "Drop in a starter truss".
    Starter,
    /// "Test it: add the load" / "Back to building".
    TestToggle,
    /// The result banner's "Back to building".
    BackToBuild,
    /// "Replay in slow motion".
    Replay,
    /// Click on a joint.
    ClickJoint(String),
    /// Click on empty grid (snapped).
    ClickGrid(i32, i32),
    /// Click on a member's handle.
    ClickMember(String),
    /// Drag started on a joint.
    JointDown(String),
    /// Pointer over the grid (snapped): hover dot / drag.
    PointerMove(i32, i32),
    /// Drag ended.
    PointerUp,
    /// Pointer left the stage.
    ClearHover,
    /// Inspector support chip.
    SetSupport(String, Option<Support>),
    /// Inspector "−".
    LessWeight(String),
    /// Inspector "+".
    MoreWeight(String),
    /// Inspector size chip.
    MemberSize(String, SizeKey),
    /// "Delete member".
    DeleteMember(String),
    /// "Delete joint".
    DeleteJoint(String),
    /// "Open in Advanced".
    OpenInAdvanced,
}

/// The row: tools rail · stage · aside.
const COLS: [Column; 3] = [
    Column::rail("Tools"),
    Column::main("Stage"),
    Column::readouts("Level and inspector"),
];

/// SCR-1 state.
#[derive(Debug)]
pub struct BridgeScreen {
    /// The game (public for tests and MCP).
    pub game: Game,
    ticker: Ticker,
    /// Water dash offset, stage px.
    flow: f32,
}

impl Default for BridgeScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl BridgeScreen {
    /// The starting state: level 1, the Member tool.
    pub fn new() -> Self {
        Self {
            game: Game::new(),
            ticker: Ticker::default(),
            flow: 0.0,
        }
    }

    /// The structure as a `.str` netlist named after the level (Share, "Open in Advanced").
    pub fn netlist_text(&self) -> String {
        let g = &self.game;
        netlist::write(g.design(), g.material(), g.level(), g.level().slug())
    }

    /// Loads a netlist into the Sandbox (⌘V, "Open in Playground"), toasting the outcome.
    fn load_netlist(&mut self, text: &str, ctx: &mut Ctx) {
        match netlist::read(text) {
            Ok((design, mat)) => {
                let n = design.nodes.len();
                self.game.open_in_sandbox(design, mat);
                ctx.toast(format!("Loaded a {n}-joint structure into the Sandbox"));
            }
            Err(e) => ctx.toast(format!("Couldn't load that netlist: {e}")),
        }
    }

    fn apply(&mut self, a: Action, ctx: &mut Ctx) {
        let g = &mut self.game;
        match a {
            Action::Tool(t) => g.pick_tool(t),
            Action::Size(s) => g.pick_size(s),
            Action::Weight(w) => g.pick_weight(w),
            Action::ToggleSag => g.toggle_sag(),
            Action::ToggleForces => g.toggle_forces(),
            Action::Finish => g.finish_line(),
            Action::Material(m) => g.pick_material(m),
            Action::Level(l) => g.pick_level(l),
            Action::Undo => g.undo(),
            Action::Clear => g.clear(),
            Action::Starter => g.starter(),
            Action::TestToggle => g.start_or_stop(),
            Action::BackToBuild => g.back_to_build(),
            Action::Replay => g.replay(),
            Action::ClickJoint(id) => g.click_joint(&id),
            Action::ClickGrid(x, y) => g.click_grid(x, y),
            Action::ClickMember(id) => g.click_member(&id),
            Action::JointDown(id) => g.joint_down(&id),
            Action::PointerMove(x, y) => g.pointer_move(x, y),
            Action::PointerUp => g.pointer_up(),
            Action::ClearHover => g.clear_hover(),
            Action::SetSupport(id, s) => g.set_support(&id, s),
            Action::LessWeight(id) => {
                let w = g.weight() as f64;
                g.add_load(&id, -w)
            }
            Action::MoreWeight(id) => {
                let w = g.weight() as f64;
                g.add_load(&id, w)
            }
            Action::MemberSize(id, s) => g.set_member_size(&id, s),
            Action::DeleteMember(id) => g.delete_member(&id),
            Action::DeleteJoint(id) => g.delete_joint(&id),
            Action::OpenInAdvanced => {
                let text = self.netlist_text();
                let name = self.game.level().slug();
                ctx.go_with(
                    Route::Advanced,
                    Carry::netlist(text, Some(name), Route::Bridge),
                );
            }
        }
    }

    /// The hint bar (`.panel`, `padding: 8px 12px`, `min-height: 52px`, wrapping with
    /// `gap: 8px 14px`): the hint grows (`flex: 1 1 220px`), the toggles sit right.
    fn hint_bar(&self, ui: &mut Ui, v: &View, actions: &mut Vec<Action>) {
        let g = &self.game;
        let w = ui.available_width();
        let inner_w = (w - 26.0).max(0.0);
        let hint_font = lab_theme::font::body(13.0);
        // the groups
        let sizes: Vec<ChipSpec> = SizeKey::ALL
            .iter()
            .map(|s| ChipSpec {
                label: s.name(),
                name: s.name(),
                look: ChipLook {
                    on: g.size() == *s,
                    ..Default::default()
                },
            })
            .collect();
        let weight_labels: Vec<String> = Game::weight_steps()
            .iter()
            .map(|w| format!("{w} t"))
            .collect();
        let weights: Vec<ChipSpec> = Game::weight_steps()
            .iter()
            .zip(&weight_labels)
            .map(|(w, l)| ChipSpec {
                label: l,
                name: l,
                look: ChipLook {
                    on: g.weight() == *w,
                    ..Default::default()
                },
            })
            .collect();
        let toggles = [
            ChipSpec {
                label: "Exaggerate sag",
                name: "Exaggerate sag",
                look: ChipLook {
                    on: g.sag(),
                    ..Default::default()
                },
            },
            ChipSpec {
                label: "Show forces",
                name: "Show forces",
                look: ChipLook {
                    on: g.show_forces(),
                    ..Default::default()
                },
            },
        ];
        let unbounded = f32::INFINITY;
        let size_lbl = lbl_galley(ui, "Size", t::TEXT_MUTED);
        let weight_lbl = lbl_galley(ui, "Weight", t::TEXT_MUTED);
        let (size_rects, size_sz) = chips_layout(ui, &sizes, unbounded, vec2(CHIP_GAP, 0.0));
        let (weight_rects, weight_sz) = chips_layout(ui, &weights, unbounded, vec2(CHIP_GAP, 0.0));
        let (tog_rects, tog_sz) = chips_layout(ui, &toggles, unbounded, vec2(6.0, 0.0));
        let finish = ghost("Finish line");
        let finish_sz = finish.desired_size(ui);
        // items: (kind, width, height)
        #[derive(Clone, Copy, PartialEq)]
        enum K {
            Hint,
            Sizes,
            Weights,
            Finish,
            Toggles,
        }
        let mut items: Vec<(K, f32)> = vec![(K::Hint, 220.0f32.min(inner_w))];
        if v.show_sizes {
            items.push((K::Sizes, size_lbl.size().x + 6.0 + size_sz.x));
        }
        if v.show_weights {
            items.push((K::Weights, weight_lbl.size().x + 6.0 + weight_sz.x));
        }
        if v.has_pending {
            items.push((K::Finish, finish_sz.x));
        }
        items.push((K::Toggles, tog_sz.x));
        // greedy lines
        let mut lines: Vec<Vec<(K, f32)>> = vec![vec![]];
        let mut used = 0.0;
        for it in items {
            let cur = lines.last_mut().unwrap();
            let need = if cur.is_empty() {
                it.1
            } else {
                used + 14.0 + it.1
            };
            if need > inner_w && !cur.is_empty() {
                lines.push(vec![it]);
                used = it.1;
            } else {
                cur.push(it);
                used = need;
            }
        }
        // the hint grows into its line's free space; heights
        let mut hint_w = 220.0f32.min(inner_w);
        for l in &mut lines {
            let total: f32 = l.iter().map(|i| i.1).sum::<f32>() + 14.0 * (l.len() - 1) as f32;
            if let Some(h) = l.iter_mut().find(|i| i.0 == K::Hint) {
                h.1 += (inner_w - total).max(0.0);
                hint_w = h.1;
            }
        }
        let hint_galley = wrapped(ui, v.hint, hint_font.clone(), t::TEXT_BODY, hint_w, None);
        let hint_h = hint_galley.size().y;
        let item_h = |k: K| match k {
            K::Hint => hint_h,
            K::Finish => finish_sz.y,
            _ => bits::CHIP_H,
        };
        let line_hs: Vec<f32> = lines
            .iter()
            .map(|l| l.iter().map(|i| item_h(i.0)).fold(0.0, f32::max))
            .collect();
        let content_h = line_hs.iter().sum::<f32>() + 8.0 * (lines.len() - 1) as f32;
        let h = (content_h + 16.0 + 2.0).max(52.0);
        let top = ui.cursor().min;
        let rect = Rect::from_min_size(top, vec2(w, h));
        ui.painter().rect(
            rect,
            CornerRadius::same(t::radius::PANEL),
            t::PANEL,
            Stroke::new(1.0, t::BORDER),
            StrokeKind::Inside,
        );
        let mut y = top.y + (h - content_h) / 2.0;
        for (l, lh) in lines.iter().zip(&line_hs) {
            let total: f32 = l.iter().map(|i| i.1).sum::<f32>() + 14.0 * (l.len() - 1) as f32;
            let mut x = top.x + 13.0;
            for (k, iw) in l {
                // margin-left: auto on the toggles
                if *k == K::Toggles {
                    x += (inner_w - total).max(0.0);
                }
                let ih = item_h(*k);
                let cy = y + lh / 2.0;
                let cell = Rect::from_min_size(pos2(x, cy - ih / 2.0), vec2(*iw, ih));
                match k {
                    K::Hint => {
                        ui.painter()
                            .galley(cell.min, hint_galley.clone(), t::TEXT_BODY);
                        label_node(ui, "bridge-hint", cell, v.hint);
                    }
                    K::Sizes | K::Weights => {
                        let (lg, specs, rects, group, salt) = if *k == K::Sizes {
                            (&size_lbl, &sizes, &size_rects, "Member size", "size")
                        } else {
                            (
                                &weight_lbl,
                                &weights,
                                &weight_rects,
                                "Weight to add",
                                "weight",
                            )
                        };
                        let mut child = ui.new_child(
                            UiBuilder::new()
                                .max_rect(cell)
                                .accessibility_role(Role::Group)
                                .accessibility_label(group),
                        );
                        set_bounds(&child, cell);
                        child.painter().galley(
                            pos2(cell.left(), text_top(cy, lg)),
                            lg.clone(),
                            t::TEXT_MUTED,
                        );
                        let o = pos2(cell.left() + lg.size().x + 6.0, cell.top());
                        if let Some(i) = chips_show(&mut child, salt, o, specs, rects) {
                            actions.push(if *k == K::Sizes {
                                Action::Size(SizeKey::ALL[i])
                            } else {
                                Action::Weight(Game::weight_steps()[i])
                            });
                        }
                    }
                    K::Finish => {
                        if ghost("Finish line").show_at(ui, cell).clicked() {
                            actions.push(Action::Finish);
                        }
                    }
                    K::Toggles => {
                        if let Some(i) = chips_show(ui, "toggles", cell.min, &toggles, &tog_rects) {
                            actions.push(if i == 0 {
                                Action::ToggleSag
                            } else {
                                Action::ToggleForces
                            });
                        }
                    }
                }
                x += iw + 14.0;
            }
            y += lh + 8.0;
        }
        ui.advance_cursor_after_rect(rect);
    }

    /// The centre column: hint bar, then the stage (`aspect-ratio: 1120 / 600`).
    fn main_column(&self, ui: &mut Ui, v: &View, actions: &mut Vec<Action>) {
        self.hint_bar(ui, v, actions);
        let w = ui.available_width();
        let top = ui.cursor().min;
        let rect = Rect::from_min_size(top, vec2(w, stage::height_for(w)));
        let inp = stage::StageInput {
            view: v,
            flow: self.flow,
            dragging: self.game.dragging().is_some(),
        };
        stage::show(ui, rect, &inp, actions);
        ui.advance_cursor_after_rect(rect);
    }

    /// Arrow keys nudge the selected joint by 2 m (BR-11).
    fn keys(&mut self, ui: &Ui, ctx: &Ctx, actions: &mut Vec<Action>) {
        let _ = actions;
        if ctx.text_focus()
            || !self.game.building()
            || !matches!(self.game.selection(), Some(Selection::Joint(_)))
        {
            return;
        }
        for (key, dx, dy) in [
            (Key::ArrowLeft, -1, 0),
            (Key::ArrowRight, 1, 0),
            (Key::ArrowUp, 0, 1),
            (Key::ArrowDown, 0, -1),
        ] {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
                self.game.nudge(dx, dy);
            }
        }
    }
}

impl Screen for BridgeScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let before = self.game.revision();
        let mut actions = Vec::new();
        self.keys(ui, ctx, &mut actions);
        let v = self.game.view();
        // the row: side by side at 1440 (the aside scrolls on its own), stacked with
        // content-height lines at 390 (`layout::columns`)
        let tool = self.game.tool();
        let me = &*self;
        layout::columns(ui, ctx, &COLS, |ui, _ctx, i| match i {
            0 => rail::show(ui, tool, &mut actions),
            1 => me.main_column(ui, &v, &mut actions),
            _ => aside::show(ui, &v, &mut actions),
        });
        for a in actions {
            self.apply(a, ctx);
        }
        if self.game.revision() != before {
            ui.ctx().request_repaint();
        }
        let _ = Id::NULL;
    }

    fn document(&self) -> Option<String> {
        Some(self.game.level().slug().to_string())
    }

    fn drive_button(&self) -> Option<Drive> {
        Some(Drive::new("Test").running(self.game.playing()))
    }

    fn drive(&mut self, _ctx: &mut Ctx) {
        self.game.start_or_stop();
        self.ticker.reset();
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        self.game.clear();
        self.ticker.reset();
    }

    fn clock(&self) -> String {
        self.game.clock()
    }

    fn share(&mut self, _ctx: &mut Ctx) -> Option<Share> {
        Some(Share::new(self.netlist_text()))
    }

    fn provenance(&self) -> Vec<Badge> {
        vec![
            Badge::exact("Solver: linear statics, exact"),
            Badge::simplified("Materials: representative"),
            Badge::illustrative("Game costs"),
        ]
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        let dt = ctx.dt();
        if self.game.advance(dt as f64) {
            ctx.wake_in(0.1);
        }
        if self.game.animating() {
            let n = self.ticker.steps(dt, (1.0 / TICK_SECONDS) as f32);
            for _ in 0..n {
                self.game.tick();
            }
            // `@keyframes flow { to { stroke-dashoffset: -48 } }` over 1.6 s
            self.flow = (self.flow + 30.0 * dt) % 48.0;
            ctx.animate();
        } else {
            self.ticker.reset();
        }
    }

    fn command(&mut self, cmd: Command, ctx: &mut Ctx) -> bool {
        match cmd {
            Command::Undo => {
                self.game.undo();
                true
            }
            Command::Delete => {
                self.game.delete_selected();
                true
            }
            Command::Paste(text) => {
                self.load_netlist(&text, ctx);
                true
            }
        }
    }

    fn receive(&mut self, carry: Carry, ctx: &mut Ctx) {
        if let Carry::Netlist { text, .. } = carry {
            self.load_netlist(&text, ctx);
        }
    }

    fn describe(&self) -> String {
        self.game.summary()
    }
}

/// Text of a galley-sized label, for tests of the hint bar.
#[doc(hidden)]
pub fn hint_text_width(ui: &Ui, text: &str) -> f32 {
    galley(ui, text, lab_theme::font::body(13.0), t::TEXT_BODY)
        .size()
        .x
}
