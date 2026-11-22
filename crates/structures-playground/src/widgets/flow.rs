//! The mockup's bars are wrapping flex rows (`display: flex; flex-wrap: wrap; align-items:
//! center; gap; min-height`), with one item pushed right by `margin-left: auto`. This module
//! is that layout as pure maths ([`flow`]) plus the "remembered size" bookkeeping for items
//! whose size is only known after drawing them ([`Remembered`]).

use eframe::egui::{Context, Id, Rect, Vec2, pos2, vec2};

/// One item of a wrapping flex row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlowItem {
    /// Its max-content size.
    pub size: Vec2,
    /// A fixed `margin-left` (the mode switcher's 12 px).
    pub margin_left: f32,
    /// `margin-left: auto`: takes the line's free space before it.
    pub auto_margin: bool,
    /// `flex-shrink: 1` with wrapping contents: never wider than the line.
    pub shrink: bool,
}

impl FlowItem {
    /// A fixed-size item.
    pub const fn fixed(size: Vec2) -> Self {
        Self {
            size,
            margin_left: 0.0,
            auto_margin: false,
            shrink: false,
        }
    }
    /// With a fixed left margin.
    pub const fn margin(mut self, m: f32) -> Self {
        self.margin_left = m;
        self
    }
    /// With `margin-left: auto`.
    pub const fn auto(mut self) -> Self {
        self.auto_margin = true;
        self
    }
    /// Shrinks to the line width (wrapping its contents).
    pub const fn shrink(mut self) -> Self {
        self.shrink = true;
        self
    }
}

/// The result of [`flow`].
#[derive(Debug, Clone, PartialEq)]
pub struct Flow {
    /// Each item's rect, relative to the content box's top-left.
    pub rects: Vec<Rect>,
    /// The indices on each line.
    pub lines: Vec<Vec<usize>>,
    /// The content box height (at least `min_height`).
    pub height: f32,
}

/// Lays out `items` in a content box `width` wide: greedy line breaking by outer size, `gap.x`
/// between items, `gap.y` between lines, items centred on their line, a single line centred in
/// `min_height`.
pub fn flow(items: &[FlowItem], width: f32, gap: Vec2, min_height: f32) -> Flow {
    let width = width.max(0.0);
    let item_w = |it: &FlowItem| {
        if it.shrink {
            it.size.x.min((width - it.margin_left).max(0.0))
        } else {
            it.size.x
        }
    };
    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    let mut used = 0.0;
    for (i, it) in items.iter().enumerate() {
        let outer = it.margin_left + item_w(it);
        let need = if cur.is_empty() {
            outer
        } else {
            used + gap.x + outer
        };
        if !cur.is_empty() && need > width + 0.01 {
            lines.push(std::mem::take(&mut cur));
            used = outer;
        } else {
            used = need;
        }
        cur.push(i);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    let heights: Vec<f32> = lines
        .iter()
        .map(|l| l.iter().map(|&i| items[i].size.y).fold(0.0, f32::max))
        .collect();
    let content: f32 = heights.iter().sum::<f32>() + gap.y * lines.len().saturating_sub(1) as f32;
    let height = content.max(min_height);
    let mut rects = vec![Rect::NOTHING; items.len()];
    let mut y = if lines.len() == 1 {
        0.0
    } else {
        ((height - content) / 2.0).max(0.0)
    };
    for (li, line) in lines.iter().enumerate() {
        let line_h = if lines.len() == 1 {
            height
        } else {
            heights[li]
        };
        let used: f32 = line
            .iter()
            .map(|&i| items[i].margin_left + item_w(&items[i]))
            .sum::<f32>()
            + gap.x * line.len().saturating_sub(1) as f32;
        let mut free = (width - used).max(0.0);
        let mut x = 0.0;
        for (k, &i) in line.iter().enumerate() {
            let it = &items[i];
            if k > 0 {
                x += gap.x;
            }
            if it.auto_margin {
                x += free;
                free = 0.0;
            }
            x += it.margin_left;
            let w = item_w(it);
            let h = it.size.y;
            rects[i] = Rect::from_min_size(pos2(x, y + (line_h - h) / 2.0), vec2(w, h));
            x += w;
        }
        y += line_h + gap.y;
    }
    Flow {
        rects,
        lines,
        height,
    }
}

/// Sizes of drawn-later items remembered from the previous pass (egui temp memory). When a
/// measurement changes, the pass is discarded and re-run (`Context::request_discard`), so the
/// frame that's shown is laid out with the current sizes.
#[derive(Debug, Clone)]
pub struct Remembered {
    id: Id,
    sizes: Vec<Vec2>,
    fresh: Vec<Vec2>,
}

impl Remembered {
    /// Loads the sizes stored under `id`.
    pub fn load(ctx: &Context, id: Id) -> Self {
        let sizes: Vec<Vec2> = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
        Self {
            id,
            sizes,
            fresh: Vec::new(),
        }
    }

    /// The remembered size of item `i` (`fallback` if never measured).
    pub fn get(&self, i: usize, fallback: Vec2) -> Vec2 {
        self.sizes.get(i).copied().unwrap_or(fallback)
    }

    /// How many items were measured last pass.
    pub fn len(&self) -> usize {
        self.sizes.len()
    }

    /// True when nothing was measured last pass.
    pub fn is_empty(&self) -> bool {
        self.sizes.is_empty()
    }

    /// Records item `i`'s size this pass (in order).
    pub fn measure(&mut self, size: Vec2) {
        self.fresh.push(size);
    }

    /// Stores this pass's sizes; asks egui to redo the pass if they changed.
    pub fn store(self, ctx: &Context, why: &'static str) {
        let changed = self.fresh.len() != self.sizes.len()
            || self
                .fresh
                .iter()
                .zip(&self.sizes)
                .any(|(a, b)| (*a - *b).length() > 0.5);
        if changed {
            ctx.data_mut(|d| d.insert_temp(self.id, self.fresh));
            ctx.request_discard(why);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Tray header at 1440: brand (≈ 311 × 26), modes (+12 margin), switcher (142 × 40)
    /// and the actions (≈ 417 × 32, pushed right) fit on one 56 px line.
    #[test]
    fn header_one_line_at_1440() {
        let items = [
            FlowItem::fixed(vec2(311.0, 26.0)),
            FlowItem::fixed(vec2(232.0, 40.0)).margin(12.0),
            FlowItem::fixed(vec2(142.0, 40.0)),
            FlowItem::fixed(vec2(417.0, 32.0)).auto().shrink(),
        ];
        let f = flow(&items, 1408.0, vec2(16.0, 16.0), 56.0);
        assert_eq!(f.lines.len(), 1);
        assert_eq!(f.height, 56.0);
        assert_eq!(f.rects[0].top(), 15.0); // centred: (56 − 26) / 2
        assert_eq!(f.rects[1].left(), 311.0 + 16.0 + 12.0);
        assert!((f.rects[3].right() - 1408.0).abs() < 1e-3, "pushed right");
        assert_eq!(f.rects[3].top(), 12.0);
    }

    /// At 390 (358 px inside) each item gets its own line, the actions shrink to the line
    /// and wrap to two 32 px rows (72 px): 26 + 40 + 40 + 72 + 3 · 16 = 226 (the mockup's
    /// 227 px header with its border).
    #[test]
    fn header_wraps_at_390() {
        let items = [
            FlowItem::fixed(vec2(311.0, 26.0)),
            FlowItem::fixed(vec2(232.0, 40.0)).margin(12.0),
            FlowItem::fixed(vec2(142.0, 40.0)),
            FlowItem::fixed(vec2(417.0, 72.0)).auto().shrink(),
        ];
        let f = flow(&items, 358.0, vec2(16.0, 16.0), 56.0);
        assert_eq!(f.lines, vec![vec![0], vec![1], vec![2], vec![3]]);
        assert_eq!(f.height, 226.0);
        assert_eq!(
            f.rects[1].left(),
            12.0,
            "the mode switcher keeps its margin"
        );
        assert_eq!(f.rects[3].width(), 358.0);
        assert_eq!(f.rects[3].left(), 0.0);
        assert_eq!(f.rects[3].top(), 26.0 + 16.0 + 40.0 + 16.0 + 40.0 + 16.0);
    }

    /// The Tray tab row at 390 (362 px inside, gap 18): tabs alone; the tool group (auto
    /// margin) and Random soup share a line, pushed right by the free space; Clear wraps.
    #[test]
    fn tab_row_wraps_like_the_mockup() {
        let items = [
            FlowItem::fixed(vec2(258.0, 44.0)),
            FlowItem::fixed(vec2(192.0, 34.0)).auto(),
            FlowItem::fixed(vec2(132.0, 30.0)),
            FlowItem::fixed(vec2(58.0, 30.0)),
        ];
        let f = flow(&items, 362.0, vec2(18.0, 18.0), 44.0);
        assert_eq!(f.lines, vec![vec![0], vec![1, 2], vec![3]]);
        // free = 362 − (192 + 18 + 132) = 20 → the tool group starts at x = 20
        assert_eq!(f.rects[1].left(), 20.0);
        assert_eq!(f.rects[3].left(), 0.0);
        assert_eq!(f.height, 44.0 + 18.0 + 34.0 + 18.0 + 30.0);
    }
}
