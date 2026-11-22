//! The aside (`<aside aria-label="Level and inspector">`, BR-5, BR-8…BR-10): the level rows
//! with best stars, the goal panel (goal, status pill, cost, "Test it: add the load"), the
//! member / joint inspector or "Built from" (material chips, Joints / Members / Weight tiles,
//! "Drop in a starter truss", "Open in Advanced"), and What's happening.

use super::Action;
use super::bits::{
    CHIP_GAP, ChipLook, ChipSpec, bar, chips_layout, chips_show, label_node, level_row, pill, tile,
    tile_height,
};
use super::stage::tone_color;
use crate::widgets::text::{galley, lbl_galley, text_top, wrapped};
use crate::widgets::{Callout, ghost, prim};
use eframe::egui::{
    CornerRadius, Id, Rect, Stroke, StrokeKind, Ui, UiBuilder, accesskit::Role, pos2, vec2,
};
use sp_engine::bridge::view::{BarTone, Inspector, View};
use sp_engine::bridge::{MaterialKey, SizeKey, Support};
use sp_theme as t;

fn material_dot(m: MaterialKey) -> eframe::egui::Color32 {
    match m {
        MaterialKey::Steel => t::STEEL.fill,
        MaterialKey::Timber => t::TIMBER.fill,
        MaterialKey::Aluminum => t::ALUMINUM.fill,
        MaterialKey::CarbonFiber => t::CARBON_DOT,
    }
}

/// A `.lbl` eyebrow at the cursor.
fn eyebrow(ui: &mut Ui, text: &str) {
    crate::widgets::lbl(ui, text);
}

/// A labelled region for a section (AccessKit `Group`), drawn by `add` from the cursor.
fn section(ui: &mut Ui, label: &'static str, gap: f32, add: impl FnOnce(&mut Ui)) {
    let w = ui.available_width();
    let top = ui.cursor().min;
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_size(top, vec2(w, f32::INFINITY)))
            .layout(eframe::egui::Layout::top_down(eframe::egui::Align::Min))
            .accessibility_role(Role::Group)
            .accessibility_label(label),
    );
    child.spacing_mut().item_spacing = vec2(gap, gap);
    add(&mut child);
    let used = Rect::from_min_size(top, vec2(w, child.min_rect().height()));
    crate::widgets::set_bounds(&child, used);
    ui.advance_cursor_after_rect(used);
}

/// A framed block (`.panel` / `.well`) with `padding: 12px; gap: 10px`.
fn framed(
    ui: &mut Ui,
    label: &'static str,
    fill: eframe::egui::Color32,
    radius: u8,
    add: impl FnOnce(&mut Ui),
) {
    let w = ui.available_width();
    let top = ui.cursor().min;
    let bg = ui.painter().add(eframe::egui::Shape::Noop);
    let inner = Rect::from_min_size(top + vec2(13.0, 13.0), vec2(w - 26.0, f32::INFINITY));
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(eframe::egui::Layout::top_down(eframe::egui::Align::Min))
            .accessibility_role(Role::Group)
            .accessibility_label(label),
    );
    child.spacing_mut().item_spacing = vec2(10.0, 10.0);
    add(&mut child);
    let h = child.min_rect().height() + 26.0;
    let rect = Rect::from_min_size(top, vec2(w, h));
    ui.painter().set(
        bg,
        eframe::egui::epaint::RectShape::new(
            rect,
            CornerRadius::same(radius),
            fill,
            Stroke::new(1.0, t::BORDER),
            StrokeKind::Inside,
        ),
    );
    crate::widgets::set_bounds(&child, rect);
    ui.advance_cursor_after_rect(rect);
}

/// A paragraph at the cursor (wrapped, CSS line height).
fn para(ui: &mut Ui, text: &str, size: f32, lh: f32, color: eframe::egui::Color32) {
    let w = ui.available_width();
    let g = wrapped(
        ui,
        text,
        lab_theme::font::body(size),
        color,
        w,
        Some(size * lh),
    );
    let rect = Rect::from_min_size(ui.cursor().min, vec2(w, g.size().y));
    ui.painter().galley(rect.min, g, color);
    label_node(ui, ("bridge-para", text), rect, text);
    ui.advance_cursor_after_rect(rect);
}

/// A row: `.lbl` caption then chips (`role="group" aria-label`), gap 6 / 8.
fn chip_row(
    ui: &mut Ui,
    salt: &str,
    group: &'static str,
    caption: &str,
    gap: f32,
    specs: &[ChipSpec<'_>],
) -> Option<usize> {
    let w = ui.available_width();
    let top = ui.cursor().min;
    let lg = lbl_galley(ui, caption, t::TEXT_MUTED);
    let chips_w = w - lg.size().x - gap;
    let (rects, size) = chips_layout(ui, specs, chips_w.max(60.0), vec2(CHIP_GAP, CHIP_GAP));
    let h = size.y.max(lg.size().y);
    let rect = Rect::from_min_size(top, vec2(w, h));
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .accessibility_role(Role::Group)
            .accessibility_label(group),
    );
    crate::widgets::set_bounds(&child, rect);
    let first_cy = top.y + super::bits::CHIP_H / 2.0;
    child.painter().galley(
        pos2(top.x, text_top(first_cy, &lg)),
        lg.clone(),
        t::TEXT_MUTED,
    );
    let clicked = chips_show(
        &mut child,
        salt,
        pos2(top.x + lg.size().x + gap, top.y),
        specs,
        &rects,
    );
    ui.advance_cursor_after_rect(rect);
    clicked
}

/// Draws the aside's content (inside the column's padding).
pub fn show(ui: &mut Ui, v: &View, actions: &mut Vec<Action>) {
    let w = ui.available_width();
    // ---- Level
    section(ui, "Level", 8.0, |ui| {
        eyebrow(ui, "Level");
        let top = ui.cursor().min;
        let mut y = top.y;
        for row in &v.levels {
            let rect = Rect::from_min_size(pos2(top.x, y), vec2(w, t::size::ROW));
            let stars = if row.best.is_empty() {
                None
            } else {
                Some(row.best.chars().filter(|c| *c == '★').count() as u8)
            };
            if level_row(
                ui,
                Id::unique(("bridge-level", row.key.key())),
                rect,
                row.name,
                stars,
                row.on,
            )
            .clicked()
            {
                actions.push(Action::Level(row.key));
            }
            y += t::size::ROW + 4.0;
        }
        ui.advance_cursor_after_rect(Rect::from_min_max(top, pos2(top.x + w, y - 4.0)));
    });

    // ---- the goal panel
    framed(ui, "Goal", t::PANEL, t::radius::PANEL, |ui| {
        para(ui, v.goal, 13.0, 1.5, t::TEXT_BODY);
        let top = ui.cursor().min;
        let iw = ui.available_width();
        let pr = pill(ui, top, &v.status.label, tone_color(v.status.tone));
        let cost_color = if v.over_budget {
            t::DANGER_INK
        } else {
            t::TEXT
        };
        let cg = galley(ui, &v.cost_text, lab_theme::font::mono(13.0), cost_color);
        let cr = Rect::from_min_size(
            pos2(top.x + iw - cg.size().x, pr.center().y - cg.size().y / 2.0),
            cg.size(),
        );
        ui.painter().galley(cr.min, cg, cost_color);
        label_node(ui, "bridge-cost", cr, &format!("Cost {}", v.cost_text));
        ui.advance_cursor_after_rect(Rect::from_min_max(top, pos2(top.x + iw, pr.bottom())));
        if prim(v.test_label).big().show(ui).clicked() {
            actions.push(Action::TestToggle);
        }
    });

    // ---- inspector / Built from
    match &v.inspector {
        Inspector::Member(mi) => {
            framed(ui, "Member", t::WELL, t::radius::WELL, |ui| {
                let top = ui.cursor().min;
                let iw = ui.available_width();
                let tg = galley(ui, &mi.title, lab_theme::font::heading(15.0), t::TEXT);
                let lg = galley(ui, &mi.length, lab_theme::font::body(12.0), t::TEXT_MUTED);
                ui.painter().galley(top, tg.clone(), t::TEXT);
                let base = top.y + tg.size().y;
                ui.painter().galley(
                    pos2(top.x + iw - lg.size().x, base - lg.size().y),
                    lg,
                    t::TEXT_MUTED,
                );
                label_node(
                    ui,
                    "bridge-mi-title",
                    Rect::from_min_size(top, tg.size()),
                    &mi.title,
                );
                ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(iw, tg.size().y)));
                // utilisation bar + percent
                let top = ui.cursor().min;
                let ug = galley(ui, &mi.ut, lab_theme::font::mono(13.0), t::TEXT);
                let h = ug.size().y.max(8.0);
                let color = match mi.tone {
                    BarTone::Zero => t::ZERO_FORCE,
                    BarTone::Tension => t::TENSION,
                    BarTone::Compression => t::COMPRESSION,
                    BarTone::Over => t::DANGER,
                };
                let bar_r = Rect::from_min_size(
                    pos2(top.x, top.y + h / 2.0 - 4.0),
                    vec2(iw - ug.size().x - 10.0, 8.0),
                );
                bar(ui.painter(), bar_r, mi.fill as f32, color);
                let ur = Rect::from_min_size(pos2(top.x + iw - ug.size().x, top.y), ug.size());
                ui.painter().galley(ur.min, ug, t::TEXT);
                label_node(ui, "bridge-mi-ut", ur, &format!("{} of its limit", mi.ut));
                ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(iw, h)));
                para(ui, &mi.text, 13.0, 1.5, t::TEXT_BODY);
                let specs: Vec<ChipSpec> = SizeKey::ALL
                    .iter()
                    .map(|s| ChipSpec {
                        label: s.name(),
                        name: s.name(),
                        look: ChipLook {
                            on: *s == mi.size,
                            ..Default::default()
                        },
                    })
                    .collect();
                if let Some(i) = chip_row(ui, "mi-size", "Size of this member", "Size", 8.0, &specs)
                {
                    actions.push(Action::MemberSize(mi.id.clone(), SizeKey::ALL[i]));
                }
                if ghost("Delete member")
                    .full_width()
                    .centered()
                    .show(ui)
                    .clicked()
                {
                    actions.push(Action::DeleteMember(mi.id.clone()));
                }
            });
        }
        Inspector::Joint(ni) => {
            framed(ui, "Joint", t::WELL, t::radius::WELL, |ui| {
                let top = ui.cursor().min;
                let iw = ui.available_width();
                let tg = galley(ui, &ni.title, lab_theme::font::heading(15.0), t::TEXT);
                let pg = galley(ui, &ni.pos, lab_theme::font::mono(12.0), t::TEXT_MUTED);
                ui.painter().galley(top, tg.clone(), t::TEXT);
                let base = top.y + tg.size().y;
                let pr = Rect::from_min_size(
                    pos2(top.x + iw - pg.size().x, base - pg.size().y),
                    pg.size(),
                );
                ui.painter().galley(pr.min, pg, t::TEXT_MUTED);
                label_node(
                    ui,
                    "bridge-ni-title",
                    Rect::from_min_size(top, tg.size()),
                    &ni.title,
                );
                label_node(ui, "bridge-ni-pos", pr, &ni.pos);
                ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(iw, tg.size().y)));
                let opts = [
                    (None, "None"),
                    (Some(Support::Pin), "Pin"),
                    (Some(Support::Roller), "Roller"),
                ];
                let specs: Vec<ChipSpec> = opts
                    .iter()
                    .map(|(s, name)| ChipSpec {
                        label: name,
                        name,
                        look: ChipLook {
                            on: ni.support == *s,
                            dim: ni.chip_dim(*s),
                            dot: None,
                        },
                    })
                    .collect();
                if let Some(i) = chip_row(ui, "ni-sup", "Support", "Support", 8.0, &specs) {
                    actions.push(Action::SetSupport(ni.id.clone(), opts[i].0));
                }
                // Weight − value +
                let top = ui.cursor().min;
                let lg = lbl_galley(ui, "Weight", t::TEXT_MUTED);
                let cy = top.y + t::size::BUTTON / 2.0;
                ui.painter()
                    .galley(pos2(top.x, text_top(cy, &lg)), lg.clone(), t::TEXT_MUTED);
                let mut x = top.x + lg.size().x + 8.0;
                let less = Rect::from_min_size(pos2(x, top.y), vec2(40.0, t::size::BUTTON));
                if ghost("−")
                    .centered()
                    .accessible_name("Less weight")
                    .show_at(ui, less)
                    .clicked()
                {
                    actions.push(Action::LessWeight(ni.id.clone()));
                }
                x += 48.0;
                let vg = galley(ui, &ni.load, lab_theme::font::mono(14.0), t::TEXT);
                let vw = vg.size().x.max(54.0);
                let vr = Rect::from_min_size(pos2(x, top.y), vec2(vw, t::size::BUTTON));
                ui.painter().galley(
                    pos2(vr.center().x - vg.size().x / 2.0, text_top(cy, &vg)),
                    vg,
                    t::TEXT,
                );
                label_node(ui, "bridge-ni-load", vr, &format!("Weight {}", ni.load));
                x += vw + 8.0;
                let more = Rect::from_min_size(pos2(x, top.y), vec2(40.0, t::size::BUTTON));
                if ghost("+")
                    .centered()
                    .accessible_name("More weight")
                    .show_at(ui, more)
                    .clicked()
                {
                    actions.push(Action::MoreWeight(ni.id.clone()));
                }
                ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(iw, t::size::BUTTON)));
                para(ui, ni.note, 12.0, 1.25, t::TEXT_MUTED);
                if ni.can_delete
                    && ghost("Delete joint")
                        .full_width()
                        .centered()
                        .show(ui)
                        .clicked()
                {
                    actions.push(Action::DeleteJoint(ni.id.clone()));
                }
            });
        }
        Inspector::Nothing(bf) => {
            section(ui, "Built from", 8.0, |ui| {
                eyebrow(ui, "Built from");
                let names: Vec<&str> = MaterialKey::ALL.iter().map(|m| m.name()).collect();
                let specs: Vec<ChipSpec> = MaterialKey::ALL
                    .iter()
                    .zip(&names)
                    .map(|(m, n)| ChipSpec {
                        label: n,
                        name: n,
                        look: ChipLook {
                            on: *m == bf.material,
                            dim: false,
                            dot: Some(material_dot(*m)),
                        },
                    })
                    .collect();
                let iw = ui.available_width();
                let (rects, size) = chips_layout(ui, &specs, iw, vec2(CHIP_GAP, CHIP_GAP));
                let top = ui.cursor().min;
                let gr = Rect::from_min_size(top, vec2(iw, size.y));
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .max_rect(gr)
                        .accessibility_role(Role::Group)
                        .accessibility_label("Material"),
                );
                crate::widgets::set_bounds(&child, gr);
                if let Some(i) = chips_show(&mut child, "mat", top, &specs, &rects) {
                    actions.push(Action::Material(MaterialKey::ALL[i]));
                }
                ui.advance_cursor_after_rect(gr);
                // three tiles
                let top = ui.cursor().min;
                let th = tile_height(ui);
                let tw = (iw - 12.0) / 3.0;
                for (k, (cap, val)) in [
                    ("Joints", bf.joints.as_str()),
                    ("Members", bf.members.as_str()),
                    ("Weight", bf.weight.as_str()),
                ]
                .into_iter()
                .enumerate()
                {
                    let r = Rect::from_min_size(
                        pos2(top.x + k as f32 * (tw + 6.0), top.y),
                        vec2(tw, th),
                    );
                    tile(ui, r, cap, val);
                }
                ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(iw, th)));
                if bf.can_starter
                    && ghost("Drop in a starter truss")
                        .full_width()
                        .centered()
                        .show(ui)
                        .clicked()
                {
                    actions.push(Action::Starter);
                }
                if ghost("Open in Advanced")
                    .icon(crate::widgets::Icon::Chart)
                    .full_width()
                    .centered()
                    .show(ui)
                    .clicked()
                {
                    actions.push(Action::OpenInAdvanced);
                }
            });
        }
    }

    // ---- What's happening
    Callout::happening(&v.explain).show(ui);
}
