//! The netlist editor (ADV-2): the mockup's line list made editable — JetBrains Mono 12 px on
//! 21 px lines (`line-height: 1.75`), `padding: 10px 0`, each line `padding: 0 14px` with its
//! number (`#4A4A4A`, `min-width: 20px`, right-aligned, `gap: 12px`), `white-space: pre`,
//! `max-height: 820px; overflow: auto`. Directives in the accent, `*` lines and `; …` tails in
//! `#7E8A7E`, arguments `#D9D9D9`.
//!
//! Errors sit on their line: a red band, the offending text underlined, the line number in red
//! and the message in a chip at the right of the view. Warnings are underlined in the series'
//! warning colour. AccessKit: a multi-line text input named "Netlist" whose description lists
//! the problems.

use super::controls::panel_header;
use super::doc::{Doc, Origin};
use crate::Badge;
use crate::widgets::badge::{BadgeLook, badge_size, paint_badge};
use crate::widgets::text::{css_rows, galley, text_top, wrapped};
use crate::widgets::{Icon, ghost};
use eframe::egui::{
    Align, CornerRadius, Frame, Galley, Margin, Rect, ScrollArea, Sense, Shape, Stroke, TextBuffer,
    TextEdit, Ui, WidgetInfo, accesskit::Role, epaint::text::LayoutJob, epaint::text::TextFormat,
    pos2, vec2,
};
use lab_theme::font;
use sp_engine::netlist::{Diagnostic, Severity};
use sp_theme::{self as t, advanced as a};
use std::sync::Arc;

/// The editor's accessible name.
pub const NAME: &str = "Netlist";
/// Line height: 12 px × 1.75.
pub const LINE_H: f32 = 21.0;
/// `max-height: 820px` of the line list (padding included).
pub const MAX_H: f32 = 820.0;
/// From the panel's left edge to the text: `padding-left 14 + number 20 + gap 12`.
const TEXT_X: f32 = 46.0;
/// The line list's `padding-top: 10px`, less the pixel Chrome's baseline in a 21 px mono line
/// box sits above egui's (measured against the reference).
const PAD_TOP: f32 = 9.0;

/// What the editor did this frame.
#[derive(Debug, Default)]
pub struct EditorOutput {
    /// The new text, when the user changed it.
    pub changed: Option<String>,
    /// "Open in Playground" was pressed.
    pub open_in_playground: bool,
}

/// The header badge: "example: 48 m truss", "from the Playground", "edited", "2 errors".
pub fn badge_for(doc: &Doc) -> (Badge, BadgeLook) {
    let n = doc.errors();
    if n > 0 {
        let s = format!("{n} error{}", if n == 1 { "" } else { "s" });
        return (Badge::plain(s), BadgeLook::DANGER);
    }
    let text = match doc.origin {
        Origin::Generated => "example: 48 m truss".to_string(),
        Origin::Received(r) => format!("from the {}", r.title()),
        Origin::File => "opened file".into(),
        Origin::Pasted => "pasted".into(),
        Origin::Edited => "edited".into(),
    };
    (Badge::plain(text), BadgeLook::default())
}

fn fmt(color: eframe::egui::Color32) -> TextFormat {
    TextFormat {
        font_id: font::mono(12.0),
        color,
        line_height: Some(LINE_H),
        valign: Align::Center,
        ..Default::default()
    }
}

/// Byte offset of 1-based char column `col` in `line` (clamped to the end).
fn byte_at(line: &str, col: usize) -> usize {
    line.char_indices()
        .nth(col.saturating_sub(1))
        .map(|(b, _)| b)
        .unwrap_or(line.len())
}

/// The coloured layout of one line (with its newline, if any). `title`: the file's first line.
///
/// As the mockup renders its lines (`dir` in `l.dc`, then `args` in `#D9D9D9`): the title
/// line `* river-crossing.str` is all comment colour; another `*` line has its `*` in the
/// comment colour and its text as arguments (the "48 m Pratt …" description).
fn append_line(job: &mut LayoutJob, line: &str, marks: &[&Diagnostic], title: bool) {
    // colour spans: (start, end, colour)
    let body = line.trim_end_matches(['\n', '\r']);
    let mut spans: Vec<(usize, usize, eframe::egui::Color32)> = Vec::new();
    let lead = body.len() - body.trim_start().len();
    if body[lead..].starts_with('*') {
        let star_end = if title || !body[lead..].starts_with("* ") {
            body.len()
        } else {
            lead + 1
        };
        spans.push((0, star_end, a::COMMENT));
        spans.push((star_end, body.len(), a::ARGS));
    } else {
        let code_end = body.find(';').unwrap_or(body.len());
        let code = &body[..code_end];
        let dir_end = code[lead..]
            .find(char::is_whitespace)
            .map(|p| lead + p)
            .unwrap_or(code.len());
        let dir_color = if code[lead..].starts_with('.') {
            a::DIRECTIVE
        } else {
            a::ARGS
        };
        spans.push((0, lead, a::ARGS));
        spans.push((lead, dir_end, dir_color));
        spans.push((dir_end, code_end, a::ARGS));
        spans.push((code_end, body.len(), a::COMMENT));
    }
    // underlines from the diagnostics, split into the colour spans
    let underline: Vec<(usize, usize, Stroke)> = marks
        .iter()
        .map(|d| {
            let s = byte_at(body, d.column);
            let e = byte_at(body, d.column + d.len.max(1)).max(s);
            let c = if d.is_error() { a::ERROR } else { a::WARNING };
            (s, if e == s { body.len() } else { e }, Stroke::new(1.0, c))
        })
        .collect();
    let mut cuts: Vec<usize> = vec![0, body.len()];
    for (s, e, _) in &spans {
        cuts.push(*s);
        cuts.push(*e);
    }
    for (s, e, _) in &underline {
        cuts.push(*s);
        cuts.push(*e);
    }
    cuts.retain(|c| *c <= body.len() && body.is_char_boundary(*c));
    cuts.sort_unstable();
    cuts.dedup();
    for w in cuts.windows(2) {
        let (s, e) = (w[0], w[1]);
        if s >= e {
            continue;
        }
        let color = spans
            .iter()
            .find(|(a, b, _)| *a <= s && e <= *b)
            .map(|x| x.2)
            .unwrap_or(a::ARGS);
        let mut f = fmt(color);
        if let Some((_, _, st)) = underline.iter().find(|(a, b, _)| *a <= s && e <= *b) {
            f.underline = *st;
        }
        job.append(&body[s..e], 0.0, f);
    }
    let nl = &line[body.len()..];
    if !nl.is_empty() {
        job.append(nl, 0.0, fmt(a::ARGS));
    }
}

/// The layout job for the whole text.
fn job(text: &str, diags: &[Diagnostic]) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (i, line) in text.split_inclusive('\n').enumerate() {
        let marks: Vec<&Diagnostic> = diags.iter().filter(|d| d.line == i + 1).collect();
        append_line(&mut job, line, &marks, i == 0);
    }
    if text.is_empty() {
        job.append("", 0.0, fmt(a::ARGS));
    }
    job.wrap.max_width = f32::INFINITY;
    job
}

/// "2 errors. Line 10: expected a number, got oops; …" for the accessible description.
pub fn summary(diags: &[Diagnostic]) -> Option<String> {
    let errors: Vec<&Diagnostic> = diags.iter().filter(|d| d.is_error()).collect();
    if errors.is_empty() {
        return None;
    }
    let n = errors.len();
    let list: Vec<String> = errors
        .iter()
        .map(|d| format!("Line {}: {}", d.line, d.message))
        .collect();
    Some(format!(
        "{n} error{}. {}",
        if n == 1 { "" } else { "s" },
        list.join("; ")
    ))
}

/// The Netlist panel's content: header, editor, footer. `ui` is inside the panel.
pub fn netlist_panel(ui: &mut Ui, doc: &Doc) -> EditorOutput {
    let mut out = EditorOutput::default();
    let (b, look) = badge_for(doc);
    let bsize = badge_size(ui, &b);
    panel_header(ui, "Netlist", bsize, |ui, r| {
        paint_badge(ui, r, &b, look);
    });

    let mut buf = doc.text.clone();
    let diags: Vec<Diagnostic> = doc.diagnostics.clone();
    let width = ui.available_width();
    let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, _wrap: f32| -> Arc<Galley> {
        css_rows(
            ui.fonts_mut(|f| f.layout_job(job(buf.as_str(), &diags))),
            LINE_H,
        )
    };
    let text_lines = doc.text.lines().count().max(1);
    let scroll = ScrollArea::both()
        .id_salt("netlist-scroll")
        .max_height(MAX_H)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let bands = ui.painter().add(Shape::Noop);
            ui.add_space(PAD_TOP);
            let r = ui.horizontal_top(|ui| {
                ui.add_space(TEXT_X);
                TextEdit::multiline(&mut buf)
                    .id_salt("netlist-text")
                    .frame(Frame::NONE)
                    .margin(Margin::ZERO)
                    .desired_width((width - TEXT_X - 14.0).max(40.0))
                    .desired_rows(1)
                    .code_editor()
                    .lock_focus(false)
                    .layouter(&mut layouter)
                    .show(ui)
            });
            ui.add_space(20.0 - PAD_TOP);
            (bands, r.inner)
        });
    let (bands, edit) = scroll.inner;
    let view = scroll.inner_rect;
    if edit.response.changed() {
        out.changed = Some(buf.clone());
    }

    // gutter numbers, error bands and message chips
    let painter = ui.painter().with_clip_rect(view);
    let mut under = Vec::new();
    let gutter_right = view.left() + TEXT_X - 12.0 - scroll.state.clamped_offset().x;
    for (i, row) in edit.galley.rows.iter().enumerate() {
        let line = i + 1;
        if line > text_lines && row.row.glyphs.is_empty() && line > 1 {
            continue; // the empty row after a trailing newline
        }
        let y = edit.galley_pos.y + row.rect().top();
        let h = row.rect().height();
        let has_err = diags.iter().any(|d| d.line == line && d.is_error());
        let num_c = if has_err { a::ERROR } else { a::GUTTER };
        let g = galley(ui, line.to_string(), font::mono(12.0), num_c);
        painter.galley(
            pos2(gutter_right - g.size().x, text_top(y + h / 2.0, &g)),
            g,
            num_c,
        );
        if has_err {
            under.push(Shape::rect_filled(
                Rect::from_min_max(pos2(view.left(), y), pos2(view.right(), y + h)),
                CornerRadius::ZERO,
                a::ERROR_BAND,
            ));
            let msg: Vec<&str> = diags
                .iter()
                .filter(|d| d.line == line && d.is_error())
                .map(|d| d.message.as_str())
                .collect();
            let text = msg.join(" · ");
            let mg = galley(ui, &text, font::body(11.0), a::ERROR);
            let w = (mg.size().x + 12.0).min(view.width() - 60.0).max(20.0);
            let chip = Rect::from_min_size(
                pos2(view.right() - 8.0 - w, y + 2.0),
                vec2(w, (h - 4.0).max(10.0)),
            );
            painter.rect(
                chip,
                CornerRadius::same(4),
                a::ERROR_CHIP,
                Stroke::new(1.0, t::DANGER_EDGE),
                eframe::egui::StrokeKind::Inside,
            );
            let mg = if mg.size().x > w - 12.0 {
                crate::widgets::text::ellipsized(ui, &text, font::body(11.0), a::ERROR, w - 12.0)
            } else {
                mg
            };
            painter.galley(
                pos2(chip.left() + 6.0, text_top(chip.center().y, &mg)),
                mg,
                a::ERROR,
            );
            let label = format!("Line {line}: {text}");
            ui.interact(
                chip,
                ui.scope_id().with(("netlist-error", line)),
                Sense::hover(),
            )
            .widget_info(|| WidgetInfo::labeled(Role::Label, true, &label));
        }
    }
    ui.painter().set(bands, Shape::Vec(under));

    let summary = summary(&diags);
    let id = edit.response.id;
    ui.ctx().accesskit_node_builder(id, |b| {
        b.set_label(NAME);
        match &summary {
            Some(s) => b.set_description(s.clone()),
            None => b.clear_description(),
        }
    });

    // footer: the state of the results and "Open in Playground"
    let top = ui.cursor().top();
    ui.painter().hline(
        ui.max_rect().x_range(),
        top + 0.5,
        Stroke::new(1.0, t::BORDER),
    );
    ui.add_space(10.0);
    let inner = ui.available_width() - 28.0;
    let note = if doc.errors() > 0 {
        "The last good result stays on screen, dimmed, until the errors are fixed."
    } else if doc.warnings() > 0 {
        "Parsed with warnings (underlined)."
    } else {
        "Edit freely: every change is parsed and solved."
    };
    let color = if doc.errors() > 0 {
        a::ERROR
    } else {
        t::TEXT_MUTED
    };
    let g = wrapped(ui, note, font::body(12.0), color, inner, Some(17.0));
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), g.size().y), Sense::hover());
    ui.painter()
        .galley(pos2(r.left() + 14.0, r.top()), g, color);
    let note_s = note.to_string();
    ui.interact(r, ui.scope_id().with("netlist-note"), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, &note_s));
    for d in diags.iter().filter(|d| d.severity == Severity::Warning) {
        let s = format!("Line {}: {}", d.line, d.message);
        let g = wrapped(ui, &s, font::body(12.0), a::WARNING, inner, Some(17.0));
        let (r, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), g.size().y + 2.0), Sense::hover());
        ui.painter()
            .galley(pos2(r.left() + 14.0, r.top() + 2.0), g, a::WARNING);
        ui.interact(
            r,
            ui.scope_id().with(("netlist-warning", d.line, d.column)),
            Sense::hover(),
        )
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        let enabled = doc.errors() == 0 && doc.netlist().is_some_and(|n| !n.nodes.is_empty());
        if ghost("Open in Playground")
            .icon(Icon::Truss)
            .enabled(enabled)
            .show(ui)
            .clicked()
        {
            out.open_in_playground = true;
        }
    });
    ui.add_space(12.0);
    out
}
