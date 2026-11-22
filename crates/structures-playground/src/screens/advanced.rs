//! SCR-2 Advanced — the bridge as a truss netlist (`docs/design/mockup/Advanced.dc.html`,
//! SPEC ADV-1…ADV-10).
//!
//! The netlist text is the source of truth ([`doc::Doc`]); the toolbar edits it, the editor
//! edits it, and everything on screen is solved from it by `sp_engine` (`netlist`, `generate`,
//! `influence` on the one `truss` solver). Layout: the toolbar row, then the body — `display:
//! flex; flex-wrap: wrap; gap: 16px; padding: 16px; align-items: flex-start` with the Netlist
//! panel (`flex: 1 1 360px; max-width: 440px`), the centre column (`flex: 999 1 560px`: the
//! elevation and the influence line) and the aside (`flex: 1 1 340px; max-width: 420px`: member
//! results, checks, badges). At 390 px they stack in that order and the page scrolls.

pub mod controls;
pub mod doc;
pub mod editor;
pub mod elevation;
pub mod influence;
pub mod results;
pub mod toolbar;

use super::{Drive, Screen, Share};
use crate::ctx::{Carry, Command, Ctx, Ticker};
use crate::router::Route;
use doc::{DRIVE_PERIOD, Doc, Origin};
use eframe::egui::{Align, Layout, Rect, Ui, UiBuilder, pos2, vec2};
use lab_ui::layout::{FlexItem, flex_lines};
use sp_engine::generate::Params;
use sp_engine::influence as engine_il;
use sp_engine::netlist::text::to_fixed;
use sp_theme::advanced as a;
use toolbar::Action;

/// Body padding and gap (`padding: 16px; gap: 16px`).
const GAP: f32 = 16.0;

/// SCR-2 state.
#[derive(Debug)]
pub struct AdvancedScreen {
    /// The document (netlist text, model, results, undo).
    pub doc: Doc,
    /// Drive across is running.
    pub playing: bool,
    ticker: Ticker,
    select_open: bool,
}

impl Default for AdvancedScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedScreen {
    /// The starting state: the mockup's defaults.
    pub fn new() -> Self {
        Self {
            doc: Doc::new(),
            playing: false,
            ticker: Ticker::default(),
            select_open: false,
        }
    }

    fn apply(&mut self, action: Action, now: f64) {
        let p = self.doc.params;
        match action {
            Action::Style(s) => self
                .doc
                .apply_generator(Params { style: s, ..p }, "style", now),
            Action::Braced(b) => self
                .doc
                .apply_generator(Params { braced: b, ..p }, "braced", now),
            Action::Material(m) => {
                self.doc
                    .apply_generator(Params { material: m, ..p }, "material", now)
            }
            Action::Size(s) => self
                .doc
                .apply_generator(Params { size: s, ..p }, "size", now),
            Action::Truck(t) => self.doc.set_truck(Some(t), None, "truck", now, true),
            Action::X(x) => {
                self.playing = false;
                self.doc.set_truck(None, Some(x), "x", now, true);
            }
        }
    }

    fn body(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let width = ui.available_width();
        let origin = ui.cursor().min;
        let inner = (width - 2.0 * GAP).max(0.0);
        let items = [
            FlexItem::new(1.0, 1.0, 360.0),
            FlexItem::new(999.0, 1.0, 560.0),
            FlexItem::new(1.0, 1.0, 340.0),
        ];
        let max = [Some(440.0f32), None, Some(420.0)];
        let lines = flex_lines(inner, GAP, &items);
        let now = ui.input(|i| i.time);
        let mut y = origin.y + GAP;
        for (li, line) in lines.iter().enumerate() {
            if li > 0 {
                y += GAP;
            }
            // max-width, the excess to the line's other columns by grow
            let mut widths: Vec<f32> = line.iter().map(|p| p.width).collect();
            let mut excess = 0.0;
            let mut frozen = vec![false; line.len()];
            for (k, p) in line.iter().enumerate() {
                if let Some(m) = max[p.index]
                    && widths[k] > m
                {
                    excess += widths[k] - m;
                    widths[k] = m;
                    frozen[k] = true;
                }
            }
            let grow: f32 = line
                .iter()
                .enumerate()
                .filter(|(k, _)| !frozen[*k])
                .map(|(_, p)| items[p.index].grow)
                .sum();
            if excess > 0.0 && grow > 0.0 {
                for (k, p) in line.iter().enumerate() {
                    if !frozen[k] {
                        widths[k] += excess * items[p.index].grow / grow;
                    }
                }
            }
            let mut x = origin.x + GAP;
            let mut line_h: f32 = 0.0;
            for (k, p) in line.iter().enumerate() {
                let rect = Rect::from_min_size(pos2(x, y), vec2(widths[k], f32::INFINITY));
                x += widths[k] + GAP;
                let mut col = ui.new_child(
                    UiBuilder::new()
                        .id_salt(("advanced-col", p.index))
                        .max_rect(rect)
                        .layout(Layout::top_down(Align::Min)),
                );
                col.spacing_mut().item_spacing = vec2(GAP, GAP);
                match p.index {
                    0 => self.netlist_column(ctx, &mut col, now),
                    1 => self.centre_column(&mut col),
                    _ => self.aside(&mut col),
                }
                line_h = line_h.max(col.min_rect().height());
            }
            y += line_h;
        }
        let used = Rect::from_min_max(origin, pos2(origin.x + width, y + GAP));
        ui.advance_cursor_after_rect(used);
    }

    fn netlist_column(&mut self, ctx: &mut Ctx, ui: &mut Ui, now: f64) {
        let doc = &self.doc;
        let (_, out) = controls::panel(ui, "Netlist", |ui| editor::netlist_panel(ui, doc));
        if let Some(text) = out.changed {
            self.doc.edit(text, now);
            ctx.wake_in(doc::DEBOUNCE as f32);
        }
        if out.open_in_playground {
            ctx.go_with(
                Route::Bridge,
                Carry::netlist(self.doc.text.clone(), None, Route::Advanced),
            );
        }
    }

    fn centre_column(&mut self, ui: &mut Ui) {
        let stale = self.doc.stale;
        let picks = self.doc.picks();
        let current = self.doc.current_pick();
        let il = self.doc.influence_line().cloned();
        let Some(good) = self.doc.good.as_ref() else {
            return;
        };
        let style = self.doc.generated.map(|g| g.style.name());
        let x = good.report.x;
        let now_kn = current
            .as_ref()
            .map(|p| engine_il::force_kn(&good.report, p.member))
            .unwrap_or(0.0);
        let mut picked = None;
        ui.scope(|ui| {
            if stale {
                ui.set_opacity(a::DIMMED);
            }
            controls::panel(ui, "Schematic", |ui| {
                elevation::elevation_panel(ui, &good.netlist, &good.report, style)
            });
            let (_, out) = controls::panel(ui, "Influence line", |ui| {
                influence::influence_panel(ui, &picks, current.as_ref(), il.as_ref(), x, now_kn)
            });
            picked = out.picked;
        });
        if let Some(k) = picked {
            self.doc.pick = k;
        }
    }

    fn aside(&mut self, ui: &mut Ui) {
        let stale = self.doc.stale;
        let Some(good) = self.doc.good.as_ref() else {
            return;
        };
        ui.scope(|ui| {
            if stale {
                ui.set_opacity(a::DIMMED);
            }
            controls::panel(ui, "Member results", |ui| {
                results::table_panel(ui, &good.report)
            });
            controls::panel(ui, "Checks", |ui| results::checks_panel(ui, &good.report));
        });
        results::badges(ui, good.netlist.k_factor);
    }
}

impl Screen for AdvancedScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let now = ui.input(|i| i.time);
        let actions = toolbar::toolbar(ui, &self.doc, &mut self.select_open);
        for act in actions {
            self.apply(act, now);
        }
        self.body(ctx, ui);
    }

    fn document(&self) -> Option<String> {
        self.doc.document()
    }

    fn drive_button(&self) -> Option<Drive> {
        Some(
            Drive::new("Drive across")
                .running(self.playing)
                .enabled(self.doc.truck_t().is_some()),
        )
    }

    fn drive(&mut self, _ctx: &mut Ctx) {
        self.playing = !self.playing;
        self.ticker.reset();
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        let now = ctx.egui().input(|i| i.time);
        self.playing = false;
        self.select_open = false;
        self.doc.reset(now);
    }

    fn clock(&self) -> String {
        format!("x = {} m", to_fixed(self.doc.x(), 1))
    }

    fn share(&mut self, _ctx: &mut Ctx) -> Option<Share> {
        Some(
            Share::new(self.doc.text.clone())
                .toast(format!("Copied {} to the clipboard", self.doc.file_name())),
        )
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        let now = ctx.egui().input(|i| i.time);
        self.doc.tick(now);
        if let Some(due) = self.doc.due_in(now) {
            ctx.wake_in(due as f32);
        }
        if self.playing {
            if self.doc.truck_t().is_none() {
                self.playing = false;
                return;
            }
            let rate = 1.0 / DRIVE_PERIOD;
            for _ in 0..self.ticker.steps(ctx.dt(), rate) {
                self.doc.drive_step(now);
            }
            ctx.wake_in(self.ticker.due_in(rate));
        }
    }

    fn command(&mut self, cmd: Command, ctx: &mut Ctx) -> bool {
        let now = ctx.egui().input(|i| i.time);
        match cmd {
            Command::Undo => self.doc.undo(),
            Command::Paste(text) if looks_like_netlist(&text) => {
                self.playing = false;
                self.doc.load(text, None, Origin::Pasted, now);
                ctx.toast("Pasted a netlist");
                true
            }
            Command::Paste(text) => {
                let ok = self.doc.paste_material(&text, now);
                if ok {
                    ctx.toast("Pasted a material");
                }
                ok
            }
            _ => false,
        }
    }

    fn receive(&mut self, carry: Carry, ctx: &mut Ctx) {
        if let Carry::Netlist { text, name, from } = carry {
            let now = ctx.egui().input(|i| i.time);
            self.playing = false;
            let origin = match from {
                Some(r) => Origin::Received(r),
                None => Origin::File,
            };
            self.doc.load(text, name, origin, now);
        }
    }

    fn describe(&self) -> String {
        let Some(g) = &self.doc.good else {
            return "Advanced: no netlist.".into();
        };
        let errors = match self.doc.errors() {
            0 => String::new(),
            n => format!(" {n} error(s) in the netlist; showing the last good result."),
        };
        format!(
            "Advanced: {} with {} nodes and {} members. {}. Truck at x = {} m.{errors}",
            self.doc.file_name(),
            g.netlist.nodes.len(),
            g.netlist.members.len(),
            g.report.status.label(),
            to_fixed(self.doc.x(), 1),
        )
    }
}

/// A pasted text is taken as a whole netlist when it has nodes or members (a lone
/// `.material` line only swaps the material).
fn looks_like_netlist(text: &str) -> bool {
    text.lines().any(|l| {
        let l = l.trim_start().to_ascii_lowercase();
        l.starts_with(".node") || l.starts_with(".member")
    })
}
