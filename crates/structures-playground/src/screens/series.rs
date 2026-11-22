//! SCR-6 Series (SR-1) — mockup `docs/design/mockup/Series.dc.html`: "What Structures adds to
//! the series". The intro with the "Educational. Not for real structural design." badge, the
//! five app cards (Structures highlighted), "Shared engines, and who uses them", "Bridges to
//! the other apps", "Netlist directives", "Challenge ladder", "Next layers" and the Provenance
//! row, on the `.paper` ground (`padding: 56px 64px; gap: 28px`).
//!
//! NAV-3: reached from the brand; it has no header, so a `.ghost` "Back" sits in the top
//! padding (Esc does the same, handled by the app). Each sibling card is a button that opens
//! the app through the injectable launcher (bundle id, `/Applications`, the lab-series
//! `dist/` folders) and toasts what happened ("Opening Breadbox…", "Breadbox isn't installed
//! on this Mac", "Bio Playground is planned: it isn't out yet"). The Structures card is this
//! app (a label).
//!
//! At 1440 the sections sit on the mockup's 12-column grid (`gap: 20px`; rows stretch to their
//! tallest item); in narrower windows the cards wrap (at least 200 px each) and the sections
//! stack in the mockup's order, one scroller.

use super::{Badge, Screen};
use crate::ctx::Ctx;
use crate::launcher::{AppId, Launch};
use crate::widgets::badge::{BadgeLook, badge_size, badge_size_in, paint_badge};
use crate::widgets::flow::{FlowItem, flow};
use crate::widgets::text::{css_line_height, galley, galley_spaced, lbl_galley, wrapped_job};
use crate::widgets::{Icon, ghost, hit, interact};
use eframe::egui::{
    Color32, CornerRadius, FontId, Galley, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind,
    Ui, UiBuilder, Vec2, WidgetInfo,
    accesskit::Role,
    epaint::{
        PathShape, PathStroke, RectShape,
        text::{LayoutJob, TextFormat, TextWrapping},
    },
    pos2, vec2,
};
use lab_ui::paint::{advance_block, focus_ring_if};
use sp_theme as t;
use std::sync::Arc;

/// The Series screen's state.
#[derive(Debug, Default)]
pub struct SeriesScreen {}

impl SeriesScreen {
    /// The starting state.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Grid gap of the lower sections (`gap: 20px`).
const GRID_GAP: f32 = 20.0;
/// Gap between the page's rows (`gap: 28px`).
const ROW_GAP: f32 = 28.0;
/// The widest window where the sections stack instead of sitting on the 12-column grid.
const GRID_MIN: f32 = 1000.0;

/// Page padding (y, x): the mockup's `56px 64px`; 16 px sides on phones.
fn padding(width: f32) -> (f32, f32) {
    if width < t::size::NARROW {
        (56.0, 16.0)
    } else {
        (56.0, 64.0)
    }
}

impl Screen for SeriesScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let full = ui.available_rect_before_wrap();
        let (pad_y, pad_x) = padding(full.width());
        let left = full.left() + pad_x;
        let w = (full.width() - 2.0 * pad_x).max(0.0);
        // Back (NAV-3) in the top padding, aligned with the content
        let back = Rect::from_min_size(pos2(left, full.top() + 10.0), vec2(0.0, 36.0));
        let b = ghost("Back").icon(Icon::Back);
        let size = b.desired_size(ui);
        if b.show_at(ui, Rect::from_min_size(back.min, size)).clicked() {
            ctx.back();
        }
        let mut y = full.top() + pad_y;
        y += intro(ui, pos2(left, y), w) + ROW_GAP;
        y += self.cards(ctx, ui, pos2(left, y), w) + ROW_GAP;
        y += lower(ui, pos2(left, y), w);
        let used = Rect::from_min_max(full.min, pos2(full.right(), y + pad_y));
        advance_block(ui, used);
    }

    fn describe(&self) -> String {
        "Series: what Structures adds to the lab series — the five apps, shared engines, \
         bridges to the other apps, netlist directives, the challenge ladder, next layers and \
         provenance."
            .into()
    }
}

// --------------------------------------------------------------------------------------------
// Text helpers

/// One run of rich text.
struct Run<'a>(&'a str, FontId, Color32);

/// Lays out `runs` wrapped at `width` with a CSS line height.
fn rich(ui: &Ui, runs: &[Run], width: f32, line_h: Option<f32>) -> Arc<Galley> {
    let mut job = LayoutJob::default();
    for Run(text, font, color) in runs {
        job.append(
            text,
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: *color,
                line_height: line_h,
                valign: eframe::egui::Align::Center,
                ..Default::default()
            },
        );
    }
    job.wrap = TextWrapping {
        max_width: width.max(1.0),
        ..Default::default()
    };
    ui.fonts_mut(|f| f.layout_job(job))
}

/// A wrapped paragraph.
fn para(ui: &Ui, text: &str, font: FontId, color: Color32, width: f32, lh: f32) -> Arc<Galley> {
    wrapped_job(ui, text, font, color, width, Some(lh), 0.0, false)
}

/// Paints `g` at `pos` and gives it an AccessKit label `name`; returns its height.
fn text_at(
    ui: &Ui,
    g: &Arc<Galley>,
    pos: Pos2,
    name: &str,
    key: impl std::hash::Hash + std::fmt::Debug,
) -> f32 {
    ui.painter().galley(pos, g.clone(), t::TEXT);
    let rect = Rect::from_min_size(pos, g.size());
    let resp = ui.interact(
        rect,
        ui.scope_id().with(("series-text", key)),
        Sense::hover(),
    );
    let name = name.to_string();
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &name));
    g.size().y
}

/// `.h` panel titles: Space Grotesk 600, `letter-spacing: -.01em`.
fn heading(ui: &Ui, text: &str, size: f32) -> Arc<Galley> {
    galley_spaced(
        ui,
        text,
        lab_theme::font::heading(size),
        t::TEXT,
        -0.01 * size,
    )
}

/// A child `Ui` for one section: an AccessKit `Group` named `label` over `rect`.
fn region(ui: &mut Ui, rect: Rect, label: &str) -> Ui {
    let child = ui.new_child(
        UiBuilder::new()
            .id_salt(("series-section", label))
            .max_rect(rect)
            .accessibility_role(Role::Group)
            .accessibility_label(label),
    );
    crate::widgets::set_bounds(&child, rect);
    child
}

/// The `.panel` background for `rect` (filled in after its row is measured).
fn panel_shape(rect: Rect) -> Shape {
    Shape::Rect(RectShape::new(
        rect,
        CornerRadius::same(t::radius::PANEL),
        t::PANEL,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    ))
}

// --------------------------------------------------------------------------------------------
// Intro

const EYEBROW: &str = "Series foundations · app 5 of 5";
const TITLE: &str = "What Structures adds to the series";
const INTRO: &str = "Same shell, same engines, one new accent. A bridge, a tower, the ground \
                     under it and the material it is made of, each with a Playground and an \
                     Advanced form of the same project.";
/// The disclaimer badge (SPEC: "Educational. Not for real structural design.").
pub const DISCLAIMER: &str = "Educational. Not for real structural design.";

/// The intro row (`align-items: flex-end; justify-content: space-between; gap: 24px`): the
/// eyebrow, the 44 px title and the 16 px lede (max 820), with the disclaimer badge at the
/// bottom right. Returns its height.
fn intro(ui: &Ui, at: Pos2, w: f32) -> f32 {
    let badge = Badge::plain(DISCLAIMER);
    let bs = badge_size(ui, &badge);
    let side_by_side = w >= 820.0 + 24.0 + bs.x;
    let text_w = if side_by_side {
        (w - 24.0 - bs.x).min(820.0)
    } else {
        w.min(820.0)
    };
    let eyebrow = lbl_galley(ui, EYEBROW, t::TEXT_MUTED);
    let title_size = if w < 600.0 { 32.0 } else { 44.0 };
    let title = wrapped_job(
        ui,
        TITLE,
        lab_theme::font::brand(title_size),
        t::TEXT,
        text_w.max(w.min(900.0)),
        Some(title_size * 1.05),
        -0.02 * title_size,
        false,
    );
    let lede = para(
        ui,
        INTRO,
        lab_theme::font::body(16.0),
        t::TEXT_CHIP,
        text_w,
        24.0,
    );
    let mut y = at.y;
    let eyebrow_lh = css_line_height(ui, &lab_theme::font::semibold(11.0));
    text_at(ui, &eyebrow, pos2(at.x, y), EYEBROW, "eyebrow");
    y += eyebrow_lh + 8.0;
    y += text_at(ui, &title, pos2(at.x, y), TITLE, "title") + 8.0;
    y += text_at(ui, &lede, pos2(at.x, y), INTRO, "lede");
    let bottom = y;
    let rect = if side_by_side {
        Rect::from_min_size(pos2(at.x + w - bs.x, bottom - bs.y), bs)
    } else {
        y += 12.0;
        Rect::from_min_size(pos2(at.x, y), bs)
    };
    paint_badge(ui, rect, &badge, BadgeLook::DANGER);
    rect.bottom().max(bottom) - at.y
}

// --------------------------------------------------------------------------------------------
// App cards

impl SeriesScreen {
    /// The five app cards (`grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 12px`).
    /// Returns the grid's height.
    fn cards(&mut self, ctx: &mut Ctx, ui: &mut Ui, at: Pos2, w: f32) -> f32 {
        const GAP: f32 = 12.0;
        let n = AppId::ALL.len();
        let cols = (((w + GAP) / (200.0 + GAP)).floor() as usize).clamp(1, n);
        let cw = (w - GAP * (cols as f32 - 1.0)) / cols as f32;
        let inner = cw - 2.0 * 14.0 - 2.0;
        let title_lh = css_line_height(ui, &lab_theme::font::heading(15.0));
        // measure: every card in a grid row is as tall as the row's tallest
        let bodies: Vec<Arc<Galley>> = AppId::ALL
            .iter()
            .map(|a| {
                let color = if a.is_current() {
                    t::SELF_CARD_TEXT
                } else {
                    t::TEXT_MUTED
                };
                para(
                    ui,
                    a.identity().domain,
                    lab_theme::font::body(12.0),
                    color,
                    inner,
                    12.0 * 1.45,
                )
            })
            .collect();
        let card_h = |i: usize| 3.0 + 14.0 + title_lh + 8.0 + bodies[i].size().y + 14.0 + 1.0;
        let mut y = at.y;
        let group = region(ui, Rect::from_min_size(at, vec2(w, 0.0)), "The lab series");
        for row in AppId::ALL.chunks(cols).enumerate() {
            let (r, apps) = row;
            let row_h = (0..apps.len())
                .map(|k| card_h(r * cols + k))
                .fold(0.0, f32::max);
            for (k, app) in apps.iter().enumerate() {
                let i = r * cols + k;
                let rect =
                    Rect::from_min_size(pos2(at.x + k as f32 * (cw + GAP), y), vec2(cw, row_h));
                if let Some(launch) = card(&group, *app, rect, &bodies[i], title_lh) {
                    let result = ctx.launch(launch);
                    ctx.toast(result.message(launch));
                    if let Launch::Failed(e) = &result {
                        log::warn!("open {}: {e}", launch.name());
                    }
                }
            }
            y += row_h + GAP;
        }
        y - GAP - at.y
    }
}

/// One app card: `.panel` with `padding: 14px; gap: 8px; border-top: 3px solid <accent>`, the
/// name (Space Grotesk 600 15 px) over the domain (12 px / 1.45, muted). The Structures card is
/// `#1C1B17` with a `#3A372E` border, ink title and `#D9D3C2` text. A sibling's card is a
/// button ("Breadbox"); returns the app when it was clicked.
fn card(ui: &Ui, app: AppId, rect: Rect, body: &Arc<Galley>, title_lh: f32) -> Option<AppId> {
    let id = app.identity();
    let current = app.is_current();
    let resp = if current {
        let r = ui.interact(rect, ui.scope_id().with(("card", app)), Sense::hover());
        let name = format!("{} (this app)", id.name);
        r.widget_info(|| WidgetInfo::labeled(Role::Label, true, &name));
        r
    } else {
        interact(
            ui,
            ui.scope_id().with(("card", app)),
            hit::target(rect),
            true,
            Role::Button,
            id.name,
            None,
        )
    };
    let hovered = resp.hovered() && !current;
    let (fill, border, title_c) = if current {
        (t::SELF_CARD, t::SELF_CARD_BORDER, t::INK)
    } else if hovered {
        (t::PANEL, t::BORDER_OVERLAY, t::TEXT_STRONG)
    } else {
        (t::PANEL, t::BORDER, t::TEXT)
    };
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(t::radius::PANEL),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    top_border(p, rect, id.accent);
    let x = rect.left() + 15.0;
    let ty = rect.top() + 3.0 + 14.0;
    let g = galley(ui, id.name, lab_theme::font::heading(15.0), title_c);
    p.galley(
        pos2(x, crate::widgets::text::text_top(ty + title_lh / 2.0, &g)),
        g,
        title_c,
    );
    p.galley(pos2(x, ty + title_lh + 8.0), body.clone(), t::TEXT_MUTED);
    focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::PANEL));
    (!current && resp.clicked()).then_some(app)
}

/// The card's `border-top: 3px solid <accent>` on a 12 px radius: a 3 px band along the top
/// edge that follows the corners down to their diagonals (where CSS hands over to the 1 px
/// side borders).
fn top_border(p: &Painter, rect: Rect, color: Color32) {
    let r = t::radius::PANEL as f32;
    let rr = r - 1.5;
    let mut pts = Vec::new();
    let arc = |c: Pos2, from: f32, to: f32, pts: &mut Vec<Pos2>| {
        let n = 8;
        for k in 0..=n {
            let a = from + (to - from) * k as f32 / n as f32;
            pts.push(c + vec2(a.cos(), a.sin()) * rr);
        }
    };
    use std::f32::consts::PI;
    arc(
        pos2(rect.left() + r, rect.top() + r),
        1.25 * PI,
        1.5 * PI,
        &mut pts,
    );
    arc(
        pos2(rect.right() - r, rect.top() + r),
        1.5 * PI,
        1.75 * PI,
        &mut pts,
    );
    p.add(PathShape::line(pts, PathStroke::new(3.0, color)));
}

// --------------------------------------------------------------------------------------------
// The lower sections

/// One section: its accessible name, its grid span (of 12) and how it draws (into a `Ui`, at a
/// point, a width) returning its content height.
type Draw = fn(&Ui, Pos2, f32) -> f32;

struct Section {
    label: &'static str,
    span: usize,
    pad: Vec2,
    draw: Draw,
}

const SECTIONS: [&[Section]; 3] = [
    &[
        Section {
            label: "Shared engines, and who uses them",
            span: 7,
            pad: vec2(20.0, 20.0),
            draw: engines,
        },
        Section {
            label: "Bridges to the other apps",
            span: 5,
            pad: vec2(20.0, 20.0),
            draw: bridges,
        },
    ],
    &[
        Section {
            label: "Netlist directives",
            span: 5,
            pad: vec2(20.0, 20.0),
            draw: directives,
        },
        Section {
            label: "Challenge ladder",
            span: 4,
            pad: vec2(20.0, 20.0),
            draw: ladder,
        },
        Section {
            label: "Next layers",
            span: 3,
            pad: vec2(20.0, 20.0),
            draw: next_layers,
        },
    ],
    &[Section {
        label: "Provenance",
        span: 12,
        pad: vec2(20.0, 16.0),
        draw: provenance,
    }],
];

/// The 12-column grid (or one column when narrow). Returns its height.
fn lower(ui: &mut Ui, at: Pos2, w: f32) -> f32 {
    let grid = w >= GRID_MIN;
    let col = (w - 11.0 * GRID_GAP) / 12.0;
    let mut y = at.y;
    let rows: Vec<Vec<&Section>> = if grid {
        SECTIONS.iter().map(|r| r.iter().collect()).collect()
    } else {
        SECTIONS
            .iter()
            .flat_map(|r| r.iter())
            .map(|s| vec![s])
            .collect()
    };
    for row in rows {
        let mut x = at.x;
        let mut placed = Vec::new();
        let mut row_h: f32 = 0.0;
        for s in row {
            let sw = if grid {
                col * s.span as f32 + GRID_GAP * (s.span as f32 - 1.0)
            } else {
                w
            };
            let bg = ui.painter().add(Shape::Noop);
            let probe = Rect::from_min_size(pos2(x, y), vec2(sw, 4000.0));
            let child = region(ui, probe, s.label);
            let h = (s.draw)(&child, pos2(x + s.pad.x + 1.0, y + s.pad.y + 1.0), {
                sw - 2.0 * s.pad.x - 2.0
            }) + 2.0 * s.pad.y
                + 2.0;
            row_h = row_h.max(h);
            placed.push((bg, child, x, sw));
            x += sw + GRID_GAP;
        }
        for (bg, child, x, sw) in placed {
            let rect = Rect::from_min_size(pos2(x, y), vec2(sw, row_h));
            ui.painter().set(bg, panel_shape(rect));
            crate::widgets::set_bounds(&child, rect);
        }
        y += row_h + GRID_GAP;
    }
    y - GRID_GAP - at.y
}

/// A section title (`h2.h` at 18 px); returns the height used including `gap`.
fn title(ui: &Ui, text: &str, size: f32, at: Pos2, gap: f32) -> f32 {
    let g = heading(ui, text, size);
    let lh = css_line_height(ui, &lab_theme::font::heading(size));
    ui.painter().galley(
        pos2(at.x, crate::widgets::text::text_top(at.y + lh / 2.0, &g)),
        g,
        t::TEXT,
    );
    let resp = ui.interact(
        Rect::from_min_size(at, vec2(10.0, lh)),
        ui.scope_id().with(("h2", text)),
        Sense::hover(),
    );
    let name = text.to_string();
    resp.widget_info(|| WidgetInfo::labeled(Role::Heading, true, &name));
    lh + gap
}

// ---- Shared engines

/// The table's columns: header, its colour.
const ENGINE_COLS: [(&str, Color32); 6] = [
    ("Engine", t::TEXT_MUTED),
    ("Circuit", t::BREADBOX.accent),
    ("Chem", t::CHEM.accent),
    ("Bio", t::BIO.accent),
    ("Weather", t::WEATHER.accent),
    ("Structures", t::ACCENT),
];

/// Rows: engine, the four siblings (uses it?), what Structures does with it.
const ENGINES: [(&str, [bool; 4], &str); 8] = [
    (
        "Playground ⇄ Advanced toggle",
        [true, true, true, true],
        "●",
    ),
    (
        "Netlist parser (.directives)",
        [true, true, true, true],
        "●",
    ),
    (
        "Sparse linear solve A·x = b",
        [true, false, false, true],
        "● new: K·u = F",
    ),
    ("ODE integrator (stiff)", [true; 4], "● sway, quakes"),
    ("Eigen solver", [false; 4], "● new: modes, buckling"),
    (
        "Grid fluid engine",
        [false, false, false, true],
        "● wind tunnel",
    ),
    ("Plot kit + provenance badges", [true; 4], "●"),
    ("Challenge framework", [true; 4], "● budgets, scores"),
];

/// What stands in for "⇄" in the text (an em space: its width at 13 px).
const SWAP_GAP: &str = "\u{2003}";

/// "⇄" at 13 px: a right arrow over a left arrow, centred on `c`.
fn swap_arrows(p: &Painter, c: Pos2, color: Color32) {
    let s = Stroke::new(1.1, color);
    let (hw, dy, head) = (5.0, 2.2, 2.2);
    let top = c.y - dy;
    let bot = c.y + dy;
    p.line_segment([pos2(c.x - hw, top), pos2(c.x + hw, top)], s);
    p.line_segment([pos2(c.x + hw - head, top - head), pos2(c.x + hw, top)], s);
    p.line_segment([pos2(c.x - hw, bot), pos2(c.x + hw, bot)], s);
    p.line_segment([pos2(c.x - hw + head, bot + head), pos2(c.x - hw, bot)], s);
}

const ENGINES_NOTE: &str = "● uses it · ○ not yet. The stiffness method is nodal analysis \
                            with springs instead of resistors, so Breadbox’s DC solver and the \
                            truss solver can be one module.";

/// The table row `r` as one AccessKit `Row` label ("Eigen solver: Circuit not yet, …;
/// Structures new: modes, buckling").
fn row_label(ui: &Ui, row: Rect, r: usize) {
    let (name, uses, ours) = ENGINES[r];
    let label = format!(
        "{name}: {}; Structures {}",
        ENGINE_COLS[1..5]
            .iter()
            .zip(uses)
            .map(|((h, _), u)| format!("{h} {}", if u { "uses it" } else { "not yet" }))
            .collect::<Vec<_>>()
            .join(", "),
        if ours == "●" {
            "uses it".to_string()
        } else {
            ours.trim_start_matches("● ").to_string()
        }
    );
    ui.interact(row, ui.scope_id().with(("engine-row", r)), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Row, true, &label));
}

/// Below this width the table stacks each row: the engine's name, then its ● / ○ and what
/// Structures does with it.
const TABLE_STACK: f32 = 560.0;

fn engines(ui: &Ui, at: Pos2, w: f32) -> f32 {
    let mut y = at.y + title(ui, "Shared engines, and who uses them", 18.0, at, 14.0);
    y += if w < TABLE_STACK {
        table_stacked(ui, pos2(at.x, y), w)
    } else {
        table(ui, pos2(at.x, y), w)
    };
    y += 14.0;
    let note = para(
        ui,
        ENGINES_NOTE,
        lab_theme::font::body(12.0),
        t::TEXT_MUTED,
        w,
        18.0,
    );
    y += text_at(ui, &note, pos2(at.x, y), ENGINES_NOTE, "engines-note");
    y - at.y
}

/// The table as the mockup draws it (auto table layout). Returns its height.
fn table(ui: &Ui, at: Pos2, w: f32) -> f32 {
    let mut y = at.y;
    let body = lab_theme::font::body(13.0);
    let mono = lab_theme::font::mono(13.0);
    let head = lab_theme::font::semibold(13.0);
    // auto table layout: max-content widths (+ padding), the slack shared in proportion
    let pad = |c: usize| if c == 0 { 8.0 } else { 6.0 };
    let mut want: Vec<f32> = ENGINE_COLS
        .iter()
        .enumerate()
        .map(|(c, (h, _))| galley(ui, *h, head.clone(), t::TEXT).size().x + 2.0 * pad(c))
        .collect();
    for (name, _, ours) in ENGINES {
        let shown = name.replace('⇄', SWAP_GAP);
        want[0] = want[0].max(galley(ui, &shown, body.clone(), t::TEXT).size().x + 16.0);
        want[5] = want[5].max(galley(ui, ours, mono.clone(), t::TEXT).size().x + 2.0);
    }
    let total: f32 = want.iter().sum();
    let widths: Vec<f32> = if total <= w {
        want.iter().map(|x| x + (w - total) * x / total).collect()
    } else {
        // too narrow: the four ● columns keep their width, the others share the rest
        let fixed: f32 = want[1..5].iter().sum();
        let rest = (w - fixed).max(0.0);
        let flex = want[0] + want[5];
        want.iter()
            .enumerate()
            .map(|(c, x)| {
                if c == 0 || c == 5 {
                    rest * x / flex
                } else {
                    *x
                }
            })
            .collect()
    };
    let xs: Vec<f32> = widths
        .iter()
        .scan(at.x, |x, w| {
            let l = *x;
            *x += w;
            Some(l)
        })
        .collect();
    let p = ui.painter();
    // header row: `padding: 6px`
    let head_lh = css_line_height(ui, &head);
    for (c, (h, color)) in ENGINE_COLS.iter().enumerate() {
        let g = galley(ui, *h, head.clone(), *color);
        let x = if c == 0 {
            xs[c] + 8.0
        } else {
            xs[c] + (widths[c] - g.size().x) / 2.0
        };
        p.galley(
            pos2(
                x,
                crate::widgets::text::text_top(y + 6.0 + head_lh / 2.0, &g),
            ),
            g,
            *color,
        );
    }
    let head_resp = ui.interact(
        Rect::from_min_size(pos2(at.x, y), vec2(w, head_lh + 12.0)),
        ui.scope_id().with("engine-head"),
        Sense::hover(),
    );
    let head_name = ENGINE_COLS.map(|c| c.0).join(" · ");
    head_resp.widget_info(|| WidgetInfo::labeled(Role::Row, true, &head_name));
    y += head_lh + 12.0;
    let body_lh = css_line_height(ui, &body);
    for (r, (name, uses, ours)) in ENGINES.iter().enumerate() {
        // the bundled fonts have no "⇄": it is drawn (two arrows) in a gap of its width
        let shown = name.replace('⇄', SWAP_GAP);
        let ng = para(ui, &shown, body.clone(), t::TEXT, widths[0] - 16.0, body_lh);
        let og = para(ui, ours, mono.clone(), t::ACCENT, widths[5] - 2.0, body_lh);
        // the mockup's "⇄" comes from a fallback font with a taller line: that row is 3 px
        // taller
        let tall = if name.contains('⇄') { 3.0 } else { 0.0 };
        let row_h = (ng.size().y + 16.0 + tall).max(og.size().y + 2.0) + 1.0;
        p.hline(at.x..=at.x + w, y + 0.5, Stroke::new(1.0, t::RULE));
        let cy = y + 1.0 + (row_h - 1.0) / 2.0;
        let name_pos = pos2(xs[0] + 8.0, cy - ng.size().y / 2.0);
        if let Some(i) = name.find('⇄') {
            let before = galley(ui, &name[..i], body.clone(), t::TEXT).size().x;
            let gap = galley(ui, SWAP_GAP, body.clone(), t::TEXT).size().x;
            let c = pos2(name_pos.x + before + gap / 2.0, name_pos.y + body_lh / 2.0);
            swap_arrows(p, c, t::TEXT);
        }
        p.galley(name_pos, ng, t::TEXT);
        for (k, used) in uses.iter().enumerate() {
            let (dot, color) = if *used {
                ("●", t::TEXT)
            } else {
                ("○", t::TEXT_MUTED)
            };
            let g = galley(ui, dot, mono.clone(), color);
            let c = k + 1;
            p.galley(
                pos2(
                    xs[c] + (widths[c] - g.size().x) / 2.0,
                    crate::widgets::text::text_top(cy, &g),
                ),
                g,
                color,
            );
        }
        p.galley(
            pos2(
                xs[5] + (widths[5] - og.size().x) / 2.0,
                cy - og.size().y / 2.0,
            ),
            og,
            t::ACCENT,
        );
        let row = Rect::from_min_size(pos2(at.x, y), vec2(w, row_h));
        row_label(ui, row, r);
        y += row_h;
    }
    y - at.y
}

/// The table for narrow windows: a header of the five app columns, then per engine its name
/// over a line of the four siblings' ● / ○ and Structures' note. Returns its height.
fn table_stacked(ui: &Ui, at: Pos2, w: f32) -> f32 {
    const DOT_COL: f32 = 60.0;
    let body = lab_theme::font::body(13.0);
    let mono = lab_theme::font::mono(13.0);
    let head = lab_theme::font::semibold(13.0);
    let body_lh = css_line_height(ui, &body);
    let head_lh = css_line_height(ui, &head);
    let ours_x = at.x + 4.0 * DOT_COL;
    let ours_w = (w - 4.0 * DOT_COL).max(40.0);
    let p = ui.painter();
    let mut y = at.y;
    for (c, (h, color)) in ENGINE_COLS.iter().enumerate().skip(1) {
        let g = galley(ui, *h, head.clone(), *color);
        let x = if c < 5 {
            at.x + (c - 1) as f32 * DOT_COL + (DOT_COL - g.size().x) / 2.0
        } else {
            ours_x + 6.0
        };
        p.galley(
            pos2(
                x,
                crate::widgets::text::text_top(y + 6.0 + head_lh / 2.0, &g),
            ),
            g,
            *color,
        );
    }
    y += head_lh + 12.0;
    for (r, (name, uses, ours)) in ENGINES.iter().enumerate() {
        let shown = name.replace('⇄', SWAP_GAP);
        let ng = para(ui, &shown, body.clone(), t::TEXT, w - 16.0, body_lh);
        let og = para(ui, ours, mono.clone(), t::ACCENT, ours_w - 12.0, body_lh);
        let top = y;
        p.hline(at.x..=at.x + w, y + 0.5, Stroke::new(1.0, t::RULE));
        y += 1.0 + 8.0;
        let name_pos = pos2(at.x + 8.0, y);
        if let Some(i) = name.find('⇄') {
            let before = galley(ui, &name[..i], body.clone(), t::TEXT).size().x;
            let gap = galley(ui, SWAP_GAP, body.clone(), t::TEXT).size().x;
            swap_arrows(
                p,
                pos2(name_pos.x + before + gap / 2.0, y + body_lh / 2.0),
                t::TEXT,
            );
        }
        y += ng.size().y + 4.0;
        p.galley(name_pos, ng, t::TEXT);
        let cy = y + body_lh / 2.0;
        for (k, used) in uses.iter().enumerate() {
            let (dot, color) = if *used {
                ("●", t::TEXT)
            } else {
                ("○", t::TEXT_MUTED)
            };
            let g = galley(ui, dot, mono.clone(), color);
            p.galley(
                pos2(
                    at.x + k as f32 * DOT_COL + (DOT_COL - g.size().x) / 2.0,
                    crate::widgets::text::text_top(cy, &g),
                ),
                g,
                color,
            );
        }
        let oh = og.size().y;
        p.galley(pos2(ours_x + 6.0, y), og, t::ACCENT);
        y += oh.max(body_lh) + 8.0;
        row_label(
            ui,
            Rect::from_min_max(pos2(at.x, top), pos2(at.x + w, y)),
            r,
        );
    }
    y - at.y
}

// ---- Bridges to the other apps

fn bridges(ui: &Ui, at: Pos2, w: f32) -> f32 {
    let mut y = at.y + title(ui, "Bridges to the other apps", 18.0, at, 12.0);
    let inner = w - 2.0 * 14.0 - 2.0;
    let mono = lab_theme::font::mono(13.0);
    let small = lab_theme::font::body(12.0);
    let head = lab_theme::font::semibold(13.0);
    let wells: [(&str, Color32, Vec<Vec<Run>>); 4] = [
        (
            "Breadbox · vibration is an RLC circuit",
            t::BREADBOX.accent,
            vec![
                vec![Run(
                    "m·x\u{308} + c·x\u{307} + k·x = F(t)",
                    mono.clone(),
                    t::TEXT,
                )],
                vec![Run(
                    "L·q\u{308} + R·q\u{307} + q/C = V(t)",
                    mono.clone(),
                    t::TEXT_CHIP,
                )],
                vec![Run(
                    "Open a tower’s sway as a circuit. A tuned mass damper is a notch filter.",
                    small.clone(),
                    t::TEXT_MUTED,
                )],
            ],
        ),
        (
            "Weather · hurricane in, wind load out",
            t::WEATHER.accent,
            vec![vec![
                Run(
                    "Export a storm from the grid sandbox as a ",
                    small.clone(),
                    t::TEXT_MUTED,
                ),
                Run(".wind", lab_theme::font::mono(12.0), t::TEXT_MUTED),
                Run(
                    " profile and point it at any tower.",
                    small.clone(),
                    t::TEXT_MUTED,
                ),
            ]],
        ),
        (
            "Chem · diffusion in metals",
            t::CHEM.accent,
            vec![vec![Run(
                "Carbon diffusing into steel (case hardening) reuses the chem diffusion kernel.",
                small.clone(),
                t::TEXT_MUTED,
            )]],
        ),
        (
            "Automata explorer · grain growth",
            t::TEXT_DIM,
            vec![vec![Run(
                "Metal grains coarsening under heat is a cellular automaton on the shared grid.",
                small.clone(),
                t::TEXT_MUTED,
            )]],
        ),
    ];
    for (i, (name, color, lines)) in wells.iter().enumerate() {
        let gap = if i == 0 { 6.0 } else { 4.0 };
        let hg = para(
            ui,
            name,
            head.clone(),
            *color,
            inner,
            css_line_height(ui, &head),
        );
        let gs: Vec<Arc<Galley>> = lines
            .iter()
            .map(|runs| {
                let lh = runs
                    .iter()
                    .map(|r| css_line_height(ui, &r.1))
                    .fold(0.0, f32::max);
                rich(ui, runs, inner, Some(lh))
            })
            .collect();
        let content = hg.size().y + gs.iter().map(|g| gap + g.size().y).sum::<f32>();
        let rect = Rect::from_min_size(pos2(at.x, y), vec2(w, content + 24.0 + 2.0));
        ui.painter().rect(
            rect,
            CornerRadius::same(t::radius::WELL),
            t::WELL,
            Stroke::new(1.0, t::BORDER),
            StrokeKind::Inside,
        );
        let x = rect.left() + 15.0;
        let mut ty = rect.top() + 13.0;
        ty += text_at(ui, &hg, pos2(x, ty), name, ("bridge", i));
        for (k, g) in gs.iter().enumerate() {
            ty += gap;
            let text: String = lines[k].iter().map(|r| r.0).collect();
            ty += text_at(ui, g, pos2(x, ty), &text, ("bridge", i, k));
        }
        y = rect.bottom() + 12.0;
    }
    y - 12.0 - at.y
}

// ---- Netlist directives

const DIRECTIVES: [(&str, &str); 13] = [
    (".node", "name x y [fix=xy|y]"),
    (".member", "id a b section"),
    (".material", "name E fy rho [curve=…]"),
    (".section", "name A I"),
    (".load", "case nodes Fx Fy | moving lane"),
    (".static", "linear | nonlinear"),
    (".buckle", "euler K | eigen n"),
    (".story", "n height mass system"),
    (".wind", "V10 alpha | file=storm.wx"),
    (".soil", "layer depth type"),
    (".foundation", "footing | mat | piles L n | caisson"),
    (".dynamic", "quake|gust t_end dt damping"),
    (".plot", "expr vs var range"),
];

fn directives(ui: &Ui, at: Pos2, w: f32) -> f32 {
    let mut y = at.y + title(ui, "Netlist directives", 18.0, at, 10.0);
    let mono = lab_theme::font::mono(12.0);
    let lh = 12.0 * 1.4;
    for (i, (d, args)) in DIRECTIVES.iter().enumerate() {
        let dg = para(ui, d, mono.clone(), t::ACCENT, 110.0, lh);
        let ag = para(
            ui,
            args,
            mono.clone(),
            t::TEXT_CHIP,
            (w - 122.0).max(1.0),
            lh,
        );
        let h = dg.size().y.max(ag.size().y);
        ui.painter().galley(pos2(at.x, y), dg, t::ACCENT);
        ui.painter().galley(pos2(at.x + 122.0, y), ag, t::TEXT_CHIP);
        let label = format!("{d} {args}");
        ui.interact(
            Rect::from_min_size(pos2(at.x, y), vec2(w, h)),
            ui.scope_id().with(("directive", i)),
            Sense::hover(),
        )
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, &label));
        y += h + 7.0;
    }
    y - 7.0 - at.y
}

// ---- Challenge ladder

/// The ladder: (highlighted lead, its colour, the rest).
const LADDER: [(&str, Option<Color32>, &str); 12] = [
    ("River crossing", Some(t::INK), ": 20 t, under $30k"),
    ("", None, "Overhang: hold 5 t out over the water"),
    ("", None, "Heavy haul: two 25 t loads, piers allowed"),
    ("", None, "Snap test: survive losing any one member"),
    (
        "",
        None,
        "Lightest bridge duel (vs a friend or the optimizer)",
    ),
    ("", None, "Material roulette"),
    ("", None, "Shake table: find the resonance"),
    ("", None, "100 floors, under budget"),
    ("", None, "Tame the sway: tune the mass damper"),
    ("", None, "The lean of Pisa, and the fix"),
    ("", None, "Liquefaction: keep it upright"),
    (
        "Boss",
        Some(t::DANGER_INK),
        ": quake + hurricane on soft clay",
    ),
];

/// A list (`ol` / `ul`): items 13 px / 1.4 `#D9D9D9`, `padding-left`, `gap`, markers outside.
fn list(
    ui: &Ui,
    at: Pos2,
    w: f32,
    items: &[Vec<Run>],
    indent: f32,
    gap: f32,
    marker: impl Fn(usize) -> String,
) -> f32 {
    let body = lab_theme::font::body(13.0);
    let lh = 13.0 * 1.4;
    let mut y = at.y;
    for (i, runs) in items.iter().enumerate() {
        let g = rich(ui, runs, w - indent, Some(lh));
        let m = galley(ui, marker(i), body.clone(), t::TEXT_DIM);
        let space = galley(ui, " ", body.clone(), t::TEXT_DIM).size().x;
        ui.painter().galley(
            pos2(
                at.x + indent - space - m.size().x,
                y + (lh - m.size().y) / 2.0,
            ),
            m,
            t::TEXT_DIM,
        );
        let text: String = runs.iter().map(|r| r.0).collect();
        y += text_at(
            ui,
            &g,
            pos2(at.x + indent, y),
            &text,
            ("li", indent as i32, i),
        ) + gap;
    }
    y - gap - at.y
}

fn ladder(ui: &Ui, at: Pos2, w: f32) -> f32 {
    let y = at.y + title(ui, "Challenge ladder", 18.0, at, 10.0);
    let body = lab_theme::font::body(13.0);
    let items: Vec<Vec<Run>> = LADDER
        .iter()
        .map(|(lead, color, rest)| {
            let mut runs = Vec::new();
            if let Some(c) = color {
                runs.push(Run(lead, body.clone(), *c));
            }
            runs.push(Run(rest, body.clone(), t::TEXT_DIM));
            runs
        })
        .collect();
    let h = list(ui, pos2(at.x, y), w, &items, 22.0, 5.0, |i| {
        format!("{}.", i + 1)
    });
    y + h - at.y
}

// ---- Next layers

const NEXT: [&str; 7] = [
    "2D/3D finite element sandbox with stress heat maps",
    "Topology optimization: watch material melt away",
    "Rigid joints and beams, not just pin-jointed trusses",
    "Fatigue, creep, temperature",
    "Phase diagrams, alloys, grain growth",
    "Scour around bridge piers",
    "Vortex shedding in the wind tunnel",
];

fn next_layers(ui: &Ui, at: Pos2, w: f32) -> f32 {
    let y = at.y + title(ui, "Next layers", 18.0, at, 10.0);
    let body = lab_theme::font::body(13.0);
    let items: Vec<Vec<Run>> = NEXT
        .iter()
        .map(|s| vec![Run(s, body.clone(), t::TEXT_DIM)])
        .collect();
    let h = list(ui, pos2(at.x, y), w, &items, 18.0, 6.0, |_| "•".into());
    y + h - at.y
}

// ---- Provenance

const PROVENANCE_NOTE: &str = "Data: Materials Project is open (credit it); MatWeb’s terms are \
                               restrictive, so don’t bundle it. Check every source’s license \
                               first.";

/// The provenance badges. (The mockup's "exact" dot here is `#4BE3A0`.)
fn provenance_badges() -> [Badge; 3] {
    [
        Badge::dot(
            "Exact for the model: truss statics, Euler buckling",
            t::BIO.accent,
        ),
        Badge::simplified(
            "Simplified: wind profile, sway, soil bearing and settlement, collapse physics",
        ),
        Badge::illustrative("Illustrative: soil presets, comfort bands, game costs"),
    ]
}

/// The Provenance row: `display: flex; flex-wrap: wrap; align-items: center; gap: 10px 24px`
/// with the `.h` title (15 px), the badges and the data note.
fn provenance(ui: &Ui, at: Pos2, w: f32) -> f32 {
    let head = heading(ui, "Provenance", 15.0);
    let head_lh = css_line_height(ui, &lab_theme::font::heading(15.0));
    let badges = provenance_badges();
    let note = galley(
        ui,
        PROVENANCE_NOTE,
        lab_theme::font::body(12.0),
        t::TEXT_MUTED,
    );
    let note_w = note.size().x;
    let note_g = if note_w > w {
        para(
            ui,
            PROVENANCE_NOTE,
            lab_theme::font::body(12.0),
            t::TEXT_MUTED,
            w,
            css_line_height(ui, &lab_theme::font::body(12.0)),
        )
    } else {
        note
    };
    let mut items = vec![FlowItem::fixed(vec2(head.size().x, head_lh))];
    items.extend(
        badges
            .iter()
            .map(|b| FlowItem::fixed(badge_size_in(ui, b, w))),
    );
    items.push(FlowItem::fixed(note_g.size()));
    let lay = flow(&items, w, vec2(24.0, 10.0), 0.0);
    let o = at.to_vec2();
    let hr = lay.rects[0].translate(o);
    ui.painter().galley(
        pos2(
            hr.left(),
            crate::widgets::text::text_top(hr.center().y, &head),
        ),
        head,
        t::TEXT,
    );
    ui.interact(hr, ui.scope_id().with("prov-h"), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Heading, true, "Provenance"));
    for (i, b) in badges.iter().enumerate() {
        paint_badge(ui, lay.rects[i + 1].translate(o), b, BadgeLook::default());
    }
    let nr = lay.rects[items.len() - 1].translate(o);
    text_at(ui, &note_g, nr.min, PROVENANCE_NOTE, "prov-note");
    lay.height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_matches_the_mockup() {
        assert_eq!(ENGINES.len(), 8);
        assert_eq!(
            ENGINE_COLS.map(|c| c.0),
            ["Engine", "Circuit", "Chem", "Bio", "Weather", "Structures"]
        );
        // the eigen solver is new in Structures; the grid fluid engine is Weather's
        assert_eq!(ENGINES[4].1, [false; 4]);
        assert_eq!(ENGINES[5].1, [false, false, false, true]);
        assert_eq!(DIRECTIVES.len(), 13);
        assert_eq!(LADDER.len(), 12);
        assert_eq!(NEXT.len(), 7);
    }

    #[test]
    fn padding_follows_the_width() {
        assert_eq!(padding(1440.0), (56.0, 64.0));
        assert_eq!(padding(390.0), (56.0, 16.0));
    }
}
