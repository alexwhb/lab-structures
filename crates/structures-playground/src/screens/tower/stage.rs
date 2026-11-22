//! The Tower stage (TW-3): the mockup's `<svg viewBox="0 0 900 620">` scaled to the stage
//! (aspect kept), drawn in its order: ground and plinth, wind streaks, wind arrows, the tower
//! floor by floor (side faces, front faces, sky slots, system lines, cap), the plumb line and
//! the damper; then the HTML overlays: the pinned "top … · H/…" chip, "Wind at the top" and
//! "sway drawn ×N".
//!
//! Geometry is the mockup's `renderVals()` (4.2 px floors from y 580, centre x 470, base
//! width 96 px, the 2.5D side offset `(0.42·w, −0.26·w)`); the lean follows the first mode
//! shape with the top displacement from [`TowerScreen::top_sway`], drawn × the exaggeration
//! × 1.05.

use super::TowerScreen;
use crate::layout;
use crate::screens::Badge;
use crate::widgets::{Chip, badge::badge_size, badge::paint_badge, world};
use eframe::egui::{
    Align2, Pos2, Rect, Sense, Shape as EShape, Stroke, Ui, WidgetInfo, accesskit::Role,
    epaint::PathShape, epaint::PathStroke, pos2, vec2,
};
use sp_engine::tower::{self, Shape, System, TowerResult};
use sp_theme as t;
use sp_theme::tower as tt;

/// The SVG's width (viewBox units).
pub const W: f32 = 900.0;
/// The SVG's height.
pub const H: f32 = 620.0;
/// Floor height on the stage, px.
pub const FLOOR_PX: f32 = 4.2;
/// The ground line's y.
pub const GROUND_Y: f32 = 580.0;
/// The tower's centre x.
pub const CENTRE_X: f32 = 470.0;
/// The tower's base width, px.
pub const BASE_W: f32 = 96.0;

/// Maps viewBox units to the screen.
#[derive(Debug, Clone, Copy)]
pub struct Map {
    origin: Pos2,
    scale: f32,
}

impl Map {
    /// The map for a stage `rect` (900 × 620 scaled to its width).
    pub fn new(rect: Rect) -> Self {
        Self {
            origin: rect.min,
            scale: rect.width() / W,
        }
    }

    /// A point.
    pub fn p(&self, x: f32, y: f32) -> Pos2 {
        pos2(
            self.origin.x + x * self.scale,
            self.origin.y + y * self.scale,
        )
    }

    /// A length.
    pub fn len(&self, l: f32) -> f32 {
        l * self.scale
    }
}

/// Where the tower's top ended up (viewBox units), for the chip and the damper.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Top {
    /// Front face left / right at the top floor.
    pub xl: f32,
    /// See [`Top::xl`].
    pub xr: f32,
    /// The top floor's top.
    pub y: f32,
    /// The 2.5D offset at the top.
    pub dx: f32,
    /// See [`Top::dx`].
    pub dy: f32,
}

/// The tower's floors as the mockup builds them: per floor the front quad, the side quad, an
/// optional sky slot and the system line segments, plus the top.
pub struct Floors {
    /// Front faces (xl, xr, yb, yt).
    pub fronts: Vec<[f32; 4]>,
    /// Side faces (four corners).
    pub sides: Vec<[Pos2; 4]>,
    /// Sky slots (xl, xr, yb, yt).
    pub holes: Vec<[f32; 4]>,
    /// System line segments.
    pub lines: Vec<[Pos2; 2]>,
    /// The top floor.
    pub top: Top,
}

/// Builds the floors (viewBox units) for the screen's settings with the top displaced by
/// `top_m` metres (the lean or the simulated sway) drawn × `ex` × 1.05.
pub fn floors(s: &TowerScreen, r: &TowerResult, top_m: f64) -> Floors {
    let n = s.input.floors;
    let to_px = |m: f64| (m * s.ex as f64 * 1.05) as f32;
    let mut out = Floors {
        fronts: Vec::with_capacity(n as usize),
        sides: Vec::with_capacity(n as usize),
        holes: Vec::new(),
        lines: Vec::new(),
        top: Top::default(),
    };
    for i in 0..n {
        let plan = tower::floor_plan(s.input.shape, i, n);
        let yb = GROUND_Y - i as f32 * FLOOR_PX;
        let yt = yb - FLOOR_PX;
        let w = BASE_W * plan.w as f32;
        let wf = BASE_W * plan.front as f32;
        let off = BASE_W * plan.offset as f32;
        let sway = top_m * tower::mode_shape(plan.z_mid / r.height);
        let cx = CENTRE_X + to_px(sway) + off;
        let (xl, xr) = (cx - wf / 2.0, cx + wf / 2.0);
        let (dx, dy) = (w * 0.42, -w * 0.26);
        out.fronts.push([xl, xr, yb, yt]);
        out.sides.push([
            pos2(xr, yb),
            pos2(xr + dx, yb + dy),
            pos2(xr + dx, yt + dy),
            pos2(xr, yt),
        ]);
        if s.input.shape == Shape::Hollow && tower::is_sky_slot(i) {
            out.holes.push([cx - wf * 0.22, cx + wf * 0.22, yb, yt]);
        }
        match s.input.system {
            System::Tube => {
                for k in 1..6 {
                    let x = xl + wf * k as f32 / 6.0;
                    out.lines.push([pos2(x, yb), pos2(x, yt)]);
                }
            }
            System::Braced => {
                let m = 10;
                let p = (i % m) as f32 / m as f32;
                let q = ((i % m) + 1) as f32 / m as f32;
                out.lines
                    .push([pos2(xl + wf * p, yb), pos2(xl + wf * q, yt)]);
                out.lines
                    .push([pos2(xr - wf * p, yb), pos2(xr - wf * q, yt)]);
            }
            System::Core | System::Outriggers => {}
        }
        if s.input.system == System::Outriggers && tower::is_outrigger_floor(i, n) {
            out.lines.push([pos2(xl, yt + 1.0), pos2(xr, yt + 1.0)]);
            out.lines.push([pos2(xl, yb - 1.0), pos2(xr, yb - 1.0)]);
        }
        if matches!(s.input.system, System::Core | System::Outriggers) {
            out.lines
                .push([pos2(cx - wf * 0.12, yb), pos2(cx - wf * 0.12, yt)]);
            out.lines
                .push([pos2(cx + wf * 0.12, yb), pos2(cx + wf * 0.12, yt)]);
        }
        if i + 1 == n {
            out.top = Top {
                xl,
                xr,
                y: yt,
                dx,
                dy,
            };
        }
    }
    out
}

/// The flattened streak curve `M x0 y0 C 640 y−30 760 y+20 900 y−4` (viewBox units).
fn streak(z: f64, y: f32) -> Vec<Pos2> {
    let p0 = pos2(560.0 + 30.0 * (z.sin() as f32), y - 6.0);
    let (c1, c2, p3) = (
        pos2(640.0, y - 30.0),
        pos2(760.0, y + 20.0),
        pos2(900.0, y - 4.0),
    );
    (0..=24)
        .map(|k| {
            let u = k as f32 / 24.0;
            let a = (1.0 - u).powi(3);
            let b = 3.0 * (1.0 - u).powi(2) * u;
            let c = 3.0 * (1.0 - u) * u * u;
            let d = u.powi(3);
            pos2(
                a * p0.x + b * c1.x + c * c2.x + d * p3.x,
                a * p0.y + b * c1.y + c * c2.y + d * p3.y,
            )
        })
        .collect()
}

/// Draws the stage into `rect`.
pub fn draw(ui: &mut Ui, rect: Rect, s: &TowerScreen, r: &TowerResult) {
    let panel = layout::graph_panel(ui, rect, None);
    let map = Map::new(rect);
    let p = panel.painter();
    let k = map.scale;

    // ground strip, line, plinth
    p.rect_filled(
        Rect::from_min_max(map.p(0.0, GROUND_Y), map.p(W, H)),
        0.0,
        tt::GROUND,
    );
    p.line_segment(
        [map.p(0.0, GROUND_Y), map.p(W, GROUND_Y)],
        Stroke::new(2.0 * k, tt::GROUND_LINE),
    );
    p.add(EShape::convex_polygon(
        vec![
            map.p(392.0, 580.0),
            map.p(552.0, 580.0),
            map.p(560.0, 600.0),
            map.p(400.0, 600.0),
        ],
        tt::PLINTH,
        Stroke::NONE,
    ));

    // wind streaks (flowing while playing) and arrows
    let fp = FLOOR_PX;
    let offset = s.flow / 1.6 * 48.0;
    let mut arrows = Vec::new();
    for z in tower::arrow_heights(r.height) {
        let y = GROUND_Y - (z / 4.0) as f32 * fp;
        let pts: Vec<Pos2> = streak(z, y).into_iter().map(|q| map.p(q.x, q.y)).collect();
        p.extend(EShape::dashed_line_with_offset(
            &pts,
            Stroke::new(1.5 * k, tt::STREAK),
            &[26.0 * k],
            &[34.0 * k],
            map.len(60.0 - offset),
        ));
        let len = tower::arrow_len(s.input.v10, z) as f32;
        let x2 = 370.0;
        arrows.push((
            [map.p(x2 - len, y), map.p(x2, y)],
            [
                map.p(x2 - 7.0, y - 5.0),
                map.p(x2, y),
                map.p(x2 - 7.0, y + 5.0),
            ],
        ));
    }
    let arrow_stroke = Stroke::new(2.2 * k, tt::WIND);
    for (shaft, head) in arrows {
        world_line(p, &shaft, arrow_stroke);
        world_line(p, &head, arrow_stroke);
    }

    // the tower
    let f = floors(s, r, s.top_sway(r));
    for side in &f.sides {
        p.add(EShape::convex_polygon(
            side.iter().map(|q| map.p(q.x, q.y)).collect(),
            tt::SIDE,
            Stroke::new(0.6 * k, tt::SIDE_EDGE),
        ));
    }
    for &[xl, xr, yb, yt] in &f.fronts {
        p.rect(
            Rect::from_min_max(map.p(xl, yt), map.p(xr, yb)),
            0.0,
            tt::FRONT,
            Stroke::new(0.7 * k, tt::FRONT_EDGE),
            eframe::egui::StrokeKind::Middle,
        );
    }
    for &[xl, xr, yb, yt] in &f.holes {
        p.rect_filled(
            Rect::from_min_max(map.p(xl, yt), map.p(xr, yb)),
            0.0,
            tt::HOLE,
        );
    }
    let sys = if s.input.system == System::Outriggers {
        tt::SYSTEM_OUTRIGGER
    } else {
        tt::SYSTEM
    };
    for seg in &f.lines {
        p.line_segment(
            [map.p(seg[0].x, seg[0].y), map.p(seg[1].x, seg[1].y)],
            Stroke::new(1.3 * k, sys),
        );
    }
    let top = f.top;
    p.add(EShape::convex_polygon(
        vec![
            map.p(top.xl, top.y),
            map.p(top.xl + top.dx, top.y + top.dy),
            map.p(top.xr + top.dx, top.y + top.dy),
            map.p(top.xr, top.y),
        ],
        tt::CAP,
        Stroke::NONE,
    ));

    // plumb line from the base's centre past the top
    let plumb_top = GROUND_Y - s.input.floors as f32 * fp - 16.0;
    p.extend(EShape::dashed_line(
        &[map.p(CENTRE_X, GROUND_Y), map.p(CENTRE_X, plumb_top)],
        Stroke::new(k, tt::PLUMB),
        3.0 * k,
        5.0 * k,
    ));

    // the tuned mass damper, swinging against the sway (its stroke drawn at a quarter of the
    // sway's exaggeration and kept on the roof: the real stroke is several times the sway)
    if s.input.damper {
        let top_px_x = (top.xl + top.xr) / 2.0 + top.dx / 2.0;
        let reach = (top.xr - top.xl) * 0.4;
        let swing = ((s.damper_stroke() * s.ex as f64 * 1.05 * 0.25) as f32).clamp(-reach, reach);
        let c = map.p(top_px_x + swing, top.y + top.dy / 2.0 - 10.0);
        p.circle(
            c,
            6.0 * k,
            tt::DAMPER,
            Stroke::new(1.5 * k, tt::DAMPER_EDGE),
        );
    }

    // overlays (CSS px, not scaled)
    let chip = Chip::new(tower::sway_chip(r));
    let cs = chip.size(&panel);
    let chip_at = map.p(top.xr + top.dx + 70.0, top.y + 30.0);
    // `transform: translate(-50%, -130%)`
    chip.show_at(
        &panel,
        pos2(chip_at.x, chip_at.y - 0.3 * cs.y),
        Align2::CENTER_BOTTOM,
    );
    world::stage_box(
        &panel,
        rect.min + vec2(12.0, 10.0),
        "Wind at the top",
        &tower::v_top_label(r),
        tt::WIND,
    );
    let b = Badge::plain(format!("sway drawn ×{}", s.ex.round()));
    let bs = badge_size(&panel, &b);
    let br = Align2::RIGHT_BOTTOM.anchor_size(rect.max - vec2(12.0, 50.0), bs);
    paint_badge(&panel, br, &b, Default::default());

    // the stage's accessible summary (`role="img" aria-label=…`)
    let label = tower::stage_label(&s.input, r);
    let resp = panel.interact(rect, panel.scope_id().with("tower-stage"), Sense::hover());
    resp.widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
    layout::graph_panel_border(ui, rect);
    let _ = t::TEXT;
}

/// A round-capped polyline (`stroke-linecap="round" stroke-linejoin="round"`).
fn world_line(p: &eframe::egui::Painter, pts: &[Pos2], stroke: Stroke) {
    p.add(PathShape::line(pts.to_vec(), PathStroke::from(stroke)));
    for q in pts {
        p.circle_filled(*q, stroke.width / 2.0, stroke.color);
    }
}
