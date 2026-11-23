//! The Advanced document (ADV-1, ADV-2, ADV-8, ADV-9): the netlist text — the source of truth —
//! where it came from, the model and results in effect, and the undo stack. UI-free.
//!
//! - The toolbar's generator controls regenerate the text ([`Doc::apply_generator`], undoable);
//!   the Truck and x sliders edit the moving-load line in place ([`Doc::set_truck`]), so they
//!   also work on a custom netlist.
//! - While the text is exactly a generated one, the model is the generator's own
//!   ([`generate::Generated::netlist`], the mockup's arithmetic to the bit); once edited, the
//!   text is parsed ~150 ms after the last keystroke ([`Doc::tick`]). With errors, the last
//!   good result stays in effect ([`Doc::stale`]) and the errors sit on their lines.
//! - The influence line is cached per model (ignoring the truck's position), so driving the
//!   truck across only re-solves one position per step.

use crate::router::Route;
use sp_engine::generate::{self, Params, SPAN};
use sp_engine::influence::{self, InfluenceLine, Pick, Report, UnitSweep};
use sp_engine::netlist::{self, Diagnostic, Netlist, Severity};

/// Seconds after the last keystroke before the netlist is re-parsed.
pub const DEBOUNCE: f64 = 0.15;
/// Drive across: the truck moves this far per step…
pub const DRIVE_STEP_M: f64 = 0.5;
/// …every this many seconds (the mockup's `setInterval(…, 90)`).
pub const DRIVE_PERIOD: f32 = 0.09;
/// Undo depth.
const UNDO_DEPTH: usize = 60;
/// Changes of one control closer together than this make one undo step (a slider drag).
const UNDO_COALESCE: f64 = 0.8;

/// Where the netlist text came from (the badge in the netlist header).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// The toolbar's generator ("example: 48 m truss").
    Generated,
    /// Handed over from another screen (the Bridge's "Open in Advanced").
    Received(Route),
    /// Opened from a file (Finder, the command line).
    File,
    /// Pasted (⌘V).
    Pasted,
    /// Edited by hand.
    Edited,
}

/// The model in effect and its results.
#[derive(Debug, Clone)]
pub struct Good {
    /// The netlist solved.
    pub netlist: Netlist,
    /// Its results with the truck where it stands.
    pub report: Report,
}

#[derive(Debug, Clone)]
struct Snapshot {
    text: String,
    params: Params,
    generated: Option<Params>,
    origin: Origin,
    name: Option<String>,
}

#[derive(Debug, Clone)]
enum IlCache {
    /// A small truss: one member's line, solved in full at every position (mockup-exact).
    Exact {
        /// The netlist it was computed for, truck position zeroed.
        key: Netlist,
        member: usize,
        line: Option<InfluenceLine>,
    },
    /// A big truss: every member for a unit truck; any weight and pick without re-solving.
    Sweep {
        /// The netlist it was computed for, truck position and weight zeroed.
        key: Netlist,
        sweep: Option<UnitSweep>,
    },
}

/// The document.
#[derive(Debug, Clone)]
pub struct Doc {
    /// The netlist text.
    pub text: String,
    /// The toolbar's generator state (kept while the text is custom, for the controls).
    pub params: Params,
    /// `Some` while the text is exactly what the generator writes for these params.
    pub generated: Option<Params>,
    /// Where the text came from.
    pub origin: Origin,
    /// The document name handed over with the text (`sandbox`), if any.
    pub name: Option<String>,
    /// The last good model and its results.
    pub good: Option<Good>,
    /// The current text's errors and warnings (sorted by line).
    pub diagnostics: Vec<Diagnostic>,
    /// The results on screen are not the current text's (it has errors): draw them dimmed.
    pub stale: bool,
    /// The influence-line pick (`topmid`, …, or a member id).
    pub pick: String,
    dirty_at: Option<f64>,
    /// The member the last parsed `.plot N(…)` named (its pick is selected when it changes).
    plot_member: Option<String>,
    undo: Vec<Snapshot>,
    last_change: Option<(&'static str, f64)>,
    il: Option<IlCache>,
}

impl Default for Doc {
    fn default() -> Self {
        Self::new()
    }
}

impl Doc {
    /// The mockup's defaults: the generated 48 m Pratt truss, steel, size 4, 40 t at 24 m.
    pub fn new() -> Self {
        let mut d = Self {
            text: String::new(),
            params: Params::default(),
            generated: None,
            origin: Origin::Generated,
            name: None,
            good: None,
            diagnostics: Vec::new(),
            stale: false,
            pick: "topmid".into(),
            dirty_at: None,
            plot_member: None,
            undo: Vec::new(),
            last_change: None,
            il: None,
        };
        d.regenerate(Params::default());
        d
    }

    // ---- reading -------------------------------------------------------------------------

    /// Number of errors in the current text.
    pub fn errors(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.is_error()).count()
    }

    /// Number of warnings in the current text.
    pub fn warnings(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    /// A parse is waiting for the debounce.
    pub fn pending(&self) -> bool {
        self.dirty_at.is_some()
    }

    /// Seconds until the pending parse is due (0 if due now; `None` if nothing is pending).
    pub fn due_in(&self, now: f64) -> Option<f64> {
        self.dirty_at.map(|t| (t + DEBOUNCE - now).max(0.0))
    }

    /// The report in effect.
    pub fn report(&self) -> Option<&Report> {
        self.good.as_ref().map(|g| &g.report)
    }

    /// The netlist in effect.
    pub fn netlist(&self) -> Option<&Netlist> {
        self.good.as_ref().map(|g| &g.netlist)
    }

    /// The file name in the toolbar (`river-crossing.str`).
    pub fn file_name(&self) -> String {
        if self.generated.is_some() {
            return generate::FILE_NAME.into();
        }
        if let Some(t) = self.netlist().and_then(|n| n.title.clone()) {
            return t;
        }
        match &self.name {
            Some(n) if n.ends_with(".str") => n.clone(),
            Some(n) => format!("{n}.str"),
            None => "untitled.str".into(),
        }
    }

    /// The document name in the header when it isn't the route's default (`river-crossing`).
    pub fn document(&self) -> Option<String> {
        if self.generated.is_some() {
            return None;
        }
        let file = self.file_name();
        let stem = file.strip_suffix(".str").unwrap_or(&file).to_string();
        (stem != "river-crossing").then_some(stem)
    }

    /// The truck's weight in tonnes (the Truck slider), if the netlist has a moving load.
    pub fn truck_t(&self) -> Option<f64> {
        if let Some(p) = self.generated {
            return Some(p.truck_t);
        }
        let t = self.netlist()?.truck.as_ref()?;
        Some(t.tonnes.unwrap_or(t.weight / (sp_engine::G * 1e3)))
    }

    /// The truck's position, m (the x slider and the clock).
    pub fn x(&self) -> f64 {
        if let Some(p) = self.generated {
            return p.x;
        }
        self.netlist().map(|n| n.truck_x()).unwrap_or(self.params.x)
    }

    /// The x slider's range: the lane (0–48 m for the generated truss).
    pub fn x_range(&self) -> (f64, f64) {
        if self.generated.is_some() {
            return (0.0, SPAN);
        }
        self.netlist()
            .and_then(|n| n.lane_extent())
            .filter(|(a, b)| b > a)
            .unwrap_or((0.0, SPAN))
    }

    /// The influence-line picks.
    pub fn picks(&self) -> Vec<Pick> {
        match &self.good {
            Some(g) => influence::picks(&g.netlist, &g.report),
            None => Vec::new(),
        }
    }

    /// The current pick (the stored key, else the first).
    pub fn current_pick(&self) -> Option<Pick> {
        let picks = self.picks();
        picks
            .iter()
            .find(|p| p.key == self.pick)
            .or(picks.first())
            .cloned()
    }

    /// The influence line of the current pick (`None` without a moving load). Cached: a small
    /// truss per model (truck position aside) and member; a big one as a [`UnitSweep`] per model
    /// (truck position and weight aside), so dragging Truck or changing the pick doesn't solve.
    pub fn influence_line(&mut self) -> Option<InfluenceLine> {
        let pick = self.current_pick()?;
        let good = self.good.as_ref()?;
        let mut key = good.netlist.clone();
        if let Some(t) = key.truck.as_mut() {
            t.x = 0.0;
        }
        if good.netlist.nodes.len() <= influence::EXACT_SWEEP_NODES {
            let fresh = matches!(&self.il, Some(IlCache::Exact { key: k, member, .. })
                if *member == pick.member && *k == key);
            if !fresh {
                let line = influence::influence_line_exact(&good.netlist, pick.member);
                self.il = Some(IlCache::Exact {
                    key,
                    member: pick.member,
                    line,
                });
            }
            return match &self.il {
                Some(IlCache::Exact { line, .. }) => line.clone(),
                _ => None,
            };
        }
        if let Some(t) = key.truck.as_mut() {
            t.weight = 0.0;
            t.tonnes = None;
        }
        let fresh = matches!(&self.il, Some(IlCache::Sweep { key: k, .. }) if *k == key);
        if !fresh {
            let sweep = influence::unit_sweep(&good.netlist);
            self.il = Some(IlCache::Sweep { key, sweep });
        }
        match &self.il {
            Some(IlCache::Sweep {
                sweep: Some(sw), ..
            }) => influence::line_from_sweep(&good.netlist, sw, pick.member),
            _ => None,
        }
    }

    // ---- changing ------------------------------------------------------------------------

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            params: self.params,
            generated: self.generated,
            origin: self.origin,
            name: self.name.clone(),
        }
    }

    /// Records an undo step before a change by `control` at `now` (coalescing a run of
    /// changes by the same control).
    fn record(&mut self, control: &'static str, now: f64) {
        let same = self
            .last_change
            .is_some_and(|(c, t)| c == control && now - t < UNDO_COALESCE);
        self.last_change = Some((control, now));
        if !same {
            self.undo.push(self.snapshot());
            if self.undo.len() > UNDO_DEPTH {
                self.undo.remove(0);
            }
        }
    }

    /// Whether there is something to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Sets the text to the generator's for `p`, with its exact model.
    fn regenerate(&mut self, p: Params) {
        let g = generate::generate(&p);
        self.text = g.text;
        self.params = g.params;
        self.generated = Some(g.params);
        self.origin = Origin::Generated;
        self.name = None;
        self.dirty_at = None;
        self.adopt(g.netlist);
    }

    /// Makes `net` the model in effect (or reports why it can't be solved).
    fn adopt(&mut self, net: Netlist) {
        match influence::analyse(&net) {
            Ok(report) => {
                // `.plot N(M3)` selects that member's pick when the line (re)names it
                let named = net.plot.as_ref().and_then(|p| p.member.clone());
                if named != self.plot_member {
                    if let Some(m) = influence::plot_member(&net)
                        && let Some(p) = influence::picks(&net, &report)
                            .into_iter()
                            .find(|p| p.member == m)
                    {
                        self.pick = p.key;
                    }
                    self.plot_member = named;
                }
                self.diagnostics = net.warnings.clone();
                self.good = Some(Good {
                    netlist: net,
                    report,
                });
                self.stale = false;
            }
            Err(e) => {
                self.diagnostics = vec![Diagnostic {
                    line: 1,
                    column: 1,
                    len: 1,
                    severity: Severity::Error,
                    message: format!("can't solve it: {e}"),
                }];
                self.stale = true;
            }
        }
    }

    /// A generator control changed (style, braced, material, size): regenerate (undoable).
    pub fn apply_generator(&mut self, p: Params, control: &'static str, now: f64) {
        self.record(control, now);
        self.regenerate(p);
    }

    /// The Truck or x slider (or the drive) moved: edit the moving-load line in place.
    /// `record` makes it undoable (the drive animation doesn't record).
    pub fn set_truck(
        &mut self,
        tonnes: Option<f64>,
        x: Option<f64>,
        control: &'static str,
        now: f64,
        record: bool,
    ) {
        if record {
            self.record(control, now);
        }
        if let Some(p) = self.generated {
            let p = Params {
                truck_t: tonnes.unwrap_or(p.truck_t),
                x: x.unwrap_or(p.x),
                ..p
            };
            self.regenerate(p);
            return;
        }
        if let Some(t) = tonnes {
            self.params.truck_t = t;
        }
        if let Some(x) = x {
            self.params.x = x;
        }
        if let Some(text) = netlist::set_truck(&self.text, tonnes, x) {
            self.text = text;
            self.reparse();
        }
    }

    /// The user typed in the editor.
    pub fn edit(&mut self, text: String, now: f64) {
        if text == self.text {
            return;
        }
        self.record("edit", now);
        self.text = text;
        if self.text == generate::generate(&self.params).text {
            self.generated = Some(self.params);
            self.origin = Origin::Generated;
            self.name = None;
        } else {
            self.generated = None;
            self.origin = Origin::Edited;
        }
        self.dirty_at = Some(now);
    }

    /// Loads a netlist from elsewhere (a hand-off, a file, a paste), parsed at once.
    pub fn load(&mut self, text: String, name: Option<String>, origin: Origin, now: f64) {
        self.record("load", now);
        self.last_change = None;
        self.text = text;
        self.generated = None;
        self.origin = origin;
        self.name = name.filter(|n| !n.trim().is_empty());
        self.reparse();
        // a netlist with the standard picks keeps the pick, otherwise the first
        if !self.picks().iter().any(|p| p.key == self.pick)
            && let Some(p) = self.picks().first()
        {
            self.pick = p.key.clone();
        }
    }

    /// A pasted `.material` line (the Materials bench's Share) replaces the netlist's first
    /// `.material` line (or goes after `.units`). Returns false when `pasted` has none.
    pub fn paste_material(&mut self, pasted: &str, now: f64) -> bool {
        let is_mat = |l: &str| l.trim_start().to_ascii_lowercase().starts_with(".material");
        let Some(line) = pasted.lines().find(|l| is_mat(l)) else {
            return false;
        };
        self.record("paste", now);
        self.last_change = None;
        let mut out = String::with_capacity(self.text.len() + line.len());
        let has_mat = self.text.lines().any(is_mat);
        let mut done = false;
        for raw in self.text.split_inclusive('\n') {
            let body = raw.trim_end_matches(['\n', '\r']);
            if !done && is_mat(body) {
                out.push_str(line.trim());
                out.push_str(&raw[body.len()..]);
                done = true;
            } else {
                out.push_str(raw);
                if !done && !has_mat && body.trim_start().to_ascii_lowercase().starts_with(".units")
                {
                    out.push_str(line.trim());
                    out.push('\n');
                    done = true;
                }
            }
        }
        if !done {
            out.push_str(line.trim());
            out.push('\n');
        }
        self.text = out;
        self.generated = None;
        self.origin = Origin::Pasted;
        self.reparse();
        true
    }

    /// Parses the text now (the debounce elapsed, or a tool changed it).
    pub fn reparse(&mut self) {
        self.dirty_at = None;
        if let Some(p) = self.generated {
            self.adopt(generate::generate(&p).netlist);
            return;
        }
        match netlist::parse(&self.text) {
            Ok(net) => {
                // keep the toolbar's truck in step with the text
                if let Some(t) = &net.truck {
                    if let Some(tt) = t.tonnes {
                        self.params.truck_t = tt;
                    }
                    self.params.x = t.x;
                }
                self.adopt(net);
            }
            Err(d) => {
                self.diagnostics = d;
                self.stale = true;
            }
        }
    }

    /// Time passes: runs the debounced parse when due. Returns true if it parsed.
    pub fn tick(&mut self, now: f64) -> bool {
        match self.dirty_at {
            Some(t) if now - t >= DEBOUNCE => {
                self.reparse();
                true
            }
            _ => false,
        }
    }

    /// Back one step (⌘Z). Returns false when there was nothing to undo.
    pub fn undo(&mut self) -> bool {
        let Some(s) = self.undo.pop() else {
            return false;
        };
        self.last_change = None;
        self.text = s.text;
        self.params = s.params;
        self.generated = s.generated;
        self.origin = s.origin;
        self.name = s.name;
        self.reparse();
        true
    }

    /// Reset: the mockup's defaults (undoable).
    pub fn reset(&mut self, now: f64) {
        self.record("reset", now);
        self.last_change = None;
        self.regenerate(Params::default());
        self.pick = "topmid".into();
    }

    /// One drive step: x + 0.5 m, back to the start after the end (the mockup's
    /// `xt >= 48 ? 0 : min(48, xt + 0.5)`).
    pub fn drive_step(&mut self, now: f64) {
        let (a, b) = self.x_range();
        let x = self.x();
        let next = if x >= b { a } else { (x + DRIVE_STEP_M).min(b) };
        self.set_truck(None, Some(next), "drive", now, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_engine::generate::Style;

    #[test]
    fn defaults_are_the_mockups() {
        let d = Doc::new();
        assert!(d.generated.is_some());
        assert_eq!(d.file_name(), "river-crossing.str");
        assert_eq!(d.document(), None);
        let r = d.report().unwrap();
        assert_eq!(r.status.label(), "Holding · worst 40%");
        assert_eq!(d.x(), 24.0);
    }

    #[test]
    fn generator_changes_are_undoable() {
        let mut d = Doc::new();
        let p = Params {
            style: Style::Howe,
            ..d.params
        };
        d.apply_generator(p, "style", 1.0);
        assert!(d.text.contains("Howe"));
        assert!(d.undo());
        assert!(d.text.contains("Pratt"));
        assert!(!d.undo());
    }

    #[test]
    fn edits_parse_after_the_debounce_and_errors_keep_the_last_result() {
        let mut d = Doc::new();
        let bad = d.text.replace(".node B3  24  0", ".node B3  24  oops");
        d.edit(bad, 10.0);
        assert!(d.pending() && d.generated.is_none() && d.origin == Origin::Edited);
        assert!(!d.tick(10.1));
        assert!(d.tick(10.2));
        assert_eq!(d.errors(), 1);
        assert_eq!(d.diagnostics[0].line, 10);
        assert!(d.stale);
        assert!(d.report().is_some(), "last good result kept");
        // fix it
        let good = d.text.replace("oops", "0");
        d.edit(good, 11.0);
        assert_eq!(d.generated, Some(d.params), "back to the generated text");
        d.tick(11.5);
        assert_eq!(d.errors(), 0);
        assert!(!d.stale);
    }

    #[test]
    fn truck_slider_edits_a_custom_netlist_in_place() {
        let mut d = Doc::new();
        let custom = d
            .text
            .replace(".load dead  B1..B5  Fy=-80", ".load dead  B1..B5  Fy=-90");
        d.edit(custom, 1.0);
        d.tick(2.0);
        d.set_truck(Some(60.0), Some(12.0), "truck", 3.0, true);
        assert!(d.text.contains(".load truck  60t  x=12  lane=B0..B6"));
        assert!(d.text.contains("Fy=-90"));
        assert_eq!(d.truck_t(), Some(60.0));
        assert_eq!(d.x(), 12.0);
    }

    #[test]
    fn drive_loops_over_the_span() {
        let mut d = Doc::new();
        d.set_truck(None, Some(47.5), "x", 0.0, true);
        d.drive_step(1.0);
        assert_eq!(d.x(), 48.0);
        d.drive_step(1.1);
        assert_eq!(d.x(), 0.0);
        d.drive_step(1.2);
        assert_eq!(d.x(), 0.5);
    }

    #[test]
    fn a_pasted_material_line_swaps_the_material() {
        let mut d = Doc::new();
        // the Materials bench's Share line
        let line = ".material steel E=200GPa fy=275MPa rho=7850 curve=steel ; tensile test at 20 °C, ε = 0.00 %, Ready";
        assert!(d.paste_material(line, 1.0));
        assert_eq!(d.errors(), 0, "{:?}", d.diagnostics);
        assert_eq!(d.warnings(), 1, "curve= is a warning");
        assert_eq!(d.netlist().unwrap().materials[0].fy, 275e6);
        assert_eq!(d.netlist().unwrap().members.len(), 21);
        assert!(!d.paste_material("no material here", 2.0));
    }

    #[test]
    fn a_plot_line_naming_a_member_selects_it() {
        let mut d = Doc::new();
        let t = d.text.replace(".plot N(M*)", ".plot N(M3)");
        d.edit(t, 1.0);
        d.tick(2.0);
        assert_eq!(d.current_pick().unwrap().name, "Bottom chord B2–B3");
        let t = d.text.replace(".plot N(M3)", ".plot N(M13)");
        d.edit(t, 3.0);
        d.tick(4.0);
        let p = d.current_pick().unwrap();
        assert_eq!(p.name, "M13 B1–T1");
        assert_eq!(d.picks()[0].key, "M13");
        assert_eq!(d.influence_line().unwrap().id, "M13");
        // the user's own pick isn't overridden by later re-parses
        d.pick = "post".into();
        d.set_truck(None, Some(10.0), "x", 5.0, true);
        assert_eq!(d.current_pick().unwrap().key, "post");
    }

    #[test]
    fn drive_needs_a_netlist_without_errors() {
        let mut d = Doc::new();
        let bad = d.text.replace(".node B1  8  0", ".node B1  8  oops");
        d.edit(bad, 1.0);
        d.tick(2.0);
        assert!(d.stale);
    }

    #[test]
    fn influence_line_is_cached_across_truck_moves() {
        let mut d = Doc::new();
        let peak = d.influence_line().unwrap().peak;
        d.set_truck(None, Some(10.0), "x", 0.0, false);
        assert!(d.il.is_some());
        assert_eq!(d.influence_line().unwrap().peak, peak);
    }
}
