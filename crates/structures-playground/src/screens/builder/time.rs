//! TIME-1 "Fast-forward 30 years" and TIME-3 the Leaning tower's run (owner: agent `time`).
//!
//! After a run that stood (any scene with foundations) the aside's result slot offers "Fast-
//! forward 30 years": the engine computes the 30 years at once
//! (`sp_engine::builder::longterm::fast_forward_design`, a few milliseconds), then the years
//! play over about 6 s — the stage draws the structure where it has settled
//! (`Game::view_settled`), the HUD chips read "Year 12.0", "Tilt 1.8°" (bridges: "Differential
//! 12 mm"), "Settled 214 mm", a plumb line and the lean line show the tilt — and afterwards a
//! years scrubber, the result line and its sentence.
//!
//! On the Leaning tower (`RunKind::Years`) the run itself is the fast-forward: the drive / goal
//! button (or the engine's `start_years`, e.g. "Watch again, slow") starts it, the years play
//! with the run overlay reading "Year 12.0 / 30", and at the end the engine gets the result
//! (`Game::finish_years`: tilt under 0.5° after 30 years, stars by the fix's cost) for its
//! banner and best stars.

use super::bits::{bar, eyebrow, label_node, para};
use super::host::{FoldIn, HudChip, card};
use super::paint::Xf;
use crate::ctx::Ctx;
use crate::screens::Drive;
use crate::widgets::text::{galley, text_top};
use crate::widgets::{Slider, ghost};
use eframe::egui::{Color32, Painter, Sense, Shape, Stroke, Ui, pos2, vec2};
use lab_theme::font;
use sp_engine::builder::game::RunResult;
use sp_engine::builder::longterm::{self, Ending, LongTerm, pisa};
use sp_engine::builder::view::SettledView;
use sp_engine::builder::{Conditions, Game, LevelKey, Outcome, Phase, RunKind, View, Years};
use sp_engine::jsnum::fixed;
use sp_theme::{self as t, time as tt};

/// Per piece of the design: axial force (N), broken, slack (`SettledView::pieces`).
type PieceStates = Vec<(f64, bool, bool)>;

/// A fast-forward on show.
#[derive(Debug, Clone)]
struct Run {
    /// What it was computed for: the design revision, level and conditions.
    rev: u64,
    level: LevelKey,
    cond: Conditions,
    lt: LongTerm,
    /// For each of the design's pieces, its index in the fast-forward's frames.
    piece_of: Vec<Option<usize>>,
    /// Seconds into the animation while it plays.
    elapsed: Option<f64>,
    /// The year on show.
    year: f64,
    /// The Leaning tower: this fast-forward is the level's run.
    years_level: bool,
    /// The Leaning tower's result, once the years have played.
    result: Option<pisa::PisaResult>,
}

impl Run {
    /// The structure at the year on show, for `Game::view_settled`.
    fn settled(&self) -> (Vec<[f64; 2]>, PieceStates) {
        let s = self.lt.at(self.year);
        let pieces = self
            .piece_of
            .iter()
            .map(|i| match i {
                Some(i) => (
                    s.force.get(*i).copied().unwrap_or(0.0),
                    s.broken.get(*i).copied().unwrap_or(false),
                    s.slack.get(*i).copied().unwrap_or(false),
                ),
                None => (0.0, false, false),
            })
            .collect();
        (s.pos, pieces)
    }
}

/// The fold-in's state.
#[derive(Debug, Default)]
pub struct Time {
    run: Option<Run>,
}

/// Whether the game's last run stood: a tower still up, a bridge with nothing broken.
fn stood(game: &Game) -> bool {
    let Some(r) = game.result() else {
        return false;
    };
    if r.years.is_some() {
        return false;
    }
    match r.outcome {
        Outcome::Tower { standing, .. } => standing,
        _ => r.first.is_none() && r.first_found.is_none(),
    }
}

/// The level's run is the fast-forward (the Leaning tower).
fn years_level(game: &Game) -> bool {
    game.setup().run == RunKind::Years
}

/// A Leaning tower run the engine started (`start_years`) that has no fast-forward yet.
fn years_running(game: &Game) -> bool {
    years_level(game) && game.phase() == Phase::Run && game.sim().is_none()
}

/// The colour of a tilt in the HUD.
fn tilt_color(tilt: f64) -> Color32 {
    let a = tilt.abs();
    if a >= 4.0 {
        tt::HUD_TILT_DANGER
    } else if a >= longterm::LEAN_OK_DEG {
        tt::HUD_TILT_WARN
    } else {
        tt::HUD_VALUE
    }
}

impl Time {
    /// Whether "Fast-forward 30 years" can start now: after a run that stood, with foundations
    /// carrying something; on the Leaning tower while building (or when its run has begun).
    pub fn available(game: &Game) -> bool {
        if !longterm::has_foundations(game.design()) {
            return false;
        }
        if years_level(game) {
            game.phase() == Phase::Build || years_running(game)
        } else {
            game.phase() == Phase::Result && stood(game)
        }
    }

    /// Computes the 30 years for the game's design, level and conditions and starts playing
    /// them (the button; the Leaning tower's run, which it starts in the engine; MCP
    /// `builder_fast_forward` with `show`). Nothing happens unless [available](Self::available).
    pub fn start(&mut self, game: &mut Game) {
        if !Self::available(game) {
            return;
        }
        let years = years_level(game);
        if years && game.phase() == Phase::Build {
            game.start_years();
        }
        let lt = longterm::fast_forward_design(game.design(), &game.setup(), game.cond());
        let piece_of = game
            .design()
            .members
            .iter()
            .map(|m| lt.piece_index(&m.id))
            .collect();
        self.run = Some(Run {
            rev: game.revision(),
            level: game.level(),
            cond: *game.cond(),
            lt,
            piece_of,
            elapsed: Some(0.0),
            year: 0.0,
            years_level: years,
            result: None,
        });
    }

    /// Jumps to the end of the 30 years (skips the animation; tests, MCP).
    pub fn finish(&mut self, game: &mut Game) {
        if let Some(r) = &mut self.run
            && r.elapsed.is_some()
        {
            // past the animation's length: the next step lands on the last year
            r.elapsed = Some(2.0 * longterm::ANIM_SECONDS);
        }
        self.advance(game, 0.0);
    }

    /// The fast-forward on show, if any (for the current design).
    pub fn long_term(&self) -> Option<&LongTerm> {
        self.run.as_ref().map(|r| &r.lt)
    }

    /// The year on show.
    pub fn year(&self) -> Option<f64> {
        self.run.as_ref().map(|r| r.year)
    }

    /// Shows another year (the scrubber), clamped to the computed span. While the years are
    /// still playing this does nothing.
    pub fn set_year(&mut self, year: f64) {
        if let Some(r) = &mut self.run
            && r.elapsed.is_none()
        {
            r.year = r.lt.at(year).year;
        }
    }

    /// The years are playing.
    pub fn playing(&self) -> bool {
        self.run.as_ref().is_some_and(|r| r.elapsed.is_some())
    }

    /// The Leaning tower's result, once its 30 years have played.
    pub fn pisa_result(&self) -> Option<&pisa::PisaResult> {
        self.run.as_ref().and_then(|r| r.result.as_ref())
    }

    /// Drops a fast-forward that no longer matches the game: the design, level or conditions
    /// changed, back to building, or (outside the Leaning tower) a new run started.
    fn validate(&mut self, game: &Game) {
        if self.run.as_ref().is_some_and(|r| {
            r.rev != game.revision()
                || r.level != game.level()
                || r.cond != *game.cond()
                || game.phase() == Phase::Build
                || (!r.years_level && game.phase() != Phase::Result)
        }) {
            self.run = None;
        }
    }

    /// The fast-forward to draw now (it matches the game's state).
    fn shown(&self, game: &Game) -> Option<&Run> {
        self.run.as_ref().filter(|r| {
            r.rev == game.revision()
                && r.level == game.level()
                && if r.years_level {
                    matches!(game.phase(), Phase::Run | Phase::Result)
                } else {
                    game.phase() == Phase::Result
                }
        })
    }

    /// Moves the animation on by `dt` seconds; at the end of a Leaning tower run hands the
    /// result to the engine.
    fn advance(&mut self, game: &mut Game, dt: f64) {
        let Some(r) = &mut self.run else {
            return;
        };
        let Some(e) = r.elapsed else {
            return;
        };
        let end = r.lt.end_year();
        let e = e + dt.max(0.0);
        r.year = longterm::anim_year(e, end);
        if r.year < end - 1e-9 {
            r.elapsed = Some(e);
            return;
        }
        r.year = end;
        r.elapsed = None;
        if r.years_level && r.result.is_none() {
            let budget = game.setup().budget.unwrap_or(pisa::BUDGET);
            let cost = pisa::fix_cost(game.design());
            let practice = game.practice();
            let res = pisa::result(&r.lt, cost, practice, budget);
            let settled = r.lt.last().map_or(0.0, |f| f.max_settle);
            if game.phase() == Phase::Run {
                let standing = !matches!(r.lt.ending, Ending::Toppled(_) | Ending::CameDown(_));
                game.finish_years(RunResult::of_years(
                    res.pass,
                    res.stars,
                    cost,
                    practice,
                    standing,
                    game.design().top(),
                    Years {
                        tilt: res.tilt,
                        settled,
                    },
                    None,
                ));
            }
            r.result = Some(res);
        }
    }

    /// The card's contents.
    fn card_body(&self, ui: &mut Ui, game: &Game) -> Press {
        let mut press = Press::None;
        let Some(r) = &self.run else {
            eyebrow(ui, "Thirty years from now");
            para(
                ui,
                if years_level(game) {
                    "This level's run is 30 years of settling: the soft clay squeezes slowly, \
                     and the heavier side sinks deeper."
                } else {
                    "Soils keep settling for years after a building goes up: soft clay for \
                     decades, sand within months, rock not at all."
                },
                13.0,
                1.5,
                t::TEXT_BODY,
            );
            if ghost("Fast-forward 30 years")
                .full_width()
                .centered()
                .show(ui)
                .clicked()
            {
                press = Press::Start;
            }
            return press;
        };
        match &r.result {
            Some(res) => {
                let color = if res.pass {
                    tt::TONE_OK
                } else {
                    tt::TONE_DANGER
                };
                title_row(ui, &res.title, &stars_text(game, res), color);
            }
            None => eyebrow(ui, "Thirty years from now"),
        }
        let end = r.lt.end_year();
        if r.elapsed.is_some() {
            let frac = (r.year / end.max(1e-9)) as f32;
            let w = ui.available_width();
            let (rect, _) = ui.allocate_exact_size(vec2(w, 8.0), Sense::hover());
            bar(ui.painter(), rect, frac, t::ACCENT);
            let text = format!("Year {} of {}", fixed(r.year, 1), fixed(longterm::YEARS, 0));
            label_node(ui, ("years-progress", "bar"), rect, &text);
            para(ui, &text, 13.0, 1.5, t::TEXT_BODY);
            if ghost("Skip to the end")
                .full_width()
                .centered()
                .show(ui)
                .clicked()
            {
                press = Press::Skip;
            }
            return press;
        }
        let mut y = r.year as f32;
        let value = format!("Year {}", fixed(r.year, 1));
        if Slider::new("Years", &mut y, 0.0..=end as f32)
            .step(longterm::STEP as f32)
            .value_text(value)
            .show(ui)
            .changed()
        {
            press = Press::Scrub(y as f64);
        }
        para(ui, &r.lt.result_line(), 13.0, 1.5, t::TEXT);
        let more = match &r.result {
            Some(res) => res
                .explain
                .strip_prefix(r.lt.result_line().as_str())
                .unwrap_or(&res.explain)
                .trim_start()
                .to_string(),
            None => r.lt.sentence(),
        };
        para(ui, &more, 13.0, 1.5, t::TEXT_BODY);
        if ghost("Play again")
            .full_width()
            .centered()
            .show(ui)
            .clicked()
        {
            press = Press::Replay;
        }
        press
    }
}

/// What the card's controls asked for.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Press {
    None,
    Start,
    Skip,
    Replay,
    Scrub(f64),
}

/// "★★☆" for a passing Leaning tower result, "practice" for a practice run, else empty.
fn stars_text(game: &Game, res: &pisa::PisaResult) -> String {
    if game.practice() {
        "practice".into()
    } else if res.pass {
        sp_engine::builder::data::stars_text(res.stars)
    } else {
        String::new()
    }
}

/// A title (Space Grotesk 600 15 in `color`) with mono stars on the right.
fn title_row(ui: &mut Ui, title: &str, stars: &str, color: Color32) {
    let w = ui.available_width();
    let tg = galley(ui, title, font::heading(15.0), color);
    let sg = galley(ui, stars, font::mono(14.0), t::ACCENT);
    let h = tg.size().y.max(sg.size().y);
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter();
    let cy = rect.center().y;
    p.galley(pos2(rect.left(), text_top(cy, &tg)), tg, color);
    let sx = rect.right() - sg.size().x;
    p.galley(pos2(sx, text_top(cy, &sg)), sg, t::ACCENT);
    let label = if stars.is_empty() {
        title.to_string()
    } else {
        format!("{title} {stars}")
    };
    label_node(ui, ("years-result", title), rect, &label);
}

impl FoldIn for Time {
    fn tick(&mut self, game: &mut Game, ctx: &mut Ctx) {
        self.validate(game);
        // a Leaning tower run the engine started ("Watch again, slow", MCP) plays the years
        if years_running(game) && self.run.as_ref().is_none_or(|r| r.result.is_some()) {
            self.run = None;
            self.start(game);
        }
        if self.playing() {
            let k = if game.slow() { 0.3 } else { 1.0 };
            self.advance(game, ctx.dt() as f64 * k);
            ctx.wake_in(1.0 / 30.0);
        }
    }

    fn adjust_view(&self, game: &Game, view: &mut View) {
        if let Some(r) = self.shown(game) {
            let (pos, pieces) = r.settled();
            *view = game.view_settled(&SettledView {
                pos: &pos,
                pieces: &pieces,
            });
            if r.years_level {
                view.run_info =
                    format!("Year {} / {}", fixed(r.year, 1), fixed(longterm::YEARS, 0));
                match &r.result {
                    Some(res) => {
                        if let Some(b) = &mut view.result {
                            b.title = res.title.clone();
                        }
                        view.explain = res.explain.clone();
                    }
                    None => {
                        view.status.label = "Settling, year by year".into();
                        view.explain = format!(
                            "Year {} of {}: the soft clay squeezes slowly and the heavier side \
                             sinks deeper, so every bit of lean puts more weight on it.",
                            fixed(r.year, 1),
                            fixed(longterm::YEARS, 0)
                        );
                    }
                }
            }
        } else if years_level(game) && game.phase() == Phase::Build {
            view.run_label = "Fast-forward 30 years";
        }
    }

    fn hud(&self, game: &Game, chips: &mut Vec<HudChip>) {
        let Some(r) = self.shown(game) else {
            return;
        };
        let s = r.lt.at(r.year);
        for (k, v) in r.lt.chips(r.year) {
            let color = if k.starts_with("Tilt") {
                tilt_color(s.tilt)
            } else {
                tt::HUD_VALUE
            };
            chips.push(HudChip::new(k.trim(), v, color));
        }
    }

    fn paint_stage(&self, game: &Game, _view: &View, painter: &Painter, xf: &Xf) {
        let Some(r) = self.shown(game) else {
            return;
        };
        let Some((base, tp)) = r.lt.lean_line(r.year) else {
            return;
        };
        let h = (tp[1] - base[1]).max(1.0);
        // the plumb line: straight up from the foundations' centroid, a little past the top
        let a = xf.p(base);
        let b = xf.p([base[0], base[1] + h * 1.08]);
        painter.extend(Shape::dashed_line(
            &[a, b],
            Stroke::new(xf.l(1.0).max(1.0), tt::PLUMB),
            xf.l(tt::PLUMB_DASH[0]).max(1.0),
            xf.l(tt::PLUMB_DASH[1]).max(1.0),
        ));
        // the lean line: from the foundations' centroid to the top floor's centroid
        painter.line_segment([a, xf.p(tp)], Stroke::new(xf.l(1.5).max(1.0), tt::LEAN));
    }

    fn results(&mut self, ui: &mut Ui, game: &mut Game, _ctx: &mut Ctx) {
        self.validate(game);
        if self.shown(game).is_none() && !Self::available(game) {
            return;
        }
        let mut press = Press::None;
        card(ui, "Thirty years from now", |ui| {
            press = self.card_body(ui, game);
        });
        match press {
            Press::None => {}
            Press::Start => self.start(game),
            Press::Skip => self.finish(game),
            Press::Replay => {
                if let Some(r) = &mut self.run {
                    r.elapsed = Some(0.0);
                    r.year = 0.0;
                }
            }
            Press::Scrub(y) => self.set_year(y),
        }
        if press != Press::None {
            ui.ctx().request_repaint();
        }
    }

    fn drive(&mut self, game: &mut Game, _ctx: &mut Ctx) -> bool {
        if !years_level(game) || game.phase() != Phase::Build {
            return false;
        }
        self.start(game);
        true
    }

    fn drive_button(&self, game: &Game) -> Option<Drive> {
        (years_level(game) && game.phase() == Phase::Build)
            .then(|| Drive::new("Fast-forward 30 years"))
    }

    fn reset(&mut self, _game: &Game) {
        self.run = None;
    }

    fn busy(&self) -> bool {
        self.playing()
    }

    fn describe(&self, game: &Game) -> Option<String> {
        let r = self.shown(game)?;
        let mut s = format!("{} (year {})", r.lt.result_line(), fixed(r.year, 1));
        if let Some(res) = &r.result {
            s = format!("{}: {s}", res.title);
        }
        if matches!(r.lt.ending, Ending::Toppled(_)) {
            s.push_str(" Toppled.");
        }
        Some(s)
    }
}
