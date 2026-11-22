//! The stage (BR-1, BR-2, BR-11): the scaled 1120 × 600 drawing of `Playground.dc.html`'s
//! inline SVG — dot grid, banks, river (dashes flow while testing), the members' 2.5D back
//! faces, members by band and size, crates with cables and tags, supports, sparks, the
//! mechanism ring, the hover dot and ghost member — plus its HTML layer: the grid surface,
//! member handles and joints as AccessKit buttons with the mockup's labels and hit sizes, the
//! toast, the loading bar, the result banner and the legend.
//!
//! Everything shown comes from [`View`]; the stage only maps grid metres to pixels
//! (`sp_engine::bridge::data::{px, py}` × the stage scale) and tones to `sp_theme` colours.

use super::Action;
use super::bits::{label_node, paint_stars, stars_width};
use crate::widgets::text::{galley, lbl_galley, text_top};
use crate::widgets::{ghost, prim};
use eframe::egui::{
    Color32, CornerRadius, CursorIcon, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind,
    Ui, Vec2, WidgetInfo, accesskit::Role, epaint::PathShape, pos2, vec2,
};
use lab_theme::contrast::over;
use lab_ui::paint::focus_ring_if;
use sp_engine::bridge::data::{STAGE_H, STAGE_W, px, py, snap};
use sp_engine::bridge::design::Band;
use sp_engine::bridge::view::{Tone, View};
use sp_engine::bridge::{SizeKey, Support};
use sp_theme::{self as t, bridge as b};

/// What the stage needs besides the view.
pub struct StageInput<'a> {
    /// The view.
    pub view: &'a View,
    /// Water dash offset, stage px (grows while the water flows).
    pub flow: f32,
    /// A joint is being dragged.
    pub dragging: bool,
}

/// Maps stage px to screen points.
#[derive(Debug, Clone, Copy)]
pub struct Xf {
    origin: Pos2,
    s: f32,
}

impl Xf {
    /// The transform for a stage drawn into `rect`.
    pub fn new(rect: Rect) -> Self {
        Self {
            origin: rect.min,
            s: rect.width() / STAGE_W as f32,
        }
    }
    /// Stage px → screen.
    pub fn p(&self, x: f64, y: f64) -> Pos2 {
        self.origin + vec2(x as f32 * self.s, y as f32 * self.s)
    }
    /// Grid metres → screen.
    pub fn m(&self, at: [f64; 2]) -> Pos2 {
        self.p(px(at[0]), py(at[1]))
    }
    /// A stage length → points.
    pub fn l(&self, v: f32) -> f32 {
        v * self.s
    }
    /// Screen → stage px.
    pub fn inv(&self, p: Pos2) -> (f64, f64) {
        (
            ((p.x - self.origin.x) / self.s) as f64,
            ((p.y - self.origin.y) / self.s) as f64,
        )
    }
}

/// The stage's height for a width (`aspect-ratio: 1120 / 600`).
pub fn height_for(width: f32) -> f32 {
    width * (STAGE_H / STAGE_W) as f32
}

/// Colour of a tone.
pub fn tone_color(tone: Tone) -> Color32 {
    match tone {
        Tone::Muted => t::TEXT_MUTED,
        Tone::Ok => t::OK,
        Tone::Warn => t::WARN,
        Tone::Danger => t::DANGER,
    }
}

/// A member band's colour, pre-blended over the stage (SVG strokes a whole path at once, so
/// shared joints don't double the opacity) and whether it is dashed.
fn band_style(band: Band) -> (Color32, bool) {
    let base = match band {
        Band::T1 | Band::T2 | Band::T3 => t::TENSION,
        Band::C1 | Band::C2 | Band::C3 => t::COMPRESSION,
        Band::Zero => t::ZERO_FORCE,
        Band::Over => t::OVER,
        Band::None => t::NO_FORCE,
    };
    let alpha = band.level().map_or(1.0, |l| t::FORCE_BANDS[l]);
    (
        over(base.gamma_multiply(alpha), b::STAGE),
        band == Band::Over,
    )
}

/// A round-capped segment.
fn round_segment(painter: &Painter, a: Pos2, c: Pos2, w: f32, color: Color32) {
    painter.line_segment([a, c], Stroke::new(w, color));
    painter.circle_filled(a, w / 2.0, color);
    painter.circle_filled(c, w / 2.0, color);
}

/// A dashed segment (`stroke-dasharray: dash gap`, restarting at `a`), round caps optional.
#[allow(clippy::too_many_arguments)]
fn dashed_segment(
    painter: &Painter,
    a: Pos2,
    c: Pos2,
    w: f32,
    color: Color32,
    dash: f32,
    gap: f32,
    offset: f32,
    round: bool,
) {
    let len = (c - a).length();
    if len <= 0.0 || dash <= 0.0 {
        return;
    }
    let dir = (c - a) / len;
    let period = dash + gap;
    // dashoffset shifts the pattern backwards along the path
    let mut s = -(offset.rem_euclid(period));
    while s < len {
        let s0 = s.max(0.0);
        let s1 = (s + dash).min(len);
        if s1 > s0 {
            let (p0, p1) = (a + dir * s0, a + dir * s1);
            if round {
                round_segment(painter, p0, p1, w, color);
            } else {
                painter.line_segment([p0, p1], Stroke::new(w, color));
            }
        }
        s += period;
    }
}

fn polygon(painter: &Painter, xf: &Xf, pts: &[(f64, f64)], fill: Color32) {
    let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| xf.p(x, y)).collect();
    painter.add(Shape::Path(PathShape::convex_polygon(
        pts,
        fill,
        Stroke::NONE,
    )));
}

/// Paints the scene (the SVG layer).
pub fn paint_scene(painter: &Painter, rect: Rect, inp: &StageInput<'_>) {
    let xf = Xf::new(rect);
    let v = inp.view;
    painter.rect_filled(rect, CornerRadius::same(t::radius::PANEL), b::STAGE);
    // the snap grid: dots every 36 px from (56, 380)
    let r = xf.l(1.3);
    // sub-pixel dots fade instead of feathering into visible specks (the browser's look)
    let dot = if r < 0.7 {
        b::GRID_DOT.gamma_multiply((r / 0.7).powi(2))
    } else {
        b::GRID_DOT
    };
    let mut gx = 56.0 - 36.0;
    while gx <= STAGE_W + 1.0 {
        let mut gy = 380.0 - 36.0 * 11.0;
        while gy <= STAGE_H + 1.0 {
            if gx >= -1.0 && gy >= -1.0 {
                painter.circle_filled(xf.p(gx, gy), r.max(0.5), dot);
            }
            gy += 36.0;
        }
        gx += 36.0;
    }
    // the river, its dashes, the riverbed, the banks
    painter.rect_filled(
        Rect::from_min_max(xf.p(197.0, 524.0), xf.p(923.0, 600.0)),
        CornerRadius::ZERO,
        b::WATER,
    );
    for (x0, x1, y) in [
        (215.0, 905.0, 540.0),
        (240.0, 900.0, 558.0),
        (205.0, 890.0, 576.0),
    ] {
        dashed_segment(
            painter,
            xf.p(x0, y),
            xf.p(x1, y),
            xf.l(2.0),
            b::WATER_FLOW,
            xf.l(18.0),
            xf.l(30.0),
            -xf.l(inp.flow),
            false,
        );
    }
    painter.rect_filled(
        Rect::from_min_max(xf.p(194.0, 590.0), xf.p(926.0, 600.0)),
        CornerRadius::ZERO,
        b::BANK,
    );
    polygon(
        painter,
        &xf,
        &[(0.0, 380.0), (200.0, 380.0), (218.0, 368.0), (0.0, 368.0)],
        b::BANK_TOP,
    );
    polygon(
        painter,
        &xf,
        &[(0.0, 380.0), (200.0, 380.0), (194.0, 600.0), (0.0, 600.0)],
        b::BANK,
    );
    polygon(
        painter,
        &xf,
        &[
            (920.0, 380.0),
            (1120.0, 380.0),
            (1120.0, 368.0),
            (938.0, 368.0),
        ],
        b::BANK_TOP,
    );
    polygon(
        painter,
        &xf,
        &[
            (920.0, 380.0),
            (1120.0, 380.0),
            (1120.0, 600.0),
            (926.0, 600.0),
        ],
        b::BANK,
    );

    // back faces, then the ghost member, then the members by band
    let off = vec2(xf.l(18.0), xf.l(-12.0));
    for m in &v.members {
        round_segment(
            painter,
            xf.m(m.a) + off,
            xf.m(m.b) + off,
            xf.l(3.0),
            b::BACK_FACE,
        );
    }
    if let Some((from, to)) = v.ghost {
        dashed_segment(
            painter,
            xf.m(from),
            xf.m(to),
            xf.l(2.0),
            b::GHOST_LINE,
            xf.l(6.0),
            xf.l(6.0),
            0.0,
            false,
        );
    }
    let band_order = [
        Band::T1,
        Band::T2,
        Band::T3,
        Band::C1,
        Band::C2,
        Band::C3,
        Band::Zero,
        Band::None,
        Band::Over,
    ];
    for band in band_order {
        let (color, dashed) = band_style(band);
        for size in SizeKey::ALL {
            let w = xf.l(size.stroke_px());
            for m in v
                .members
                .iter()
                .filter(|m| m.band == band && m.size == size)
            {
                if dashed {
                    dashed_segment(
                        painter,
                        xf.m(m.a),
                        xf.m(m.b),
                        w,
                        color,
                        xf.l(9.0),
                        xf.l(6.0),
                        0.0,
                        true,
                    );
                } else {
                    round_segment(painter, xf.m(m.a), xf.m(m.b), w, color);
                }
            }
        }
    }

    // crates: cable, side, top, front
    for l in &v.loads {
        let (x, y) = (px(l.at[0]), py(l.at[1]));
        let w = 12.0 + l.tonnes.max(0.0).sqrt() * 4.0;
        let h = w * 0.8;
        let top = y + 9.0;
        painter.line_segment(
            [xf.p(x, y + 3.0), xf.p(x, top)],
            Stroke::new(xf.l(1.2), b::SUPPORT),
        );
        polygon(
            painter,
            &xf,
            &[
                (x + w / 2.0, top),
                (x + w / 2.0 + 6.0, top - 4.0),
                (x + w / 2.0 + 6.0, top - 4.0 + h),
                (x + w / 2.0, top + h),
            ],
            b::CRATE_SIDE,
        );
        polygon(
            painter,
            &xf,
            &[
                (x - w / 2.0, top),
                (x - w / 2.0 + 6.0, top - 4.0),
                (x + w / 2.0 + 6.0, top - 4.0),
                (x + w / 2.0, top),
            ],
            b::CRATE_TOP,
        );
        let front = Rect::from_min_max(xf.p(x - w / 2.0, top), xf.p(x + w / 2.0, top + h));
        painter.rect(
            front,
            CornerRadius::ZERO,
            b::CRATE,
            Stroke::new(xf.l(1.0), b::CRATE_STROKE),
            StrokeKind::Middle,
        );
    }
    // supports at the joints' own positions
    for s in &v.supports {
        let (x, y) = (px(s.at[0]), py(s.at[1]));
        polygon(
            painter,
            &xf,
            &[(x, y), (x - 9.0, y + 13.0), (x + 9.0, y + 13.0)],
            b::SUPPORT,
        );
        if s.kind == Support::Roller {
            for dx in [-5.0, 5.0] {
                painter.circle_filled(xf.p(x + dx, y + 20.0), xf.l(3.0), b::SUPPORT);
            }
        }
    }
    // sparks
    for k in &v.sparks {
        let c = xf.m(*k);
        let d = xf.l(6.0);
        let w = xf.l(2.5);
        round_segment(painter, c + vec2(-d, -d), c + vec2(d, d), w, t::DANGER);
        round_segment(painter, c + vec2(d, -d), c + vec2(-d, d), w, t::DANGER);
    }
    // the mechanism ring: r 13, dashed 4 3
    if let Some(bad) = v.bad {
        let c = xf.m(bad);
        let rr = xf.l(13.0);
        let n = 64;
        let pts: Vec<Pos2> = (0..=n)
            .map(|i| {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                c + vec2(a.cos() * rr, a.sin() * rr)
            })
            .collect();
        painter.extend(Shape::dashed_line(
            &pts,
            Stroke::new(xf.l(2.0), t::DANGER),
            xf.l(4.0),
            xf.l(3.0),
        ));
    }
    if let Some(h) = v.hover {
        painter.circle_filled(xf.m(h), xf.l(5.0), b::HOVER_DOT);
    }
}

/// Draws the whole stage into `rect` and collects what was clicked.
pub fn show(ui: &mut Ui, rect: Rect, inp: &StageInput<'_>, actions: &mut Vec<Action>) {
    let v = inp.view;
    let xf = Xf::new(rect);
    let mut child = crate::layout::graph_panel(ui, rect, Some(b::STAGE));
    let ui = &mut child;
    let painter = ui.painter().clone();
    paint_scene(&painter, rect, inp);
    // the scene's summary for assistive tech (`role="img" aria-label`)
    let img = ui.interact(rect, Id::unique("bridge-stage-img"), Sense::hover());
    let label = v.stage_label.clone();
    img.widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));

    // the grid surface (a button under everything else)
    let surface = ui.interact(rect, Id::unique("bridge-surface"), Sense::click());
    let sl = v.surface_label.to_string();
    surface.widget_info(|| WidgetInfo::labeled(Role::Button, true, &sl));
    if surface.clicked()
        && let Some(p) = surface.interact_pointer_pos()
    {
        let (sx, sy) = xf.inv(p);
        let (gx, gy) = snap(sx, sy);
        actions.push(Action::ClickGrid(gx, gy));
    }
    if surface.hovered() && v.crosshair {
        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
    }
    focus_ring_if(
        ui,
        &surface,
        rect.shrink(1.0),
        CornerRadius::same(t::radius::PANEL),
    );

    // pointer: hover dot / drag
    let pointer = ui.input(|i| i.pointer.latest_pos());
    let primary_down = ui.input(|i| i.pointer.primary_down());
    match pointer.filter(|p| rect.contains(*p)) {
        Some(p) => {
            let (sx, sy) = xf.inv(p);
            let (gx, gy) = snap(sx, sy);
            if inp.dragging {
                if primary_down {
                    actions.push(Action::PointerMove(gx, gy));
                }
            } else if v.crosshair {
                actions.push(Action::PointerMove(gx, gy));
            }
        }
        None => {
            if v.hover.is_some() {
                actions.push(Action::ClearHover);
            }
        }
    }
    if inp.dragging && !primary_down {
        actions.push(Action::PointerUp);
    }

    // member handles
    for h in &v.handles {
        let c = xf.m(h.at);
        let font = lab_theme::font::mono_medium(10.0);
        let g = galley(ui, &h.text, font, t::TEXT_DIM);
        let (w, hh, radius) = if h.numeric {
            ((g.size().x + 14.0).max(14.0), t::size::HANDLE_NUM, 5u8)
        } else {
            (14.0, t::size::HANDLE, 7u8)
        };
        let r = Rect::from_center_size(c, vec2(w, hh));
        let resp = ui.interact(r, Id::unique(("bridge-handle", &h.id)), Sense::click());
        let label = h.label.clone();
        resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
        let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        if resp.clicked() {
            actions.push(Action::ClickMember(h.id.clone()));
        }
        let (border, fg) = if h.selected {
            (t::ACCENT, t::INK)
        } else if h.over {
            (t::DANGER, t::DANGER_SOFT)
        } else {
            (t::BORDER_OVERLAY, t::TEXT_DIM)
        };
        painter.rect(
            r,
            CornerRadius::same(radius),
            b::HANDLE,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );
        if !h.text.is_empty() {
            let g = galley(ui, &h.text, lab_theme::font::mono_medium(10.0), fg);
            painter.galley(pos2(c.x - g.size().x / 2.0, text_top(c.y, &g)), g, fg);
        }
        focus_ring_if(ui, &resp, r, CornerRadius::same(radius));
    }

    // joints: 30 × 30 buttons with an 11 px dot
    for j in &v.joints {
        let c = xf.m(j.at);
        let r = Rect::from_center_size(c, Vec2::splat(t::size::JOINT_HIT));
        let resp = ui.interact(
            r,
            Id::unique(("bridge-joint", &j.id)),
            Sense::click_and_drag(),
        );
        let label = j.label.clone();
        resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
        let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        if resp.drag_started() {
            actions.push(Action::JointDown(j.id.clone()));
        }
        if resp.clicked() {
            actions.push(Action::ClickJoint(j.id.clone()));
        }
        let hovered = resp.hovered();
        let d = t::size::JOINT_DOT * if hovered { 1.25 } else { 1.0 };
        let dot = Rect::from_center_size(c, Vec2::splat(d));
        let radius = if j.locked { 3.0 * d / 11.0 } else { d / 2.0 };
        if j.pending {
            painter.rect_filled(
                dot.expand(5.0),
                CornerRadius::same((radius + 5.0) as u8),
                b::JOINT_PENDING_GLOW,
            );
        } else if j.bad {
            painter.rect_filled(
                dot.expand(4.0),
                CornerRadius::same((radius + 4.0) as u8),
                b::JOINT_BAD_HALO,
            );
        } else if j.selected {
            painter.rect_filled(
                dot.expand(4.0),
                CornerRadius::same((radius + 4.0) as u8),
                b::JOINT_SELECTED_HALO,
            );
        }
        let (fill, ring) = if j.pending {
            (t::ACCENT, t::INK)
        } else if j.bad {
            (b::STAGE, t::DANGER)
        } else if hovered {
            (b::STAGE, t::INK)
        } else if j.locked {
            (b::STAGE, t::ACCENT)
        } else {
            (b::STAGE, b::JOINT_RING)
        };
        painter.rect(
            dot,
            CornerRadius::same(radius.round() as u8),
            fill,
            Stroke::new(2.0, ring),
            StrokeKind::Inside,
        );
        focus_ring_if(ui, &resp, r.shrink(4.0), CornerRadius::same(12));
    }

    // load tags under the crates (pointer-events: none)
    for l in &v.loads {
        let (x, y) = (px(l.at[0]), py(l.at[1]));
        let w = 12.0 + l.tonnes.max(0.0).sqrt() * 4.0;
        let top = y + 9.0 + w * 0.8 + 10.0;
        let g = galley(ui, &l.tag, lab_theme::font::mono_medium(11.0), t::LOAD_TAG);
        let anchor = xf.p(x, top);
        let r = Rect::from_min_size(
            pos2(anchor.x - g.size().x / 2.0 - 6.0, anchor.y),
            vec2(g.size().x + 12.0, g.size().y + 2.0),
        );
        painter.rect_filled(r, CornerRadius::same(4), b::TAG);
        painter.galley(pos2(r.left() + 6.0, r.top() + 1.0), g, t::LOAD_TAG);
    }

    legend(ui, &painter, rect, v);
    overlays(ui, &painter, rect, v, actions);
    crate::layout::graph_panel_border(ui, rect);
}

/// The legend (bottom-left): "Stretched (tension) · Squeezed (compression) · Will break ·
/// brighter = closer to its limit", wrapping inside the stage.
fn legend(ui: &Ui, painter: &Painter, rect: Rect, v: &View) {
    let font = lab_theme::font::body(12.0);
    // `position: absolute; left: 12px` shrink-to-fit: up to the stage's right edge
    let max_w = rect.width() - 12.0 - 20.0 - 2.0;
    let items: Vec<(Option<u8>, &str)> = vec![
        (Some(0), v.legend[0]),
        (Some(1), v.legend[1]),
        (Some(2), v.legend[2]),
        (None, v.legend[3]),
    ];
    let line_h = crate::widgets::text::css_line_height(ui, &font);
    let mut placed = Vec::new();
    let (mut x, mut y, mut w_max) = (0.0f32, 0.0f32, 0.0f32);
    for (sw, text) in &items {
        let g = galley(
            ui,
            *text,
            font.clone(),
            if sw.is_some() {
                t::TEXT_DIM
            } else {
                t::TEXT_MUTED
            },
        );
        let w = g.size().x + if sw.is_some() { 24.0 } else { 0.0 };
        if x > 0.0 && x + w > max_w {
            x = 0.0;
            y += line_h + 6.0;
        }
        placed.push((x, y, *sw, g));
        x += w + 14.0;
        w_max = w_max.max(x - 14.0);
    }
    let h = y + line_h;
    let bg = Rect::from_min_size(
        pos2(rect.left() + 12.0, rect.bottom() - 10.0 - h - 12.0 - 2.0),
        vec2(w_max + 20.0 + 2.0, h + 12.0 + 2.0),
    );
    painter.rect(
        bg,
        CornerRadius::same(8),
        b::LEGEND,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    let o = bg.min + vec2(11.0, 7.0);
    for (x, y, sw, g) in placed {
        let cy = o.y + y + line_h / 2.0;
        let mut tx = o.x + x;
        match sw {
            Some(0) | Some(1) => {
                let c = if sw == Some(0) {
                    t::TENSION
                } else {
                    t::COMPRESSION
                };
                painter.rect_filled(
                    Rect::from_min_size(pos2(tx, cy - 2.0), vec2(18.0, 4.0)),
                    CornerRadius::same(2),
                    c,
                );
                tx += 24.0;
            }
            Some(_) => {
                // `border-top: 3px dashed`: Chrome draws two 6 px dashes in 18 px
                dashed_segment(
                    painter,
                    pos2(tx, cy),
                    pos2(tx + 18.0, cy),
                    3.0,
                    t::DANGER,
                    6.0,
                    6.0,
                    0.0,
                    false,
                );
                tx += 24.0;
            }
            None => {}
        }
        let col = if sw.is_some() {
            t::TEXT_DIM
        } else {
            t::TEXT_MUTED
        };
        painter.galley(pos2(tx, text_top(cy, &g)), g, col);
    }
}

/// The toast, the loading bar and the result banner (top centre, 14 px down).
fn overlays(ui: &mut Ui, painter: &Painter, rect: Rect, v: &View, actions: &mut Vec<Action>) {
    let top = rect.top() + 14.0;
    let cx = rect.center().x;
    if let Some(r) = &v.result {
        let title_font = lab_theme::font::heading(15.0);
        let color = if r.held { t::OK } else { t::DANGER };
        let tg = galley(ui, &r.title, title_font, color);
        let stars_n = r.stars.chars().filter(|c| *c == '★').count() as u8;
        let has_stars = !r.stars.is_empty();
        let replay = ghost("Replay in slow motion");
        let back = prim("Back to building");
        let rs = replay.desired_size(ui);
        let bs = back.desired_size(ui);
        let mut items: Vec<Vec2> = vec![tg.size()];
        if has_stars {
            items.push(vec2(stars_width(13.0), 14.0));
        }
        if r.replay {
            items.push(rs);
        }
        items.push(bs);
        let max_w = rect.width() * 0.9 - 28.0 - 2.0;
        // wrap (gap 8 12), centred lines
        let mut lines: Vec<Vec<usize>> = vec![vec![]];
        let mut lw = 0.0;
        for (i, s) in items.iter().enumerate() {
            let need = if lines.last().unwrap().is_empty() {
                s.x
            } else {
                lw + 12.0 + s.x
            };
            if need > max_w && !lines.last().unwrap().is_empty() {
                lines.push(vec![i]);
                lw = s.x;
            } else {
                lines.last_mut().unwrap().push(i);
                lw = need;
            }
        }
        let line_w = |l: &Vec<usize>| {
            l.iter().map(|&i| items[i].x).sum::<f32>() + 12.0 * (l.len().saturating_sub(1)) as f32
        };
        let line_h = |l: &Vec<usize>| l.iter().map(|&i| items[i].y).fold(0.0, f32::max);
        let inner_w = lines.iter().map(line_w).fold(0.0, f32::max);
        let inner_h: f32 =
            lines.iter().map(line_h).sum::<f32>() + 8.0 * (lines.len().saturating_sub(1)) as f32;
        let banner = Rect::from_min_size(
            pos2(cx - (inner_w + 30.0) / 2.0, top),
            vec2(inner_w + 30.0, inner_h + 22.0),
        );
        painter.rect(
            banner,
            CornerRadius::same(12),
            b::BANNER,
            Stroke::new(1.0, if r.held { t::OK_EDGE } else { t::DANGER_EDGE }),
            StrokeKind::Inside,
        );
        let mut y = banner.top() + 11.0;
        let mut k = 0;
        for l in &lines {
            let lh = line_h(l);
            let mut x = cx - line_w(l) / 2.0;
            for &i in l {
                let s = items[i];
                let cell = Rect::from_min_size(pos2(x, y + (lh - s.y) / 2.0), s);
                match (i, k) {
                    (0, _) => {
                        painter.galley(cell.min, tg.clone(), color);
                        label_node(ui, "bridge-result", cell, &r.title);
                    }
                    _ if has_stars && i == 1 => {
                        paint_stars(painter, cell.left(), cell.center().y, stars_n, 13.0);
                        label_node(ui, "bridge-stars", cell, &r.stars);
                    }
                    _ if r.replay && i == items.len() - 2 => {
                        if ghost("Replay in slow motion").show_at(ui, cell).clicked() {
                            actions.push(Action::Replay);
                        }
                    }
                    _ => {
                        if prim("Back to building").show_at(ui, cell).clicked() {
                            actions.push(Action::BackToBuild);
                        }
                    }
                }
                x += s.x + 12.0;
                k += 1;
            }
            y += lh + 8.0;
        }
    } else if let Some(l) = &v.loading {
        let lg = lbl_galley(ui, "Loading", t::TEXT_MUTED);
        let pg = galley(ui, &l.text, lab_theme::font::mono(12.0), t::TEXT);
        let w = 2.0 + 28.0 + lg.size().x + 10.0 + 180.0 + 10.0 + pg.size().x;
        let h = 2.0 + 16.0 + lg.size().y.max(pg.size().y);
        let r = Rect::from_min_size(pos2(cx - w / 2.0, top), vec2(w, h));
        painter.rect(
            r,
            CornerRadius::same(10),
            t::OVERLAY,
            Stroke::new(1.0, t::BORDER_OVERLAY),
            StrokeKind::Inside,
        );
        let cy = r.center().y;
        let mut x = r.left() + 15.0;
        painter.galley(pos2(x, text_top(cy, &lg)), lg.clone(), t::TEXT_MUTED);
        x += lg.size().x + 10.0;
        let bar = Rect::from_min_size(pos2(x, cy - 4.0), vec2(180.0, 8.0));
        super::bits::bar(painter, bar, l.frac as f32, t::ACCENT);
        x += 190.0;
        painter.galley(pos2(x, text_top(cy, &pg)), pg, t::TEXT);
        label_node(ui, "bridge-loading", r, &format!("Loading {}", l.text));
    }
    if let Some(text) = &v.toast {
        // the toast sits where the banner would (the mockup stacks them; a toast only shows
        // while building, where no banner is)
        let g = galley(ui, text, lab_theme::font::body(13.0), t::TEXT);
        let w = g.size().x + 24.0 + 2.0;
        let h = g.size().y + 12.0 + 2.0;
        let r = Rect::from_min_size(pos2(cx - w / 2.0, top), vec2(w, h));
        painter.rect(
            r,
            CornerRadius::same(8),
            t::OVERLAY,
            Stroke::new(1.0, t::BORDER_OVERLAY),
            StrokeKind::Inside,
        );
        painter.galley(pos2(r.left() + 13.0, r.top() + 7.0), g, t::TEXT);
        let resp = label_node(ui, "bridge-toast", r, text);
        let _ = resp;
    }
}
