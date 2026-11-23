//! The hint bar over the stage (BLD-1): a `.panel` (`padding: 8px 12px; min-height: 52px`,
//! wrapping with `gap: 8px 14px`, items centred) holding the tool's chip group (the piece
//! palette with swatch bars, the foundation chips, or "Crate" and the weights), the hint
//! (`flex: 1 1 200px`, 13 px `#BDBDBD`), "Finish line" while a chain is open, and "Show
//! forces". Laid out like CSS flex-wrap: the chip group is one item at its max-content width
//! that shrinks (wrapping its chips) when it has a line to itself; the hint takes the free
//! space of its line.

use super::bits::{
    CHIP_GAP, CHIP_H, ChipLook, ChipSpec, chip_at, chip_size, chips_layout, label_node,
};
use crate::widgets::ghost;
use crate::widgets::set_bounds;
use crate::widgets::text::{lbl_galley, text_top, wrapped};
use eframe::egui::{
    CornerRadius, Id, Rect, Stroke, StrokeKind, Ui, UiBuilder, accesskit::Role, pos2, vec2,
};
use sp_theme::{self as t, builder as b};

/// The tool's chip group.
pub struct HintGroup<'a> {
    /// Id salt.
    pub salt: &'a str,
    /// `aria-label` ("Piece to build", "Foundation type", "Weight to add").
    pub group: &'a str,
    /// A `.lbl` caption before the chips ("Crate").
    pub caption: Option<&'a str>,
    /// The chips.
    pub chips: Vec<ChipSpec>,
}

/// What the bar shows.
pub struct HintBar<'a> {
    /// The tool's hint.
    pub hint: &'a str,
    /// The tool's chip group.
    pub group: Option<HintGroup<'a>>,
    /// "Finish line" (a chain is open).
    pub finish: bool,
    /// "Show forces" is on.
    pub forces: bool,
}

/// What was clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintClick {
    /// The group's chip `i`.
    Chip(usize),
    /// "Finish line".
    Finish,
    /// "Show forces".
    Forces,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum K {
    Group,
    Hint,
    Finish,
    Forces,
}

/// Draws the bar at the cursor, full width; returns what was clicked.
pub fn show(ui: &mut Ui, bar: &HintBar<'_>) -> Option<HintClick> {
    let w = ui.available_width();
    let inner_w = (w - 26.0).max(40.0);
    let hint_font = lab_theme::font::body(13.0);
    let forces = [ChipSpec {
        label: "Show forces".into(),
        name: "Show forces".into(),
        look: ChipLook {
            on: bar.forces,
            ..Default::default()
        },
    }];
    let forces_w = chip_size(ui, "Show forces", &forces[0].look).x;
    let finish_w = ghost("Finish line").desired_size(ui).x;
    let caption = bar
        .group
        .as_ref()
        .and_then(|g| g.caption)
        .map(|c| lbl_galley(ui, c, t::TEXT_MUTED));
    let cap_w = caption.as_ref().map_or(0.0, |g| g.size().x + 6.0);
    // the group at max-content and its smallest (its widest chip)
    let (group_max, group_min) = bar.group.as_ref().map_or((0.0, 0.0), |g| {
        let (_, size) = chips_layout(ui, &g.chips, f32::INFINITY, vec2(CHIP_GAP, CHIP_GAP));
        let widest = g
            .chips
            .iter()
            .map(|c| chip_size(ui, &c.label, &c.look).x)
            .fold(0.0, f32::max);
        (cap_w + size.x, cap_w + widest)
    });
    let mut items: Vec<(K, f32)> = Vec::new();
    if bar.group.is_some() {
        items.push((K::Group, group_max));
    }
    items.push((K::Hint, 200.0f32.min(inner_w)));
    if bar.finish {
        items.push((K::Finish, finish_w));
    }
    items.push((K::Forces, forces_w));
    // greedy lines on the hypothetical sizes
    let mut lines: Vec<Vec<(K, f32)>> = vec![vec![]];
    let mut used = 0.0;
    for it in items {
        let cur = lines.last_mut().expect("a line");
        let need = if cur.is_empty() {
            it.1
        } else {
            used + 14.0 + it.1
        };
        if need > inner_w && !cur.is_empty() {
            lines.push(vec![it]);
            used = it.1;
        } else {
            cur.push(it);
            used = need;
        }
    }
    // shrink an over-full line (the group, then the hint), grow the hint into free space
    for l in &mut lines {
        let total = |l: &Vec<(K, f32)>| {
            l.iter().map(|i| i.1).sum::<f32>() + 14.0 * (l.len().saturating_sub(1)) as f32
        };
        let over = total(l) - inner_w;
        if over > 0.0 {
            for k in [K::Group, K::Hint] {
                let over = total(l) - inner_w;
                if over <= 0.0 {
                    break;
                }
                if let Some(i) = l.iter_mut().find(|i| i.0 == k) {
                    let min = if k == K::Group { group_min } else { 60.0 };
                    i.1 = (i.1 - over).max(min.min(i.1));
                }
            }
        }
        let free = inner_w - total(l);
        if free > 0.0
            && let Some(h) = l.iter_mut().find(|i| i.0 == K::Hint)
        {
            h.1 += free;
        }
    }
    let hint_w = lines
        .iter()
        .flatten()
        .find(|i| i.0 == K::Hint)
        .map_or(200.0, |i| i.1);
    let hint_galley = wrapped(ui, bar.hint, hint_font, b::HINT_TEXT, hint_w, None);
    let group_w = lines
        .iter()
        .flatten()
        .find(|i| i.0 == K::Group)
        .map_or(0.0, |i| i.1);
    let group_layout = bar.group.as_ref().map(|g| {
        chips_layout(
            ui,
            &g.chips,
            (group_w - cap_w).max(1.0),
            vec2(CHIP_GAP, CHIP_GAP),
        )
    });
    let item_h = |k: K| match k {
        K::Hint => hint_galley.size().y,
        K::Finish => t::size::BUTTON,
        K::Forces => CHIP_H,
        K::Group => group_layout.as_ref().map_or(CHIP_H, |(_, s)| s.y),
    };
    let line_hs: Vec<f32> = lines
        .iter()
        .map(|l| l.iter().map(|i| item_h(i.0)).fold(0.0, f32::max))
        .collect();
    let content_h = line_hs.iter().sum::<f32>() + 8.0 * (lines.len().saturating_sub(1)) as f32;
    let h = (content_h + 16.0 + 2.0).max(52.0);
    let top = ui.cursor().min;
    let rect = Rect::from_min_size(top, vec2(w, h));
    ui.painter().rect(
        rect,
        CornerRadius::same(t::radius::PANEL),
        t::PANEL,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    // every item's cell, then one tiling of all the bar's hit targets (chips on wrapped lines
    // sit 8 px apart)
    let mut cells: Vec<(K, Rect)> = Vec::new();
    let mut y = top.y + (h - content_h) / 2.0;
    for (l, lh) in lines.iter().zip(&line_hs) {
        let mut x = top.x + 13.0;
        for (k, iw) in l {
            let ih = item_h(*k);
            let cy = y + lh / 2.0;
            cells.push((
                *k,
                Rect::from_min_size(pos2(x, cy - ih / 2.0), vec2(*iw, ih)),
            ));
            x += iw + 14.0;
        }
        y += lh + 8.0;
    }
    let cell_of = |k: K| cells.iter().find(|c| c.0 == k).map(|c| c.1);
    let group_rects: Vec<Rect> = match (cell_of(K::Group), &group_layout) {
        (Some(cell), Some((rects, _))) => rects
            .iter()
            .map(|r| r.translate(vec2(cell.left() + cap_w, cell.top())))
            .collect(),
        _ => Vec::new(),
    };
    let mut visuals = group_rects.clone();
    let finish_cell = cell_of(K::Finish);
    let forces_cell = cell_of(K::Forces);
    visuals.extend(finish_cell);
    visuals.extend(forces_cell);
    let targets = crate::widgets::hit::tile(&visuals);
    let n = group_rects.len();
    let finish_target = finish_cell.map(|_| targets[n]);
    let forces_target = targets[n + finish_cell.is_some() as usize];
    let mut clicked = None;
    for (k, cell) in cells.iter().copied() {
        {
            match k {
                K::Hint => {
                    ui.painter()
                        .galley(cell.min, hint_galley.clone(), b::HINT_TEXT);
                    label_node(ui, "builder-hint", cell, bar.hint);
                }
                K::Group => {
                    if let Some(g) = &bar.group {
                        let mut child = ui.new_child(
                            UiBuilder::new()
                                .max_rect(cell)
                                .accessibility_role(Role::Group)
                                .accessibility_label(g.group),
                        );
                        set_bounds(&child, cell);
                        if let Some(cg) = &caption {
                            child.painter().galley(
                                pos2(cell.left(), text_top(cell.top() + CHIP_H / 2.0, cg)),
                                cg.clone(),
                                t::TEXT_MUTED,
                            );
                        }
                        for (i, c) in g.chips.iter().enumerate() {
                            let id = Id::unique(("builder-chip", g.salt, i));
                            if chip_at(
                                &mut child,
                                id,
                                group_rects[i],
                                targets[i],
                                &c.label,
                                &c.name,
                                c.look,
                            )
                            .clicked()
                            {
                                clicked = Some(HintClick::Chip(i));
                            }
                        }
                    }
                }
                K::Finish => {
                    let mut b = ghost("Finish line");
                    if let Some(tg) = finish_target {
                        b = b.hit(tg);
                    }
                    if b.show_at(ui, cell).clicked() {
                        clicked = Some(HintClick::Finish);
                    }
                }
                K::Forces => {
                    let f = &forces[0];
                    let id = Id::unique(("builder-chip", "forces", 0));
                    if chip_at(ui, id, cell, forces_target, &f.label, &f.name, f.look).clicked() {
                        clicked = Some(HintClick::Forces);
                    }
                }
            }
        }
    }
    ui.advance_cursor_after_rect(rect);
    clicked
}
