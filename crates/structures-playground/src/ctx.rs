//! The per-frame screen context: what a screen may ask of the app while it draws.
//!
//! A [`Ctx`] is built by the app each frame and handed to the active screen's hooks. Through it
//! a screen navigates (optionally carrying state to the next screen, NAV-4), shows toasts,
//! copies to the clipboard, saves files, reads the app's RNG seed and frame time, paces its
//! frames ([`Ctx::animate`], [`Ctx::wake_in`]) and learns whether a text field has focus.

use crate::launcher::{AppId, Launch, Launcher};
use crate::router::Route;
use crate::widgets::Toasts;
use eframe::egui;
use std::path::PathBuf;

/// State handed from one screen to another (NAV-4), delivered to the target's
/// [`Screen::receive`](crate::screens::Screen::receive) right after its `enter`.
#[derive(Debug, Clone, PartialEq)]
pub enum Carry {
    /// A truss netlist (`.str` text): Bridge "Open in Advanced" → Advanced (ADV-9), Advanced
    /// "Open in Playground" → Bridge (loads into the Sandbox), a `.str` opened from Finder or
    /// the command line → Advanced.
    Netlist {
        /// The netlist text (parse it with `sp_engine`).
        text: String,
        /// The document name to show (a file stem such as `river-crossing`), if known.
        name: Option<String>,
        /// The screen it came from (`None`: from outside the app, e.g. a file).
        from: Option<Route>,
    },
    /// Tower "Send base reactions to Ground ›" → Ground: the building standing on the ground.
    Building {
        /// Storeys (Ground turns them into the building load).
        floors: u32,
        /// The tower's base shear in N (shown as a note).
        base_shear_n: f64,
        /// The tower's overturning moment at the base in N·m (shown as a note).
        overturning_nm: f64,
    },
    /// Ground "‹ Back to the Tower" → Tower: the building's floors (the Tower keeps the rest
    /// of its settings).
    Tower {
        /// Storeys.
        floors: u32,
    },
}

impl Carry {
    /// A netlist from screen `from`.
    pub fn netlist(text: impl Into<String>, name: Option<&str>, from: Route) -> Self {
        Carry::Netlist {
            text: text.into(),
            name: name.map(str::to_string),
            from: Some(from),
        }
    }

    /// Where it came from (`None` for a netlist from outside the app).
    pub fn from(&self) -> Option<Route> {
        match self {
            Carry::Netlist { from, .. } => *from,
            Carry::Building { .. } => Some(Route::Tower),
            Carry::Tower { .. } => Some(Route::Ground),
        }
    }
}

/// A navigation request made during a frame; the app applies it after the frame.
#[derive(Debug, Clone, PartialEq)]
pub struct NavRequest {
    /// Where to go.
    pub route: Route,
    /// Payload delivered to the target's [`Screen::receive`](crate::screens::Screen::receive).
    pub carry: Option<Carry>,
}

/// Keyboard commands the app routes to the active screen (NAV-5). The app handles Space
/// itself (it calls [`Screen::drive`](crate::screens::Screen::drive)); these are the rest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command {
    /// ⌘Z (when no text field has focus): undo (the Bridge).
    Undo,
    /// Delete / Backspace (when no text field has focus): delete the selection (the Bridge).
    Delete,
    /// ⌘V (when no text field has focus): pasted text (a netlist on the Bridge / Advanced).
    Paste(String),
}

/// App services shared by every frame (owned by the app).
#[derive(Debug, Clone)]
pub struct Services {
    /// Opens sibling apps.
    pub launcher: Launcher,
    /// Where files are written when there's no save dialog (tests, headless).
    pub out_dir: PathBuf,
    /// Show the native save dialog (`false` in tests: write straight into `out_dir`).
    pub save_dialog: bool,
    /// The app's RNG seed (fixed in tests, from the clock otherwise).
    pub rng_seed: u64,
    /// Screens may start animating when first shown; off in tests.
    pub autoplay: bool,
}

/// What the app keeps across frames on behalf of screens (clipboard log, saved files).
#[derive(Debug, Default, Clone)]
pub struct Outbox {
    /// The last text copied with [`Ctx::copy`].
    pub last_copied: Option<String>,
    /// Every file written with [`Ctx::save_text`], in order.
    pub saved: Vec<PathBuf>,
}

/// The screen context for one frame. See the module docs.
pub struct Ctx<'a> {
    pub(crate) route: Route,
    pub(crate) last_world: Route,
    pub(crate) previous: Option<Route>,
    pub(crate) nav: Option<NavRequest>,
    pub(crate) toasts: &'a mut Toasts,
    pub(crate) services: &'a Services,
    pub(crate) outbox: &'a mut Outbox,
    pub(crate) egui: egui::Context,
    pub(crate) body_height: f32,
}

impl<'a> Ctx<'a> {
    /// The route being drawn.
    pub fn route(&self) -> Route {
        self.route
    }

    /// The Playground world used last (where "Playground" goes).
    pub fn last_world(&self) -> Route {
        self.last_world
    }

    /// Navigate to `route` after this frame.
    pub fn go(&mut self, route: Route) {
        self.nav = Some(NavRequest { route, carry: None });
    }

    /// Navigate to `route` and hand it `carry` (NAV-4).
    pub fn go_with(&mut self, route: Route, carry: Carry) {
        self.nav = Some(NavRequest {
            route,
            carry: Some(carry),
        });
    }

    /// Back to the screen before this one (Series "Back", Esc; NAV-3): the last world when
    /// there is none.
    pub fn back(&mut self) {
        self.go(self.previous.unwrap_or(self.last_world));
    }

    /// The screen shown before this one, if any.
    pub fn previous(&self) -> Option<Route> {
        self.previous
    }

    /// The navigation requested so far this frame.
    pub fn pending(&self) -> Option<&NavRequest> {
        self.nav.as_ref()
    }

    /// Show a toast.
    pub fn toast(&mut self, text: impl Into<String>) {
        self.toasts.push(text);
    }

    /// Copy `text` to the system clipboard (and remember it: `StructuresApp::last_copied`).
    /// Share (NAV-6) goes through here.
    pub fn copy(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.egui.copy_text(text.clone());
        self.outbox.last_copied = Some(text);
    }

    /// Save `contents` as a file named like `suggested` (e.g. `river-crossing.str`): the native
    /// save dialog in the app, straight into the output folder in tests. Returns the path
    /// written, `Ok(None)` if the user cancelled.
    pub fn save_text(
        &mut self,
        suggested: &str,
        contents: &str,
    ) -> anyhow::Result<Option<PathBuf>> {
        let path = if self.services.save_dialog {
            let mut dialog = rfd::FileDialog::new().set_file_name(suggested);
            if let Some(ext) = std::path::Path::new(suggested)
                .extension()
                .and_then(|e| e.to_str())
            {
                dialog = dialog.add_filter(ext, &[ext]);
            }
            match dialog.save_file() {
                Some(p) => p,
                None => return Ok(None),
            }
        } else {
            std::fs::create_dir_all(&self.services.out_dir)?;
            self.services.out_dir.join(suggested)
        };
        std::fs::write(&path, contents)?;
        self.outbox.saved.push(path.clone());
        Ok(Some(path))
    }

    /// Open a sibling app; the result says whether it was found.
    pub fn launch(&mut self, app: AppId) -> Launch {
        self.services.launcher.launch(app)
    }

    /// The app's RNG seed (deterministic in tests). Derive per-screen seeds from it.
    pub fn rng_seed(&self) -> u64 {
        self.services.rng_seed
    }

    /// Whether screens should start running when first shown.
    pub fn autoplay(&self) -> bool {
        self.services.autoplay
    }

    /// The frame's time step in seconds. Accumulate it with a [`Ticker`].
    ///
    /// After a frame that asked for this one ([`Ctx::animate`], [`Ctx::wake_in`]) it is the
    /// time that really passed, capped at [`MAX_DT`] (or the wait asked for plus [`SLACK`]) so
    /// a stall doesn't jump the simulation; after an idle stretch it is egui's `stable_dt`
    /// (≤ [`MAX_DT`]), so pressing Run doesn't fast-forward. (egui's own `stable_dt` alone
    /// would be wrong for delayed frames: it falls back to a predicted 1/60 s.)
    pub fn dt(&self) -> f32 {
        let before = self.pacing().before;
        self.egui.input(|i| match before {
            Some(wait) => i.unstable_dt.clamp(0.0, (wait + SLACK).max(MAX_DT)),
            None => i.stable_dt.clamp(0.0, MAX_DT),
        })
    }

    /// True while a text field has keyboard focus (don't treat letters as shortcuts).
    pub fn text_focus(&self) -> bool {
        self.egui.text_edit_focused()
    }

    /// The height under the header (what side-by-side columns fill).
    pub fn body_height(&self) -> f32 {
        self.body_height
    }

    /// The egui context (repaints, input, memory).
    pub fn egui(&self) -> &egui::Context {
        &self.egui
    }

    /// Ask for the next frame at display rate, for motion that changes every frame (a
    /// collapse, a swaying tower, flowing water while testing): call it every frame while it
    /// moves. Vsync
    /// paces those frames while the window is on screen. Nothing does when it isn't: eframe
    /// doesn't paint a window it knows is hidden (occluded or minimised), and macOS doesn't
    /// hold back a window that opened where it isn't composited (another Space, behind a
    /// full-screen app) without reporting it occluded. So frames are never asked for less than
    /// [`MIN_FRAME`] apart (at most 240 a second, above any Mac display's refresh, so vsync
    /// still sets the pace on screen), and every [`HIDDEN_FRAME`] while known to be hidden.
    ///
    /// A simulation that only changes when its next step is due uses [`Ctx::wake_in`].
    pub fn animate(&self) {
        let hidden = self.egui.input(|i| i.viewport().visible() == Some(false));
        self.wake_in(if hidden { HIDDEN_FRAME } else { MIN_FRAME });
    }

    /// Ask for the next frame in `secs` seconds (when a running simulation's next step is
    /// due: `ctx.wake_in(ticker.due_in(speed))`) and draw nothing until then unless input
    /// arrives. Waits are kept between [`MIN_FRAME`] and [`MAX_WAIT`], so a running screen
    /// still draws a few frames a second and [`Ctx::dt`] keeps its time.
    pub fn wake_in(&self, secs: f32) {
        let wait = if secs.is_nan() {
            MAX_WAIT
        } else {
            secs.clamp(MIN_FRAME, MAX_WAIT)
        };
        self.egui.request_repaint_after_secs(wait);
        self.asked(wait);
    }

    /// This pass's [`Pacing`] record (rolled over from the pass before).
    fn pacing(&self) -> Pacing {
        let pass = self.egui.cumulative_pass_nr();
        let p = self
            .egui
            .data(|d| d.get_temp::<Pacing>(Pacing::id()))
            .unwrap_or_default();
        if p.pass == pass {
            p
        } else {
            Pacing {
                pass,
                asked: None,
                before: if p.pass + 1 == pass { p.asked } else { None },
            }
        }
    }

    /// Notes that this pass asked for a frame in `wait` seconds.
    fn asked(&self, wait: f32) {
        let mut p = self.pacing();
        p.asked = Some(p.asked.map_or(wait, |a| a.min(wait)));
        self.egui.data_mut(|d| d.insert_temp(Pacing::id(), p));
    }
}

/// The longest [`Ctx::dt`] after a frame at display rate (s): a stall doesn't jump the
/// simulation.
pub const MAX_DT: f32 = 0.1;
/// The longest wait [`Ctx::wake_in`] asks for (s): a running screen draws at least 10 frames
/// a second.
pub const MAX_WAIT: f32 = 0.1;
/// The shortest frame interval [`Ctx::animate`] and [`Ctx::wake_in`] ask for (s): 240 frames
/// a second, faster than any Mac display refreshes, for when vsync doesn't pace the window.
pub const MIN_FRAME: f32 = 1.0 / 240.0;
/// How late a frame asked for with [`Ctx::wake_in`] may come and still keep its time (s):
/// event-loop latency and a vsync interval.
pub const SLACK: f32 = 0.02;
/// The frame interval of [`Ctx::animate`] while the window is hidden (s).
pub const HIDDEN_FRAME: f32 = 0.1;

/// The frames asked for in a pass, kept in egui's temporary memory so [`Ctx::dt`] knows
/// whether (and how far ahead) the pass before scheduled this one.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Pacing {
    /// The pass (`egui::Context::cumulative_pass_nr`) this record is about.
    pass: u64,
    /// The shortest wait asked for in `pass` (`None`: no frame asked for).
    asked: Option<f32>,
    /// What the pass before `pass` asked for.
    before: Option<f32>,
}

impl Pacing {
    fn id() -> egui::Id {
        egui::Id::unique("structures-playground/pacing")
    }
}

/// Turns frame time into whole steps at a rate: `let n = ticker.steps(ctx.dt(), speed)`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Ticker {
    acc: f32,
}

impl Ticker {
    /// At most this many steps per frame (a long stall doesn't fast-forward).
    pub const MAX_STEPS: u32 = 8;

    /// Adds `dt` seconds and returns how many steps of `1 / rate` seconds are due.
    pub fn steps(&mut self, dt: f32, rate: f32) -> u32 {
        if rate <= 0.0 || !dt.is_finite() {
            return 0;
        }
        self.acc += dt.max(0.0);
        let period = 1.0 / rate;
        // a hair of tolerance so 0.05 + 0.05 s at 10 /s is one step despite float error
        let n = (self.acc / period + 1e-4).floor();
        self.acc = self.acc.max(n * period);
        self.acc -= n * period;
        if n > Self::MAX_STEPS as f32 {
            self.acc = 0.0;
            return Self::MAX_STEPS;
        }
        n as u32
    }

    /// Forget the accumulated time (after a pause).
    pub fn reset(&mut self) {
        self.acc = 0.0;
    }

    /// Seconds until the next step at `rate` is due (what to pass to [`Ctx::wake_in`]);
    /// infinite for a rate of 0.
    pub fn due_in(&self, rate: f32) -> f32 {
        if rate > 0.0 && rate.is_finite() {
            (1.0 / rate - self.acc).max(0.0)
        } else {
            f32::INFINITY
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticker_counts_steps() {
        let mut t = Ticker::default();
        assert_eq!(t.steps(0.05, 10.0), 0);
        assert_eq!(t.steps(0.05, 10.0), 1);
        assert_eq!(t.steps(0.25, 10.0), 2);
        assert_eq!(t.steps(0.05, 10.0), 1);
        assert_eq!(t.steps(10.0, 30.0), Ticker::MAX_STEPS);
        assert_eq!(t.steps(0.0, 30.0), 0);
        assert_eq!(t.steps(1.0, 0.0), 0);
    }

    #[test]
    fn ticker_says_when_the_next_step_is_due() {
        let mut t = Ticker::default();
        assert!((t.due_in(10.0) - 0.1).abs() < 1e-6);
        assert_eq!(t.steps(0.03, 10.0), 0);
        assert!((t.due_in(10.0) - 0.07).abs() < 1e-6);
        assert_eq!(t.steps(0.07, 10.0), 1);
        assert!((t.due_in(10.0) - 0.1).abs() < 1e-4);
        assert_eq!(t.due_in(0.0), f32::INFINITY);
    }

    /// Runs one egui pass at `time` (window hidden or not) with a `Ctx` handed to `f`;
    /// returns the repaint delay the pass asked for.
    fn pass(
        egui: &egui::Context,
        time: f64,
        hidden: bool,
        mut f: impl FnMut(&Ctx),
    ) -> std::time::Duration {
        let services = Services {
            launcher: Launcher::recording(&[]),
            out_dir: std::env::temp_dir(),
            save_dialog: false,
            rng_seed: 7,
            autoplay: false,
        };
        let (mut toasts, mut outbox) = (Toasts::default(), Outbox::default());
        let mut raw = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        if hidden {
            let info = raw.viewports.entry(egui::ViewportId::ROOT).or_default();
            info.occluded = Some(true);
            info.minimized = Some(false);
        }
        let mut out = egui.run_ui(raw, |_ui| {
            let ctx = Ctx {
                route: Route::Bridge,
                last_world: Route::Bridge,
                previous: None,
                nav: None,
                toasts: &mut toasts,
                services: &services,
                outbox: &mut outbox,
                egui: egui.clone(),
                body_height: 600.0,
            };
            f(&ctx);
        });
        out.textures_delta.clear();
        out.viewport_output[&egui::ViewportId::ROOT].repaint_delay
    }

    /// Delayed frames keep the time that really passed (egui's `stable_dt` would say 1/60 s),
    /// stalls are capped, idle stretches don't count, and a hidden window doesn't spin.
    #[test]
    fn frames_are_paced_and_dt_follows_them() {
        let egui = egui::Context::default();
        let close = |a: f32, b: f32| (a - b).abs() < 1e-3;
        let mut dt = -1.0;
        let ms = |d: std::time::Duration| d.as_secs_f32() * 1e3;

        // egui repaints its first passes anyway
        pass(&egui, 0.1, false, |_| {});
        pass(&egui, 0.2, false, |_| {});
        // wake_in: a repaint in that many seconds, at most MAX_WAIT
        assert!(close(
            ms(pass(&egui, 1.0, false, |c| c.wake_in(0.08))),
            80.0
        ));
        let delay = pass(&egui, 1.09, false, |c| {
            dt = c.dt();
            c.wake_in(5.0);
        });
        assert!(close(dt, 0.09), "a delayed frame keeps its 90 ms: {dt}");
        assert!(close(ms(delay), MAX_WAIT * 1e3), "waits are capped");
        pass(&egui, 1.19, false, |c| {
            dt = c.dt();
            c.wake_in(0.1);
        });
        assert!(close(dt, 0.1));
        // a stall after a wait of 0.1 s counts as 0.1 + SLACK
        pass(&egui, 3.0, false, |c| {
            c.animate();
            dt = c.dt(); // asking first doesn't change what this frame was
        });
        assert!(close(dt, 0.1 + SLACK), "{dt}");

        // animate: a repaint at display rate (never asked for sooner than MIN_FRAME); dt
        // capped at MAX_DT
        let delay = pass(&egui, 3.05, false, |c| {
            dt = c.dt();
            c.animate();
        });
        assert!(close(ms(delay), MIN_FRAME * 1e3), "{delay:?}");
        assert!(close(dt, 0.05));
        pass(&egui, 3.5, false, |c| dt = c.dt());
        assert!(close(dt, MAX_DT));

        // idle (nothing asked for; egui draws one more frame after a repaint request), then a
        // frame 10 s later: not a jump
        pass(&egui, 3.52, false, |_| {});
        pass(&egui, 13.5, false, |c| dt = c.dt());
        assert!(dt <= 1.0 / 60.0 + 1e-4, "after idling: {dt}");

        // hidden: animate asks for a frame in HIDDEN_FRAME, and dt keeps that time
        let delay = pass(&egui, 14.0, true, |c| c.animate());
        assert!(close(ms(delay), HIDDEN_FRAME * 1e3), "{delay:?}");
        pass(&egui, 14.1, true, |c| dt = c.dt());
        assert!(close(dt, HIDDEN_FRAME));
    }

    #[test]
    fn carry_builders() {
        let c = Carry::netlist(".node A 0 0 fix=xy", Some("mine"), Route::Bridge);
        assert_eq!(c.from(), Some(Route::Bridge));
        let Carry::Netlist { text, name, .. } = c else {
            panic!("a netlist")
        };
        assert_eq!(
            (text.as_str(), name.as_deref()),
            (".node A 0 0 fix=xy", Some("mine"))
        );
        let b = Carry::Building {
            floors: 60,
            base_shear_n: 1.0,
            overturning_nm: 2.0,
        };
        assert_eq!(b.from(), Some(Route::Tower));
        assert_eq!(Carry::Tower { floors: 60 }.from(), Some(Route::Ground));
    }
}
