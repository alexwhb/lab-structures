//! LAB-1 the sample drawer (owner: agent `lab`): a Materials card opens it over the aside; it
//! shows the v1 tensile rig (`widgets::tensile::rig`) and stress–strain plot
//! (`widgets::tensile::plot`) for a sample of that piece type, with Pull, Push (types that
//! resist pushing), Fresh sample, a temperature slider (−40…900 °C, the FIRE-1 factors), the
//! sample's length for Push, the force in t and kN against the builder's breaking force, the
//! status, tiles and "What's happening"; "Close" or Esc closes it. Every number and string comes
//! from `sp_engine::builder::sample` ([`Sample::view`]); the pull is paced at the engine's 20
//! steps a second (`ctx.wake_in`), nothing repaints while it is idle.

use super::bits::{eyebrow, label_node, para, section, status_pill};
use super::host::FoldIn;
use crate::ctx::Ctx;
use crate::widgets::tensile::{
    self, CurveLabel, PLOT_VIEW, PLOT_X1, PLOT_Y1, PlotScene, RigReadout, RigScene, Tick, ViewBox,
};
use crate::widgets::text::{galley, wrapped};
use crate::widgets::{Callout, Icon, Slider, StatTile, ghost, prim};
use eframe::egui::{Color32, Pos2, Rect, Ui, UiBuilder, pos2, vec2};
use sp_engine::builder::sample::bench::{TEMP_MAX_C, TEMP_MIN_C, TEMP_STEP_C};
use sp_engine::builder::sample::view::Pt;
use sp_engine::builder::sample::{Sample, SampleView, Test, Tone};
use sp_engine::builder::{Game, PieceType};
use sp_theme as t;
use sp_theme::fire as tf;

/// The drawer's accessible group name.
pub const GROUP: &str = "Sample drawer";
/// The Close button's accessible name.
pub const CLOSE: &str = "Close the sample drawer";

/// The fold-in: whether the drawer is open, and the sample on the bench.
#[derive(Debug, Default)]
pub struct Lab {
    open: bool,
    sample: Sample,
}

impl Lab {
    /// The drawer is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The sample on the bench (tests, MCP).
    pub fn sample(&self) -> &Sample {
        &self.sample
    }

    /// The sample, to drive it (MCP `sample_test`).
    pub fn sample_mut(&mut self) -> &mut Sample {
        &mut self.sample
    }

    /// Opens the drawer on `kind`.
    pub fn open(&mut self, kind: PieceType) {
        self.sample.open(kind);
        self.open = true;
    }

    /// Closes the drawer (a running test pauses).
    pub fn close(&mut self) {
        self.sample.pause();
        self.open = false;
    }

    fn show(&mut self, ui: &mut Ui) {
        let v = self.sample.view();
        let mut close = false;
        section(ui, GROUP, 12.0, |ui| {
            close = header(ui, &v);
            controls(ui, &mut self.sample, &v);
            let w = ui.available_width();
            let (rig_rect, _) =
                ui.allocate_exact_size(vec2(w, (w * 0.92).round()), eframe::egui::Sense::hover());
            tensile::rig(ui, rig_rect, &rig_scene(&v));
            limit(ui, &v);
            let (plot_rect, _) =
                ui.allocate_exact_size(vec2(w, (w * 0.72).round()), eframe::egui::Sense::hover());
            tensile::plot(ui, plot_rect, &plot_scene(&v));
            limit_line(ui, plot_rect, &v);
            sliders(ui, &mut self.sample, &v);
            tiles(ui, &v);
            Callout::happening(&v.explain).show(ui);
        });
        if close {
            self.close();
        }
    }
}

impl FoldIn for Lab {
    fn tick(&mut self, _game: &mut Game, ctx: &mut Ctx) {
        if !self.open {
            return;
        }
        self.sample.advance(ctx.dt() as f64);
        if let Some(s) = self.sample.next_tick_in() {
            ctx.wake_in(s as f32);
        }
    }

    fn open_sample(&mut self, piece: PieceType) -> bool {
        self.open(piece);
        true
    }

    fn drawer(&mut self, ui: &mut Ui, _game: &mut Game, _ctx: &mut Ctx) -> bool {
        if !self.open {
            return false;
        }
        self.show(ui);
        true
    }

    fn escape(&mut self) -> bool {
        if self.open {
            self.close();
            true
        } else {
            false
        }
    }

    fn busy(&self) -> bool {
        self.open && self.sample.playing()
    }

    fn describe(&self, _game: &Game) -> Option<String> {
        self.open.then(|| self.sample.describe())
    }
}

/// The eyebrow + Close row, the title, the subtitle and the status pill. Returns whether Close
/// was pressed.
fn header(ui: &mut Ui, v: &SampleView) -> bool {
    let top = ui.cursor().min;
    let w = ui.available_width();
    let close = ghost("Close").accessible_name(CLOSE).height(30.0);
    let cw = close.desired_size(ui).x;
    let pressed = close
        .show_at(
            ui,
            Rect::from_min_size(pos2(top.x + w - cw, top.y), vec2(cw, 30.0)),
        )
        .clicked();
    let mut eye = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(
        pos2(top.x, top.y + 8.0),
        vec2(w - cw - 8.0, 30.0),
    )));
    eyebrow(&mut eye, "Test a sample");
    ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(w, 30.0)));

    let font = lab_theme::font::heading(16.0);
    let g = galley(ui, &v.title, font, t::TEXT);
    let r = Rect::from_min_size(ui.cursor().min, g.size());
    ui.painter().galley(r.min, g, t::TEXT);
    label_node(ui, "lab-title", r, &v.title);
    ui.advance_cursor_after_rect(r);
    para(ui, &v.subtitle, 12.0, 1.5, t::TEXT_MUTED);

    let origin = ui.cursor().min;
    let pill = status_pill(ui, origin, &v.status_label, tone_color(v.status_tone), w);
    ui.advance_cursor_after_rect(pill);
    pressed
}

/// Pull | Push, then Fresh sample.
fn controls(ui: &mut Ui, s: &mut Sample, v: &SampleView) {
    let top = ui.cursor().min;
    let w = ui.available_width();
    let half = ((w - 8.0) / 2.0).floor();
    let h = 36.0;
    let pull_icon = if v.pull_label == "Pause" {
        Icon::Pause
    } else {
        Icon::Play
    };
    if prim(v.pull_label)
        .icon(pull_icon)
        .centered()
        .accessible_name(if v.pull_label == "Pause" {
            "Pause pulling"
        } else {
            "Pull"
        })
        .show_at(ui, Rect::from_min_size(top, vec2(half, h)))
        .clicked()
    {
        s.pull();
    }
    let push_icon = if v.push_label == "Pause" {
        Icon::Pause
    } else {
        Icon::Play
    };
    if ghost(v.push_label)
        .icon(push_icon)
        .centered()
        .enabled(v.can_push)
        .accessible_name(if v.push_label == "Pause" {
            "Pause pushing"
        } else {
            "Push"
        })
        .show_at(
            ui,
            Rect::from_min_size(pos2(top.x + w - half, top.y), vec2(half, h)),
        )
        .clicked()
    {
        s.push();
    }
    ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(w, h)));
    if ghost("Fresh sample")
        .icon(Icon::Restart)
        .full_width()
        .centered()
        .show(ui)
        .clicked()
    {
        s.fresh_sample();
    }
    if !v.push_note.is_empty() {
        para(ui, &v.push_note, 12.0, 1.5, t::TEXT_MUTED);
    }
}

/// The rig's scene from the engine's geometry.
fn rig_scene(v: &SampleView) -> RigScene {
    let r = &v.rig;
    let i = type_index(v.kind);
    RigScene {
        head: r.head,
        head_top: r.head_top,
        specimen: r.specimen.clone(),
        fill: if r.charred {
            tf::CHARRED
        } else {
            tf::SAMPLE_FILL[i]
        },
        gauge: r.gauge,
        crack: r.crack.clone(),
        readout: Some(RigReadout {
            title: "Load cell".into(),
            value: v.load_cell.clone(),
            note: v.load_cell_note.clone(),
        }),
        badge: Some(match v.test {
            Test::Pull => "stretch exaggerated".into(),
            Test::Push => "squeeze exaggerated".into(),
        }),
        label: v.rig_label.clone(),
    }
}

/// The plot's scene from the engine's geometry: the curve in the piece's colour, the
/// room-temperature ghost when heated, the done trace and marker, labels at the peaks.
fn plot_scene(v: &SampleView) -> PlotScene {
    let g = &v.plot;
    let ticks = |ts: &[sp_engine::builder::sample::Tick]| -> Vec<Tick> {
        ts.iter()
            .map(|t| Tick {
                label: t.label.clone(),
                pos: t.pos,
            })
            .collect()
    };
    let mut labels = Vec::new();
    if let Some(p) = peak(&g.curve) {
        labels.push(CurveLabel {
            name: v.kind.name().into(),
            at: p,
            selected: true,
        });
    }
    if let Some(p) = peak(&g.ghost) {
        labels.push(CurveLabel {
            name: "20 \u{b0}C".into(),
            at: p,
            selected: false,
        });
    }
    PlotScene {
        axes: g.axes,
        grid: g.grid.clone(),
        ghosts: if g.ghost.is_empty() {
            Vec::new()
        } else {
            vec![g.ghost.clone()]
        },
        curve: g.curve.clone(),
        curve_color: tf::SAMPLE_LINE[type_index(v.kind)],
        done: g.done.clone(),
        marker: Some(g.marker),
        ticks_y: ticks(&g.ticks_y),
        ticks_x: ticks(&g.ticks_x),
        y_title: g.y_title.into(),
        x_title: g.x_title.into(),
        labels,
        label: v.plot_label.clone(),
    }
}

/// The highest point of a polyline (smallest y in the plot's viewBox).
fn peak(pts: &[Pt]) -> Option<Pt> {
    pts.iter()
        .copied()
        .fold(None, |best: Option<Pt>, p| match best {
            Some(b) if b[1] <= p[1] => Some(b),
            _ => Some(p),
        })
}

/// The builder's limit as a dashed red line across the plot, with a legend in the plot's top
/// right corner (clear of the curve: the stress axis runs 15 % past the peak).
fn limit_line(ui: &mut Ui, rect: Rect, v: &SampleView) {
    let Some(l) = v.plot.limit else {
        return;
    };
    let vb = ViewBox::meet(rect.shrink(1.0), PLOT_VIEW);
    let (a, b) = (vb.pt(l[0]), vb.pt(l[1]));
    let painter = ui.painter().with_clip_rect(rect);
    let dash = |p: &eframe::egui::Painter, from: Pos2, to: Pos2| {
        super::paint::dashed(
            p,
            &[from, to],
            vb.w(1.5),
            tf::LIMIT_LINE,
            vb.w(6.0),
            vb.w(4.0),
            0.0,
            false,
        );
    };
    dash(&painter, a, b);
    let g = galley(
        ui,
        &v.plot.limit_label,
        lab_theme::font::mono(10.0),
        tf::LIMIT_TEXT,
    );
    let corner = vb.pt([PLOT_X1, PLOT_Y1]);
    let at = Pos2::new(corner.x - g.size().x, corner.y + 2.0);
    let r = Rect::from_min_size(at, g.size());
    let y = r.center().y;
    dash(&painter, pos2(at.x - 22.0, y), pos2(at.x - 4.0, y));
    painter.galley(at, g, tf::LIMIT_TEXT);
    label_node(ui, "lab-limit-line", r, &v.plot.limit_label);
}

/// "A Steel piece this size breaks at 153 t." and, past it, the warning; the force now in t and
/// kN.
fn limit(ui: &mut Ui, v: &SampleView) {
    let top = ui.cursor().min;
    let w = ui.available_width();
    let force = format!("{} \u{b7} {}", v.force_t, v.force_kn);
    let fg = galley(ui, &force, lab_theme::font::mono(14.0), t::ACCENT);
    let kg = galley(ui, "Force now", lab_theme::font::body(12.0), t::TEXT_MUTED);
    let row_h = fg.size().y.max(kg.size().y);
    ui.painter().galley(
        pos2(top.x, top.y + (row_h - kg.size().y) / 2.0),
        kg,
        t::TEXT_MUTED,
    );
    let fr = Rect::from_min_size(pos2(top.x + w - fg.size().x, top.y), fg.size());
    ui.painter().galley(fr.min, fg, t::ACCENT);
    label_node(
        ui,
        "lab-force",
        Rect::from_min_size(top, vec2(w, row_h)),
        &format!("Force now {force}"),
    );
    ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(w, row_h)));
    let colour = if v.over_limit {
        t::CAUTION
    } else {
        t::TEXT_BODY
    };
    let g = wrapped(
        ui,
        &v.limit,
        lab_theme::font::body(13.0),
        colour,
        w,
        Some(13.0 * 1.5),
    );
    let r = Rect::from_min_size(ui.cursor().min, vec2(w, g.size().y));
    ui.painter().galley(r.min, g, colour);
    label_node(ui, "lab-limit", r, &v.limit);
    ui.advance_cursor_after_rect(r);
    if !v.limit_note.is_empty() {
        para(ui, &v.limit_note, 12.0, 1.5, t::CAUTION);
    }
}

/// The temperature slider and its note; the sample length (types that push).
fn sliders(ui: &mut Ui, s: &mut Sample, v: &SampleView) {
    let mut temp = s.temp() as f32;
    let colour_note = if v.temp_hot {
        t::CAUTION
    } else {
        t::TEXT_MUTED
    };
    if Slider::new(
        "Temperature",
        &mut temp,
        TEMP_MIN_C as f32..=TEMP_MAX_C as f32,
    )
    .step(TEMP_STEP_C as f32)
    .value_text(v.temp_label.clone())
    .show(ui)
    .changed()
    {
        s.set_temp(temp as f64);
    }
    para(ui, &v.temp_note, 12.0, 1.5, colour_note);
    if v.can_push {
        let mut len = s.length() as f32;
        let max = s.max_length() as f32;
        if Slider::new("Sample length", &mut len, 1.0..=max)
            .step(1.0)
            .value_text(v.length_label.clone())
            .show(ui)
            .changed()
        {
            s.set_length(len as f64);
        }
        para(
            ui,
            "For Push: a longer piece of the same section buckles sooner.",
            12.0,
            1.5,
            t::TEXT_MUTED,
        );
    }
}

/// The tiles, two per row.
fn tiles(ui: &mut Ui, v: &SampleView) {
    let w = ui.available_width();
    let col = ((w - 8.0) / 2.0).floor();
    for row in v.tiles.chunks(2) {
        let top = ui.cursor().min;
        let mut h: f32 = 0.0;
        for (i, tile) in row.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(top.x + i as f32 * (col + 8.0), top.y),
                vec2(col, 200.0),
            );
            let mut child = ui.new_child(UiBuilder::new().max_rect(r));
            let value = tile.value.clone();
            let resp = StatTile::new(tile.key, &value).compact().show(&mut child);
            h = h.max(resp.rect.height());
        }
        ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(w, h)));
    }
}

/// The colour of a status tone.
fn tone_color(tone: Tone) -> Color32 {
    match tone {
        Tone::Muted => t::TEXT_MUTED,
        Tone::Ok => t::OK,
        Tone::Accent => t::ACCENT,
        Tone::Caution => t::CAUTION,
        Tone::Danger => t::DANGER,
    }
}

/// `PieceType` → its index in `types()` order (the theme's tables).
fn type_index(k: PieceType) -> usize {
    PieceType::ALL.iter().position(|p| *p == k).unwrap_or(0)
}
