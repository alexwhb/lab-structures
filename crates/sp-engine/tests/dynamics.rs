//! `sp_engine::dynamics` against closed forms (TW-4): RK4's order, free decay frequency and
//! logarithmic decrement, statics, the steady-state amplitude at resonance, the tuned mass
//! damper's notch and Den Hartog's peak reduction, Newmark's unconditional stability, the seeded
//! gusts, and the tower's Play gusts model.

use sp_engine::dynamics::{
    Gusts, Linear, Newmark, SwayParams, SwaySim, den_hartog, invert, playback_speed, rk4_step,
    rk4_step2,
};
use sp_engine::tower::{self, TowerInput};
use std::f64::consts::PI;

/// RK4 is 4th order: halving the step cuts the error ~16×.
#[test]
fn rk4_is_fourth_order() {
    let err = |h: f64| {
        let mut y = [1.0];
        let mut t = 0.0;
        let n = (1.0 / h).round() as usize;
        for _ in 0..n {
            y = rk4_step(|_, y: &[f64; 1]| [-y[0]], t, &y, h);
            t += h;
        }
        (y[0] - (-1.0_f64).exp()).abs()
    };
    let (e1, e2) = (err(0.1), err(0.05));
    let order = (e1 / e2).log2();
    assert!((order - 4.0).abs() < 0.15, "order {order}");
    assert!(e2 < 1e-7);
}

/// A damped oscillator: x(t) from (x0 = 1, v0 = 0) for `cycles` periods with `steps` per
/// period, by RK4 or Newmark. Returns samples (t, x).
fn free_decay(f: f64, zeta: f64, cycles: f64, steps: f64, newmark: bool) -> Vec<(f64, f64)> {
    let sys = Linear::sdof(2.0, f, zeta);
    let h = 1.0 / f / steps;
    let n = (cycles * steps) as usize;
    let mut out = Vec::with_capacity(n + 1);
    if newmark {
        let mut nm = Newmark::new(sys, h, [1.0], [0.0], [0.0]).unwrap();
        out.push((0.0, 1.0));
        for _ in 0..n {
            nm.step([0.0]);
            out.push((nm.t, nm.x[0]));
        }
    } else {
        let m_inv = invert(&sys.m).unwrap();
        let (mut x, mut v, mut t) = ([1.0], [0.0], 0.0);
        out.push((0.0, 1.0));
        for _ in 0..n {
            (x, v) = rk4_step2(|_, x, v| sys.accel(&m_inv, x, v, &[0.0]), t, &x, &v, h);
            t += h;
            out.push((t, x[0]));
        }
    }
    out
}

/// Upward zero crossings (linear interpolation) and positive peaks.
fn crossings_and_peaks(s: &[(f64, f64)]) -> (Vec<f64>, Vec<f64>) {
    let mut ups = Vec::new();
    let mut peaks = Vec::new();
    for w in s.windows(3) {
        let ((t0, x0), (t1, x1), (_, x2)) = (w[0], w[1], w[2]);
        if x0 < 0.0 && x1 >= 0.0 {
            ups.push(t0 + (t1 - t0) * (-x0) / (x1 - x0));
        }
        if x1 > x0 && x1 >= x2 && x1 > 0.0 {
            // Parabolic refinement of the peak height.
            let denom = x0 - 2.0 * x1 + x2;
            let peak = if denom.abs() > 1e-15 {
                x1 - (x0 - x2).powi(2) / (8.0 * denom)
            } else {
                x1
            };
            peaks.push(peak);
        }
    }
    (ups, peaks)
}

#[test]
fn free_decay_has_the_damped_frequency_and_log_decrement() {
    let (f, zeta) = (1.0, 0.02);
    for newmark in [false, true] {
        let s = free_decay(f, zeta, 12.0, 200.0, newmark);
        let (ups, peaks) = crossings_and_peaks(&s);
        assert!(ups.len() >= 10 && peaks.len() >= 10);
        let td = (ups[ups.len() - 1] - ups[0]) / (ups.len() - 1) as f64;
        let fd = f * (1.0 - zeta * zeta).sqrt();
        assert!(
            (1.0 / td - fd).abs() / fd < 2e-4,
            "newmark={newmark}: fd {} vs {fd}",
            1.0 / td
        );
        let n = peaks.len() - 1;
        let dec = (peaks[0] / peaks[n]).ln() / n as f64;
        let expect = 2.0 * PI * zeta / (1.0 - zeta * zeta).sqrt();
        assert!(
            (dec - expect).abs() / expect < 0.01,
            "newmark={newmark}: decrement {dec} vs {expect}"
        );
    }
}

#[test]
fn a_constant_force_settles_at_the_static_deflection() {
    let sys = Linear::sdof(1000.0, 0.5, 0.2);
    let k = sys.k[0][0];
    let mut nm = Newmark::new(sys, 0.05, [0.0], [0.0], [5000.0]).unwrap();
    for _ in 0..2000 {
        nm.step([5000.0]);
    }
    assert!((nm.x[0] - 5000.0 / k).abs() < 1e-9 * (5000.0 / k).max(1.0));
    assert_eq!(sys.statics(&[5000.0]).unwrap()[0], 5000.0 / k);
}

/// Steady-state amplitude under `F0·sin(2πf t)` after transients die.
fn steady_amplitude<const N: usize>(sys: Linear<N>, f_drive: f64, f0: f64, h: f64) -> [f64; N] {
    let mut nm = Newmark::new(sys, h, [0.0; N], [0.0; N], [0.0; N]).unwrap();
    let period = 1.0 / f_drive;
    let total = 400.0 * period;
    let mut amp = [0.0_f64; N];
    let force = |t: f64| {
        let mut f = [0.0; N];
        f[0] = f0 * (2.0 * PI * f_drive * t).sin();
        f
    };
    while nm.t < total {
        nm.step(force(nm.t + h));
        if nm.t > total - 20.0 * period {
            for i in 0..N {
                amp[i] = amp[i].max(nm.x[i].abs());
            }
        }
    }
    amp
}

#[test]
fn resonance_amplitude_is_f0_over_2_zeta_k() {
    let (m, f, zeta, f0) = (1.0e6, 0.2, 0.05, 1.0e5);
    let sys = Linear::sdof(m, f, zeta);
    let k = sys.k[0][0];
    let amp = steady_amplitude(sys, f, f0, 1.0 / f / 64.0)[0];
    let expect = f0 / (2.0 * zeta * k);
    assert!(
        (amp - expect).abs() / expect < 0.01,
        "amplitude {amp} vs {expect}"
    );
}

/// The complex frequency response of the main mass, |X1| / (F0/k1), solved exactly.
fn frf_main(sys: &Linear<2>, f: f64) -> f64 {
    let w = 2.0 * PI * f;
    // A = K − ω²M + iωC (2×2 complex); solve A·X = [1, 0].
    let a = |i: usize, j: usize| (sys.k[i][j] - w * w * sys.m[i][j], w * sys.c[i][j]);
    let mul = |(a, b): (f64, f64), (c, d): (f64, f64)| (a * c - b * d, a * d + b * c);
    let sub = |(a, b): (f64, f64), (c, d): (f64, f64)| (a - c, b - d);
    let det = sub(mul(a(0, 0), a(1, 1)), mul(a(0, 1), a(1, 0)));
    // X1 = A11 / det.
    let (nr, ni) = a(1, 1);
    let d2 = det.0 * det.0 + det.1 * det.1;
    let x1 = (
        (nr * det.0 + ni * det.1) / d2,
        (ni * det.0 - nr * det.1) / d2,
    );
    let k1 = sys.k[0][0] + sys.k[0][1]; // k1 + kd − kd
    (x1.0 * x1.0 + x1.1 * x1.1).sqrt() * k1
}

#[test]
fn an_undamped_absorber_is_a_notch_at_its_frequency() {
    let (m, f, mu) = (1.0e6, 0.1, 0.02);
    let sys = Linear::with_absorber(m, f, 0.015, mu, f, 0.0);
    // Exactly at the absorber's frequency the main mass doesn't move.
    assert!(frf_main(&sys, f) < 1e-9, "{}", frf_main(&sys, f));
    // Without it the same mass at the same frequency swings 1/(2ζ) = 33× its static deflection.
    let bare = Linear::with_absorber(m, f, 0.015, mu, 0.0, 0.0); // absorber disconnected
    assert!((frf_main(&bare, f) - 1.0 / 0.03).abs() < 1e-6);
}

#[test]
fn den_hartog_tuning_cuts_the_resonant_peak() {
    let (m, f, zeta, mu) = (1.0e6, 0.1, 0.015, 0.02);
    let (fd, zd) = den_hartog(f, mu);
    assert!((fd - f / 1.02).abs() < 1e-15);
    assert!((zd - (3.0 * mu / (8.0 * 1.02_f64.powi(3))).sqrt()).abs() < 1e-15);
    let tmd = Linear::with_absorber(m, f, zeta, mu, fd, zd);
    let bare = Linear::sdof(m, f, zeta);
    let bare2 = Linear::with_absorber(m, f, zeta, mu, 0.0, 0.0); // absorber disconnected
    let sweep = |sys: &Linear<2>| {
        (0..=2000)
            .map(|i| frf_main(sys, f * (0.6 + 0.8 * i as f64 / 2000.0)))
            .fold(0.0, f64::max)
    };
    let (peak_tmd, peak_bare) = (sweep(&tmd), sweep(&bare2));
    assert!((peak_bare - 1.0 / (2.0 * zeta * (1.0 - zeta * zeta).sqrt())).abs() < 0.05);
    // Den Hartog: √(1 + 2/μ) ≈ 10 for an undamped main mass; less with its own damping.
    assert!(peak_tmd < (1.0 + 2.0 / mu).sqrt(), "TMD peak {peak_tmd}");
    assert!(
        peak_bare / peak_tmd > 3.0,
        "reduction {}",
        peak_bare / peak_tmd
    );

    // The time-domain integrator agrees with the exact frequency response.
    let k1 = bare.k[0][0];
    for ratio in [0.9, 1.0, 1.08] {
        let amp = steady_amplitude(tmd, f * ratio, 1.0, 1.0 / f / 96.0)[0] * k1;
        let exact = frf_main(&tmd, f * ratio);
        assert!(
            (amp - exact).abs() / exact < 0.01,
            "at {ratio}: {amp} vs {exact}"
        );
    }
}

/// Average acceleration is unconditionally stable and conserves energy without damping.
#[test]
fn newmark_is_unconditionally_stable() {
    // A stiff 2-DOF system (frequencies 1 Hz and ~1000 Hz) with a step of 0.01 s: ωh ≈ 63 on
    // the stiff mode, where any explicit method blows up.
    let sys = Linear {
        m: [[1.0, 0.0], [0.0, 1e-6]],
        c: [[0.0, 0.0], [0.0, 0.0]],
        k: [[(2.0 * PI).powi(2) + 40.0, -40.0], [-40.0, 40.0]],
    };
    let mut nm = Newmark::new(sys, 0.01, [1.0, 1.0], [0.0, 0.0], [0.0, 0.0]).unwrap();
    let energy = |nm: &Newmark<2>| {
        let (x, v) = (nm.x, nm.v);
        let ke = 0.5 * (sys.m[0][0] * v[0] * v[0] + sys.m[1][1] * v[1] * v[1]);
        let pe = 0.5
            * (sys.k[0][0] * x[0] * x[0]
                + 2.0 * sys.k[0][1] * x[0] * x[1]
                + sys.k[1][1] * x[1] * x[1]);
        ke + pe
    };
    let e0 = energy(&nm);
    for _ in 0..100_000 {
        nm.step([0.0, 0.0]);
    }
    let e1 = energy(&nm);
    assert!(e1.is_finite() && (e1 - e0).abs() / e0 < 1e-6, "{e0} → {e1}");
    // And an explicit RK4 with the same step on the same system diverges.
    let m_inv = invert(&sys.m).unwrap();
    let (mut x, mut v) = ([1.0, 1.0], [0.0, 0.0]);
    for i in 0..200 {
        (x, v) = rk4_step2(
            |_, x, v| sys.accel(&m_inv, x, v, &[0.0, 0.0]),
            i as f64 * 0.01,
            &x,
            &v,
            0.01,
        );
    }
    let blew_up = x[1].is_nan() || x[1].abs() >= 1e3;
    assert!(blew_up, "RK4 should blow up here: {x:?}");
}

#[test]
fn bad_inputs_are_errors_not_panics() {
    let sys = Linear::sdof(1.0, 1.0, 0.0);
    assert!(Newmark::new(sys, 0.0, [0.0], [0.0], [0.0]).is_err());
    assert!(Newmark::new(sys, f64::NAN, [0.0], [0.0], [0.0]).is_err());
    let zero_mass = Linear::sdof(0.0, 1.0, 0.0);
    assert!(Newmark::new(zero_mass, 0.01, [0.0], [0.0], [0.0]).is_err());
}

#[test]
fn gusts_are_seeded_zero_mean_unit_variance() {
    let sample = |seed: u64| {
        let mut g = Gusts::new(seed, 10.0, 0.5);
        (0..200_000)
            .map(|i| g.at(i as f64 * 1.0))
            .collect::<Vec<_>>()
    };
    let a = sample(7);
    assert_eq!(a, sample(7), "same seed, same series");
    assert_ne!(a[..100], sample(8)[..100], "another seed, another series");
    let mean = a.iter().sum::<f64>() / a.len() as f64;
    let var = a.iter().map(|g| (g - mean).powi(2)).sum::<f64>() / a.len() as f64;
    assert!(mean.abs() < 0.05, "mean {mean}");
    assert!((var - 1.0).abs() < 0.1, "variance {var}");
    // Correlated over ~tau: neighbours 1 s apart are close, 100 s apart independent.
    let corr = |lag: usize| {
        let n = a.len() - lag;
        (0..n).map(|i| a[i] * a[i + lag]).sum::<f64>() / n as f64
    };
    assert!((corr(1) - (-0.1_f64).exp()).abs() < 0.05, "{}", corr(1));
    assert!(corr(100).abs() < 0.05, "{}", corr(100));
    // Garbage in, finite out.
    let mut g = Gusts::new(1, f64::NAN, -1.0);
    assert!(g.at(5.0).is_finite() && g.at(f64::INFINITY).is_finite());
}

fn rms_swing(params: SwayParams, periods: f64) -> (f64, f64) {
    let mut sim = SwaySim::new(params);
    let period = 1.0 / params.f1;
    let (mut sum, mut sum2, mut n) = (0.0, 0.0, 0.0);
    let dt = period / 30.0;
    let mut t = 0.0;
    while t < periods * period {
        sim.advance(dt);
        t += dt;
        if t > 5.0 * period {
            sum += sim.top();
            sum2 += sim.swing().powi(2);
            n += 1.0;
        }
    }
    (sum / n, (sum2 / n).sqrt())
}

#[test]
fn play_gusts_sways_around_the_drawn_lean_and_the_damper_calms_it() {
    let input = TowerInput::default();
    let r = tower::calc(&input);
    let p = tower::sway_params(&input, &r, 42);
    assert!((p.mean_top - 0.85 * r.delta).abs() < 1e-12);
    assert_eq!(p.mass_ratio, 0.0);
    let (mean, rms) = rms_swing(p, 600.0);
    assert!(
        (mean - p.mean_top).abs() / p.mean_top < 0.05,
        "mean {mean} vs {}",
        p.mean_top
    );
    // Calibrated to the mockup's dynamic amplitude: rms ≈ dyn / 3, so gust peaks (≈ 3σ) are
    // about `dyn` (materials' review: 0.08 turbulence gave rms 1.4 × dyn).
    println!(
        "rms {rms:.4} m, dyn {:.4} m, ratio {:.3}",
        r.dyn_amp,
        rms / r.dyn_amp
    );
    assert!(
        rms > 0.2 * r.dyn_amp && rms < 0.5 * r.dyn_amp,
        "rms {rms} vs dyn {}",
        r.dyn_amp
    );
    let damped_in = TowerInput {
        damper: true,
        ..input
    };
    let rd = tower::calc(&damped_in);
    let pd = tower::sway_params(&damped_in, &rd, 42);
    assert!(pd.mass_ratio > 0.0);
    let (_, rms_d) = rms_swing(pd, 600.0);
    assert!(rms_d < 0.75 * rms, "damper: {rms_d} vs {rms}");
}

/// At the defaults the drawn top never swings through vertical into the wind, for any seed
/// (500 periods each), and its peaks stay within about 1.5 × `dyn` of the lean.
#[test]
fn the_default_tower_never_swings_upwind() {
    let input = TowerInput::default();
    let r = tower::calc(&input);
    for seed in [1, 7, 42, 99, 12345] {
        let p = tower::sway_params(&input, &r, seed);
        let mut sim = SwaySim::new(p);
        let period = 1.0 / p.f1;
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for _ in 0..(500 * 30) {
            sim.advance(period / 30.0);
            lo = lo.min(sim.top());
            hi = hi.max(sim.top());
        }
        assert!(lo > 0.0, "seed {seed}: the top reaches {lo} m (upwind)");
        let peak = (hi - p.mean_top).max(p.mean_top - lo);
        assert!(
            peak < 1.6 * r.dyn_amp,
            "seed {seed}: peak swing {peak} m vs dyn {}",
            r.dyn_amp
        );
    }
}

#[test]
fn lock_in_makes_it_swing_harder() {
    // A braced tube at 60 floors in 30 m/s sits in the lock-in band.
    let input = TowerInput {
        system: tower::System::Braced,
        ..TowerInput::default()
    };
    let r = tower::calc(&input);
    assert!(r.lock, "ratio {}", r.ratio);
    let p = tower::sway_params(&input, &r, 3);
    assert!(p.vortex > 0.0);
    let (_, rms_lock) = rms_swing(p, 400.0);
    let (_, rms_free) = rms_swing(SwayParams { vortex: 0.0, ..p }, 400.0);
    assert!(rms_lock > 1.3 * rms_free, "{rms_lock} vs {rms_free}");
}

#[test]
fn sway_is_deterministic_and_retunes_without_a_jump() {
    let input = TowerInput::default();
    let r = tower::calc(&input);
    let p = tower::sway_params(&input, &r, 9);
    let run = || {
        let mut s = SwaySim::new(p);
        (0..500)
            .map(|_| {
                s.advance(0.05);
                s.top()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());

    let mut s = SwaySim::new(p);
    s.advance(30.0);
    let before = s.top();
    let swing = s.swing();
    let taller = TowerInput {
        floors: 61,
        damper: true,
        ..input
    };
    let r2 = tower::calc(&taller);
    s.retune(tower::sway_params(&taller, &r2, 9));
    assert!(s.has_damper());
    assert!((s.swing() - swing).abs() < 1e-12);
    assert!(
        (s.top() - before).abs() < 0.1 * r.delta,
        "jump {before} → {}",
        s.top()
    );
}

#[test]
fn garbage_parameters_do_not_panic() {
    for p in [
        SwayParams {
            f1: f64::NAN,
            zeta: -1.0,
            modal_mass: 0.0,
            mass_ratio: f64::INFINITY,
            mean_top: f64::NAN,
            turbulence: f64::NAN,
            vortex: -3.0,
            fs: f64::NAN,
            seed: 0,
        },
        SwayParams {
            f1: 1e9,
            zeta: 0.0,
            modal_mass: 1e30,
            mass_ratio: 0.5,
            mean_top: 1e9,
            turbulence: 1.0,
            vortex: 1.0,
            fs: 1e9,
            seed: u64::MAX,
        },
    ] {
        let mut s = SwaySim::new(p);
        s.advance(1.0);
        s.advance(f64::NAN);
        s.advance(1e12);
        assert!(s.top().is_finite() && s.damper_stroke().is_finite() && s.time().is_finite());
    }
}

#[test]
fn playback_runs_the_default_tower_at_the_mockups_pace() {
    let r = tower::calc(&TowerInput::default());
    let shown = playback_speed(r.f1) * r.f1;
    assert!((shown - 0.55).abs() < 0.01, "{shown}");
    // Taller sways visibly slower, shorter faster, within 0.25–1.2 Hz.
    let tall = tower::calc(&TowerInput {
        floors: 120,
        ..TowerInput::default()
    });
    let short = tower::calc(&TowerInput {
        floors: 5,
        ..TowerInput::default()
    });
    let (st, ss) = (
        playback_speed(tall.f1) * tall.f1,
        playback_speed(short.f1) * short.f1,
    );
    assert!(st < shown && shown < ss && st >= 0.25 && ss <= 1.2);
    assert!(playback_speed(f64::NAN).is_finite());
}

/// ENG-4-style budget: a second of animation (30 frames) of the 2-DOF model is cheap.
#[test]
fn sway_steps_are_cheap() {
    let input = TowerInput {
        damper: true,
        ..TowerInput::default()
    };
    let r = tower::calc(&input);
    let mut s = SwaySim::new(tower::sway_params(&input, &r, 1));
    let speed = playback_speed(r.f1);
    let t0 = std::time::Instant::now();
    for _ in 0..30 * 60 {
        s.advance(speed / 30.0);
    }
    let per_frame = t0.elapsed().as_secs_f64() / (30.0 * 60.0);
    assert!(per_frame < 2e-4, "{per_frame} s per frame");
}
