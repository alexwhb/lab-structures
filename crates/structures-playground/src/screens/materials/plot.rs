//! MT-4 — the stress–strain plot (`Materials.dc.html`, `<div class="graph">` with an `<svg
//! viewBox="0 0 560 400">`): axes, grid, the other four materials as ghosts, the selected
//! curve, the done-so-far trace and the marker; the tick labels, axis titles and curve labels
//! are HTML positioned in percent of the box, as in the mockup (so they sit slightly off the
//! fitted SVG when the box is stretched, exactly like the reference). Geometry from
//! [`sp_engine::materials::PlotGeometry`].

use super::draw::ViewBox;
use crate::widgets::text::{css_line_height, galley};
use eframe::egui::{Pos2, Rect, Sense, Shape, Stroke, Ui, WidgetInfo, accesskit::Role, pos2};
use sp_engine::materials::View;
use sp_engine::materials::view::PLOT_VIEW;
use sp_theme::materials as mt;

/// Draws the plot into `rect` (the `.graph` box).
pub fn show(ui: &mut Ui, rect: Rect, v: &View) {
    let stage = crate::layout::graph_panel(ui, rect, None);
    // The SVG and the labels are `position: absolute` in the padding box (inside the border).
    let outer = rect;
    let rect = outer.shrink(1.0);
    let vb = ViewBox::meet(rect, PLOT_VIEW);
    let p = stage.painter().clone();
    let g = &v.plot;

    let line = |pts: &[[f64; 2]], stroke: Stroke| {
        if pts.len() >= 2 {
            p.add(Shape::line(vb.pts(pts), stroke));
        }
    };
    line(&g.axes, Stroke::new(vb.w(1.2), mt::AXIS));
    for l in &g.grid {
        line(l, Stroke::new(vb.w(1.0), mt::GRID));
    }
    for (_, ghost) in &g.ghosts {
        line(ghost, Stroke::new(vb.w(1.5), mt::GHOST_CURVE));
    }
    let colour = mt::SAMPLES[v.kind.index()].line;
    line(&g.curve, Stroke::new(vb.w(3.0), colour));
    round_joins(&p, &vb, &g.curve, vb.w(3.0), colour);
    line(&g.done, Stroke::new(vb.w(3.0), mt::DONE));
    round_joins(&p, &vb, &g.done, vb.w(3.0), mt::DONE);
    let m = vb.pt(g.marker);
    p.circle(
        m,
        vb.w(6.0),
        mt::DONE,
        Stroke::new(vb.w(2.0), mt::MARKER_RING),
    );

    // HTML labels, positioned in % of the box.
    let at = |xp: f64, yp: f64| -> Pos2 {
        pos2(
            rect.left() + rect.width() * xp as f32,
            rect.top() + rect.height() * yp as f32,
        )
    };
    let tick_font = lab_theme::font::mono(10.0);
    let tick_h = css_line_height(&stage, &tick_font);
    for tk in &g.ticks_y {
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
    for tk in &g.ticks_x {
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
    let sg = galley(&stage, "stress MPa", title_font.clone(), mt::AXIS_TITLE);
    let sp = at(0.12, 0.02);
    p.galley(
        pos2(sp.x, sp.y + text_offset(&sg, title_h)),
        sg,
        mt::AXIS_TITLE,
    );
    let eg = galley(&stage, "strain %", title_font, mt::AXIS_TITLE);
    let ep = at(0.98, 0.94);
    p.galley(
        pos2(ep.x - eg.size().x, ep.y + text_offset(&eg, title_h)),
        eg,
        mt::AXIS_TITLE,
    );
    for cl in &g.labels {
        // left: x%; top: y%; translate(4px, -110%); 11 px, 600 / 400
        let (font, colour) = if cl.selected {
            (lab_theme::font::semibold(11.0), mt::LABEL_ON)
        } else {
            (lab_theme::font::body(11.0), mt::LABEL_OFF)
        };
        let lh = css_line_height(&stage, &font);
        let gal = galley(&stage, cl.name, font, colour);
        let pt = at(cl.at[0] / PLOT_VIEW[0], cl.at[1] / PLOT_VIEW[1]);
        p.galley(
            pos2(pt.x + 4.0, pt.y - 1.1 * lh + text_offset(&gal, lh)),
            gal,
            colour,
        );
    }

    crate::layout::graph_panel_border(ui, outer);
    // `role="img" aria-label="{{plotLabel}}"`.
    let resp = ui.interact(outer, ui.scope_id().with("materials-plot"), Sense::hover());
    let label = v.plot_label.clone();
    resp.widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
}

/// Offset from a CSS line box's top to the galley's top so the text sits where Chrome puts it.
fn text_offset(g: &eframe::egui::Galley, line_h: f32) -> f32 {
    crate::widgets::text::text_top(line_h / 2.0, g)
}

/// `stroke-linejoin: round`: a dot at every inner corner of a thick polyline.
fn round_joins(
    p: &eframe::egui::Painter,
    vb: &ViewBox,
    pts: &[[f64; 2]],
    w: f32,
    c: eframe::egui::Color32,
) {
    if pts.len() < 3 {
        return;
    }
    for q in &pts[1..pts.len() - 1] {
        p.circle_filled(vb.pt(*q), w / 2.0, c);
    }
}
