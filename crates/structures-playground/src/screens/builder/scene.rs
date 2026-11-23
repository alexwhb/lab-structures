//! The stage's SVG layer (`<svg viewBox="0 0 1120 600">` in `Builder.dc.html`), painted from
//! the engine's [`View`] (every shape already in stage px) in the mockup's order: grid dots,
//! water, flow lines, soil, grains, riverbed, the soil's top faces and edge, foundations, wind
//! streaks and arrows, the line pieces' 2.5D back copies, the ghost piece, Floor façades (face,
//! side, windows), deck tops and fronts, snow, the lane line, the line pieces by type, the force
//! colours by band, crates, vehicles, foundation markers, the target line, splashes, sparks,
//! warning rings, the hover dot and falling snow.
//!
//! Stroke widths are stage px (the SVG's user units): they scale with the stage's size, not
//! with the zoom. This module maps piece types, soils and bands to `sp_theme::builder` colours
//! and owns nothing but pixels.

use super::paint::{self, Xf, cubic, dashed, fill, fill_any, fill_stroke, quad, segment};
use eframe::egui::{Color32, CornerRadius, Painter, Pos2, Rect, Stroke, vec2};
use sp_engine::builder::view::{Band, MemberView, Pt, Quad, View};
use sp_engine::builder::{PieceType, Soil, VehicleKind};
use sp_theme::{self as t, builder as b};

/// What moves on the stage between engine frames (screen-side animation phases).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Anim {
    /// Water and wind dash offset, stage px (grows while running: `stroke-dashoffset`).
    pub flow: f32,
    /// Snowfall cycle 0…1 (`@keyframes snowfall`, 2.6 s).
    pub fall: f32,
    /// Quake shake cycle 0…1 (`@keyframes shake`, 0.2 s).
    pub shake: f32,
}

/// A band's stroke colour (`t1 … c3`, `x`).
pub fn band_color(band: Band) -> Color32 {
    let o = b::BAND_OPACITY;
    match band {
        Band::T1 => t::TENSION.gamma_multiply(o[0]),
        Band::T2 => t::TENSION.gamma_multiply(o[1]),
        Band::T3 => b::TENSION_HOT,
        Band::C1 => t::COMPRESSION.gamma_multiply(o[0]),
        Band::C2 => t::COMPRESSION.gamma_multiply(o[1]),
        Band::C3 => b::COMPRESSION_HOT,
        Band::Over => t::OVER,
    }
}

/// The SVG paint order of the band paths.
const BAND_ORDER: [Band; 7] = [
    Band::T1,
    Band::T2,
    Band::T3,
    Band::C1,
    Band::C2,
    Band::C3,
    Band::Over,
];

/// The look of a piece type on the stage.
pub fn look(kind: PieceType) -> b::PieceLook {
    b::PIECES[kind as usize]
}

/// A soil's colours.
pub fn soil_look(soil: Soil) -> b::SoilLook {
    b::SOILS[soil as usize]
}

/// A capsule (segment with round caps) as one convex shape, so a translucent stroke doesn't
/// double up at its caps.
pub fn capsule(painter: &Painter, a: Pos2, c: Pos2, w: f32, color: Color32) {
    let d = c - a;
    let len = d.length();
    if !len.is_finite() || !(a.x.is_finite() && a.y.is_finite()) {
        return;
    }
    let r = w / 2.0;
    if len < 1e-3 {
        painter.circle_filled(a, r, color);
        return;
    }
    let dir = d / len;
    let n = vec2(-dir.y, dir.x);
    let mut pts = Vec::with_capacity(26);
    for i in 0..=12 {
        let ang = std::f32::consts::PI * i as f32 / 12.0;
        let (s, co) = ang.sin_cos();
        pts.push(c + n * (r * co) + dir * (r * s));
    }
    for i in 0..=12 {
        let ang = std::f32::consts::PI * i as f32 / 12.0;
        let (s, co) = ang.sin_cos();
        pts.push(a - n * (r * co) - dir * (r * s));
    }
    fill(painter, pts, color);
}

/// A piece's line on screen: straight, or the slack sag curve (`M a Q c b`).
fn line_pts(xf: &Xf, m: &MemberView) -> Vec<Pos2> {
    match m.sag {
        Some(c) => quad(xf.pt(m.a), xf.pt(c), xf.pt(m.b), 16),
        None => vec![xf.pt(m.a), xf.pt(m.b)],
    }
}

/// Strokes a piece's line (`round` caps or butt).
fn stroke_line(painter: &Painter, pts: &[Pos2], w: f32, color: Color32, round: bool) {
    if pts.len() == 2 {
        if round {
            capsule(painter, pts[0], pts[1], w, color);
        } else {
            segment(painter, pts[0], pts[1], w, color, false);
        }
    } else {
        paint::polyline(painter, pts, w, color, round);
    }
}

fn quad_pts(xf: &Xf, q: &Quad) -> Vec<Pos2> {
    q.iter().map(|p| xf.pt(*p)).collect()
}

fn rect_pts(xf: &Xf, r: &[f64; 4]) -> Vec<Pos2> {
    let [x, y, w, h] = *r;
    vec![
        xf.sp(x, y),
        xf.sp(x + w, y),
        xf.sp(x + w, y + h),
        xf.sp(x, y + h),
    ]
}

fn line2(painter: &Painter, xf: &Xf, l: &[Pt; 2], w: f32, color: Color32, round: bool) {
    segment(painter, xf.pt(l[0]), xf.pt(l[1]), xf.l(w), color, round);
}

/// The snap-grid dots (`<pattern id="sgrid">`: r 1.3 at `origin + k·step`).
fn grid_dots(painter: &Painter, rect: Rect, xf: &Xf, origin: Pt, step: f64) {
    if !(step.is_finite() && step > 0.5 && origin[0].is_finite() && origin[1].is_finite()) {
        return;
    }
    let r = xf.l(1.3);
    // sub-pixel dots fade instead of feathering into specks (the browser's look)
    let dot = if r < 0.7 {
        b::GRID_DOT.gamma_multiply((r / 0.7).powi(2))
    } else {
        b::GRID_DOT
    };
    let x0 = origin[0] - (origin[0] / step).floor() * step;
    let y0 = origin[1] - (origin[1] / step).floor() * step;
    let mut x = x0;
    let mut n = 0;
    while x <= 1121.0 && n < 6000 {
        let mut y = y0;
        while y <= 601.0 && n < 6000 {
            let p = xf.sp(x, y);
            if rect.expand(2.0).contains(p) {
                painter.circle_filled(p, r.max(0.5), dot);
            }
            y += step;
            n += 1;
        }
        x += step;
    }
}

/// Paints the scene for `v` into the stage `rect` (the SVG layer, back to front).
pub fn paint(painter: &Painter, rect: Rect, xf: &Xf, v: &View, anim: Anim) {
    grid_dots(painter, rect, xf, v.grid.origin, v.grid.step);
    let flow = if v.flowing { anim.flow } else { 0.0 };
    let tr = &v.terrain;
    // water and its flow lines (dashed 18 30, flowing while running)
    if let Some(w) = &tr.water_poly {
        let pts: Vec<Pos2> = w.iter().map(|p| xf.pt(*p)).collect();
        fill_any(painter, &pts, b::WATER);
    }
    for l in &tr.flow {
        dashed(
            painter,
            &[xf.pt(l[0]), xf.pt(l[1])],
            xf.l(2.0),
            b::WATER_FLOW,
            xf.l(18.0),
            xf.l(30.0),
            -xf.l(flow),
            false,
        );
    }
    // soil, grains, riverbed, top faces, edge
    let sl = soil_look(tr.soil);
    let soil: Vec<Pos2> = tr.soil_poly.iter().map(|p| xf.pt(*p)).collect();
    fill_any(painter, &soil, sl.body);
    let g = xf.l(2.0);
    for p in &tr.grains {
        painter.rect_filled(
            Rect::from_min_size(xf.pt(*p), vec2(g, g)),
            CornerRadius::ZERO,
            b::GRAIN,
        );
    }
    if let Some(bed) = &tr.bed_poly {
        let pts: Vec<Pos2> = bed.iter().map(|p| xf.pt(*p)).collect();
        fill_any(painter, &pts, soil_look(tr.bed).body);
    }
    for q in &tr.tops {
        fill(painter, quad_pts(xf, q), sl.top);
    }
    for e in &tr.edges {
        line2(painter, xf, e, 3.0, b::GROUND_EDGE, false);
    }
    // foundations
    let edge = Stroke::new(xf.l(1.0), b::FOUNDATION_EDGE);
    for f in &v.foundations {
        fill_stroke(painter, rect_pts(xf, &f.pad), b::FOUNDATION, edge);
        if let Some(p) = &f.pile {
            fill_stroke(painter, rect_pts(xf, p), b::FOUNDATION, edge);
        }
    }
    // wind: streaks (dashed 20 28, flowing while running), then arrows
    for w in &v.wind {
        let s = &w.streak;
        let pts = cubic(xf.pt(s[0]), xf.pt(s[1]), xf.pt(s[2]), xf.pt(s[3]), 32);
        dashed(
            painter,
            &pts,
            xf.l(1.5),
            b::WIND_STREAK,
            xf.l(20.0),
            xf.l(28.0),
            -xf.l(flow),
            false,
        );
    }
    for w in &v.wind {
        let lw = xf.l(2.0);
        segment(painter, xf.pt(w.from), xf.pt(w.to), lw, b::WIND_ARROW, true);
        let head: Vec<Pos2> = w.head.iter().map(|p| xf.pt(*p)).collect();
        paint::polyline(painter, &head, lw, b::WIND_ARROW, true);
    }
    // back copies, the ghost piece
    for m in &v.members {
        if let Some(bk) = &m.back {
            capsule(painter, xf.pt(bk[0]), xf.pt(bk[1]), xf.l(3.0), b::BACK_FACE);
        }
    }
    if let Some(gh) = &v.ghost {
        dashed(
            painter,
            &[xf.pt(gh[0]), xf.pt(gh[1])],
            xf.l(2.0),
            b::GHOST_LINE,
            xf.l(6.0),
            xf.l(6.0),
            0.0,
            false,
        );
    }
    // decks: façades, their sides, window lines; slab tops, fronts; snow; lane lines
    let decks: Vec<_> = v.members.iter().filter_map(|m| m.deck.as_ref()).collect();
    for d in &decks {
        if let Some(f) = &d.facade {
            fill_stroke(
                painter,
                quad_pts(xf, f),
                b::FACADE,
                Stroke::new(xf.l(0.8), b::FACADE_EDGE),
            );
        }
    }
    for d in &decks {
        if let Some(s) = &d.side {
            fill(painter, quad_pts(xf, s), b::FACADE_SIDE);
        }
    }
    for d in &decks {
        if let Some(w) = &d.window {
            line2(painter, xf, w, 1.2, b::WINDOW_LINE, false);
        }
    }
    for d in &decks {
        fill(painter, quad_pts(xf, &d.top), b::DECK_TOP);
    }
    for d in &decks {
        fill_stroke(
            painter,
            quad_pts(xf, &d.front),
            b::DECK_FRONT,
            Stroke::new(xf.l(0.8), b::DECK_EDGE),
        );
    }
    for d in &decks {
        if let Some(s) = &d.snow {
            fill(painter, quad_pts(xf, s), b::SNOW);
        }
    }
    for d in &decks {
        if let Some(l) = &d.line {
            dashed(
                painter,
                &[xf.pt(l[0]), xf.pt(l[1])],
                xf.l(1.0),
                b::LANE_LINE,
                xf.l(6.0),
                xf.l(8.0),
                0.0,
                false,
            );
        }
    }
    // line pieces by type (the mockup's path order), then the force colours by band
    for kind in [
        PieceType::Concrete,
        PieceType::Wood,
        PieceType::Steel,
        PieceType::Rope,
        PieceType::Cable,
    ] {
        let l = look(kind);
        for m in v
            .members
            .iter()
            .filter(|m| m.kind == kind && m.deck.is_none())
        {
            stroke_line(painter, &line_pts(xf, m), xf.l(l.width), l.swatch, l.round);
        }
    }
    for band in BAND_ORDER {
        let c = band_color(band);
        for m in v.members.iter().filter(|m| m.band == Some(band)) {
            let pts = line_pts(xf, m);
            if band == Band::Over {
                dashed(painter, &pts, xf.l(3.5), c, xf.l(9.0), xf.l(6.0), 0.0, true);
            } else {
                stroke_line(painter, &pts, xf.l(2.5), c, true);
            }
        }
    }
    // crates: wires, sides, tops, fronts
    for c in &v.crates {
        line2(painter, xf, &c.wire, 1.2, b::CRATE_WIRE, false);
    }
    for c in &v.crates {
        fill(painter, quad_pts(xf, &c.side), b::CRATE_SIDE);
    }
    for c in &v.crates {
        fill(painter, quad_pts(xf, &c.top), b::CRATE_TOP);
    }
    for c in &v.crates {
        fill_stroke(
            painter,
            rect_pts(xf, &c.body),
            b::CRATE,
            Stroke::new(xf.l(1.0), b::CRATE_EDGE),
        );
    }
    // vehicles: bodies by kind, cabs, windows, wheels
    let vedge = Stroke::new(xf.l(1.0), b::VEHICLE_EDGE);
    for (kind, color) in [
        (VehicleKind::Car, b::CAR),
        (VehicleKind::Bus, b::BUS),
        (VehicleKind::Truck, b::TRUCK_BOX),
    ] {
        for veh in v.vehicles.iter().filter(|veh| veh.kind == kind) {
            for q in &veh.body {
                fill_stroke(painter, quad_pts(xf, q), color, vedge);
            }
        }
    }
    for veh in &v.vehicles {
        if let Some(q) = &veh.cab {
            fill_stroke(painter, quad_pts(xf, q), b::TRUCK_CAB, vedge);
        }
    }
    for veh in &v.vehicles {
        for q in &veh.windows {
            fill(painter, quad_pts(xf, q), b::VEHICLE_WINDOW);
        }
    }
    for veh in &v.vehicles {
        for (c, r) in &veh.wheels {
            painter.circle(
                xf.pt(*c),
                xf.l(*r as f32),
                b::WHEEL,
                Stroke::new(xf.l(1.2), b::WHEEL_EDGE),
            );
        }
    }
    // foundation markers, the target line
    for f in &v.foundations {
        fill(
            painter,
            f.tri.iter().map(|p| xf.pt(*p)).collect(),
            b::SUPPORT,
        );
    }
    if let Some(y) = v.target_y {
        dashed(
            painter,
            &[xf.sp(0.0, y), xf.sp(1120.0, y)],
            xf.l(1.5),
            b::TARGET_LINE,
            xf.l(10.0),
            xf.l(8.0),
            0.0,
            false,
        );
    }
    // splashes, sparks, rings, the hover dot
    for s in &v.splashes {
        let ([x, y], r) = (s.at, s.r);
        let w = xf.l(2.0);
        let a = quad(
            xf.sp(x - r, y),
            xf.sp(x - r / 2.0, y - r * 0.9),
            xf.sp(x, y),
            12,
        );
        let c = quad(
            xf.sp(x, y),
            xf.sp(x + r / 2.0, y - r * 0.9),
            xf.sp(x + r, y),
            12,
        );
        paint::polyline(painter, &a, w, b::SPLASH, true);
        paint::polyline(painter, &c, w, b::SPLASH, true);
    }
    for k in &v.sparks {
        let [x, y] = *k;
        let w = xf.l(2.5);
        segment(
            painter,
            xf.sp(x - 6.0, y - 6.0),
            xf.sp(x + 6.0, y + 6.0),
            w,
            t::DANGER,
            true,
        );
        segment(
            painter,
            xf.sp(x + 6.0, y - 6.0),
            xf.sp(x - 6.0, y + 6.0),
            w,
            t::DANGER,
            true,
        );
    }
    for r in &v.rings {
        paint::dashed_ring(
            painter,
            xf.pt(*r),
            xf.l(13.0),
            xf.l(2.0),
            t::DANGER,
            xf.l(4.0),
            xf.l(3.0),
        );
    }
    if let Some(h) = v.hover {
        painter.circle_filled(xf.pt(h), xf.l(5.0), b::HOVER_DOT);
    }
    // falling snow, drifting (−14, 60) px per 2.6 s cycle
    let (dx, dy) = (-14.0 * anim.fall as f64, 60.0 * anim.fall as f64);
    let fr = xf.l(1.6);
    for f in &v.flakes {
        let p = xf.sp(f[0] + dx, f[1] + dy);
        if rect.contains(p) {
            painter.circle_filled(p, fr, b::FLAKE);
        }
    }
}

/// Where the stage's quake shake puts it `phase` (0…1) through its 0.2 s cycle (`@keyframes
/// shake`: 0 → (−3, 1) → (3, −1) → (−2, 0) → 0 CSS px).
pub fn shake_offset(phase: f32) -> [f32; 2] {
    let keys = [
        (0.0, [0.0, 0.0]),
        (0.25, [-3.0, 1.0]),
        (0.5, [3.0, -1.0]),
        (0.75, [-2.0, 0.0]),
        (1.0, [0.0, 0.0]),
    ];
    let p = phase.rem_euclid(1.0);
    for w in keys.windows(2) {
        let ((t0, a), (t1, c)) = (w[0], w[1]);
        if p <= t1 {
            let k = (p - t0) / (t1 - t0);
            return [a[0] + (c[0] - a[0]) * k, a[1] + (c[1] - a[1]) * k];
        }
    }
    [0.0, 0.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shake_keyframes() {
        assert_eq!(shake_offset(0.0), [0.0, 0.0]);
        assert_eq!(shake_offset(0.25), [-3.0, 1.0]);
        assert_eq!(shake_offset(0.5), [3.0, -1.0]);
        assert_eq!(shake_offset(1.25), [-3.0, 1.0]);
    }

    #[test]
    fn colours_follow_the_mockup() {
        assert_eq!(band_color(Band::Over), t::OVER);
        assert_eq!(band_color(Band::T3), b::TENSION_HOT);
        assert_eq!(look(PieceType::Steel), b::STEEL);
        assert_eq!(soil_look(Soil::Dense), b::SOILS[1]);
    }
}
