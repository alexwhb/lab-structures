//! The Bridge game state machine: editing with the mockup's placement rules and toasts, undo,
//! the Test ramp (progressive failure), the collapse and the result (`Playground.dc.html`
//! `class Component`: `tick()`, `finishHold()`, the editing methods and the handlers built in
//! `renderVals()`).
//!
//! The screen owns nothing but pixels: it forwards clicks and keys to [`Game`] methods, calls
//! [`Game::tick`] every 33 ms while [`Game::animating`], [`Game::advance`] with the frame time
//! (toasts fade after 2.6 s), and draws [`Game::view`].

use super::data::{
    GRID_STEP, LevelKey, MAX_LOAD_T, MaterialKey, SizeKey, Support, WEIGHT_STEPS, X_MAX, X_MIN,
    Y_MAX, Y_MIN, stars_for, underground,
};
use super::design::{Analysis, Bar, Design, Joint};
use super::sim::Sim;
use crate::truss::FailureMode;
use serde::{Deserialize, Serialize};

/// Undo keeps this many steps.
pub const UNDO_DEPTH: usize = 40;
/// The Test ramp adds this to the load factor per tick.
pub const LAMBDA_STEP: f64 = 1.0 / 55.0;
/// At most this many members break per tick (the cascade continues next tick).
pub const MAX_BREAKS_PER_TICK: usize = 30;
/// One tick of the Test ramp / collapse, s (`setInterval(…, 33)`).
pub const TICK_SECONDS: f64 = 0.033;
/// Toasts fade after this many seconds.
pub const TOAST_SECONDS: f64 = 2.6;

/// The tools rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tool {
    /// Inspect; drag joints to reshape.
    Select,
    /// Draw members joint to joint (the default).
    Member,
    /// Place loose joints.
    Joint,
    /// Make a joint a pin or roller.
    Support,
    /// Hang weight on a joint.
    Weight,
    /// Delete joints and members.
    Erase,
}

impl Tool {
    /// In the rail's order.
    pub const ALL: [Tool; 6] = [
        Tool::Select,
        Tool::Member,
        Tool::Joint,
        Tool::Support,
        Tool::Weight,
        Tool::Erase,
    ];

    /// The rail label ("Select").
    pub fn name(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Member => "Member",
            Tool::Joint => "Joint",
            Tool::Support => "Support",
            Tool::Weight => "Weight",
            Tool::Erase => "Erase",
        }
    }

    /// The tooltip (`title`).
    pub fn tip(self) -> &'static str {
        match self {
            Tool::Select => "Select and drag joints",
            Tool::Member => "Draw members joint to joint",
            Tool::Joint => "Place loose joints",
            Tool::Support => "Make a joint a pin or roller",
            Tool::Weight => "Hang weight on a joint",
            Tool::Erase => "Delete joints and members",
        }
    }

    /// The hint bar text while building (`hints`); the Member hint changes once a member is
    /// started.
    pub fn hint(self, pending: bool) -> &'static str {
        match self {
            Tool::Select => "Click a member or joint to inspect it. Drag joints to reshape.",
            Tool::Member if pending => {
                "Click the next joint, or empty grid to make one there. Click the glowing joint to stop."
            }
            Tool::Member => {
                "Click a joint to start a member, or click empty grid to drop a joint and start there."
            }
            Tool::Joint => "Click the grid to place a joint.",
            Tool::Support => "Click a joint on the ground: none → pin → roller.",
            Tool::Weight => "Click any joint to hang weight on it. Click again to stack more.",
            Tool::Erase => "Click a joint or a member’s dot to delete it.",
        }
    }

    /// The Member and Joint tools place joints on the grid (crosshair, hover dot).
    pub fn places_joints(self) -> bool {
        matches!(self, Tool::Member | Tool::Joint)
    }
}

/// Where the game is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    /// Building: the structure is solved at full load on every change.
    Build,
    /// The Test ramp is running.
    Testing,
    /// The collapse is playing.
    Collapse,
    /// The collapse has finished: the result banner shows.
    Collapsed,
    /// It carried the full load: the result banner shows.
    Held,
}

/// What is selected (inspector).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Selection {
    /// A joint, by id.
    Joint(String),
    /// A member, by id.
    Member(String),
}

/// One entry in the Test log: a member that broke, or how the structure gave way.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    /// The member that broke (`None` for a mechanism or a loose load).
    pub member: Option<String>,
    /// The load factor when it happened.
    pub lam: f64,
    /// How the member failed.
    pub mode: Option<FailureMode>,
    /// The joint that swung free (mechanism) or whose weight hung on nothing (loose load).
    pub bad: Option<String>,
    /// A weight hanging on nothing.
    pub loose: bool,
}

/// The Bridge game.
#[derive(Debug, Clone)]
pub struct Game {
    level: LevelKey,
    design: Design,
    mat: MaterialKey,
    tool: Tool,
    size: SizeKey,
    weight: u32,
    sag: bool,
    nums: bool,
    best: [u8; 4],
    pending: Option<String>,
    sel: Option<Selection>,
    phase: Phase,
    lam: f64,
    broken: Vec<String>,
    log: Vec<LogEntry>,
    stars: u8,
    hover: Option<(i32, i32)>,
    drag: Option<String>,
    toast: Option<String>,
    toast_age: f64,
    hist: Vec<Design>,
    sim: Option<Sim>,
    analysis: Analysis,
    revision: u64,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    /// A fresh game: level 1, the Member tool, Medium, 10 t, sag exaggerated, forces hidden.
    pub fn new() -> Game {
        let design = Design::for_level(LevelKey::River);
        let analysis = design.analyse(MaterialKey::Steel, 1.0, &[]);
        Game {
            level: LevelKey::River,
            design,
            mat: MaterialKey::Steel,
            tool: Tool::Member,
            size: SizeKey::Medium,
            weight: 10,
            sag: true,
            nums: false,
            best: [0; 4],
            pending: None,
            sel: None,
            phase: Phase::Build,
            lam: 0.0,
            broken: Vec::new(),
            log: Vec::new(),
            stars: 0,
            hover: None,
            drag: None,
            toast: None,
            toast_age: 0.0,
            hist: Vec::new(),
            sim: None,
            analysis,
            revision: 0,
        }
    }

    // ---------------------------------------------------------------------------------------
    // Reading

    /// The current level.
    pub fn level(&self) -> LevelKey {
        self.level
    }
    /// The structure.
    pub fn design(&self) -> &Design {
        &self.design
    }
    /// The material it's built from.
    pub fn material(&self) -> MaterialKey {
        self.mat
    }
    /// The active tool.
    pub fn tool(&self) -> Tool {
        self.tool
    }
    /// The size new members get.
    pub fn size(&self) -> SizeKey {
        self.size
    }
    /// The weight the Weight tool (and the inspector's − / +) adds, tonnes.
    pub fn weight(&self) -> u32 {
        self.weight
    }
    /// "Exaggerate sag" (displacements drawn × 40).
    pub fn sag(&self) -> bool {
        self.sag
    }
    /// "Show forces" (kN on each member's handle).
    pub fn show_forces(&self) -> bool {
        self.nums
    }
    /// Best stars per level (index [`LevelKey::index`]).
    pub fn best(&self) -> [u8; 4] {
        self.best
    }
    /// Best stars of one level.
    pub fn best_for(&self, l: LevelKey) -> u8 {
        self.best[l.index()]
    }
    /// The joint a member is being drawn from (the glowing joint).
    pub fn pending(&self) -> Option<&str> {
        self.pending.as_deref()
    }
    /// The selection.
    pub fn selection(&self) -> Option<&Selection> {
        self.sel.as_ref()
    }
    /// The phase.
    pub fn phase(&self) -> Phase {
        self.phase
    }
    /// The load factor λ of the Test ramp (1 once held).
    pub fn lam(&self) -> f64 {
        self.lam
    }
    /// Members broken during the Test, in order.
    pub fn broken(&self) -> &[String] {
        &self.broken
    }
    /// The Test log.
    pub fn log(&self) -> &[LogEntry] {
        &self.log
    }
    /// Stars of the last held Test.
    pub fn stars(&self) -> u8 {
        self.stars
    }
    /// The grid point under the pointer (Member / Joint tools).
    pub fn hover(&self) -> Option<(i32, i32)> {
        self.hover
    }
    /// The joint being dragged.
    pub fn dragging(&self) -> Option<&str> {
        self.drag.as_deref()
    }
    /// The toast showing, if any.
    pub fn toast(&self) -> Option<&str> {
        self.toast.as_deref()
    }
    /// The collapse simulation, while it plays or its result shows.
    pub fn sim(&self) -> Option<&Sim> {
        self.sim.as_ref()
    }
    /// The solve shown now (the mockup's `r` in `renderVals`): at full load while building,
    /// at λ while testing, at full load during the collapse.
    pub fn analysis(&self) -> &Analysis {
        &self.analysis
    }
    /// Undo steps available.
    pub fn undo_depth(&self) -> usize {
        self.hist.len()
    }
    /// Bumped on every change (screens can cache what they derive from the game).
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Building (not testing or looking at a result).
    pub fn building(&self) -> bool {
        self.phase == Phase::Build
    }
    /// The collapse simulation is what's drawn (`inSim`).
    pub fn in_sim(&self) -> bool {
        matches!(self.phase, Phase::Collapse | Phase::Collapsed) && self.sim.is_some()
    }
    /// Something moves: call [`Game::tick`] every [`TICK_SECONDS`].
    pub fn animating(&self) -> bool {
        matches!(self.phase, Phase::Testing | Phase::Collapse)
    }
    /// The header's drive button reads "Pause" (`playing`).
    pub fn playing(&self) -> bool {
        self.animating()
    }

    // ---------------------------------------------------------------------------------------
    // Internals

    fn refresh(&mut self) {
        let lam = if self.in_sim() || self.phase == Phase::Build {
            1.0
        } else {
            self.lam
        };
        self.analysis = self.design.analyse(self.mat, lam, &self.broken);
        self.revision = self.revision.wrapping_add(1);
    }

    fn push(&mut self) {
        self.push_design(self.design.clone());
    }

    fn push_design(&mut self, d: Design) {
        self.hist.push(d);
        if self.hist.len() > UNDO_DEPTH {
            self.hist.remove(0);
        }
    }

    /// Records `before` as an undo step if the design changed since (refused edits — an
    /// underground spot, a pair already joined, a joint already at 200 t — leave no step).
    fn commit(&mut self, before: Design) {
        if before != self.design {
            self.push_design(before);
        }
    }

    /// Shows a toast (fades after [`TOAST_SECONDS`]).
    pub fn say(&mut self, text: impl Into<String>) {
        self.toast = Some(text.into());
        self.toast_age = 0.0;
        self.revision = self.revision.wrapping_add(1);
    }

    /// Advances the toast clock by `dt` seconds. Returns true while a toast shows.
    pub fn advance(&mut self, dt: f64) -> bool {
        if self.toast.is_some() {
            if dt.is_finite() && dt > 0.0 {
                self.toast_age += dt;
            }
            if self.toast_age > TOAST_SECONDS {
                self.toast = None;
                self.revision = self.revision.wrapping_add(1);
            }
        }
        self.toast.is_some()
    }

    fn add_node(&mut self, x: i32, y: i32) -> Option<String> {
        if underground(x, y) {
            self.say("That spot is underground.");
            return None;
        }
        if let Some(n) = self.design.joint_at(x, y) {
            return Some(n.id.clone());
        }
        let id = format!("J{}", self.design.n_n + 1);
        self.design.nodes.push(Joint {
            id: id.clone(),
            x,
            y,
            sup: None,
            locked: false,
        });
        self.design.n_n += 1;
        Some(id)
    }

    fn add_member(&mut self, a: &str, b: &str) {
        if a == b {
            return;
        }
        if self.design.joined(a, b) {
            self.say("Those two are already joined.");
            return;
        }
        if self.design.joint(a).is_none() || self.design.joint(b).is_none() {
            return;
        }
        let id = format!("M{}", self.design.n_m + 1);
        self.design.members.push(Bar {
            id,
            a: a.to_string(),
            b: b.to_string(),
            size: self.size,
        });
        self.design.n_m += 1;
    }

    fn level_load(&self, id: &str) -> Option<f64> {
        self.level
            .level()
            .loads
            .iter()
            .find(|(i, _)| format!("J{}", i + 1) == id)
            .map(|(_, t)| *t)
    }

    fn supports_ok(&self, x: i32, y: i32) -> bool {
        self.level.level().sup.allows(x, y)
    }

    // ---------------------------------------------------------------------------------------
    // Settings

    /// Picks a tool (drops a started member and the hover dot).
    pub fn pick_tool(&mut self, t: Tool) {
        self.tool = t;
        self.pending = None;
        self.hover = None;
        self.revision = self.revision.wrapping_add(1);
    }

    /// Picks the size for new members.
    pub fn pick_size(&mut self, s: SizeKey) {
        self.size = s;
        self.revision = self.revision.wrapping_add(1);
    }

    /// Picks the weight step (tonnes, 1–200; normally one of [`WEIGHT_STEPS`]).
    pub fn pick_weight(&mut self, t: u32) {
        self.weight = t.clamp(1, MAX_LOAD_T as u32);
        self.revision = self.revision.wrapping_add(1);
    }

    /// Toggles "Exaggerate sag".
    pub fn toggle_sag(&mut self) {
        self.sag = !self.sag;
        self.revision = self.revision.wrapping_add(1);
    }

    /// Toggles "Show forces".
    pub fn toggle_forces(&mut self) {
        self.nums = !self.nums;
        self.revision = self.revision.wrapping_add(1);
    }

    /// Picks the material ("Built from"). Only while building (building is paused while
    /// testing or looking at the result).
    pub fn pick_material(&mut self, m: MaterialKey) {
        if !self.building() {
            return;
        }
        self.mat = m;
        self.refresh();
    }

    /// Sets the best stars (restoring saved progress).
    pub fn set_best(&mut self, best: [u8; 4]) {
        self.best = best.map(|b| b.min(3));
        self.revision = self.revision.wrapping_add(1);
    }

    /// Picks a level: its joints and loads, steel, building, undo emptied (`levelState`).
    pub fn pick_level(&mut self, k: LevelKey) {
        self.sim = None;
        self.hist.clear();
        self.level = k;
        self.design = Design::for_level(k);
        self.mat = MaterialKey::Steel;
        self.pending = None;
        self.sel = None;
        self.phase = Phase::Build;
        self.lam = 0.0;
        self.broken.clear();
        self.log.clear();
        self.hover = None;
        self.drag = None;
        self.refresh();
    }

    /// "Clear" / the header's Reset: start this level again.
    pub fn clear(&mut self) {
        self.pick_level(self.level);
    }

    /// Loads a design into the Sandbox ("Open in Playground", ⌘V): the level becomes the
    /// Sandbox, the design and material replace what was there, undo starts empty.
    pub fn open_in_sandbox(&mut self, design: Design, mat: MaterialKey) {
        self.pick_level(LevelKey::Sandbox);
        self.design = design;
        self.mat = mat;
        self.refresh();
    }

    // ---------------------------------------------------------------------------------------
    // Editing

    /// A click on a joint (`clickNode`), by the active tool.
    pub fn click_joint(&mut self, id: &str) {
        if !self.building() {
            return;
        }
        let Some(n) = self.design.joint(id).cloned() else {
            return;
        };
        match self.tool {
            Tool::Select | Tool::Joint => {
                self.sel = Some(Selection::Joint(n.id));
                self.revision = self.revision.wrapping_add(1);
            }
            Tool::Member => {
                match self.pending.clone() {
                    None => self.pending = Some(n.id),
                    Some(p) if p == n.id => self.pending = None,
                    Some(p) => {
                        let before = self.design.clone();
                        self.add_member(&p, &n.id);
                        self.commit(before);
                        self.pending = Some(n.id);
                    }
                }
                self.refresh();
            }
            Tool::Support => self.set_support(&n.id, Support::cycle(n.sup)),
            Tool::Weight => self.add_load(&n.id, self.weight as f64),
            Tool::Erase => self.delete_joint(&n.id),
        }
    }

    /// A click on empty grid at grid point (x, y) (`clickSurface`; see [`super::data::snap`]).
    pub fn click_grid(&mut self, x: i32, y: i32) {
        if !self.building() {
            return;
        }
        let (x, y) = (x.clamp(X_MIN, X_MAX), y.clamp(Y_MIN, Y_MAX));
        match self.tool {
            Tool::Joint => {
                let before = self.design.clone();
                if let Some(id) = self.add_node(x, y) {
                    self.sel = Some(Selection::Joint(id));
                }
                self.commit(before);
                self.refresh();
            }
            Tool::Member => {
                let before = self.design.clone();
                let Some(id) = self.add_node(x, y) else {
                    return;
                };
                if let Some(p) = self.pending.clone() {
                    self.add_member(&p, &id);
                }
                self.commit(before);
                self.pending = Some(id);
                self.refresh();
            }
            Tool::Select => {
                self.sel = None;
                self.revision = self.revision.wrapping_add(1);
            }
            _ => {}
        }
    }

    /// A click on a member's handle: Erase deletes it, anything else selects it and switches
    /// to Select.
    pub fn click_member(&mut self, id: &str) {
        if !self.building() || self.design.member(id).is_none() {
            return;
        }
        if self.tool == Tool::Erase {
            self.delete_member(id);
        } else {
            self.sel = Some(Selection::Member(id.to_string()));
            self.tool = Tool::Select;
            self.pending = None;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Pointer down on a joint: with Select, starts dragging it (unless it's a level joint).
    pub fn joint_down(&mut self, id: &str) {
        if !self.building() || self.tool != Tool::Select {
            return;
        }
        let Some(n) = self.design.joint(id) else {
            return;
        };
        if n.locked {
            return;
        }
        self.push();
        self.drag = Some(id.to_string());
        self.sel = Some(Selection::Joint(id.to_string()));
        self.revision = self.revision.wrapping_add(1);
    }

    /// The pointer moved to grid point (x, y): drags the dragged joint there (not underground,
    /// not onto another joint; a support that isn't allowed there drops), or moves the hover
    /// dot.
    pub fn pointer_move(&mut self, x: i32, y: i32) {
        if !self.building() {
            return;
        }
        let (x, y) = (x.clamp(X_MIN, X_MAX), y.clamp(Y_MIN, Y_MAX));
        if let Some(d) = self.drag.clone() {
            if (y as f64) < super::data::ground(x as f64)
                || self
                    .design
                    .nodes
                    .iter()
                    .any(|n| n.id != d && n.x == x && n.y == y)
            {
                return;
            }
            let ok = self.supports_ok(x, y);
            if let Some(n) = self.design.nodes.iter_mut().find(|n| n.id == d) {
                if n.x == x && n.y == y && self.hover.is_none() {
                    return;
                }
                n.x = x;
                n.y = y;
                if n.sup.is_some() && !ok {
                    n.sup = None;
                }
            }
            self.hover = None;
            self.refresh();
            return;
        }
        if self.tool.places_joints() && self.hover != Some((x, y)) {
            self.hover = Some((x, y));
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Pointer released (or left the stage): ends a drag. A drag that didn't move the joint
    /// leaves no undo step.
    pub fn pointer_up(&mut self) {
        if self.drag.take().is_some() {
            if self.hist.last() == Some(&self.design) {
                self.hist.pop();
            }
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Hides the hover dot (pointer left the stage).
    pub fn clear_hover(&mut self) {
        if self.hover.take().is_some() {
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// "Finish line": stop drawing members.
    pub fn finish_line(&mut self) {
        self.pending = None;
        self.revision = self.revision.wrapping_add(1);
    }

    /// Sets a joint's support, by the level's rules (`setSup`).
    pub fn set_support(&mut self, id: &str, sup: Option<Support>) {
        if !self.building() {
            return;
        }
        let Some(n) = self.design.joint(id) else {
            return;
        };
        if n.locked {
            self.say("This support is part of the level.");
            return;
        }
        if sup.is_some() && !self.supports_ok(n.x, n.y) {
            let msg = self.level.level().sup.refusal();
            self.say(msg);
            return;
        }
        let before = self.design.clone();
        if let Some(n) = self.design.nodes.iter_mut().find(|n| n.id == id) {
            n.sup = sup;
        }
        self.commit(before);
        self.refresh();
    }

    /// Adds `t` tonnes to a joint's weight (negative takes off), 0–200 t; the level's own
    /// weight can't be reduced (`addLoad`).
    pub fn add_load(&mut self, id: &str, t: f64) {
        if !self.building() || !t.is_finite() {
            return;
        }
        let Some(n) = self.design.joint(id) else {
            return;
        };
        let cur = self.design.load(id);
        let nv = (cur + t).clamp(0.0, MAX_LOAD_T);
        if n.locked
            && let Some(min) = self.level_load(id)
            && nv < min
        {
            self.say("The level’s load stays on.");
            return;
        }
        let before = self.design.clone();
        self.design.set_load(id, nv);
        self.commit(before);
        self.refresh();
    }

    /// Changes a member's size (inspector Size chips).
    pub fn set_member_size(&mut self, id: &str, size: SizeKey) {
        if !self.building() || self.design.member(id).is_none() {
            return;
        }
        let before = self.design.clone();
        if let Some(m) = self.design.members.iter_mut().find(|m| m.id == id) {
            m.size = size;
        }
        self.commit(before);
        self.refresh();
    }

    /// Deletes a member (`delMember`).
    pub fn delete_member(&mut self, id: &str) {
        if !self.building() {
            return;
        }
        let before = self.design.clone();
        self.design.members.retain(|m| m.id != id);
        self.commit(before);
        self.sel = None;
        self.refresh();
    }

    /// Deletes a joint with its members and weight (`delNode`); level joints stay.
    pub fn delete_joint(&mut self, id: &str) {
        if !self.building() {
            return;
        }
        match self.design.joint(id) {
            Some(n) if !n.locked => {}
            _ => {
                self.say("This joint is part of the level.");
                return;
            }
        }
        self.push(); // a joint that isn't locked always goes
        self.design.set_load(id, 0.0);
        self.design.nodes.retain(|n| n.id != id);
        self.design.members.retain(|m| m.a != id && m.b != id);
        self.sel = None;
        if self.pending.as_deref() == Some(id) {
            self.pending = None;
        }
        self.refresh();
    }

    /// Delete / Backspace: deletes the selected joint or member.
    pub fn delete_selected(&mut self) {
        match self.sel.clone() {
            Some(Selection::Joint(id)) => self.delete_joint(&id),
            Some(Selection::Member(id)) => self.delete_member(&id),
            None => {}
        }
    }

    /// Arrow keys: moves the selected joint by (dx, dy) grid steps (2 m each) with the drag
    /// rules (not underground, not onto another joint, inside the stage; a support that isn't
    /// allowed there drops). Level joints stay put.
    pub fn nudge(&mut self, dx: i32, dy: i32) {
        if !self.building() {
            return;
        }
        let Some(Selection::Joint(id)) = self.sel.clone() else {
            return;
        };
        let Some(n) = self.design.joint(&id).cloned() else {
            return;
        };
        if n.locked {
            self.say("This joint is part of the level.");
            return;
        }
        let x = n.x.saturating_add(dx.saturating_mul(GRID_STEP));
        let y = n.y.saturating_add(dy.saturating_mul(GRID_STEP));
        if !(X_MIN..=X_MAX).contains(&x) || !(Y_MIN..=Y_MAX).contains(&y) {
            return;
        }
        if underground(x, y) {
            self.say("That spot is underground.");
            return;
        }
        if self.design.joint_at(x, y).is_some() {
            return;
        }
        let ok = self.supports_ok(x, y);
        self.push();
        if let Some(j) = self.design.nodes.iter_mut().find(|j| j.id == id) {
            j.x = x;
            j.y = y;
            if j.sup.is_some() && !ok {
                j.sup = None;
            }
        }
        self.refresh();
    }

    /// "Drop in a starter truss" (Sandbox, nothing built yet).
    pub fn starter(&mut self) {
        if !self.building() {
            return;
        }
        self.push();
        self.design = Design::starter();
        self.pending = None;
        self.refresh();
    }

    /// The Sandbox's starter button shows (`canStarter`).
    pub fn can_starter(&self) -> bool {
        self.level == LevelKey::Sandbox && self.design.members.is_empty()
    }

    /// Undo (⌘Z): the structure before the last change (40 steps), while building.
    pub fn undo(&mut self) {
        if !self.building() {
            return;
        }
        let Some(d) = self.hist.pop() else {
            return;
        };
        self.design = d;
        self.sel = None;
        self.pending = None;
        self.refresh();
    }

    // ---------------------------------------------------------------------------------------
    // Test

    /// "Test it: add the load" / the header's Test (`startOrStop`): starts the ramp, or — when
    /// testing or looking at a result — goes back to building. Refuses (with a toast) while
    /// a level's loaded joint isn't connected.
    pub fn start_or_stop(&mut self) {
        if !self.building() {
            self.back_to_build();
            return;
        }
        let orphan = self
            .level
            .level()
            .loads
            .iter()
            .any(|(i, _)| !self.design.has_member_at(&format!("J{}", i + 1)));
        if orphan {
            self.say("Connect the loaded joint to your structure first.");
            return;
        }
        self.phase = Phase::Testing;
        self.lam = 0.0;
        self.broken.clear();
        self.log.clear();
        self.sel = None;
        self.pending = None;
        self.hover = None;
        self.drag = None;
        self.refresh();
    }

    /// "Back to building".
    pub fn back_to_build(&mut self) {
        self.sim = None;
        self.phase = Phase::Build;
        self.lam = 0.0;
        self.broken.clear();
        self.log.clear();
        self.refresh();
    }

    /// "Replay in slow motion": the collapse again at 0.3×.
    pub fn replay(&mut self) {
        if !matches!(self.phase, Phase::Collapse | Phase::Collapsed) {
            return;
        }
        self.sim = Some(Sim::new(&self.design, self.mat, &self.broken, true));
        self.phase = Phase::Collapse;
        self.refresh();
    }

    fn collapse(&mut self) {
        self.sim = Some(Sim::new(&self.design, self.mat, &self.broken, false));
        self.phase = Phase::Collapse;
    }

    /// One 33 ms tick (`tick()`): the Test ramp (λ += 1/55, re-solve, break the worst member
    /// while it's over its limit — up to 30 per tick — collapse on a loose weight or a
    /// mechanism, hold at λ = 1) or three steps of the collapse simulation.
    pub fn tick(&mut self) {
        match self.phase {
            Phase::Testing => self.test_tick(),
            Phase::Collapse => {
                let done = match self.sim.as_mut() {
                    Some(sim) => sim.tick(),
                    None => true,
                };
                if done {
                    self.phase = Phase::Collapsed;
                }
                self.revision = self.revision.wrapping_add(1);
            }
            _ => {}
        }
    }

    fn test_tick(&mut self) {
        let lam = (self.lam + LAMBDA_STEP).min(1.0);
        let mut broken = self.broken.clone();
        let mut log = self.log.clone();
        let r0 = self.design.analyse(self.mat, lam, &broken);
        let loose = self.design.loads.iter().find_map(|(id, _)| {
            let used = self
                .design
                .joint_index(id)
                .is_some_and(|i| r0.used.get(i).copied().unwrap_or(false));
            (!used).then(|| id.clone())
        });
        if let Some(bad) = loose {
            self.lam = lam;
            self.log = vec![LogEntry {
                member: None,
                lam,
                mode: None,
                bad: Some(bad),
                loose: true,
            }];
            self.collapse();
            self.refresh();
            return;
        }
        let mut r = r0;
        // the cascade stopped because the structure settled (not because of the cap)
        let mut settled = false;
        for k in 0..MAX_BREAKS_PER_TICK {
            if k > 0 {
                r = self.design.analyse(self.mat, lam, &broken);
            }
            if !r.ok {
                let bad = r
                    .bad
                    .and_then(|i| self.design.nodes.get(i))
                    .map(|n| n.id.clone());
                log.push(LogEntry {
                    member: None,
                    lam,
                    mode: None,
                    bad,
                    loose: false,
                });
                self.lam = lam;
                self.broken = broken;
                self.log = log;
                self.collapse();
                self.refresh();
                return;
            }
            match r.worst_state() {
                Some((w, q)) if q.utilisation > 1.0 => {
                    let id = self.design.members[w].id.clone();
                    log.push(LogEntry {
                        member: Some(id.clone()),
                        lam,
                        mode: Some(q.mode),
                        bad: None,
                        loose: false,
                    });
                    broken.push(id);
                }
                _ => {
                    settled = true;
                    break;
                }
            }
        }
        // At full load it holds only once the cascade has settled: if this tick's 30 breaks
        // ran out first, the next tick goes on breaking at λ = 1 (the mockup's `tick()` would
        // call it held with members still over their limit; review finding).
        if lam >= 1.0 && settled {
            self.finish_hold(broken, log);
            return;
        }
        self.lam = lam;
        self.broken = broken;
        self.log = log;
        self.refresh();
    }

    fn finish_hold(&mut self, broken: Vec<String>, log: Vec<LogEntry>) {
        let r = self.design.analyse(self.mat, 1.0, &broken);
        let stars = stars_for(r.cost, self.level.level().budget);
        let i = self.level.index();
        if stars > self.best[i] {
            self.best[i] = stars;
        }
        self.phase = Phase::Held;
        self.lam = 1.0;
        self.broken = broken;
        self.log = log;
        self.stars = stars;
        self.refresh();
    }

    /// Runs the whole Test without drawing: ticks until it holds or the collapse has played
    /// (at most `max_ticks`). Returns the final phase. For tests and the MCP `bridge_test`.
    pub fn run_test(&mut self, max_ticks: usize) -> Phase {
        if self.building() {
            self.start_or_stop();
        }
        for _ in 0..max_ticks {
            if !self.animating() {
                break;
            }
            self.tick();
        }
        self.phase
    }

    /// The weight steps offered (`[1, 5, 10, 25]`).
    pub fn weight_steps() -> [u32; 4] {
        WEIGHT_STEPS
    }
}
