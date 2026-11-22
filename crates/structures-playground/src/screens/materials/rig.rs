//! MT-3 — the tensile rig in 2.5D (`Materials.dc.html`, `<svg viewBox="0 0 420 560">`): the
//! frame (base, columns, top beam), the crosshead rising with the stretch, the bottom grip, the
//! dog-bone sample stretching, necking and tearing with a red crack, the extensometer, plus the
//! "Load cell" chip (top left) and the "stretch exaggerated" badge (bottom right). The moving
//! geometry comes from [`sp_engine::materials::RigGeometry`].

use super::draw::{ViewBox, fill_convex, fill_polygon, outline};
use crate::screens::Badge;
use crate::widgets::badge::{badge_size, paint_badge};
use crate::widgets::text::{css_line_height, galley, lbl_galley, paint_line};
use crate::widgets::{BadgeLook, set_bounds};
use eframe::egui::{
    CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, accesskit::Role, vec2,
};
use sp_engine::materials::View;
use sp_engine::materials::view::RIG_VIEW;
use sp_theme as t;
use sp_theme::materials as mt;

/// Draws the rig into `rect` (the `.graph` stage box, `aspect-ratio: 420 / 560`).
pub fn show(ui: &mut Ui, rect: Rect, v: &View) {
    let mut stage = crate::layout::graph_panel(ui, rect, None);
    let vb = ViewBox::meet(rect, RIG_VIEW);
    let p = stage.painter().clone();
    let r = &v.rig;
    let px = |x: f64, y: f64| vb.pt([x, y]);

    // The frame.
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

    // The crosshead (block + beam through the columns) and its top face.
    let head_stroke = Stroke::new(vb.w(1.0), mt::HEAD_STROKE);
    for h in r.head {
        p.rect(vb.rect(h), 0.0, mt::HEAD, head_stroke, StrokeKind::Middle);
    }
    fill_convex(&p, vb.pts(&r.head_top), mt::HEAD_TOP);
    // The bottom grip.
    p.rect(
        vb.rect([176.0, 452.0, 244.0, 480.0]),
        0.0,
        mt::HEAD,
        head_stroke,
        StrokeKind::Middle,
    );

    // The sample (one piece, or two halves once torn).
    let fill = mt::SAMPLES[v.kind.index()].fill;
    for poly in &r.specimen {
        let ps = vb.pts(poly);
        fill_polygon(&p, &ps, fill);
        outline(&p, &ps, Stroke::new(vb.w(1.0), mt::SPECIMEN_STROKE));
    }
    // The extensometer.
    let gauge = Stroke::new(vb.w(1.5), mt::GAUGE);
    for seg in r.gauge {
        p.line_segment([vb.pt(seg[0]), vb.pt(seg[1])], gauge);
    }
    // The crack.
    if let Some(c) = r.crack {
        p.add(eframe::egui::Shape::line(
            vb.pts(&c),
            Stroke::new(vb.w(2.0), mt::CRACK),
        ));
    }

    load_cell(&mut stage, rect, v);
    let b = Badge::plain("stretch exaggerated");
    let size = badge_size(&stage, &b);
    let br = Rect::from_min_size(rect.max - vec2(10.0, 10.0) - size, size);
    paint_badge(&stage, br, &b, BadgeLook::default());

    crate::layout::graph_panel_border(ui, rect);
    // `role="img" aria-label="{{rigLabel}}"`.
    let resp = ui.interact(rect, ui.scope_id().with("materials-rig"), Sense::hover());
    let label = v.rig_label.clone();
    resp.widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
}

/// The load cell chip: `position: absolute; left: 10px; top: 10px; padding: 8px 10px;
/// border-radius: 8px; background: rgba(10,10,10,.82); border: 1px solid #242424; gap: 2px`
/// with `.lbl` "Load cell", the force (mono 16, accent) and "50 mm² section" (11 px muted).
fn load_cell(ui: &mut Ui, stage: Rect, v: &View) {
    let lg = lbl_galley(ui, "Load cell", t::TEXT_MUTED);
    let ff = lab_theme::font::mono(16.0);
    let fg = galley(ui, &v.force_label, ff.clone(), t::ACCENT);
    let nf = lab_theme::font::body(11.0);
    let ng = galley(ui, &v.section_label, nf.clone(), t::TEXT_MUTED);
    let (lh, fh, nh) = (
        css_line_height(ui, &lab_theme::font::semibold(11.0)),
        css_line_height(ui, &ff),
        css_line_height(ui, &nf),
    );
    let w = lg.size().x.max(fg.size().x).max(ng.size().x) + 2.0 * 10.0 + 2.0;
    let h = lh + fh + nh + 2.0 * 2.0 + 2.0 * 8.0 + 2.0;
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
    y += fh + 2.0;
    paint_line(p, &ng, x, y, nh, t::TEXT_MUTED);
    let child = ui.new_child(
        eframe::egui::UiBuilder::new()
            .max_rect(rect)
            .accessibility_role(Role::Group)
            .accessibility_label("Load cell"),
    );
    set_bounds(&child, rect);
    let resp = child.interact(rect, child.scope_id().with("force"), Sense::hover());
    let label = format!("Load cell {} \u{b7} {}", v.force_label, v.section_label);
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &label));
}
