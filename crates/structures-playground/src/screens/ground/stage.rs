//! The Ground stage (GD-3): the mockup's `<svg viewBox="0 0 900 600">` scaled to the stage
//! (aspect kept), drawn in its order: the soil block (surface, body, topsoil band, grains),
//! bedrock and its cracks, the water table (wet sand), sand boils, the building group
//! (`translate(450, 260 + sink) rotate(tilt × 3)`: foundation, side, front, windows, roof), the
//! plumb line, the pressure bulb; then the HTML overlays: the "tilt … · sank …" chip, the
//! "bedrock" and soil labels, and the legend. While quaking and playing the whole stage shakes
//! (the CSS `shake` keyframes, 0.18 s).

use super::GroundScreen;
use crate::layout;
use crate::widgets::Chip;
use crate::widgets::text::{css_line_height, galley, text_top};
use eframe::egui::{
    Align2, CornerRadius, Pos2, Rect, Sense, Shape as EShape, Stroke, StrokeKind, Ui, WidgetInfo,
    accesskit::Role, pos2, vec2,
};
use lab_theme::font;
use sp_engine::ground::{self, Foundation, GroundResult};
use sp_theme as t;
use sp_theme::ground as tg;

/// The SVG's width (viewBox units).
pub const W: f32 = 900.0;
/// The SVG's height.
pub const H: f32 = 600.0;
/// The ground surface's y.
pub const SURFACE_Y: f32 = 260.0;
/// The bedrock's top y (30 m down).
pub const BEDROCK_Y: f32 = 530.0;
/// Pixels per metre of pile.
pub const PILE_PX_PER_M: f32 = 9.0;

/// The building's drawn sink (px) and tilt (degrees, clockwise): the mockup's
/// `min(130, settle × 0.18)` and `min(24, tilt × 3)`.
pub fn sink_and_tilt(r: &GroundResult) -> (f32, f32) {
    (
        (r.settle * 0.18).min(130.0) as f32,
        (r.tilt * 3.0).min(24.0) as f32,
    )
}

/// The CSS `shake` keyframes (0.18 s, linear): offset at time `t`, px.
pub fn shake_offset(t: f32) -> eframe::egui::Vec2 {
    const KEYS: [(f32, f32, f32); 6] = [
        (0.0, 0.0, 0.0),
        (0.2, -4.0, 1.0),
        (0.4, 4.0, -1.0),
        (0.6, -3.0, 0.0),
        (0.8, 3.0, 1.0),
        (1.0, 0.0, 0.0),
    ];
    let u = (t / 0.18).rem_euclid(1.0);
    for w in KEYS.windows(2) {
        let (a, b) = (w[0], w[1]);
        if u <= b.0 {
            let f = (u - a.0) / (b.0 - a.0);
            return vec2(a.1 + (b.1 - a.1) * f, a.2 + (b.2 - a.2) * f);
        }
    }
    vec2(0.0, 0.0)
}

#[derive(Clone, Copy)]
struct Map {
    origin: Pos2,
    scale: f32,
}

impl Map {
    fn p(&self, x: f32, y: f32) -> Pos2 {
        pos2(
            self.origin.x + x * self.scale,
            self.origin.y + y * self.scale,
        )
    }
}

/// The building group's transform: building coordinates → viewBox.
#[derive(Clone, Copy)]
struct Group {
    tx: f32,
    ty: f32,
    cos: f32,
    sin: f32,
}

impl Group {
    fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.tx + x * self.cos - y * self.sin,
            self.ty + x * self.sin + y * self.cos,
        )
    }
}

/// Rectangles of the foundation in building coordinates (x0, y0, x1, y1).
pub fn foundation_rects(found: Foundation, pile_len: f64) -> Vec<[f32; 4]> {
    match found {
        Foundation::Footing => [-84.0, -42.0, 0.0, 42.0, 84.0]
            .iter()
            .map(|x| [x - 14.0, 0.0, x + 14.0, 16.0])
            .collect(),
        Foundation::Mat => vec![[-112.0, 0.0, 112.0, 18.0]],
        Foundation::Piles => {
            let lp = pile_len as f32 * PILE_PX_PER_M;
            let mut v = vec![[-108.0, 0.0, 108.0, 14.0]];
            v.extend(
                [-90.0, -54.0, -18.0, 18.0, 54.0, 90.0]
                    .iter()
                    .map(|x| [x - 4.0, 14.0, x + 4.0, lp]),
            );
            v
        }
        Foundation::Caisson => {
            let mut v = vec![[-108.0, 0.0, 108.0, 14.0]];
            v.extend(
                [-62.0, 62.0]
                    .iter()
                    .map(|x| [x - 15.0, 14.0, x + 15.0, 272.0]),
            );
            v
        }
    }
}

/// Draws the stage into `rect`.
pub fn draw(ui: &mut Ui, rect: Rect, s: &GroundScreen, r: &GroundResult) {
    let input = s.input.sanitized();
    let shaking = input.quake && s.playing;
    let rect_drawn = if shaking {
        rect.translate(shake_offset(s.anim))
    } else {
        rect
    };
    let panel = layout::graph_panel(ui, rect_drawn, None);
    let map = Map {
        origin: rect_drawn.min,
        scale: rect_drawn.width() / W,
    };
    let k = map.scale;
    let p = panel.painter();
    let sw = tg::SOILS[input.soil as usize];

    // soil surface, body, topsoil band, grains
    p.add(EShape::convex_polygon(
        vec![
            map.p(60.0, 232.0),
            map.p(900.0, 232.0),
            map.p(900.0, 260.0),
            map.p(0.0, 260.0),
        ],
        sw.top,
        Stroke::NONE,
    ));
    p.rect_filled(
        Rect::from_min_max(map.p(0.0, 260.0), map.p(900.0, 530.0)),
        0.0,
        sw.body,
    );
    p.rect_filled(
        Rect::from_min_max(map.p(0.0, 260.0), map.p(900.0, 278.0)),
        0.0,
        tg::TOPSOIL,
    );
    for i in 0..140 {
        let x = (i as f64 * 97.3) % 900.0;
        let y = 284.0 + (i as f64 * 53.7) % 240.0;
        p.rect_filled(
            Rect::from_min_size(map.p(x as f32, y as f32), vec2(2.0 * k, 2.0 * k)),
            0.0,
            tg::GRAIN,
        );
    }
    // bedrock and its cracks
    p.rect_filled(
        Rect::from_min_max(map.p(0.0, 530.0), map.p(900.0, 600.0)),
        0.0,
        tg::BEDROCK,
    );
    let crack = Stroke::new(1.5 * k, tg::BEDROCK_CRACK);
    p.line_segment([map.p(0.0, 530.0), map.p(900.0, 530.0)], crack);
    for pts in [
        [(80.0, 548.0), (120.0, 560.0), (150.0, 552.0)],
        [(300.0, 570.0), (340.0, 556.0), (380.0, 574.0)],
        [(620.0, 552.0), (660.0, 566.0), (700.0, 556.0)],
    ] {
        p.line_segment(
            [map.p(pts[0].0, pts[0].1), map.p(pts[1].0, pts[1].1)],
            crack,
        );
        p.line_segment(
            [map.p(pts[1].0, pts[1].1), map.p(pts[2].0, pts[2].1)],
            crack,
        );
    }
    // the water table (flows while playing: `.flow` 1.6 s per 48 px)
    if input.soil.wet() {
        let off = if s.playing {
            (s.anim / 1.6).rem_euclid(1.0) * 48.0
        } else {
            0.0
        };
        p.extend(EShape::dashed_line_with_offset(
            &[map.p(0.0, 287.0), map.p(900.0, 287.0)],
            Stroke::new(1.5 * k, tg::WATER),
            &[10.0 * k],
            &[8.0 * k],
            (18.0 - off % 18.0) * k,
        ));
    }
    // sand boils
    if r.liquefied && input.years > 0.0 {
        let tick = sp_engine::jsnum::js_round(input.years * 8.0) as i64;
        for (i, x) in [140.0, 230.0, 650.0, 760.0, 330.0, 580.0]
            .iter()
            .enumerate()
        {
            let rr = 6.0 + ((i as i64 * 7 + tick).rem_euclid(6)) as f32;
            ellipse(p, map.p(*x, 260.0), rr * k, rr * 0.5 * k, tg::BOIL);
        }
    }

    // the building
    let (sink, tilt) = sink_and_tilt(r);
    let th = tilt.to_radians();
    let g = Group {
        tx: 450.0,
        ty: 260.0 + sink,
        cos: th.cos(),
        sin: th.sin(),
    };
    let quad = |x0: f32, y0: f32, x1: f32, y1: f32| -> Vec<Pos2> {
        [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
            .iter()
            .map(|&(x, y)| {
                let (a, b) = g.apply(x, y);
                map.p(a, b)
            })
            .collect()
    };
    let poly = |pts: &[(f32, f32)]| -> Vec<Pos2> {
        pts.iter()
            .map(|&(x, y)| {
                let (a, b) = g.apply(x, y);
                map.p(a, b)
            })
            .collect()
    };
    for [x0, y0, x1, y1] in foundation_rects(input.foundation, input.pile_len) {
        p.add(EShape::convex_polygon(
            quad(x0, y0, x1, y1),
            tg::FOUNDATION,
            Stroke::new(k, tg::FOUNDATION_EDGE),
        ));
    }
    let bh = (input.floors as f32 * 6.0).min(200.0);
    let (dx, dy) = (46.0, -28.0);
    p.add(EShape::convex_polygon(
        poly(&[
            (100.0, 0.0),
            (100.0 + dx, dy),
            (100.0 + dx, dy - bh),
            (100.0, -bh),
        ]),
        tg::BUILDING_SIDE,
        Stroke::NONE,
    ));
    p.add(EShape::convex_polygon(
        quad(-100.0, -bh, 100.0, 0.0),
        tg::BUILDING_FRONT,
        Stroke::new(0.8 * k, tg::BUILDING_EDGE),
    ));
    let rows = input.floors.min(33);
    for i in 0..rows {
        let y = -(i as f32 + 0.5) * bh / rows as f32;
        let a = poly(&[(-86.0, y), (86.0, y)]);
        p.line_segment([a[0], a[1]], Stroke::new(2.0 * k, tg::WINDOWS));
    }
    p.add(EShape::convex_polygon(
        poly(&[
            (-100.0, -bh),
            (-100.0 + dx, dy - bh),
            (100.0 + dx, dy - bh),
            (100.0, -bh),
        ]),
        tg::ROOF,
        Stroke::NONE,
    ));

    // plumb line
    p.extend(EShape::dashed_line(
        &[map.p(450.0, 236.0), map.p(450.0, 40.0)],
        Stroke::new(k, tg::PLUMB),
        3.0 * k,
        5.0 * k,
    ));
    // the pressure bulb
    let bulb_depth: f32 = if r.deep {
        if r.to_rock { 0.0 } else { 150.0 }
    } else {
        130.0
    };
    let below = if r.deep {
        if input.foundation == Foundation::Piles {
            input.pile_len as f32 * PILE_PX_PER_M
        } else {
            270.0
        }
    } else {
        18.0
    };
    let bulb_top = SURFACE_Y + sink + below;
    if bulb_depth > 0.0 && bulb_top < 520.0 {
        // the mockup rounds the end points (`toFixed(0)`)
        let top = bulb_top.round();
        let ctrl = bulb_top + bulb_depth;
        let pts: Vec<Pos2> = (0..=32)
            .map(|i| {
                let u = i as f32 / 32.0;
                let (p0, c1, c2, p3) = ((330.0, top), (330.0, ctrl), (570.0, ctrl), (570.0, top));
                let a = (1.0 - u).powi(3);
                let b = 3.0 * (1.0 - u).powi(2) * u;
                let c = 3.0 * (1.0 - u) * u * u;
                let d = u.powi(3);
                map.p(
                    a * p0.0 + b * c1.0 + c * c2.0 + d * p3.0,
                    a * p0.1 + b * c1.1 + c * c2.1 + d * p3.1,
                )
            })
            .collect();
        p.extend(EShape::dashed_line(
            &pts,
            Stroke::new(1.5 * k, tg::BULB),
            4.0 * k,
            4.0 * k,
        ));
    }

    // overlays (CSS px)
    let chip = Chip::new(ground::stage_chip(r));
    let cs = chip.size(&panel);
    let at = pos2(
        rect_drawn.left() + rect_drawn.width() * 0.74,
        rect_drawn.top() + rect_drawn.height() * 0.10,
    );
    chip.show_at(&panel, pos2(at.x, at.y - 0.3 * cs.y), Align2::CENTER_BOTTOM);
    // the soil name and "bedrock", each on a dark chip so they read on every soil (MB-7)
    let mono = font::mono(11.0);
    let line = css_line_height(&panel, &mono);
    let bg = galley(&panel, "bedrock", mono.clone(), t::TEXT_MUTED);
    let by = rect_drawn.top() + rect_drawn.height() * 0.84;
    label_on_chip(
        &panel,
        pos2(rect_drawn.right() - 12.0 - bg.size().x, by),
        line,
        bg,
        t::TEXT_MUTED,
    );
    let name = input.soil.def().name.to_lowercase();
    let sg = galley(&panel, &name, mono, tg::SOIL_LABEL);
    let sy = rect_drawn.top() + rect_drawn.height() * 0.64;
    label_on_chip(
        &panel,
        pos2(rect_drawn.left() + 12.0, sy),
        line,
        sg,
        tg::SOIL_LABEL,
    );
    legend(&panel, rect_drawn, input.soil.wet());

    let label = ground::stage_label(&input, r);
    let resp = panel.interact(rect, panel.scope_id().with("ground-stage"), Sense::hover());
    resp.widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
    layout::graph_panel_border(ui, rect_drawn);
}

/// A stage label whose text box starts at `at` (CSS `left` / `top`), on a [`tg::LABEL_CHIP`]
/// (`padding: 2px 6px`, radius 4) so it reaches 4.5:1 on every soil (SPEC MB-7).
fn label_on_chip(
    ui: &Ui,
    at: Pos2,
    line: f32,
    g: std::sync::Arc<eframe::egui::Galley>,
    color: eframe::egui::Color32,
) {
    let chip = Rect::from_min_size(
        pos2(at.x - 6.0, at.y - 2.0),
        vec2(g.size().x + 12.0, line + 4.0),
    );
    let p = ui.painter();
    p.rect_filled(chip, CornerRadius::same(4), tg::LABEL_CHIP);
    p.galley(pos2(at.x, text_top(at.y + line / 2.0, &g)), g, color);
}

/// The legend box at the stage's bottom-left (`left: 12px; bottom: 10px; padding: 6px 10px;
/// gap: 6px 14px`, 12 px `#D9D9D9`, 18 px dashed swatches).
fn legend(ui: &Ui, stage: Rect, wet: bool) {
    let f = font::body(12.0);
    let mut items = vec![("Pressure bulb under the foundation", tg::BULB_LEGEND)];
    if wet {
        items.push(("Water table", tg::WATER));
    }
    let gs: Vec<_> = items
        .iter()
        .map(|(s, _)| galley(ui, *s, f.clone(), t::TEXT_DIM))
        .collect();
    let line = css_line_height(ui, &f);
    let max_w = stage.width() - 24.0 - 22.0;
    // flex-wrap rows
    let mut rows: Vec<Vec<usize>> = vec![vec![]];
    let mut x = 0.0;
    for (i, g) in gs.iter().enumerate() {
        let w = 18.0 + 6.0 + g.size().x;
        if x > 0.0 && x + 14.0 + w > max_w {
            rows.push(vec![]);
            x = 0.0;
        }
        x += if x > 0.0 { 14.0 } else { 0.0 } + w;
        if let Some(r) = rows.last_mut() {
            r.push(i);
        }
    }
    let row_w = |r: &Vec<usize>| -> f32 {
        r.iter().map(|&i| 18.0 + 6.0 + gs[i].size().x).sum::<f32>()
            + 14.0 * (r.len().saturating_sub(1)) as f32
    };
    let w = rows.iter().map(row_w).fold(0.0, f32::max) + 22.0;
    let h = rows.len() as f32 * line + (rows.len() - 1) as f32 * 6.0 + 14.0;
    let rect = Rect::from_min_size(
        pos2(stage.left() + 12.0, stage.bottom() - 10.0 - h),
        vec2(w, h),
    );
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(t::radius::WELL),
        tg::LEGEND,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    let mut y = rect.top() + 7.0;
    for r in &rows {
        let mut x = rect.left() + 11.0;
        let cy = y + line / 2.0;
        for &i in r {
            p.extend(EShape::dashed_line(
                &[pos2(x, cy), pos2(x + 18.0, cy)],
                Stroke::new(2.0, items[i].1),
                6.0,
                3.0,
            ));
            x += 24.0;
            p.galley(pos2(x, text_top(cy, &gs[i])), gs[i].clone(), t::TEXT_DIM);
            x += gs[i].size().x + 14.0;
        }
        y += line + 6.0;
    }
    let names: Vec<&str> = items.iter().map(|(s, _)| *s).collect();
    let name = format!("Legend: {}", names.join(", "));
    let resp = ui.interact(rect, ui.scope_id().with("ground-legend"), Sense::hover());
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &name));
}

fn ellipse(p: &eframe::egui::Painter, c: Pos2, rx: f32, ry: f32, color: eframe::egui::Color32) {
    let pts: Vec<Pos2> = (0..24)
        .map(|i| {
            let a = i as f32 / 24.0 * std::f32::consts::TAU;
            pos2(c.x + rx * a.cos(), c.y + ry * a.sin())
        })
        .collect();
    p.add(EShape::convex_polygon(pts, color, Stroke::NONE));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shake_follows_the_keyframes() {
        assert_eq!(shake_offset(0.0), vec2(0.0, 0.0));
        let at = shake_offset(0.036); // 20 %
        assert!((at.x + 4.0).abs() < 1e-3 && (at.y - 1.0).abs() < 1e-3);
        assert!(shake_offset(0.18).length() < 1e-3);
    }

    #[test]
    fn piles_reach_their_length() {
        let r = foundation_rects(Foundation::Piles, 34.0);
        assert_eq!(r.len(), 7);
        assert_eq!(r[1][3], 34.0 * 9.0);
    }
}
