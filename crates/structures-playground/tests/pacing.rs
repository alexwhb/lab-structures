//! Frame pacing (PLAT-3): a screen at rest asks for no frames (0 % CPU when idle); a toast asks
//! for a few until it fades; the smoke path runs headless and prints its line (PLAT-1).

mod common;

use common::*;
use eframe::egui::ViewportId;
use std::time::Duration;
use structures_playground::Route;
use structures_playground::smoke::{SmokeArgs, run_headless};

/// The repaint the last frame asked for.
fn asked(h: &App) -> Duration {
    h.output().viewport_output[&ViewportId::ROOT].repaint_delay
}

#[test]
fn idle_screens_ask_for_no_frames() {
    let mut h = harness(Route::Series);
    for r in Route::ALL {
        goto(&mut h, r);
        h.run_steps(6);
        assert!(
            asked(&h) >= Duration::from_secs(1),
            "{r:?} at rest asks for a frame in {:?} ({:?})",
            asked(&h),
            h.ctx.repaint_causes()
        );
    }
}

#[test]
fn a_toast_paces_itself_and_fades() {
    let mut h = harness(Route::Series);
    h.state_mut().toast("Copied");
    h.step();
    let d = asked(&h);
    assert!(d <= Duration::from_millis(250), "{d:?}");
    // it's gone after 2.6 s of egui time (kittest steps 1/60 s)
    assert!(step_until(&mut h, 400, |h| toasts(h).is_empty()));
    h.run_steps(4);
    assert!(asked(&h) >= Duration::from_secs(1), "{:?}", asked(&h));
}

#[test]
fn smoke_line_has_the_fields() {
    let r = run_headless(&SmokeArgs {
        seconds: 0.3,
        headless: true,
    });
    let line = r.line();
    assert!(r.ok(), "{line}");
    for field in [
        "route=bridge",
        "mode=headless",
        "first_frame_ms=",
        "frames=",
        "status=ok",
    ] {
        assert!(line.contains(field), "{line}");
    }
}
