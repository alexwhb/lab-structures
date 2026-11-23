//! Painting primitives for the stage, to SVG's rules: round / butt caps, dash patterns that
//! restart on every subpath (`stroke-dasharray`, `stroke-dashoffset`), quadratic and cubic
//! Béziers flattened to polylines, convex fills, rings — and [`Xf`], the camera transform
//! from world metres to screen points.

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, epaint::PathShape, pos2, vec2};

/// The stage's drawing size (`viewBox="0 0 1120 600"`).
pub const STAGE_W: f32 = 1120.0;
/// See [`STAGE_W`].
pub const STAGE_H: f32 = 600.0;

/// The stage's height for a width: `aspect-ratio: 1120 / 600` holds for the content box inside
/// the 1 px border (`box-sizing: content-box`), where the SVG is drawn.
pub fn height_for(width: f32) -> f32 {
    (width - 2.0).max(0.0) * STAGE_H / STAGE_W + 2.0
}

/// The camera transform: world metres → stage px (the mockup's `cam()`: `X(x) = 560 + (x −
/// cx)·SC`, `Y(y) = 300 − (y − cy)·SC`) → screen points (the stage drawn into a rect, plus
/// the quake shake offset).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Xf {
    origin: Pos2,
    /// Points per stage px.
    s: f32,
    cx: f64,
    cy: f64,
    /// Stage px per metre (`SC`).
    sc: f64,
}

impl Xf {
    /// The transform for the stage drawn into `rect` looking at (`cx`, `cy`) m with `sc` stage
    /// px per metre, shifted by `shake` CSS px.
    pub fn new(rect: Rect, cx: f64, cy: f64, sc: f64, shake: [f32; 2]) -> Self {
        let s = rect.width() / STAGE_W;
        Self {
            origin: rect.min + vec2(shake[0], shake[1]),
            s,
            cx,
            cy,
            sc,
        }
    }

    /// The transform for the engine's camera.
    pub fn of(rect: Rect, cam: &sp_engine::builder::Camera, shake: [f32; 2]) -> Self {
        Self::new(rect, cam.view.cx, cam.view.cy, cam.sc, shake)
    }

    /// A stage-px point (the engine's view) → screen.
    pub fn pt(&self, p: [f64; 2]) -> Pos2 {
        self.sp(p[0], p[1])
    }

    /// World metres → stage px.
    pub fn stage(&self, at: [f64; 2]) -> (f64, f64) {
        (
            560.0 + (at[0] - self.cx) * self.sc,
            300.0 - (at[1] - self.cy) * self.sc,
        )
    }

    /// World metres → screen.
    pub fn p(&self, at: [f64; 2]) -> Pos2 {
        let (x, y) = self.stage(at);
        self.sp(x, y)
    }

    /// World metres plus a stage-px offset → screen (the 2.5D back copies, `pt(p, ox, oy)`).
    pub fn po(&self, at: [f64; 2], ox: f64, oy: f64) -> Pos2 {
        let (x, y) = self.stage(at);
        self.sp(x + ox, y + oy)
    }

    /// Stage px → screen.
    pub fn sp(&self, x: f64, y: f64) -> Pos2 {
        self.origin + vec2(x as f32 * self.s, y as f32 * self.s)
    }

    /// A stage-px length → points.
    pub fn l(&self, v: f32) -> f32 {
        v * self.s
    }

    /// Points per metre.
    pub fn ppm(&self) -> f32 {
        self.sc as f32 * self.s
    }

    /// Stage px per metre (`SC`).
    pub fn sc(&self) -> f64 {
        self.sc
    }

    /// Screen → stage px.
    pub fn inv(&self, p: Pos2) -> (f64, f64) {
        (
            ((p.x - self.origin.x) / self.s) as f64,
            ((p.y - self.origin.y) / self.s) as f64,
        )
    }

    /// Screen → world metres.
    pub fn world(&self, p: Pos2) -> [f64; 2] {
        let (x, y) = self.inv(p);
        [
            self.cx + (x - 560.0) / self.sc,
            self.cy - (y - 300.0) / self.sc,
        ]
    }
}

/// A segment with round caps (`stroke-linecap="round"`) or butt caps.
pub fn segment(painter: &Painter, a: Pos2, b: Pos2, w: f32, color: Color32, round: bool) {
    if !(a.x.is_finite() && a.y.is_finite() && b.x.is_finite() && b.y.is_finite()) {
        return;
    }
    painter.line_segment([a, b], Stroke::new(w, color));
    if round {
        painter.circle_filled(a, w / 2.0, color);
        painter.circle_filled(b, w / 2.0, color);
    }
}

/// A polyline (one subpath) with round or butt caps and round joins.
pub fn polyline(painter: &Painter, pts: &[Pos2], w: f32, color: Color32, round: bool) {
    if pts.len() < 2 || pts.iter().any(|p| !(p.x.is_finite() && p.y.is_finite())) {
        return;
    }
    painter.add(Shape::line(pts.to_vec(), Stroke::new(w, color)));
    let inner = if round { pts } else { &pts[1..pts.len() - 1] };
    for p in inner {
        painter.circle_filled(*p, w / 2.0, color);
    }
}

/// A dashed polyline: `dash` on, `gap` off, the pattern starting `offset` along (positive
/// `stroke-dashoffset` shifts it back), continuous over the polyline's corners.
#[allow(clippy::too_many_arguments)]
pub fn dashed(
    painter: &Painter,
    pts: &[Pos2],
    w: f32,
    color: Color32,
    dash: f32,
    gap: f32,
    offset: f32,
    round: bool,
) {
    let period = dash + gap;
    if pts.len() < 2 || dash <= 0.0 || period <= 0.0 || !offset.is_finite() {
        return;
    }
    // where we are in the pattern at the start
    let mut phase = offset.rem_euclid(period);
    let mut run: Vec<Pos2> = Vec::new();
    for win in pts.windows(2) {
        let (a, b) = (win[0], win[1]);
        let len = (b - a).length();
        if !len.is_finite() || len <= 0.0 {
            continue;
        }
        let dir = (b - a) / len;
        let mut s = 0.0f32;
        while s < len - 1e-5 {
            let on = phase < dash;
            let left = if on { dash - phase } else { period - phase };
            let step = left.min(len - s);
            if on {
                if run.is_empty() {
                    run.push(a + dir * s);
                }
                run.push(a + dir * (s + step));
            }
            s += step;
            phase += step;
            if on && phase >= dash - 1e-5 {
                // the dash ended: draw it (a dash that runs past a corner joins up)
                if run.len() > 1 {
                    polyline(painter, &run, w, color, round);
                }
                run.clear();
            }
            if phase >= period - 1e-5 {
                phase = 0.0;
            }
        }
    }
    if run.len() > 1 {
        polyline(painter, &run, w, color, round);
    }
}

/// A filled convex polygon.
pub fn fill(painter: &Painter, pts: Vec<Pos2>, color: Color32) {
    if pts.len() < 3 || pts.iter().any(|p| !(p.x.is_finite() && p.y.is_finite())) {
        return;
    }
    painter.add(Shape::Path(PathShape::convex_polygon(
        pts,
        color,
        Stroke::NONE,
    )));
}

/// A filled simple polygon, convex or not (ear clipping into one mesh), e.g. the river's
/// U-shaped soil outline.
pub fn fill_any(painter: &Painter, pts: &[Pos2], color: Color32) {
    let n = pts.len();
    if n < 3 || pts.iter().any(|p| !(p.x.is_finite() && p.y.is_finite())) {
        return;
    }
    // orientation (shoelace): the ear test needs it
    let area: f32 = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let cross = |a: Pos2, b: Pos2, c: Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let mut idx: Vec<usize> = (0..n).collect();
    let mut mesh = eframe::egui::Mesh::default();
    for p in pts {
        mesh.colored_vertex(*p, color);
    }
    let mut guard = 0;
    while idx.len() > 3 && guard < 4 * n * n {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for k in 0..m {
            let (i0, i1, i2) = (idx[(k + m - 1) % m], idx[k], idx[(k + 1) % m]);
            let (a, b, c) = (pts[i0], pts[i1], pts[i2]);
            if cross(a, b, c) * sign <= 0.0 {
                continue; // reflex
            }
            let inside = idx.iter().any(|&j| {
                if j == i0 || j == i1 || j == i2 {
                    return false;
                }
                let p = pts[j];
                cross(a, b, p) * sign > 0.0
                    && cross(b, c, p) * sign > 0.0
                    && cross(c, a, p) * sign > 0.0
            });
            if inside {
                continue;
            }
            mesh.add_triangle(i0 as u32, i1 as u32, i2 as u32);
            idx.remove(k);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if idx.len() == 3 {
        mesh.add_triangle(idx[0] as u32, idx[1] as u32, idx[2] as u32);
    }
    painter.add(Shape::mesh(mesh));
}

/// A filled convex polygon with an outline.
pub fn fill_stroke(painter: &Painter, pts: Vec<Pos2>, color: Color32, stroke: Stroke) {
    if pts.len() < 3 || pts.iter().any(|p| !(p.x.is_finite() && p.y.is_finite())) {
        return;
    }
    painter.add(Shape::Path(PathShape::convex_polygon(pts, color, stroke)));
}

/// A quadratic Bézier `a → (control c) → b` as `n + 1` points.
pub fn quad(a: Pos2, c: Pos2, b: Pos2, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n.max(1) as f32;
            let u = 1.0 - t;
            pos2(
                u * u * a.x + 2.0 * u * t * c.x + t * t * b.x,
                u * u * a.y + 2.0 * u * t * c.y + t * t * b.y,
            )
        })
        .collect()
}

/// A cubic Bézier as `n + 1` points.
pub fn cubic(a: Pos2, c1: Pos2, c2: Pos2, b: Pos2, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n.max(1) as f32;
            let u = 1.0 - t;
            let (k0, k1, k2, k3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            pos2(
                k0 * a.x + k1 * c1.x + k2 * c2.x + k3 * b.x,
                k0 * a.y + k1 * c1.y + k2 * c2.y + k3 * b.y,
            )
        })
        .collect()
}

/// The points of a circle of radius `r` around `c` (closed: first = last).
pub fn circle_pts(c: Pos2, r: f32, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            c + vec2(a.cos() * r, a.sin() * r)
        })
        .collect()
}

/// A dashed ring (the "Sags and folds" / overloaded foundation ring: r 13, dashed 4 3).
pub fn dashed_ring(
    painter: &Painter,
    c: Pos2,
    r: f32,
    w: f32,
    color: Color32,
    dash: f32,
    gap: f32,
) {
    dashed(
        painter,
        &circle_pts(c, r, 64),
        w,
        color,
        dash,
        gap,
        0.0,
        false,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_round_trips() {
        let rect = Rect::from_min_size(pos2(100.0, 50.0), vec2(560.0, 300.0));
        let xf = Xf::new(rect, 28.0, 4.4, 18.0, [0.0, 0.0]);
        assert_eq!(xf.pt([560.0, 300.0]), rect.center());
        // the view centre sits in the middle of the stage
        let c = xf.p([28.0, 4.4]);
        assert!((c - rect.center()).length() < 1e-3);
        let w = xf.world(xf.p([10.0, -3.0]));
        assert!((w[0] - 10.0).abs() < 1e-3 && (w[1] + 3.0).abs() < 1e-3);
        assert!((xf.ppm() - 9.0).abs() < 1e-4);
    }

    #[test]
    fn beziers_end_on_their_ends() {
        let (a, c, b) = (pos2(0.0, 0.0), pos2(5.0, 10.0), pos2(10.0, 0.0));
        let q = quad(a, c, b, 8);
        assert_eq!(q.first(), Some(&a));
        assert_eq!(q.last(), Some(&b));
        assert!((q[4].y - 5.0).abs() < 1e-4);
    }
}
