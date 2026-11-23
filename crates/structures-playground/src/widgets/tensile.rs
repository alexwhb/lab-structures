//! The tensile rig and the stress–strain plot (from the v1 Materials screen, MT-3 / MT-4),
//! generic over what they draw, for the builder's sample drawer (LAB-1) and anything else that
//! tests a sample.
//!
//! Both painters take a plain scene in the mockup's SVG viewBox units — the rig's
//! `0 0 420 560` ([`RIG_VIEW`]), the plot's `0 0 560 400` ([`PLOT_VIEW`]) — fitted into the
//! given rect with `preserveAspectRatio="xMidYMid meet"`, on a `.graph` panel:
//!
//! - [`rig`] draws a [`RigScene`]: the static frame (base, columns, top beam, bottom grip), the
//!   crosshead, the sample polygon(s) in its colour, the extensometer, the red crack, an
//!   optional readout chip (top left: "Load cell" · `12.4 kN` · note) and badge (bottom right:
//!   "stretch exaggerated"). [`RigScene::dogbone`] builds the moving parts of a pulled (or
//!   pushed: negative stretch) dog-bone sample, necking and tearing — any material.
//! - [`plot`] draws a [`PlotScene`]: axes, grid, ghost curves, the curve, the done-so-far trace,
//!   the marker, tick labels, axis titles and curve name labels (HTML-positioned in percent of
//!   the box, as the mockup). [`PlotScene::axes`] makes the axes, grid and ticks for any
//!   ranges; [`PlotScene::point`] maps (strain, stress) into the plot area.
//!
//! [`RigScene::from_bench`] / [`PlotScene::from_bench`] adapt the v1 bench
//! (`sp_engine::materials::View`), so the painters stay checked against the Materials mockup.
//! Colours come from `sp_theme::materials`.

use super::badge::{badge_size, paint_badge};
use super::icons::fill_polygon;
use super::text::{css_line_height, galley, lbl_galley, paint_line, text_top};
use super::{BadgeLook, set_bounds};
use crate::screens::Badge;
use eframe::egui::{
    Color32, CornerRadius, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder,
    WidgetInfo, accesskit::Role, pos2, vec2,
};
use sp_theme as t;
use sp_theme::materials as mt;

/// A point in viewBox units.
pub type Pt = [f64; 2];

/// The rig's viewBox (`0 0 420 560`).
pub const RIG_VIEW: [f64; 2] = [420.0, 560.0];
/// The plot's viewBox (`0 0 560 400`).
pub const PLOT_VIEW: [f64; 2] = [560.0, 400.0];
/// The plot area in the plot viewBox: x from 56 (0) to 544, y from 350 (0) up to 20.
pub const PLOT_X0: f64 = 56.0;
/// See [`PLOT_X0`].
pub const PLOT_X1: f64 = 544.0;
/// See [`PLOT_X0`].
pub const PLOT_Y0: f64 = 350.0;
/// See [`PLOT_X0`].
pub const PLOT_Y1: f64 = 20.0;

/// The sample's centre line in the rig viewBox.
pub const RIG_CX: f64 = 210.0;
/// The unstretched sample's top (upper grip end) and the fixed bottom, rig viewBox.
pub const RIG_TOP_Y: f64 = 190.0;
/// See [`RIG_TOP_Y`].
pub const RIG_BOT_Y: f64 = 452.0;
/// The dog-bone's grip half-width, rig viewBox.
pub const RIG_GRIP_HALF: f64 = 24.0;
/// The dog-bone's gauge half-width, rig viewBox.
pub const RIG_GAUGE_HALF: f64 = 13.0;
/// The v1 bench's drawn stretch: rig units per % strain ("stretch exaggerated").
pub const RIG_PX_PER_PERCENT: f64 = 4.5;

/// An SVG `viewBox` drawn into a box with the default `preserveAspectRatio` (uniform scale to
/// fit, centred).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewBox {
    /// Screen position of the viewBox origin.
    pub origin: Pos2,
    /// Screen px per viewBox unit.
    pub scale: f32,
}

impl ViewBox {
    /// The viewBox `size` fitted into `rect`.
    pub fn meet(rect: Rect, size: [f64; 2]) -> Self {
        let (w, h) = (size[0] as f32, size[1] as f32);
        let scale = (rect.width() / w).min(rect.height() / h).max(0.0);
        let origin = rect.center() - vec2(w * scale, h * scale) / 2.0;
        Self { origin, scale }
    }

    /// A viewBox point on screen.
    pub fn pt(&self, p: Pt) -> Pos2 {
        self.origin + vec2(p[0] as f32, p[1] as f32) * self.scale
    }

    /// Many points.
    pub fn pts(&self, ps: &[Pt]) -> Vec<Pos2> {
        ps.iter().map(|p| self.pt(*p)).collect()
    }

    /// A viewBox rectangle `[x0, y0, x1, y1]` on screen.
    pub fn rect(&self, r: [f64; 4]) -> Rect {
        Rect::from_two_pos(self.pt([r[0], r[1]]), self.pt([r[2], r[3]]))
    }

    /// A stroke width in viewBox units on screen.
    pub fn w(&self, width: f32) -> f32 {
        width * self.scale
    }
}

// ============================================================================================
// The rig

/// A readout chip over the rig's top left corner (`.lbl` title, mono value in the accent,
/// muted note): "Load cell" · `12.4 kN` · "50 mm² section".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigReadout {
    /// The eyebrow ("Load cell").
    pub title: String,
    /// The value ("12.4 kN").
    pub value: String,
    /// The note under it ("50 mm² section"; may be empty).
    pub note: String,
}

/// What [`rig`] draws: the moving parts in rig viewBox units (the frame and the bottom grip
/// are static).
#[derive(Debug, Clone, PartialEq)]
pub struct RigScene {
    /// The crosshead: its block `[x0, y0, x1, y1]` and the beam through the columns.
    pub head: [[f64; 4]; 2],
    /// The crosshead block's top face (2.5D).
    pub head_top: [Pt; 4],
    /// The sample outline: one polygon, or two halves once torn.
    pub specimen: Vec<Vec<Pt>>,
    /// The sample's fill (outlined in the mockup's near-black).
    pub fill: Color32,
    /// The extensometer: its bar and two knife edges.
    pub gauge: [[Pt; 2]; 3],
    /// The red crack line at the break.
    pub crack: Option<Vec<Pt>>,
    /// The chip at the top left.
    pub readout: Option<RigReadout>,
    /// The badge at the bottom right ("stretch exaggerated").
    pub badge: Option<String>,
    /// The accessible description (`role="img"`).
    pub label: String,
}

impl RigScene {
    /// A dog-bone sample of colour `fill`, its top `stretch` rig units above the unstretched
    /// position (negative: pushed shorter), its gauge necked by `necking` (0 – 0.55 of the
    /// gauge width), torn in two at the gauge middle when `broken` (the halves keep the necked
    /// width when `necked_break`, as a ductile metal; a brittle one snaps square). The crosshead
    /// sits on the sample's top. No readout, badge or label yet.
    pub fn dogbone(
        stretch: f64,
        necking: f64,
        broken: bool,
        necked_break: bool,
        fill: Color32,
    ) -> Self {
        let stretch = if stretch.is_finite() {
            stretch.clamp(-120.0, 140.0)
        } else {
            0.0
        };
        let necking = if necking.is_finite() {
            necking.clamp(0.0, 0.9)
        } else {
            0.0
        };
        let top_y = RIG_TOP_Y - stretch;
        let bot_y = RIG_BOT_Y;
        let mid_y = (top_y + 40.0 + bot_y) / 2.0;
        let (cx, gw, sw) = (RIG_CX, RIG_GAUGE_HALF, RIG_GRIP_HALF);
        let gw_n = gw * (1.0 - necking);
        let side = |sgn: f64| -> [Pt; 7] {
            [
                [cx + sgn * sw, top_y],
                [cx + sgn * sw, top_y + 40.0],
                [cx + sgn * gw, top_y + 60.0],
                [cx + sgn * gw_n, mid_y],
                [cx + sgn * gw, bot_y - 20.0],
                [cx + sgn * sw, bot_y],
                [cx + sgn * sw, bot_y + 20.0],
            ]
        };
        let specimen = if !broken {
            let mut poly: Vec<Pt> = side(-1.0).to_vec();
            poly.extend(side(1.0).iter().rev());
            vec![poly]
        } else {
            let gap = 10.0;
            let w2 = if necked_break { gw_n } else { gw };
            vec![
                vec![
                    [cx - sw, top_y],
                    [cx + sw, top_y],
                    [cx + sw, top_y + 40.0],
                    [cx + gw, top_y + 60.0],
                    [cx + w2, mid_y - gap],
                    [cx - w2, mid_y - gap],
                    [cx - gw, top_y + 60.0],
                    [cx - sw, top_y + 40.0],
                ],
                vec![
                    [cx - w2, mid_y + gap],
                    [cx + w2, mid_y + gap],
                    [cx + gw, bot_y - 20.0],
                    [cx + sw, bot_y],
                    [cx + sw, bot_y + 20.0],
                    [cx - sw, bot_y + 20.0],
                    [cx - sw, bot_y],
                    [cx - gw, bot_y - 20.0],
                ],
            ]
        };
        let crack = broken.then(|| {
            vec![
                [cx - gw_n - 4.0, mid_y - 10.0],
                [cx - 4.0, mid_y - 4.0],
                [cx + 4.0, mid_y - 9.0],
                [cx + gw_n + 4.0, mid_y - 3.0],
            ]
        });
        let gauge = [
            [
                [cx + sw + 10.0, top_y + 60.0],
                [cx + sw + 10.0, bot_y - 20.0],
            ],
            [
                [cx + sw + 4.0, top_y + 60.0],
                [cx + sw + 16.0, top_y + 60.0],
            ],
            [
                [cx + sw + 4.0, bot_y - 20.0],
                [cx + sw + 16.0, bot_y - 20.0],
            ],
        ];
        let hy = top_y - 30.0;
        Self {
            head: [
                [150.0, hy, 270.0, hy + 30.0],
                [112.0, hy + 6.0, 308.0, hy + 18.0],
            ],
            head_top: [
                [150.0, hy],
                [162.0, hy - 8.0],
                [282.0, hy - 8.0],
                [270.0, hy],
            ],
            specimen,
            fill,
            gauge,
            crack,
            readout: None,
            badge: None,
            label: String::new(),
        }
    }

    /// With the readout chip.
    pub fn readout(mut self, title: &str, value: &str, note: &str) -> Self {
        self.readout = Some(RigReadout {
            title: title.into(),
            value: value.into(),
            note: note.into(),
        });
        self
    }

    /// With the bottom-right badge.
    pub fn badge(mut self, text: &str) -> Self {
        self.badge = Some(text.into());
        self
    }

    /// With the accessible description.
    pub fn label(mut self, text: &str) -> Self {
        self.label = text.into();
        self
    }

    /// The v1 Materials bench's rig (geometry from `sp_engine::materials`, exact to the
    /// mockup): its sample colour, "Load cell" chip and "stretch exaggerated" badge.
    pub fn from_bench(v: &sp_engine::materials::View) -> Self {
        let r = &v.rig;
        Self {
            head: r.head,
            head_top: r.head_top,
            specimen: r.specimen.clone(),
            fill: mt::SAMPLES[v.kind.index()].fill,
            gauge: r.gauge,
            crack: r.crack.map(|c| c.to_vec()),
            readout: Some(RigReadout {
                title: "Load cell".into(),
                value: v.force_label.clone(),
                note: v.section_label.clone(),
            }),
            badge: Some("stretch exaggerated".into()),
            label: v.rig_label.clone(),
        }
    }
}

/// Draws the rig into `rect` (a `.graph` box; the mockup's is `aspect-ratio: 420 / 560`).
pub fn rig(ui: &mut Ui, rect: Rect, scene: &RigScene) {
    let mut stage = crate::layout::graph_panel(ui, rect, None);
    // the SVG and the overlays are `position: absolute` in the padding box (inside the border)
    let inner = rect.shrink(1.0);
    let vb = ViewBox::meet(inner, RIG_VIEW);
    let p = stage.painter().clone();
    let px = |x: f64, y: f64| vb.pt([x, y]);

    // the frame
    fill_convex(
        &p,
        vec![
            px(60.0, 500.0),
            px(360.0, 500.0),
            px(392.0, 480.0),
            px(92.0, 480.0),
        ],
        mt::BASE_TOP,
    );
    p.rect_filled(vb.rect([60.0, 500.0, 360.0, 530.0]), 0.0, mt::BASE_FRONT);
    p.rect_filled(vb.rect([90.0, 60.0, 112.0, 500.0]), 0.0, mt::COLUMN);
    p.rect_filled(vb.rect([308.0, 60.0, 330.0, 500.0]), 0.0, mt::COLUMN);
    for x in [112.0, 330.0] {
        fill_convex(
            &p,
            vec![
                px(x, 60.0),
                px(x + 14.0, 52.0),
                px(x + 14.0, 488.0),
                px(x, 500.0),
            ],
            mt::COLUMN_SIDE,
        );
    }
    p.rect_filled(vb.rect([80.0, 34.0, 340.0, 60.0]), 0.0, mt::BEAM);

    // the crosshead (block + beam through the columns) and its top face
    let head_stroke = Stroke::new(vb.w(1.0), mt::HEAD_STROKE);
    for h in scene.head {
        p.rect(vb.rect(h), 0.0, mt::HEAD, head_stroke, StrokeKind::Middle);
    }
    fill_convex(&p, vb.pts(&scene.head_top), mt::HEAD_TOP);
    // the bottom grip
    p.rect(
        vb.rect([176.0, 452.0, 244.0, 480.0]),
        0.0,
        mt::HEAD,
        head_stroke,
        StrokeKind::Middle,
    );

    // the sample (one piece, or two halves once torn)
    for poly in &scene.specimen {
        let ps = vb.pts(poly);
        fill_polygon(&p, &ps, scene.fill);
        if ps.len() >= 2 {
            p.add(Shape::closed_line(
                ps,
                Stroke::new(vb.w(1.0), mt::SPECIMEN_STROKE),
            ));
        }
    }
    // the extensometer
    let gauge = Stroke::new(vb.w(1.5), mt::GAUGE);
    for seg in scene.gauge {
        p.line_segment([vb.pt(seg[0]), vb.pt(seg[1])], gauge);
    }
    // the crack
    if let Some(c) = &scene.crack
        && c.len() >= 2
    {
        p.add(Shape::line(vb.pts(c), Stroke::new(vb.w(2.0), mt::CRACK)));
    }

    if let Some(r) = &scene.readout {
        readout_chip(&mut stage, inner, r);
    }
    if let Some(text) = &scene.badge {
        let b = Badge::plain(text.clone());
        let size = badge_size(&stage, &b);
        let br = Rect::from_min_size(inner.max - vec2(10.0, 10.0) - size, size);
        paint_badge(&stage, br, &b, BadgeLook::default());
    }

    crate::layout::graph_panel_border(ui, rect);
    // `role="img" aria-label="…"`
    let resp = ui.interact(rect, ui.scope_id().with("tensile-rig"), Sense::hover());
    let label = scene.label.clone();
    resp.widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
}

/// The readout chip: `position: absolute; left: 10px; top: 10px; padding: 8px 10px;
/// border-radius: 8px; background: rgba(10,10,10,.82); border: 1px solid #242424; gap: 2px`
/// with the `.lbl` title, the value (mono 16, accent) and the note (11 px muted). A `Group`
/// named by the title, holding one label with all three.
fn readout_chip(ui: &mut Ui, stage: Rect, r: &RigReadout) {
    let lg = lbl_galley(ui, &r.title, t::TEXT_MUTED);
    let ff = lab_theme::font::mono(16.0);
    let fg = galley(ui, &r.value, ff.clone(), t::ACCENT);
    let nf = lab_theme::font::body(11.0);
    let ng = galley(ui, &r.note, nf.clone(), t::TEXT_MUTED);
    let (lh, fh, nh) = (
        css_line_height(ui, &lab_theme::font::semibold(11.0)),
        css_line_height(ui, &ff),
        if r.note.is_empty() {
            0.0
        } else {
            css_line_height(ui, &nf)
        },
    );
    let gaps = if r.note.is_empty() { 2.0 } else { 4.0 };
    let w = lg.size().x.max(fg.size().x).max(ng.size().x) + 2.0 * 10.0 + 2.0;
    let h = lh + fh + nh + gaps + 2.0 * 8.0 + 2.0;
    let rect = Rect::from_min_size(stage.min + vec2(10.0, 10.0), vec2(w, h));
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(t::radius::WELL),
        mt::OVERLAY,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    let x = rect.left() + 11.0;
    let mut y = rect.top() + 9.0;
    paint_line(p, &lg, x, y, lh, t::TEXT_MUTED);
    y += lh + 2.0;
    paint_line(p, &fg, x, y, fh, t::ACCENT);
    if !r.note.is_empty() {
        y += fh + 2.0;
        paint_line(p, &ng, x, y, nh, t::TEXT_MUTED);
    }
    let child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .accessibility_role(Role::Group)
            .accessibility_label(r.title.clone()),
    );
    set_bounds(&child, rect);
    let resp = child.interact(rect, child.scope_id().with("value"), Sense::hover());
    let label = if r.note.is_empty() {
        format!("{} {}", r.title, r.value)
    } else {
        format!("{} {} \u{b7} {}", r.title, r.value, r.note)
    };
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &label));
}

fn fill_convex(painter: &Painter, ps: Vec<Pos2>, fill: Color32) {
    painter.add(Shape::convex_polygon(ps, fill, Stroke::NONE));
}

// ============================================================================================
// The plot

/// A tick label: its text and its position along the axis (plot viewBox units).
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    /// The label ("100", "0.5").
    pub label: String,
    /// x for a strain tick, y for a stress tick.
    pub pos: f64,
}

/// A curve's name label, anchored at a point (text starts 4 px right of it, its bottom just
/// above it): `#F5F5F5` 600 when `selected`, else `#8A8A8A` 400.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveLabel {
    /// The name.
    pub name: String,
    /// The anchor, plot viewBox units.
    pub at: Pt,
    /// The selected curve's label.
    pub selected: bool,
}

/// What [`plot`] draws, in plot viewBox units.
#[derive(Debug, Clone, PartialEq)]
pub struct PlotScene {
    /// The axes polyline (`M56 20 V350 H544`).
    pub axes: [Pt; 3],
    /// Grid lines.
    pub grid: Vec<[Pt; 2]>,
    /// Other curves, drawn as grey ghosts.
    pub ghosts: Vec<Vec<Pt>>,
    /// The curve.
    pub curve: Vec<Pt>,
    /// The curve's colour (`sp_theme::materials::SAMPLES[..].line` for the v1 samples).
    pub curve_color: Color32,
    /// The done-so-far trace (in the accent; empty: none).
    pub done: Vec<Pt>,
    /// The marker (an accent dot) at the current point.
    pub marker: Option<Pt>,
    /// Stress tick labels (`pos` = y).
    pub ticks_y: Vec<Tick>,
    /// Strain tick labels (`pos` = x).
    pub ticks_x: Vec<Tick>,
    /// The y axis title, top left ("stress MPa").
    pub y_title: String,
    /// The x axis title, bottom right ("strain %").
    pub x_title: String,
    /// Curve name labels.
    pub labels: Vec<CurveLabel>,
    /// The accessible description (`role="img"`).
    pub label: String,
}

impl PlotScene {
    /// An empty plot with axes, grid and tick labels for strain 0 – `x_max` (ticks at
    /// `x_ticks`) and stress 0 – `y_max` (ticks at `y_ticks`), titled "stress MPa" /
    /// "strain %". Ranges that aren't positive and finite fall back to 1.
    pub fn axes(x_max: f64, x_ticks: &[f64], y_max: f64, y_ticks: &[f64]) -> Self {
        let mut s = Self {
            axes: [[PLOT_X0, PLOT_Y1], [PLOT_X0, PLOT_Y0], [PLOT_X1, PLOT_Y0]],
            grid: Vec::new(),
            ghosts: Vec::new(),
            curve: Vec::new(),
            curve_color: t::TEXT,
            done: Vec::new(),
            marker: None,
            ticks_y: Vec::new(),
            ticks_x: Vec::new(),
            y_title: "stress MPa".into(),
            x_title: "strain %".into(),
            labels: Vec::new(),
            label: String::new(),
        };
        let (xm, ym) = (positive(x_max), positive(y_max));
        for &v in y_ticks.iter().filter(|v| v.is_finite()) {
            let y = s.y(v, ym);
            s.grid.push([[PLOT_X0, y], [PLOT_X1, y]]);
            s.ticks_y.push(Tick {
                label: sp_engine::jsnum::to_string(v),
                pos: y,
            });
        }
        for &v in x_ticks.iter().filter(|v| v.is_finite()) {
            let x = s.x(v, xm);
            s.grid.push([[x, PLOT_Y1], [x, PLOT_Y0]]);
            s.ticks_x.push(Tick {
                label: sp_engine::jsnum::to_string(v),
                pos: x,
            });
        }
        s
    }

    fn x(&self, v: f64, max: f64) -> f64 {
        PLOT_X0 + (v / max).clamp(-0.1, 1.1) * (PLOT_X1 - PLOT_X0)
    }

    fn y(&self, v: f64, max: f64) -> f64 {
        PLOT_Y0 - (v / max).clamp(-0.1, 1.1) * (PLOT_Y0 - PLOT_Y1)
    }

    /// A (strain, stress) point in the plot area for these ranges (kept a little inside the
    /// box when out of range; non-finite values sit at 0).
    pub fn point(strain: f64, stress: f64, x_max: f64, y_max: f64) -> Pt {
        let f = |v: f64| if v.is_finite() { v } else { 0.0 };
        let (xm, ym) = (positive(x_max), positive(y_max));
        [
            PLOT_X0 + (f(strain) / xm).clamp(-0.1, 1.1) * (PLOT_X1 - PLOT_X0),
            PLOT_Y0 - (f(stress) / ym).clamp(-0.1, 1.1) * (PLOT_Y0 - PLOT_Y1),
        ]
    }

    /// The v1 Materials bench's plot (geometry from `sp_engine::materials`, exact to the
    /// mockup): ghosts of the other four samples, the selected curve in its colour, labels.
    pub fn from_bench(v: &sp_engine::materials::View) -> Self {
        let g = &v.plot;
        Self {
            axes: g.axes,
            grid: g.grid.clone(),
            ghosts: g.ghosts.iter().map(|(_, c)| c.clone()).collect(),
            curve: g.curve.clone(),
            curve_color: mt::SAMPLES[v.kind.index()].line,
            done: g.done.clone(),
            marker: Some(g.marker),
            ticks_y: g
                .ticks_y
                .iter()
                .map(|t| Tick {
                    label: t.label.clone(),
                    pos: t.pos,
                })
                .collect(),
            ticks_x: g
                .ticks_x
                .iter()
                .map(|t| Tick {
                    label: t.label.clone(),
                    pos: t.pos,
                })
                .collect(),
            y_title: "stress MPa".into(),
            x_title: "strain %".into(),
            labels: g
                .labels
                .iter()
                .map(|l| CurveLabel {
                    name: l.name.into(),
                    at: l.at,
                    selected: l.selected,
                })
                .collect(),
            label: v.plot_label.clone(),
        }
    }
}

fn positive(v: f64) -> f64 {
    if v.is_finite() && v > 0.0 { v } else { 1.0 }
}

/// Draws the plot into `rect` (a `.graph` box).
pub fn plot(ui: &mut Ui, rect: Rect, scene: &PlotScene) {
    let stage = crate::layout::graph_panel(ui, rect, None);
    // the SVG and the labels are `position: absolute` in the padding box (inside the border)
    let outer = rect;
    let rect = outer.shrink(1.0);
    let vb = ViewBox::meet(rect, PLOT_VIEW);
    let p = stage.painter().clone();

    let line = |pts: &[Pt], stroke: Stroke| {
        if pts.len() >= 2 {
            p.add(Shape::line(vb.pts(pts), stroke));
        }
    };
    line(&scene.axes, Stroke::new(vb.w(1.2), mt::AXIS));
    for l in &scene.grid {
        line(l, Stroke::new(vb.w(1.0), mt::GRID));
    }
    for ghost in &scene.ghosts {
        line(ghost, Stroke::new(vb.w(1.5), mt::GHOST_CURVE));
    }
    line(&scene.curve, Stroke::new(vb.w(3.0), scene.curve_color));
    round_joins(&p, &vb, &scene.curve, vb.w(3.0), scene.curve_color);
    line(&scene.done, Stroke::new(vb.w(3.0), mt::DONE));
    round_joins(&p, &vb, &scene.done, vb.w(3.0), mt::DONE);
    if let Some(m) = scene.marker {
        p.circle(
            vb.pt(m),
            vb.w(6.0),
            mt::DONE,
            Stroke::new(vb.w(2.0), mt::MARKER_RING),
        );
    }

    // HTML labels, positioned in % of the box
    let at = |xp: f64, yp: f64| -> Pos2 {
        pos2(
            rect.left() + rect.width() * xp as f32,
            rect.top() + rect.height() * yp as f32,
        )
    };
    let tick_font = lab_theme::font::mono(10.0);
    let tick_h = css_line_height(&stage, &tick_font);
    for tk in &scene.ticks_y {
        // left: 4px; top: y%; translateY(-50%)
        let gal = galley(&stage, &tk.label, tick_font.clone(), mt::TICK);
        let y = at(0.0, tk.pos / PLOT_VIEW[1]).y;
        p.galley(
            pos2(
                rect.left() + 4.0,
                y - tick_h / 2.0 + text_offset(&gal, tick_h),
            ),
            gal,
            mt::TICK,
        );
    }
    for tk in &scene.ticks_x {
        // left: x%; top: 90%; translateX(-50%)
        let gal = galley(&stage, &tk.label, tick_font.clone(), mt::TICK);
        let pt = at(tk.pos / PLOT_VIEW[0], 0.9);
        p.galley(
            pos2(pt.x - gal.size().x / 2.0, pt.y + text_offset(&gal, tick_h)),
            gal,
            mt::TICK,
        );
    }
    let title_font = lab_theme::font::mono(11.0);
    let title_h = css_line_height(&stage, &title_font);
    let sg = galley(&stage, &scene.y_title, title_font.clone(), mt::AXIS_TITLE);
    let sp = at(0.12, 0.02);
    p.galley(
        pos2(sp.x, sp.y + text_offset(&sg, title_h)),
        sg,
        mt::AXIS_TITLE,
    );
    let eg = galley(&stage, &scene.x_title, title_font, mt::AXIS_TITLE);
    let ep = at(0.98, 0.94);
    p.galley(
        pos2(ep.x - eg.size().x, ep.y + text_offset(&eg, title_h)),
        eg,
        mt::AXIS_TITLE,
    );
    for cl in &scene.labels {
        // left: x%; top: y%; translate(4px, -110%); 11 px, 600 / 400
        let (font, colour) = if cl.selected {
            (lab_theme::font::semibold(11.0), mt::LABEL_ON)
        } else {
            (lab_theme::font::body(11.0), mt::LABEL_OFF)
        };
        let lh = css_line_height(&stage, &font);
        let gal = galley(&stage, &cl.name, font, colour);
        let pt = at(cl.at[0] / PLOT_VIEW[0], cl.at[1] / PLOT_VIEW[1]);
        p.galley(
            pos2(pt.x + 4.0, pt.y - 1.1 * lh + text_offset(&gal, lh)),
            gal,
            colour,
        );
    }

    crate::layout::graph_panel_border(ui, outer);
    // `role="img" aria-label="…"`
    let resp = ui.interact(outer, ui.scope_id().with("tensile-plot"), Sense::hover());
    let label = scene.label.clone();
    resp.widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
}

/// Offset from a CSS line box's top to the galley's top so the text sits where Chrome puts it.
fn text_offset(g: &eframe::egui::Galley, line_h: f32) -> f32 {
    text_top(line_h / 2.0, g)
}

/// `stroke-linejoin: round`: a dot at every inner corner of a thick polyline.
fn round_joins(p: &Painter, vb: &ViewBox, pts: &[Pt], w: f32, c: Color32) {
    if pts.len() < 3 {
        return;
    }
    for q in &pts[1..pts.len() - 1] {
        p.circle_filled(vb.pt(*q), w / 2.0, c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_engine::materials::{Bench, Kind};

    #[test]
    fn meet_centres_and_scales() {
        let r = Rect::from_min_size(pos2(0.0, 0.0), vec2(542.0, 400.0));
        let vb = ViewBox::meet(r, PLOT_VIEW);
        assert!((vb.scale - 542.0 / 560.0).abs() < 1e-6);
        assert!((vb.pt([0.0, 0.0]).y - (400.0 - 400.0 * vb.scale) / 2.0).abs() < 1e-4);
        assert!((vb.pt([560.0, 0.0]).x - 542.0).abs() < 1e-3);
    }

    /// The generic dog-bone is the v1 bench's geometry for the same stretch, necking and
    /// break (so the drawer's sample looks exactly like the Materials mockup's).
    #[test]
    fn the_dogbone_matches_the_bench() {
        for (kind, strain) in [
            (Kind::Steel, 0.0),
            (Kind::Steel, 12.0),
            (Kind::Aluminum, 30.0),
            (Kind::Concrete, 1.0),
            (Kind::Timber, 0.4),
        ] {
            let mut bench = Bench::default();
            bench.pick(kind);
            bench.set_strain(strain);
            let v = bench.view();
            let from = RigScene::from_bench(&v);
            let ext = strain.min(bench.now().break_strain) * RIG_PX_PER_PERCENT;
            let ours = RigScene::dogbone(
                ext,
                v.rig.necking,
                bench.now().broken,
                kind.curve().necks(),
                from.fill,
            );
            assert_eq!(ours.specimen, from.specimen, "{kind:?} {strain}");
            assert_eq!(ours.head, from.head);
            assert_eq!(ours.head_top, from.head_top);
            assert_eq!(ours.gauge, from.gauge);
            assert_eq!(ours.crack, from.crack, "{kind:?} {strain}");
        }
    }

    #[test]
    fn garbage_in_stays_finite() {
        let r = RigScene::dogbone(f64::NAN, f64::INFINITY, true, true, t::TEXT);
        assert!(r.specimen.iter().flatten().flatten().all(|v| v.is_finite()));
        let p = PlotScene::axes(f64::NAN, &[1.0, f64::NAN], -5.0, &[100.0]);
        assert_eq!(p.ticks_x.len(), 1);
        assert!(p.grid.iter().flatten().flatten().all(|v| v.is_finite()));
        let q = PlotScene::point(f64::NAN, 1e300, 0.0, 700.0);
        assert!(q[0].is_finite() && q[1].is_finite());
        assert_eq!(q[0], PLOT_X0);
    }

    #[test]
    fn axes_put_ticks_where_points_go() {
        let p = PlotScene::axes(25.0, &[5.0, 25.0], 700.0, &[100.0, 700.0]);
        assert_eq!(p.ticks_x[1].pos, PLOT_X1);
        assert_eq!(p.ticks_y[1].pos, PLOT_Y1);
        assert_eq!(
            PlotScene::point(25.0, 700.0, 25.0, 700.0),
            [PLOT_X1, PLOT_Y1]
        );
        assert_eq!(PlotScene::point(0.0, 0.0, 25.0, 700.0), [PLOT_X0, PLOT_Y0]);
        assert_eq!(p.ticks_x[0].label, "5");
    }
}
