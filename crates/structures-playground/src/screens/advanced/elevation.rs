//! "Elevation · .static result" (ADV-4): the truss drawn from the netlist in the mockup's
//! `viewBox="0 0 1000 270"` (scaled to the panel's width, `aspect-ratio: 1000 / 270`) on the
//! `.graph` ground: members coloured by force (blue tension, orange compression, grey near
//! zero, dashed red over the limit), node circles, supports and ground line, the truck arrow,
//! and the HTML labels on top (node ids mono 11, member ids mono 10 on a dark chip).
//!
//! The mockup draws its truss at `x = 60 + 18.5·x_m`, `y = 200 − 18.5·y_m`; any netlist is
//! fitted the same way (888 × 150 units for the truss, the lowest point on y = 200), which
//! gives exactly the mockup's numbers for the 48 m × 6 m truss.

use super::controls::{panel_header, pill, pill_size};
use crate::widgets::text::{galley, text_top};
use eframe::egui::{
    Color32, CornerRadius, Pos2, Rect, Sense, Shape, Stroke, Ui, WidgetInfo, accesskit::Role,
    epaint::PathShape, pos2, vec2,
};
use lab_theme::font;
use sp_engine::influence::{Level, Report, Tone};
use sp_engine::netlist::Netlist;
use sp_theme::{self as t, advanced as a};

/// The colour of a check / status level.
pub fn level_color(l: Level) -> Color32 {
    match l {
        Level::Good => a::CHECK_GOOD,
        Level::Accent => a::CHECK_ACCENT,
        Level::Bad => a::CHECK_BAD,
        Level::Neutral => a::CHECK_NEUTRAL,
    }
}

/// The view transform: netlist metres → viewBox units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// viewBox x of x = 0 m.
    pub x0: f32,
    /// viewBox y of y = 0 m.
    pub y0: f32,
    /// Units per metre.
    pub sc: f32,
}

impl View {
    /// Fits the nodes into 888 × 150 units from (60, 200) (the mockup's frame).
    pub fn fit(net: &Netlist) -> View {
        let (mut minx, mut maxx, mut miny, mut maxy) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for n in &net.nodes {
            minx = minx.min(n.x);
            maxx = maxx.max(n.x);
            miny = miny.min(n.y);
            maxy = maxy.max(n.y);
        }
        if net.nodes.is_empty() {
            return View {
                x0: 60.0,
                y0: 200.0,
                sc: 18.5,
            };
        }
        let w = (maxx - minx).max(1e-9);
        let h = (maxy - miny).max(1e-9);
        let sc = (888.0 / w).min(150.0 / h).clamp(1e-3, 1e6);
        let x0 = 60.0 + (888.0 - w * sc) / 2.0 - minx * sc;
        let y0 = 200.0 + miny * sc;
        View {
            x0: x0 as f32,
            y0: y0 as f32,
            sc: sc as f32,
        }
    }

    /// viewBox point of (x, y) metres.
    pub fn pt(&self, x: f64, y: f64) -> Pos2 {
        pos2(self.x0 + x as f32 * self.sc, self.y0 - y as f32 * self.sc)
    }
}

fn tone_colors(tone: Tone) -> (Color32, Color32) {
    match tone {
        Tone::Zero => (a::MEMBER_ZERO, a::LABEL_ZERO),
        Tone::Tension => (a::MEMBER_TENSION, a::LABEL_TENSION),
        Tone::Compression => (a::MEMBER_COMPRESSION, a::LABEL_COMPRESSION),
        Tone::Failed => (a::MEMBER_OVER, a::LABEL_OVER),
    }
}

/// A dashed line `dash` on, `gap` off.
fn dashed(a: Pos2, b: Pos2, dash: f32, gap: f32, stroke: Stroke) -> Vec<Shape> {
    let len = a.distance(b);
    if len <= 0.0 {
        return Vec::new();
    }
    let dir = (b - a) / len;
    let mut out = Vec::new();
    let mut s = 0.0;
    while s < len {
        let e = (s + dash).min(len);
        out.push(Shape::line_segment([a + dir * s, a + dir * e], stroke));
        s += dash + gap;
    }
    out
}

/// The panel's content: header with the status pill, then the stage.
pub fn elevation_panel(ui: &mut Ui, net: &Netlist, report: &Report, style: Option<&str>) {
    let status = report.status.label();
    let color = level_color(report.status.level());
    let psize = pill_size(ui, &status);
    panel_header(ui, "Elevation · .static result", psize, |ui, r| {
        pill(ui, r, &status, color);
    });
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, width * 270.0 / 1000.0), Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);
    lab_ui::paint::paint_grid(&painter, rect, rect.min, &t::GRAPH);
    let k = width / 1000.0;
    let view = View::fit(net);
    let to = |p: Pos2| pos2(rect.left() + p.x * k, rect.top() + p.y * k);
    let px = |n: usize| {
        let node = &net.nodes[n];
        to(view.pt(node.x, node.y))
    };

    // members: zero, tension, compression, then over (the mockup's path order)
    let order = [Tone::Zero, Tone::Tension, Tone::Compression, Tone::Failed];
    for tone in order {
        for (i, m) in report.truss.members.iter().enumerate() {
            if report.tones.get(i).copied() != Some(tone)
                || m.a >= net.nodes.len()
                || m.b >= net.nodes.len()
            {
                continue;
            }
            let (c, _) = tone_colors(tone);
            let (pa, pb) = (px(m.a), px(m.b));
            if tone == Tone::Failed {
                painter.extend(dashed(pa, pb, 8.0 * k, 5.0 * k, Stroke::new(3.0 * k, c)));
            } else {
                painter.line_segment([pa, pb], Stroke::new(2.5 * k, c));
                painter.circle_filled(pa, 1.25 * k, c);
                painter.circle_filled(pb, 1.25 * k, c);
            }
        }
    }
    // nodes
    for n in 0..net.nodes.len() {
        let p = px(n);
        painter.circle(p, 4.0 * k, a::NODE_FILL, Stroke::new(1.5 * k, a::NODE_RING));
    }
    // supports and the ground line
    let mut ground: Option<(f32, f32, f32)> = None;
    for &s in &report.supports {
        let node = &net.nodes[s];
        let p = view.pt(node.x, node.y);
        let h = if node.fix.x { 12.0 } else { 10.0 };
        let tri = vec![
            to(pos2(p.x, p.y + 3.0)),
            to(pos2(p.x - 8.0, p.y + 3.0 + h)),
            to(pos2(p.x + 8.0, p.y + 3.0 + h)),
        ];
        painter.add(PathShape::convex_polygon(
            tri,
            a::SUPPORT,
            Stroke::new(1.5 * k, a::SUPPORT),
        ));
        let gy = p.y + 20.0;
        ground = Some(match ground {
            None => (p.x, p.x, gy),
            Some((l, r, y)) => (l.min(p.x), r.max(p.x), y.max(gy)),
        });
    }
    if let Some((l, r, y)) = ground
        && r > l
    {
        painter.line_segment(
            [to(pos2(l, y)), to(pos2(r, y))],
            Stroke::new(1.5 * k, a::SUPPORT),
        );
    }
    // the truck arrow
    if let Some(tr) = net.truck.as_ref().filter(|t| t.weight > 0.0) {
        let lane_y = net
            .lane_nodes()
            .first()
            .map(|&n| net.nodes[n].y)
            .unwrap_or(0.0);
        let p = view.pt(report.x, lane_y);
        let _ = tr;
        let stroke = Stroke::new(2.0 * k, a::TRUCK);
        painter.line_segment(
            [to(pos2(p.x, p.y - 50.0)), to(pos2(p.x, p.y - 6.0))],
            stroke,
        );
        painter.add(Shape::line(
            vec![
                to(pos2(p.x - 6.0, p.y - 14.0)),
                to(pos2(p.x, p.y - 6.0)),
                to(pos2(p.x + 6.0, p.y - 14.0)),
            ],
            stroke,
        ));
    }
    // node labels (HTML spans: not scaled)
    let miny = net.nodes.iter().map(|n| n.y).fold(f64::MAX, f64::min);
    for node in &net.nodes {
        let p = view.pt(node.x, node.y);
        let ly = if node.y > miny {
            p.y - 16.0
        } else {
            p.y + 32.0
        };
        let c = to(pos2(p.x, ly));
        let g = galley(ui, &node.id, font::mono(11.0), a::NODE_LABEL);
        painter.galley(
            pos2(c.x - g.size().x / 2.0, text_top(c.y, &g)),
            g,
            a::NODE_LABEL,
        );
    }
    // member labels, at 30 % along when another member shares the midpoint (crossing braces)
    let mids: Vec<Pos2> = report
        .truss
        .members
        .iter()
        .map(|m| {
            let (pa, pb) = (
                view.pt(net.nodes[m.a].x, net.nodes[m.a].y),
                view.pt(net.nodes[m.b].x, net.nodes[m.b].y),
            );
            pos2((pa.x + pb.x) / 2.0, (pa.y + pb.y) / 2.0)
        })
        .collect();
    for (i, m) in report.truss.members.iter().enumerate() {
        let (pa, pb) = (
            view.pt(net.nodes[m.a].x, net.nodes[m.a].y),
            view.pt(net.nodes[m.b].x, net.nodes[m.b].y),
        );
        let shared = mids
            .iter()
            .enumerate()
            .any(|(j, q)| j != i && q.distance(mids[i]) < 1e-3);
        let at = if shared {
            pos2(pa.x + (pb.x - pa.x) * 0.3, pa.y + (pb.y - pa.y) * 0.3)
        } else {
            mids[i]
        };
        let c = to(at);
        let (_, tc) = tone_colors(report.tones.get(i).copied().unwrap_or(Tone::Zero));
        let g = galley(ui, &m.id, font::mono(10.0), tc);
        let chip = Rect::from_center_size(c, vec2(g.size().x + 6.0, 13.0));
        painter.rect_filled(chip, CornerRadius::same(3), a::LABEL_BG);
        painter.galley(pos2(chip.left() + 3.0, text_top(c.y, &g)), g, tc);
    }
    let label = format!(
        "{} elevation with member forces. {}",
        match style {
            Some(s) => format!("{s} truss"),
            None => "Truss".to_string(),
        },
        if report.failed() {
            "Over its limit."
        } else {
            "Holding."
        }
    );
    ui.interact(rect, ui.scope_id().with("elevation"), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
}
