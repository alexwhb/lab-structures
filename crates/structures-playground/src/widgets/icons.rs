//! Every icon the mockup uses, drawn from its own SVG data (paths, rects, circles) as egui
//! shapes: the transport icons (play, pause, restart), the mode icons (truss, chart), the
//! Bridge's tools rail (select, member, joint, support, weight, erase, undo, clear), Back and
//! the logo mark.
//!
//! No sparkle / magic-wand / four-point-star icons, ever (SPEC LOOK-3).
//!
//! ```ignore
//! Icon::Play.paint(ui.painter(), rect, sp_theme::INK); // `currentColor` = INK
//! ```

use eframe::egui::{
    Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, epaint::PathShape, epaint::PathStroke, pos2,
    vec2,
};
use sp_theme as t;

/// What colour a layer uses.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Ink {
    /// The colour passed to [`Icon::paint`] (CSS `currentColor`).
    Current,
    /// A fixed colour from the mockup (the logo).
    Fixed(Color32),
}

#[derive(Debug, Clone, Copy)]
enum Prim {
    Path(&'static str),
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rx: f32,
    },
    Circle {
        cx: f32,
        cy: f32,
        r: f32,
    },
}

#[derive(Debug, Clone, Copy)]
struct Layer {
    prim: Prim,
    fill: Option<Ink>,
    stroke: Option<(Ink, f32)>,
}

const fn path_fill(d: &'static str, ink: Ink) -> Layer {
    Layer {
        prim: Prim::Path(d),
        fill: Some(ink),
        stroke: None,
    }
}
const fn path_stroke(d: &'static str, ink: Ink, w: f32) -> Layer {
    Layer {
        prim: Prim::Path(d),
        fill: None,
        stroke: Some((ink, w)),
    }
}
const fn rect_fill(x: f32, y: f32, w: f32, h: f32, rx: f32, ink: Ink) -> Layer {
    Layer {
        prim: Prim::Rect { x, y, w, h, rx },
        fill: Some(ink),
        stroke: None,
    }
}
const fn circle_fill(cx: f32, cy: f32, r: f32, ink: Ink) -> Layer {
    Layer {
        prim: Prim::Circle { cx, cy, r },
        fill: Some(ink),
        stroke: None,
    }
}

const ACC: Ink = Ink::Fixed(t::ACCENT);

/// A 22 px tool-rail icon: one stroked path (`stroke-width="1.6"`, Playground `toolDefs`).
const fn tool(d: &'static str) -> Layer {
    path_stroke(d, Ink::Current, 1.6)
}

/// The mockup's icons. Draw with [`Icon::paint`] into a square rect of the CSS size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    /// Test / Drive across / Play gusts / Play years / Pull (12 px).
    Play,
    /// Pause (12 px).
    Pause,
    /// Reset: an open circle with an arrow head (drawn 12 px, viewBox 16).
    Restart,
    /// Playground mode: a truss (14 px).
    Truss,
    /// Advanced mode: axes with result lines (14 px).
    Chart,
    /// Back (Series): a left chevron (14 px).
    Back,
    /// Bridge tools rail: Select (22 px).
    Select,
    /// Bridge tools rail: Member.
    Member,
    /// Bridge tools rail: Joint.
    Joint,
    /// Bridge tools rail: Support.
    Support,
    /// Bridge tools rail: Weight.
    Weight,
    /// Bridge tools rail: Erase.
    Erase,
    /// Bridge tools rail: Undo.
    Undo,
    /// Bridge tools rail: Clear (a bin).
    Clear,
    /// The 26 px logo mark: the truss glyph in a sand-outlined rounded square with a tension
    /// blue joint.
    Logo,
}

impl Icon {
    /// Every icon (for the widget gallery).
    pub const ALL: [Icon; 15] = [
        Icon::Play,
        Icon::Pause,
        Icon::Restart,
        Icon::Truss,
        Icon::Chart,
        Icon::Back,
        Icon::Select,
        Icon::Member,
        Icon::Joint,
        Icon::Support,
        Icon::Weight,
        Icon::Erase,
        Icon::Undo,
        Icon::Clear,
        Icon::Logo,
    ];

    /// A short name (gallery labels, debugging).
    pub fn name(self) -> &'static str {
        match self {
            Icon::Play => "play",
            Icon::Pause => "pause",
            Icon::Restart => "restart",
            Icon::Truss => "truss",
            Icon::Chart => "chart",
            Icon::Back => "back",
            Icon::Select => "select",
            Icon::Member => "member",
            Icon::Joint => "joint",
            Icon::Support => "support",
            Icon::Weight => "weight",
            Icon::Erase => "erase",
            Icon::Undo => "undo",
            Icon::Clear => "clear",
            Icon::Logo => "logo",
        }
    }

    /// The SVG `viewBox` (width, height).
    fn viewbox(self) -> (f32, f32) {
        match self {
            Icon::Play | Icon::Pause => (12.0, 12.0),
            Icon::Truss | Icon::Chart | Icon::Back => (14.0, 14.0),
            Icon::Restart => (16.0, 16.0),
            Icon::Select
            | Icon::Member
            | Icon::Joint
            | Icon::Support
            | Icon::Weight
            | Icon::Erase
            | Icon::Undo
            | Icon::Clear => (22.0, 22.0),
            Icon::Logo => (26.0, 26.0),
        }
    }

    /// The size the mockup draws it at (CSS `width` / `height`).
    pub fn css_size(self) -> eframe::egui::Vec2 {
        match self {
            Icon::Restart => vec2(12.0, 12.0),
            _ => {
                let (w, h) = self.viewbox();
                vec2(w, h)
            }
        }
    }

    fn layers(self) -> &'static [Layer] {
        use Ink::Current as C;
        match self {
            Icon::Play => const { &[path_fill("M3 1.5l7 4.5-7 4.5z", C)] },
            Icon::Pause => {
                const {
                    &[
                        rect_fill(2.0, 1.5, 2.6, 9.0, 0.6, C),
                        rect_fill(7.4, 1.5, 2.6, 9.0, 0.6, C),
                    ]
                }
            }
            Icon::Restart => {
                const {
                    &[
                        path_stroke("M2.5 8a5.5 5.5 0 1 0 1.7-4", C, 1.8),
                        path_stroke("M2.5 2.5V6H6", C, 1.8),
                    ]
                }
            }
            Icon::Truss => const { &[path_stroke("M1 11h12M1 11l3-6h6l3 6M4 5l3 6 3-6", C, 1.3)] },
            Icon::Chart => {
                const {
                    &[
                        path_stroke("M1.5 1.5v11h11", C, 1.3),
                        path_stroke("M3.5 4h4M3.5 6.5h6M3.5 9h3", C, 1.3),
                    ]
                }
            }
            Icon::Back => const { &[path_stroke("M8.5 3 4.5 7l4 4", C, 1.5)] },
            Icon::Select => const { &[tool("M5 3l12 7-5 1.5L9.5 17z")] },
            Icon::Member => {
                const {
                    &[tool(
                        "M4 18L18 4M4 18a1.6 1.6 0 1 0 0.01 0M18 4a1.6 1.6 0 1 0 0.01 0",
                    )]
                }
            }
            Icon::Joint => {
                const { &[tool("M11 7a4 4 0 1 0 0.01 0M11 2v3M11 17v3M2 11h3M17 11h3")] }
            }
            Icon::Support => const { &[tool("M11 5L4 16H18Z M3 19H19")] },
            Icon::Weight => const { &[tool("M5 9h12v9H5ZM5 9l2-3h12l-2 3M17 18l2-3V6M11 2v4")] },
            Icon::Erase => const { &[tool("M6 6l10 10M16 6L6 16")] },
            Icon::Undo => const { &[tool("M8 6L4 10l4 4"), tool("M4 10h9a5 5 0 0 1 0 10h-3")] },
            Icon::Clear => const { &[tool("M4 6h14M9 6V4h4v2M6 6l1 13h8l1-13")] },
            Icon::Logo => {
                const {
                    &[
                        Layer {
                            prim: Prim::Rect {
                                x: 1.0,
                                y: 1.0,
                                w: 24.0,
                                h: 24.0,
                                rx: 6.0,
                            },
                            fill: Some(Ink::Fixed(t::GHOST)),
                            stroke: Some((ACC, 1.5)),
                        },
                        path_stroke("M5 18h16M5 18l4-9h8l4 9M9 9l4 9 4-9", ACC, 1.6),
                        circle_fill(13.0, 18.0, 1.8, Ink::Fixed(t::TENSION)),
                    ]
                }
            }
        }
    }

    /// Paints the icon scaled into `rect` (aspect kept, centred), `color` for `currentColor`.
    pub fn paint(self, painter: &Painter, rect: Rect, color: Color32) {
        let (vw, vh) = self.viewbox();
        let scale = (rect.width() / vw).min(rect.height() / vh);
        let origin = rect.center() - vec2(vw, vh) * scale / 2.0;
        let map = |p: Pos2| origin + p.to_vec2() * scale;
        let ink = |i: Ink| match i {
            Ink::Current => Some(color),
            Ink::Fixed(c) => Some(c),
        };
        for layer in self.layers() {
            let fill_c = layer.fill.and_then(ink);
            let stroke = layer
                .stroke
                .and_then(|(i, w)| ink(i).map(|c| Stroke::new(w * scale, c)));
            match layer.prim {
                Prim::Rect { x, y, w, h, rx } => {
                    let r = Rect::from_min_size(map(pos2(x, y)), vec2(w, h) * scale);
                    let rad = rx * scale;
                    if let Some(c) = fill_c {
                        painter.rect_filled(r, rad, c);
                    }
                    if let Some(s) = stroke {
                        painter.rect_stroke(r, rad, s, eframe::egui::StrokeKind::Middle);
                    }
                }
                Prim::Circle { cx, cy, r } => {
                    let c = map(pos2(cx, cy));
                    if let Some(f) = fill_c {
                        painter.circle_filled(c, r * scale, f);
                    }
                    if let Some(s) = stroke {
                        painter.circle_stroke(c, r * scale, s);
                    }
                }
                Prim::Path(d) => {
                    let subpaths = parse_path(d);
                    if let Some(c) = fill_c {
                        for sp in &subpaths {
                            let pts: Vec<Pos2> = sp.points.iter().map(|p| map(*p)).collect();
                            fill_polygon(painter, &pts, c);
                        }
                    }
                    if let Some(s) = stroke {
                        for sp in &subpaths {
                            let pts: Vec<Pos2> = sp.points.iter().map(|p| map(*p)).collect();
                            stroke_polyline(painter, &pts, sp.closed, s);
                        }
                    }
                }
            }
        }
    }
}

// --------------------------------------------------------------------------------------------
// SVG path data → polylines

/// One subpath of an SVG path, flattened to a polyline (viewBox units).
#[derive(Debug, Clone, PartialEq)]
pub struct Subpath {
    pub points: Vec<Pos2>,
    pub closed: bool,
}

/// Parses SVG path data (`M L H V C S A Z`, absolute and relative, implicit repeats) into
/// flattened subpaths. Unknown commands end the parse (never panics).
pub fn parse_path(d: &str) -> Vec<Subpath> {
    let toks = tokenize(d);
    let mut out: Vec<Subpath> = Vec::new();
    let mut cur: Vec<Pos2> = Vec::new();
    let mut pen = Pos2::ZERO;
    let mut start = Pos2::ZERO;
    let mut last_ctrl: Option<Pos2> = None;
    let mut i = 0;
    let mut cmd = 'M';
    let flush = |cur: &mut Vec<Pos2>, out: &mut Vec<Subpath>, closed: bool| {
        if cur.len() > 1 {
            out.push(Subpath {
                points: std::mem::take(cur),
                closed,
            });
        } else {
            cur.clear();
        }
    };
    while i < toks.len() {
        if let Tok::Cmd(c) = toks[i] {
            cmd = c;
            i += 1;
            if c == 'Z' || c == 'z' {
                flush(&mut cur, &mut out, true);
                pen = start;
                last_ctrl = None;
                continue;
            }
        }
        let rel = cmd.is_ascii_lowercase();
        let base = if rel { pen.to_vec2() } else { vec2(0.0, 0.0) };
        let num = |i: &mut usize| -> Option<f32> {
            match toks.get(*i) {
                Some(Tok::Num(v)) => {
                    *i += 1;
                    Some(*v)
                }
                _ => None,
            }
        };
        match cmd.to_ascii_uppercase() {
            'M' => {
                let (Some(x), Some(y)) = (num(&mut i), num(&mut i)) else {
                    break;
                };
                flush(&mut cur, &mut out, false);
                pen = pos2(x, y) + base;
                start = pen;
                cur.push(pen);
                // further pairs are implicit line-tos
                cmd = if rel { 'l' } else { 'L' };
                last_ctrl = None;
            }
            'L' => {
                let (Some(x), Some(y)) = (num(&mut i), num(&mut i)) else {
                    break;
                };
                pen = pos2(x, y) + base;
                cur.push(pen);
                last_ctrl = None;
            }
            'H' => {
                let Some(x) = num(&mut i) else { break };
                pen.x = if rel { pen.x + x } else { x };
                cur.push(pen);
                last_ctrl = None;
            }
            'V' => {
                let Some(y) = num(&mut i) else { break };
                pen.y = if rel { pen.y + y } else { y };
                cur.push(pen);
                last_ctrl = None;
            }
            'C' => {
                let v: Option<Vec<f32>> = (0..6).map(|_| num(&mut i)).collect();
                let Some(v) = v else { break };
                let c1 = pos2(v[0], v[1]) + base;
                let c2 = pos2(v[2], v[3]) + base;
                let end = pos2(v[4], v[5]) + base;
                cubic(&mut cur, pen, c1, c2, end);
                last_ctrl = Some(c2);
                pen = end;
            }
            'S' => {
                let v: Option<Vec<f32>> = (0..4).map(|_| num(&mut i)).collect();
                let Some(v) = v else { break };
                let c1 = last_ctrl.map_or(pen, |c| pen + (pen - c));
                let c2 = pos2(v[0], v[1]) + base;
                let end = pos2(v[2], v[3]) + base;
                cubic(&mut cur, pen, c1, c2, end);
                last_ctrl = Some(c2);
                pen = end;
            }
            'A' => {
                let v: Option<Vec<f32>> = (0..7).map(|_| num(&mut i)).collect();
                let Some(v) = v else { break };
                let end = pos2(v[5], v[6]) + base;
                arc(
                    &mut cur,
                    pen,
                    v[0],
                    v[1],
                    v[2],
                    v[3] != 0.0,
                    v[4] != 0.0,
                    end,
                );
                pen = end;
                last_ctrl = None;
            }
            _ => break,
        }
        if cur.is_empty() {
            cur.push(pen);
        }
    }
    flush(&mut cur, &mut out, false);
    out
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tok {
    Cmd(char),
    Num(f32),
}

fn tokenize(d: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let b = d.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c.is_ascii_alphabetic() && c != 'e' && c != 'E' {
            toks.push(Tok::Cmd(c));
            i += 1;
        } else if c.is_ascii_digit() || c == '-' || c == '+' || c == '.' {
            let s = i;
            i += 1;
            let mut dot = c == '.';
            while i < b.len() {
                let ch = b[i] as char;
                if ch.is_ascii_digit() {
                    i += 1;
                } else if ch == '.' && !dot {
                    dot = true;
                    i += 1;
                } else if (ch == 'e' || ch == 'E') && i + 1 < b.len() {
                    i += 1;
                    if b[i] == b'-' || b[i] == b'+' {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            if let Ok(v) = d[s..i].parse::<f32>() {
                toks.push(Tok::Num(v));
            }
        } else {
            i += 1;
        }
    }
    toks
}

fn cubic(out: &mut Vec<Pos2>, p0: Pos2, c1: Pos2, c2: Pos2, p3: Pos2) {
    const N: usize = 12;
    for k in 1..=N {
        let t = k as f32 / N as f32;
        let u = 1.0 - t;
        let p = p0.to_vec2() * (u * u * u)
            + c1.to_vec2() * (3.0 * u * u * t)
            + c2.to_vec2() * (3.0 * u * t * t)
            + p3.to_vec2() * (t * t * t);
        out.push(p.to_pos2());
    }
}

/// SVG elliptical arc (endpoint parameterisation, F.6.5 of the SVG spec), flattened.
#[allow(clippy::too_many_arguments)]
fn arc(
    out: &mut Vec<Pos2>,
    p0: Pos2,
    rx: f32,
    ry: f32,
    phi_deg: f32,
    large: bool,
    sweep: bool,
    p1: Pos2,
) {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-6 || ry < 1e-6 || p0 == p1 {
        out.push(p1);
        return;
    }
    let phi = phi_deg.to_radians();
    let (s, c) = phi.sin_cos();
    let dx = (p0.x - p1.x) / 2.0;
    let dy = (p0.y - p1.y) / 2.0;
    let x1 = c * dx + s * dy;
    let y1 = -s * dx + c * dy;
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        let k = lambda.sqrt();
        rx *= k;
        ry *= k;
    }
    let num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut coef = if den > 0.0 {
        (num / den).max(0.0).sqrt()
    } else {
        0.0
    };
    if large == sweep {
        coef = -coef;
    }
    let cx1 = coef * rx * y1 / ry;
    let cy1 = -coef * ry * x1 / rx;
    let cx = c * cx1 - s * cy1 + (p0.x + p1.x) / 2.0;
    let cy = s * cx1 + c * cy1 + (p0.y + p1.y) / 2.0;
    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| {
        let dot = ux * vx + uy * vy;
        let len = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
        let a = (dot / len).clamp(-1.0, 1.0).acos();
        if ux * vy - uy * vx < 0.0 { -a } else { a }
    };
    let ux = (x1 - cx1) / rx;
    let uy = (y1 - cy1) / ry;
    let vx = (-x1 - cx1) / rx;
    let vy = (-y1 - cy1) / ry;
    let theta1 = angle(1.0, 0.0, ux, uy);
    let mut dtheta = angle(ux, uy, vx, vy);
    let tau = std::f32::consts::TAU;
    if !sweep && dtheta > 0.0 {
        dtheta -= tau;
    } else if sweep && dtheta < 0.0 {
        dtheta += tau;
    }
    let n = ((dtheta.abs() / tau) * 32.0).ceil().max(2.0) as usize;
    for k in 1..=n {
        let th = theta1 + dtheta * k as f32 / n as f32;
        let (st, ct) = th.sin_cos();
        let x = c * rx * ct - s * ry * st + cx;
        let y = s * rx * ct + c * ry * st + cy;
        out.push(pos2(x, y));
    }
}

// --------------------------------------------------------------------------------------------
// Painting helpers

/// Fills a simple polygon (convex or not): convex ones use egui's anti-aliased fill; concave
/// ones are ear-clipped into a mesh with a hairline outline for anti-aliasing.
pub fn fill_polygon(painter: &Painter, pts: &[Pos2], color: Color32) {
    let mut pts: Vec<Pos2> = pts.to_vec();
    if pts.len() > 2 && pts.first() == pts.last() {
        pts.pop();
    }
    if pts.len() < 3 {
        return;
    }
    if is_convex(&pts) {
        painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
        return;
    }
    let mut mesh = Mesh::default();
    for p in &pts {
        mesh.colored_vertex(*p, color);
    }
    for [a, b, c] in ear_clip(&pts) {
        mesh.add_triangle(a, b, c);
    }
    painter.add(Shape::mesh(mesh));
    painter.add(PathShape::closed_line(pts, PathStroke::new(0.6, color)));
}

/// Strokes a polyline with round caps (open paths) like the mockup's
/// `stroke-linecap="round"`.
pub fn stroke_polyline(painter: &Painter, pts: &[Pos2], closed: bool, stroke: Stroke) {
    if pts.len() < 2 {
        return;
    }
    let shape = if closed {
        PathShape::closed_line(pts.to_vec(), PathStroke::from(stroke))
    } else {
        PathShape::line(pts.to_vec(), PathStroke::from(stroke))
    };
    painter.add(shape);
    // round joins / caps: a dot at every vertex hides miter gaps at sharp corners
    for p in pts {
        painter.circle_filled(*p, stroke.width / 2.0, stroke.color);
    }
}

fn cross(o: Pos2, a: Pos2, b: Pos2) -> f32 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

fn is_convex(pts: &[Pos2]) -> bool {
    let n = pts.len();
    let mut sign = 0.0f32;
    for i in 0..n {
        let c = cross(pts[i], pts[(i + 1) % n], pts[(i + 2) % n]);
        if c.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = c.signum();
        } else if c.signum() != sign {
            return false;
        }
    }
    true
}

/// Ear clipping for a simple polygon; returns vertex-index triangles.
fn ear_clip(pts: &[Pos2]) -> Vec<[u32; 3]> {
    let n = pts.len();
    let area: f32 = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let ccw = area > 0.0;
    let mut idx: Vec<usize> = (0..n).collect();
    let mut tris = Vec::new();
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for k in 0..m {
            let (ia, ib, ic) = (idx[(k + m - 1) % m], idx[k], idx[(k + 1) % m]);
            let (a, b, c) = (pts[ia], pts[ib], pts[ic]);
            let cr = cross(a, b, c);
            if (cr > 0.0) != ccw || cr.abs() < 1e-9 {
                continue;
            }
            let inside = idx
                .iter()
                .any(|&j| j != ia && j != ib && j != ic && point_in_tri(pts[j], a, b, c));
            if inside {
                continue;
            }
            tris.push([ia as u32, ib as u32, ic as u32]);
            idx.remove(k);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if idx.len() == 3 {
        tris.push([idx[0] as u32, idx[1] as u32, idx[2] as u32]);
    }
    tris
}

fn point_in_tri(p: Pos2, a: Pos2, b: Pos2, c: Pos2) -> bool {
    let d1 = cross(p, a, b);
    let d2 = cross(p, b, c);
    let d3 = cross(p, c, a);
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_relative_and_implicit_commands() {
        let p = parse_path("M3 1.5l7 4.5-7 4.5z");
        assert_eq!(p.len(), 1);
        assert!(p[0].closed);
        assert_eq!(
            p[0].points,
            vec![pos2(3.0, 1.5), pos2(10.0, 6.0), pos2(3.0, 10.5)]
        );
        // implicit line-tos after M, two subpaths with H
        let p = parse_path("M7 1.5 12.5 4.5 7 7.5 1.5 4.5z");
        assert_eq!(p[0].points.len(), 4);
        let p = parse_path("M2 3.5h10M2 7h6");
        assert_eq!(p.len(), 2);
        assert_eq!(p[1].points, vec![pos2(2.0, 7.0), pos2(8.0, 7.0)]);
    }

    #[test]
    fn arcs_and_curves_end_where_they_should() {
        // the heart: two arcs back to (7, 3.3) and then (11.9, 7.4)
        let p = parse_path("M7 12.2 2.1 7.4A2.9 2.9 0 0 1 7 3.3a2.9 2.9 0 0 1 4.9 4.1z");
        let pts = &p[0].points;
        let near = |a: Pos2, b: Pos2| (a - b).length() < 1e-3;
        assert!(pts.iter().any(|q| near(*q, pos2(7.0, 3.3))));
        assert!(near(*pts.last().unwrap(), pos2(11.9, 7.4)));
        // the restart arc sweeps most of a circle around (8, 8)
        let p = parse_path("M2.5 8a5.5 5.5 0 1 0 1.7-4");
        let end = *p[0].points.last().unwrap();
        assert!(near(end, pos2(4.2, 4.0)), "{end:?}");
        for q in &p[0].points {
            assert!(((*q - pos2(8.0, 8.0)).length() - 5.5).abs() < 0.6, "{q:?}");
        }
        // smooth cubic reflects the previous control point
        let p = parse_path("M3 2c0 4 10 4 10 8s-5 4-5 4");
        assert!(near(*p[0].points.last().unwrap(), pos2(8.0, 14.0)));
    }

    #[test]
    fn garbage_never_panics() {
        for d in [
            "",
            "Z",
            "M",
            "M1",
            "Lx",
            "M1 2 L",
            "Q1 2 3 4",
            "M1e 2",
            "--..",
            "a1 1 0 0 0",
        ] {
            let _ = parse_path(d);
        }
    }

    #[test]
    fn ear_clipping_covers_a_concave_polygon() {
        // an L shape (area 3)
        let pts = [
            pos2(0.0, 0.0),
            pos2(2.0, 0.0),
            pos2(2.0, 1.0),
            pos2(1.0, 1.0),
            pos2(1.0, 2.0),
            pos2(0.0, 2.0),
        ];
        assert!(!is_convex(&pts));
        let tris = ear_clip(&pts);
        assert_eq!(tris.len(), 4);
        let area: f32 = tris
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| pts[i as usize]);
                cross(a, b, c).abs() / 2.0
            })
            .sum();
        assert!((area - 3.0).abs() < 1e-4, "{area}");
    }
}
