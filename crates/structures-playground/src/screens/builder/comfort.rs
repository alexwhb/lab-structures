//! CMF-1 comfort meter and CMF-2 "Compare shapes" (owner: agent `time`).
//!
//! In the aside's result slot, on open-ground runs and their result: the v1 Tower's banded
//! comfort meter drawn in the builder's look (`.well`, `.lbl`, mono numbers) — the peak value,
//! the bands, the level's limit and the v1 sentence — and after the result a "Compare shapes"
//! button that runs the same storm headless on four versions of the player's tower and shows
//! the v1 "Same storm, four shapes" bars (the player's first) with the verdict. Every number
//! and sentence comes from `sp_engine::builder::comfort`.

use super::bits::{eyebrow, label_node, para};
use super::host::{FoldIn, card};
use crate::ctx::Ctx;
use crate::widgets::text::{css_line_height, galley, text_top};
use crate::widgets::{ghost, world::compare_row};
use eframe::egui::{Color32, CornerRadius, Rect, Sense, Ui, pos2, vec2};
use lab_theme::font;
use sp_engine::builder::comfort::{self, Comparison, Meter};
use sp_engine::builder::{Conditions, Game, Outcome, Phase, Scene, Setup};
use sp_engine::tower::Tone;
use sp_theme::{self as t, time as tt};

/// The colour of a comfort tone (`sp_theme::time`).
pub fn tone_color(tone: Tone) -> Color32 {
    match tone {
        Tone::Plain => t::TEXT,
        Tone::Ok => tt::TONE_OK,
        Tone::Marginal => tt::TONE_MARGINAL,
        Tone::Caution => tt::TONE_CAUTION,
        Tone::Alert => tt::TONE_ALERT,
        Tone::Danger => tt::TONE_DANGER,
    }
}

/// What the meter shows: the peak so far while running, the run's peak after it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    /// Peak comfort value, milli-g.
    pub mg: f64,
    /// The level's limit.
    pub limit: Option<f64>,
    /// The run is over (the result's value).
    pub done: bool,
}

/// The meter's reading for the game's state: open ground with Floors, running or after a run.
pub fn reading(game: &Game) -> Option<Reading> {
    let setup = game.setup();
    if setup.scene != Scene::Flat {
        return None;
    }
    match game.phase() {
        Phase::Run => game.sim().filter(|s| s.top_i.is_some()).map(|s| Reading {
            mg: s.milli_g(),
            limit: setup.comfort,
            done: false,
        }),
        Phase::Result => match game.result().filter(|r| r.years.is_none())?.outcome {
            Outcome::Tower { mg, .. } if game.design().top() > 0.0 => Some(Reading {
                mg,
                limit: setup.comfort,
                done: true,
            }),
            _ => None,
        },
        Phase::Build => None,
    }
}

/// The fold-in's state: the last "Compare shapes" and what it was computed for.
#[derive(Debug, Default)]
pub struct Comfort {
    shapes: Option<(u64, Conditions, Comparison)>,
}

/// The level and conditions of the run on show ("the same storm"): the simulation's — the
/// conditions chips may have changed since the run ended — or the current ones without a run.
fn run_basis(game: &Game) -> (Setup, Conditions) {
    match game.sim() {
        Some(s) => (s.setup.clone(), s.cond),
        None => (game.setup(), *game.cond()),
    }
}

impl Comfort {
    /// The comparison on show (for the current design and the run's conditions).
    pub fn comparison(&self, game: &Game) -> Option<&Comparison> {
        let (_, cond) = run_basis(game);
        self.shapes
            .as_ref()
            .filter(|(rev, c, _)| *rev == game.revision() && *c == cond)
            .map(|(_, _, c)| c)
    }

    /// Runs "Compare shapes" for the game's design in the run's level and conditions (what
    /// the button does; MCP may call it too).
    pub fn compare(&mut self, game: &Game) -> &Comparison {
        let (setup, cond) = run_basis(game);
        let c = comfort::compare(game.design(), &setup, &cond);
        &self.shapes.insert((game.revision(), cond, c)).2
    }
}

/// The banded meter: a 14 px bar (radius 7) of the four bands, the white marker at the value,
/// the level's limit as a tick with its label above, and the tick row under it.
fn meter(ui: &mut Ui, m: &Meter) {
    let w = ui.available_width();
    let tf = font::mono(10.0);
    let tl = css_line_height(ui, &tf);
    let limit_h = if m.limit.is_some() { tl + 4.0 } else { 0.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(w, limit_h + 14.0 + 6.0 + tl), Sense::hover());
    let p = ui.painter();
    let bar = Rect::from_min_size(rect.min + vec2(0.0, limit_h), vec2(w, 14.0));
    let x_of = |pct: f64| bar.left() + bar.width() * (pct as f32 / 100.0).clamp(0.0, 1.0);
    let stretches = Meter::stretches();
    let n = stretches.len();
    for (i, s) in stretches.iter().enumerate() {
        let (x0, x1) = (x_of(comfort::pct(s.from)), x_of(comfort::pct(s.to)));
        let r = Rect::from_min_max(pos2(x0, bar.top()), pos2(x1, bar.bottom()));
        let cr = CornerRadius {
            nw: if i == 0 { 7 } else { 0 },
            sw: if i == 0 { 7 } else { 0 },
            ne: if i + 1 == n { 7 } else { 0 },
            se: if i + 1 == n { 7 } else { 0 },
        };
        p.rect_filled(r, cr, tt::COMFORT_BANDS[i.min(3)]);
    }
    if let (Some(lp), Some(_)) = (m.limit_pct, m.limit) {
        let x = x_of(lp);
        let tick = Rect::from_min_max(pos2(x - 1.0, bar.top() - 4.0), pos2(x + 1.0, bar.bottom()));
        p.rect_filled(tick, CornerRadius::ZERO, tt::LIMIT);
        let g = galley(ui, m.limit_text(), tf.clone(), tt::LIMIT);
        let gx = (x - g.size().x / 2.0).clamp(bar.left(), bar.right() - g.size().x);
        p.galley(pos2(gx, rect.top()), g, tt::LIMIT);
    }
    // the marker, clipped to the bar (v1: `overflow: hidden`)
    let mx = x_of(m.marker_pct);
    let marker =
        Rect::from_min_size(pos2(mx - 2.0, bar.top() - 2.0), vec2(4.0, 18.0)).intersect(bar);
    p.rect_filled(marker, CornerRadius::same(2), tt::METER_MARKER);
    let ty = bar.bottom() + 6.0 + tl / 2.0;
    let ticks = Meter::ticks();
    let last = ticks.len().saturating_sub(1);
    for (i, (label, pct)) in ticks.iter().enumerate() {
        let g = galley(ui, label, tf.clone(), t::TEXT_MUTED);
        let x = x_of(*pct);
        let gx = if i == 0 {
            x
        } else if i == last {
            x - g.size().x
        } else {
            x - g.size().x / 2.0
        };
        p.galley(pos2(gx, text_top(ty, &g)), g, t::TEXT_MUTED);
    }
    label_node(ui, ("comfort-meter", "bar"), bar, &m.label());
}

/// A label / value row: the `.lbl` eyebrow left, the mono 12 px value right.
fn title_row(ui: &mut Ui, title: &str, value: &str, color: Color32) {
    let w = ui.available_width();
    let vg = galley(ui, value, font::mono(12.0), color);
    let top = ui.cursor().min;
    let mut row = ui.new_child(
        eframe::egui::UiBuilder::new()
            .max_rect(Rect::from_min_size(top, vec2(w - vg.size().x - 8.0, 40.0))),
    );
    eyebrow(&mut row, title);
    let h = row.min_rect().height().max(vg.size().y);
    let rect = Rect::from_min_size(top, vec2(w, h));
    let vpos = pos2(rect.right() - vg.size().x, text_top(top.y + h / 2.0, &vg));
    let vrect = Rect::from_min_size(vpos, vg.size());
    ui.painter().galley(vpos, vg, color);
    label_node(ui, ("comfort-value", title), vrect, value);
    ui.advance_cursor_after_rect(rect);
}

impl FoldIn for Comfort {
    fn results(&mut self, ui: &mut Ui, game: &mut Game, _ctx: &mut Ctx) {
        let Some(r) = reading(game) else {
            return;
        };
        let m = Meter::new(r.mg, r.limit);
        let shapes = self.comparison(game).cloned();
        let mut press = false;
        card(ui, "Comfort", |ui| {
            let title = if r.done {
                "Comfort at the top floor"
            } else {
                "Comfort · peak so far"
            };
            title_row(ui, title, &m.value(), tone_color(m.tone()));
            meter(ui, &m);
            para(ui, m.sentence(), 13.0, 1.5, tone_color(m.tone()));
            if !r.done || !comfort::applies(game.design(), &game.setup()) {
                return;
            }
            match &shapes {
                None => {
                    press = ghost("Compare shapes")
                        .full_width()
                        .centered()
                        .show(ui)
                        .clicked();
                }
                Some(c) => {
                    eyebrow(ui, "Same storm, four shapes");
                    for (i, b) in c.bars.iter().enumerate() {
                        compare_row(
                            ui,
                            b.variant.name(),
                            i == 0,
                            b.width as f32,
                            tone_color(b.tone(r.limit)),
                            &b.value(),
                        );
                    }
                    para(ui, &c.verdict, 13.0, 1.5, t::TEXT_BODY);
                    para(
                        ui,
                        "Peak acceleration at the top floor: the same pieces in the same storm.",
                        11.0,
                        1.4,
                        t::TEXT_MUTED,
                    );
                }
            }
        });
        if press {
            self.compare(game);
            ui.ctx().request_repaint();
        }
    }

    fn reset(&mut self, _game: &Game) {
        self.shapes = None;
    }

    fn describe(&self, game: &Game) -> Option<String> {
        let r = reading(game)?;
        let m = Meter::new(r.mg, r.limit);
        let mut s = format!("comfort {} ({})", m.value(), m.sentence());
        if let Some(c) = self.comparison(game) {
            let bars: Vec<String> = c.bars.iter().map(|b| b.label()).collect();
            s.push_str(&format!(
                "; four shapes: {}. {}",
                bars.join(", "),
                c.verdict
            ));
        }
        Some(s)
    }
}
