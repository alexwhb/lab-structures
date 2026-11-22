//! Structural dynamics: a small, tested ODE toolkit and the Tower's "Play gusts" model (TW-4).
//!
//! - [`rk4_step`] / [`rk4_step2`]: classical fixed-step Runge–Kutta 4 for `y' = f(t, y)` and for
//!   second-order systems `ẍ = a(t, x, ẋ)` (explicit, 4th order; fine for non-stiff problems).
//! - [`Linear`] + [`Newmark`]: linear `M·ẍ + C·ẋ + K·x = F(t)` with `N` degrees of freedom,
//!   integrated by Newmark's average-acceleration method (γ = ½, β = ¼: implicit, second order,
//!   **unconditionally stable** and free of numerical damping, so it is safe for stiff systems
//!   and any step).
//! - [`Gusts`]: a deterministic, seeded gust series (an Ornstein–Uhlenbeck process, unit
//!   variance, sampled on a fixed grid and interpolated), the turbulence around the mean wind.
//! - [`SwaySim`]: the tower's first mode as a damped single-degree-of-freedom oscillator
//!   `m·ẍ + c·ẋ + k·x = F(t)`, or, with the tuned mass damper, a two-degree-of-freedom system
//!   (Den Hartog tuning), driven by `F(t) = k·x̄·(1 + 2·Iᵤ·g(t) + v·sin 2πfₛt)`: gusts around
//!   the mean force that holds the mean lean `x̄`, plus vortex shedding near lock-in.
//!   [`crate::tower::sway_params`] builds its parameters from a tower.
//!
//! Tests (`tests/dynamics.rs`) check the integrators against closed forms: free decay frequency
//! and logarithmic decrement, the static solution, the steady-state amplitude at resonance
//! `F₀/(2ζk)`, and the damper's notch (a tuned absorber cancels the main mass's response at
//! its frequency; Den Hartog's tuning cuts the resonant peak about 3×).
//!
//! Nothing panics: non-finite or non-positive parameters are replaced by safe values.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// An `N × N` matrix (row major).
pub type Mat<const N: usize> = [[f64; N]; N];

/// One classical RK4 step of `y' = f(t, y)` from `(t, y)` with step `h`.
pub fn rk4_step<const N: usize>(
    f: impl Fn(f64, &[f64; N]) -> [f64; N],
    t: f64,
    y: &[f64; N],
    h: f64,
) -> [f64; N] {
    let add = |a: &[f64; N], b: &[f64; N], s: f64| -> [f64; N] {
        let mut o = *a;
        for i in 0..N {
            o[i] += s * b[i];
        }
        o
    };
    let k1 = f(t, y);
    let k2 = f(t + h / 2.0, &add(y, &k1, h / 2.0));
    let k3 = f(t + h / 2.0, &add(y, &k2, h / 2.0));
    let k4 = f(t + h, &add(y, &k3, h));
    let mut out = *y;
    for i in 0..N {
        out[i] += h / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
    }
    out
}

/// One RK4 step of the second-order system `ẍ = a(t, x, ẋ)`; returns the new `(x, ẋ)`.
pub fn rk4_step2<const N: usize>(
    a: impl Fn(f64, &[f64; N], &[f64; N]) -> [f64; N],
    t: f64,
    x: &[f64; N],
    v: &[f64; N],
    h: f64,
) -> ([f64; N], [f64; N]) {
    let axpy = |y: &[f64; N], d: &[f64; N], s: f64| -> [f64; N] {
        let mut o = *y;
        for i in 0..N {
            o[i] += s * d[i];
        }
        o
    };
    let (k1x, k1v) = (*v, a(t, x, v));
    let (x2, v2) = (axpy(x, &k1x, h / 2.0), axpy(v, &k1v, h / 2.0));
    let (k2x, k2v) = (v2, a(t + h / 2.0, &x2, &v2));
    let (x3, v3) = (axpy(x, &k2x, h / 2.0), axpy(v, &k2v, h / 2.0));
    let (k3x, k3v) = (v3, a(t + h / 2.0, &x3, &v3));
    let (x4, v4) = (axpy(x, &k3x, h), axpy(v, &k3v, h));
    let (k4x, k4v) = (v4, a(t + h, &x4, &v4));
    let mut xo = *x;
    let mut vo = *v;
    for i in 0..N {
        xo[i] += h / 6.0 * (k1x[i] + 2.0 * k2x[i] + 2.0 * k3x[i] + k4x[i]);
        vo[i] += h / 6.0 * (k1v[i] + 2.0 * k2v[i] + 2.0 * k3v[i] + k4v[i]);
    }
    (xo, vo)
}

/// `A⁻¹` by Gauss–Jordan elimination with partial pivoting; `None` when (numerically) singular
/// or not finite.
pub fn invert<const N: usize>(a: &Mat<N>) -> Option<Mat<N>> {
    let mut m = *a;
    let mut inv = [[0.0; N]; N];
    for (i, row) in inv.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    let scale = m
        .iter()
        .flat_map(|r| r.iter())
        .fold(0.0_f64, |s, v| s.max(v.abs()));
    if !scale.is_finite() || scale == 0.0 {
        return None;
    }
    for col in 0..N {
        let piv = (col..N).max_by(|&i, &j| m[i][col].abs().total_cmp(&m[j][col].abs()))?;
        if m[piv][col].abs() <= 1e-14 * scale {
            return None;
        }
        m.swap(col, piv);
        inv.swap(col, piv);
        let d = m[col][col];
        for j in 0..N {
            m[col][j] /= d;
            inv[col][j] /= d;
        }
        for i in 0..N {
            if i != col {
                let f = m[i][col];
                if f != 0.0 {
                    for j in 0..N {
                        m[i][j] -= f * m[col][j];
                        inv[i][j] -= f * inv[col][j];
                    }
                }
            }
        }
    }
    inv.iter()
        .all(|r| r.iter().all(|v| v.is_finite()))
        .then_some(inv)
}

fn mat_vec<const N: usize>(a: &Mat<N>, x: &[f64; N]) -> [f64; N] {
    let mut o = [0.0; N];
    for i in 0..N {
        for j in 0..N {
            o[i] += a[i][j] * x[j];
        }
    }
    o
}

/// Why a linear system can't be integrated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DynamicsError {
    /// The mass matrix is singular (or not finite).
    #[error("the mass matrix is singular")]
    SingularMass,
    /// The effective stiffness of the implicit step is singular (or not finite).
    #[error("the effective stiffness is singular")]
    SingularStep,
    /// The step is not a positive finite number.
    #[error("the time step must be positive and finite")]
    BadStep,
}

/// A linear structural system `M·ẍ + C·ẋ + K·x = F(t)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Linear<const N: usize> {
    /// Mass matrix.
    pub m: Mat<N>,
    /// Damping matrix.
    pub c: Mat<N>,
    /// Stiffness matrix.
    pub k: Mat<N>,
}

impl Linear<1> {
    /// A single-degree-of-freedom oscillator of mass `m`, natural frequency `f` (Hz) and damping
    /// ratio `zeta`.
    pub fn sdof(m: f64, f: f64, zeta: f64) -> Self {
        let w = 2.0 * PI * f;
        Self {
            m: [[m]],
            c: [[2.0 * zeta * m * w]],
            k: [[m * w * w]],
        }
    }
}

impl Linear<2> {
    /// A main mass `m` (frequency `f`, damping `zeta`) carrying a tuned mass damper of mass
    /// `mu·m` with its own frequency `fd` and damping ratio `zd`. Degrees of freedom: the main
    /// mass and the damper mass (absolute displacements).
    pub fn with_absorber(m: f64, f: f64, zeta: f64, mu: f64, fd: f64, zd: f64) -> Self {
        let w = 2.0 * PI * f;
        let wd = 2.0 * PI * fd;
        let (k1, c1) = (m * w * w, 2.0 * zeta * m * w);
        let md = mu * m;
        let (kd, cd) = (md * wd * wd, 2.0 * zd * md * wd);
        Self {
            m: [[m, 0.0], [0.0, md]],
            c: [[c1 + cd, -cd], [-cd, cd]],
            k: [[k1 + kd, -kd], [-kd, kd]],
        }
    }
}

impl<const N: usize> Linear<N> {
    /// Accelerations `M⁻¹(F − C·ẋ − K·x)` (for explicit integrators); `m_inv` from [`invert`].
    pub fn accel(&self, m_inv: &Mat<N>, x: &[f64; N], v: &[f64; N], f: &[f64; N]) -> [f64; N] {
        let cv = mat_vec(&self.c, v);
        let kx = mat_vec(&self.k, x);
        let mut r = [0.0; N];
        for i in 0..N {
            r[i] = f[i] - cv[i] - kx[i];
        }
        mat_vec(m_inv, &r)
    }

    /// The static solution `K⁻¹·F`.
    pub fn statics(&self, f: &[f64; N]) -> Option<[f64; N]> {
        invert(&self.k).map(|ki| mat_vec(&ki, f))
    }
}

/// Den Hartog's optimal tuning of a damper with mass ratio `mu` on an undamped main mass of
/// frequency `f`: `(fd, ζd) = (f/(1+μ), √(3μ / (8(1+μ)³)))`.
pub fn den_hartog(f: f64, mu: f64) -> (f64, f64) {
    (
        f / (1.0 + mu),
        (3.0 * mu / (8.0 * (1.0 + mu).powi(3))).sqrt(),
    )
}

/// Newmark average-acceleration integration (γ = ½, β = ¼) of a [`Linear`] system with a fixed
/// step: implicit, unconditionally stable for any step, second-order accurate, no numerical
/// damping (the period lengthens by about `(ωh)²/12`).
#[derive(Debug, Clone)]
pub struct Newmark<const N: usize> {
    sys: Linear<N>,
    h: f64,
    keff_inv: Mat<N>,
    /// Displacements.
    pub x: [f64; N],
    /// Velocities.
    pub v: [f64; N],
    /// Accelerations.
    pub a: [f64; N],
    /// Time.
    pub t: f64,
}

impl<const N: usize> Newmark<N> {
    const BETA: f64 = 0.25;
    const GAMMA: f64 = 0.5;

    /// Starts from `(x0, v0)` at `t = 0` under the force `f0` (for the initial acceleration),
    /// stepping by `h`.
    pub fn new(
        sys: Linear<N>,
        h: f64,
        x0: [f64; N],
        v0: [f64; N],
        f0: [f64; N],
    ) -> Result<Self, DynamicsError> {
        if !(h.is_finite() && h > 0.0) {
            return Err(DynamicsError::BadStep);
        }
        let m_inv = invert(&sys.m).ok_or(DynamicsError::SingularMass)?;
        let (b, g) = (Self::BETA, Self::GAMMA);
        let mut keff = sys.k;
        for i in 0..N {
            for j in 0..N {
                keff[i][j] += g / (b * h) * sys.c[i][j] + 1.0 / (b * h * h) * sys.m[i][j];
            }
        }
        let keff_inv = invert(&keff).ok_or(DynamicsError::SingularStep)?;
        let a = sys.accel(&m_inv, &x0, &v0, &f0);
        Ok(Self {
            sys,
            h,
            keff_inv,
            x: x0,
            v: v0,
            a,
            t: 0.0,
        })
    }

    /// The step.
    pub fn step_size(&self) -> f64 {
        self.h
    }

    /// The system.
    pub fn system(&self) -> &Linear<N> {
        &self.sys
    }

    /// Advances one step; `f_next` is the force at the end of the step.
    pub fn step(&mut self, f_next: [f64; N]) {
        let (b, g, h) = (Self::BETA, Self::GAMMA, self.h);
        let mut mp = [0.0; N];
        let mut cp = [0.0; N];
        for i in 0..N {
            mp[i] =
                self.x[i] / (b * h * h) + self.v[i] / (b * h) + (1.0 / (2.0 * b) - 1.0) * self.a[i];
            cp[i] = g / (b * h) * self.x[i]
                + (g / b - 1.0) * self.v[i]
                + h * (g / (2.0 * b) - 1.0) * self.a[i];
        }
        let mpm = mat_vec(&self.sys.m, &mp);
        let cpc = mat_vec(&self.sys.c, &cp);
        let mut p = f_next;
        for i in 0..N {
            p[i] += mpm[i] + cpc[i];
        }
        let x1 = mat_vec(&self.keff_inv, &p);
        let mut a1 = [0.0; N];
        let mut v1 = [0.0; N];
        for i in 0..N {
            a1[i] = (x1[i] - self.x[i]) / (b * h * h)
                - self.v[i] / (b * h)
                - (1.0 / (2.0 * b) - 1.0) * self.a[i];
            v1[i] = self.v[i] + h * ((1.0 - g) * self.a[i] + g * a1[i]);
        }
        if x1.iter().chain(&v1).chain(&a1).all(|v| v.is_finite()) {
            self.x = x1;
            self.v = v1;
            self.a = a1;
        }
        self.t += h;
    }
}

// ============================================================================================
// Gusts

/// SplitMix64: a tiny, high-quality seeded generator (no dependencies, reproducible).
#[derive(Debug, Clone)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    /// A generator from `seed`.
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in (0, 1).
    pub fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    /// Standard normal (Box–Muller).
    pub fn normal(&mut self) -> f64 {
        let (u1, u2) = (self.uniform(), self.uniform());
        (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos()
    }
}

/// A deterministic gust series `g(t)`: zero mean, unit variance, correlation time `tau`
/// (an Ornstein–Uhlenbeck process sampled every `dt` and linearly interpolated). Query it with
/// non-decreasing `t` ([`Gusts::at`]); the same seed gives the same series.
#[derive(Debug, Clone)]
pub struct Gusts {
    rng: SplitMix64,
    dt: f64,
    decay: f64,
    kick: f64,
    t0: f64,
    g0: f64,
    g1: f64,
}

impl Gusts {
    /// A series with correlation time `tau` sampled every `dt` (both seconds; non-positive or
    /// non-finite values become 1 s and `tau/16`).
    pub fn new(seed: u64, tau: f64, dt: f64) -> Self {
        let tau = if tau.is_finite() && tau > 0.0 {
            tau
        } else {
            1.0
        };
        let dt = if dt.is_finite() && dt > 0.0 {
            dt
        } else {
            tau / 16.0
        };
        let decay = (-dt / tau).exp();
        let kick = (1.0 - decay * decay).sqrt();
        let mut rng = SplitMix64::new(seed);
        let g0 = rng.normal();
        let g1 = decay * g0 + kick * rng.normal();
        Self {
            rng,
            dt,
            decay,
            kick,
            t0: 0.0,
            g0,
            g1,
        }
    }

    /// `g(t)` for `t ≥` the last query (earlier times return the current interval's value).
    pub fn at(&mut self, t: f64) -> f64 {
        if !t.is_finite() {
            return self.g0;
        }
        // Skip far ahead in bounded work: at most 100 000 samples per call.
        let mut guard = 0;
        while t >= self.t0 + self.dt && guard < 100_000 {
            self.t0 += self.dt;
            self.g0 = self.g1;
            self.g1 = self.decay * self.g0 + self.kick * self.rng.normal();
            guard += 1;
        }
        let s = ((t - self.t0) / self.dt).clamp(0.0, 1.0);
        self.g0 + (self.g1 - self.g0) * s
    }
}

// ============================================================================================
// The tower's sway

/// Parameters of the Play gusts model ([`crate::tower::sway_params`] builds them).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SwayParams {
    /// First natural frequency, Hz.
    pub f1: f64,
    /// Structural damping ratio of the first mode.
    pub zeta: f64,
    /// Generalised (modal) mass of the first mode, kg.
    pub modal_mass: f64,
    /// Tuned mass damper mass / modal mass (0 = no damper).
    pub mass_ratio: f64,
    /// Mean top deflection under the mean wind, m.
    pub mean_top: f64,
    /// Gust turbulence intensity `σᵤ/V̄` (the force fluctuates by `2·Iᵤ·g(t)`).
    pub turbulence: f64,
    /// Vortex-shedding force amplitude as a fraction of the mean force.
    pub vortex: f64,
    /// Frequency of the vortex-shedding force, Hz (locked onto `f1` inside the lock-in band).
    pub fs: f64,
    /// Gust series seed.
    pub seed: u64,
}

impl SwayParams {
    /// Replaces non-finite or out-of-range values with safe ones.
    pub fn sanitized(self) -> Self {
        let pos = |v: f64, d: f64| if v.is_finite() && v > 0.0 { v } else { d };
        let nonneg = |v: f64| if v.is_finite() && v > 0.0 { v } else { 0.0 };
        Self {
            f1: pos(self.f1, 1.0).clamp(1e-4, 1e3),
            zeta: nonneg(self.zeta).min(2.0),
            modal_mass: pos(self.modal_mass, 1.0),
            mass_ratio: nonneg(self.mass_ratio).min(1.0),
            mean_top: if self.mean_top.is_finite() {
                self.mean_top
            } else {
                0.0
            },
            turbulence: nonneg(self.turbulence).min(1.0),
            vortex: nonneg(self.vortex).min(1.0),
            fs: nonneg(self.fs).min(1e4),
            seed: self.seed,
        }
    }
}

/// Integration steps per natural period.
pub const STEPS_PER_PERIOD: f64 = 48.0;
/// The gusts' correlation time, in natural periods.
pub const GUST_TAU_PERIODS: f64 = 1.0;
/// Gusts ramp in over this many periods after a (re)start.
pub const GUST_RAMP_PERIODS: f64 = 1.0;
/// The mockup's sway animation frequency (`sin(2π·0.55·t)`), Hz, at the reference tower.
pub const PLAYBACK_HZ: f64 = 0.55;
/// The reference tower's first frequency for [`playback_speed`] (the default tower), Hz.
pub const PLAYBACK_REF_F1: f64 = 0.1;

/// How much faster than real time Play gusts runs, so the sway reads on screen: the default
/// tower (f1 ≈ 0.1 Hz, a 9.9 s period) sways at the mockup's 0.55 Hz; taller towers still sway
/// visibly slower and short ones faster (`0.55·(f1/0.1)^0.3`, kept within 0.25–1.2 Hz).
pub fn playback_speed(f1: f64) -> f64 {
    let f1 = if f1.is_finite() && f1 > 0.0 { f1 } else { 1.0 };
    let shown = (PLAYBACK_HZ * (f1 / PLAYBACK_REF_F1).powf(0.3)).clamp(0.25, 1.2);
    shown / f1
}

#[derive(Debug, Clone)]
enum Integrator {
    One(Newmark<1>),
    Two(Newmark<2>),
}

/// The tower's first mode under gusting wind (see the module docs). Coordinates are the top
/// floor's along-wind displacement (the mode shape is 1 at the top), metres.
#[derive(Debug, Clone)]
pub struct SwaySim {
    p: SwayParams,
    k: f64,
    gusts: Gusts,
    integ: Integrator,
    pending: f64,
    ramp_from: f64,
}

impl SwaySim {
    /// Starts at rest at the mean lean.
    pub fn new(params: SwayParams) -> Self {
        Self::start(params, 0.0, 0.0, 0.0, 0.0)
    }

    fn start(params: SwayParams, dx: f64, dv: f64, stroke: f64, t: f64) -> Self {
        let p = params.sanitized();
        let period = 1.0 / p.f1;
        let h = period / STEPS_PER_PERIOD;
        let k = p.modal_mass * (2.0 * PI * p.f1).powi(2);
        let gusts = Gusts::new(
            p.seed,
            period * GUST_TAU_PERIODS,
            period / STEPS_PER_PERIOD * 2.0,
        );
        let mut sim = Self {
            p,
            k,
            gusts,
            integ: Integrator::One(fallback_sdof()),
            pending: 0.0,
            ramp_from: t,
        };
        let f0 = sim.force(t);
        let x1 = p.mean_top + dx;
        sim.integ = if p.mass_ratio > 0.0 {
            let (fd, zd) = den_hartog(p.f1, p.mass_ratio);
            let sys = Linear::with_absorber(p.modal_mass, p.f1, p.zeta, p.mass_ratio, fd, zd);
            match Newmark::new(sys, h, [x1, x1 + stroke], [dv, dv], [f0, 0.0]) {
                Ok(mut n) => {
                    n.t = t;
                    Integrator::Two(n)
                }
                Err(_) => Integrator::One(fallback_sdof()),
            }
        } else {
            let sys = Linear::sdof(p.modal_mass, p.f1, p.zeta);
            match Newmark::new(sys, h, [x1], [dv], [f0]) {
                Ok(mut n) => {
                    n.t = t;
                    Integrator::One(n)
                }
                Err(_) => Integrator::One(fallback_sdof()),
            }
        };
        sim
    }

    /// Changes the tower (floors, system, damper, wind…) while playing: the swing (offset from
    /// the mean and velocity) carries over, so the animation doesn't jump.
    pub fn retune(&mut self, params: SwayParams) {
        if params.sanitized() == self.p {
            return;
        }
        let (dx, dv) = (self.top() - self.p.mean_top, self.velocity());
        let stroke = self.damper_stroke();
        let t = self.time();
        *self = Self::start(params, dx, dv, stroke, t);
    }

    /// The parameters (sanitized).
    pub fn params(&self) -> &SwayParams {
        &self.p
    }

    /// The force on the first mode at time `t`, N.
    fn force(&mut self, t: f64) -> f64 {
        let period = 1.0 / self.p.f1;
        let ramp = ((t - self.ramp_from) / (period * GUST_RAMP_PERIODS)).clamp(0.0, 1.0);
        let ramp = ramp * ramp * (3.0 - 2.0 * ramp);
        let g = self.gusts.at(t);
        let vortex = self.p.vortex * (2.0 * PI * self.p.fs * t).sin();
        self.k * self.p.mean_top * (1.0 + ramp * (2.0 * self.p.turbulence * g + vortex))
    }

    /// Advances by `dt` seconds of simulated time (fixed internal steps of a 48th of the
    /// period; at most 20 000 steps per call, the rest is dropped).
    pub fn advance(&mut self, dt: f64) {
        if !(dt.is_finite() && dt > 0.0) {
            return;
        }
        let h = 1.0 / self.p.f1 / STEPS_PER_PERIOD;
        self.pending += dt;
        let mut steps = 0;
        while self.pending >= h && steps < 20_000 {
            let t_next = self.time() + h;
            let f = self.force(t_next);
            match &mut self.integ {
                Integrator::One(n) => n.step([f]),
                Integrator::Two(n) => n.step([f, 0.0]),
            }
            self.pending -= h;
            steps += 1;
        }
        if steps == 20_000 {
            self.pending = 0.0;
        }
    }

    /// Simulated time, s.
    pub fn time(&self) -> f64 {
        match &self.integ {
            Integrator::One(n) => n.t,
            Integrator::Two(n) => n.t,
        }
    }

    /// Top floor displacement, m (mean lean + swing).
    pub fn top(&self) -> f64 {
        match &self.integ {
            Integrator::One(n) => n.x[0],
            Integrator::Two(n) => n.x[0],
        }
    }

    /// Top floor velocity, m/s.
    pub fn velocity(&self) -> f64 {
        match &self.integ {
            Integrator::One(n) => n.v[0],
            Integrator::Two(n) => n.v[0],
        }
    }

    /// Top floor acceleration, m/s².
    pub fn accel(&self) -> f64 {
        match &self.integ {
            Integrator::One(n) => n.a[0],
            Integrator::Two(n) => n.a[0],
        }
    }

    /// The swing: top displacement minus the mean lean, m.
    pub fn swing(&self) -> f64 {
        self.top() - self.p.mean_top
    }

    /// The damper mass's displacement relative to the top floor, m (0 without a damper).
    pub fn damper_stroke(&self) -> f64 {
        match &self.integ {
            Integrator::One(_) => 0.0,
            Integrator::Two(n) => n.x[1] - n.x[0],
        }
    }

    /// Whether the damper is simulated.
    pub fn has_damper(&self) -> bool {
        matches!(self.integ, Integrator::Two(_))
    }
}

fn fallback_sdof() -> Newmark<1> {
    // 1 kg, 1 Hz: always invertible.
    let sys = Linear::sdof(1.0, 1.0, 0.0);
    let k = sys.k[0][0];
    let m = sys.m[0][0];
    Newmark {
        sys,
        h: 1.0 / STEPS_PER_PERIOD,
        keff_inv: [[1.0 / (k + 16.0 * STEPS_PER_PERIOD * STEPS_PER_PERIOD * m / 4.0)]],
        x: [0.0],
        v: [0.0],
        a: [0.0],
        t: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invert_two_by_two() {
        let a = [[4.0, 1.0], [2.0, 3.0]];
        let i = invert(&a).unwrap();
        let p = [
            [
                a[0][0] * i[0][0] + a[0][1] * i[1][0],
                a[0][0] * i[0][1] + a[0][1] * i[1][1],
            ],
            [
                a[1][0] * i[0][0] + a[1][1] * i[1][0],
                a[1][0] * i[0][1] + a[1][1] * i[1][1],
            ],
        ];
        for (r, row) in p.iter().enumerate() {
            for (c, v) in row.iter().enumerate() {
                assert!((v - if r == c { 1.0 } else { 0.0 }).abs() < 1e-12);
            }
        }
        assert!(invert(&[[1.0, 2.0], [2.0, 4.0]]).is_none());
        assert!(invert(&[[f64::NAN]]).is_none());
    }

    #[test]
    fn fallback_is_consistent_with_newmark() {
        let n = Newmark::new(
            Linear::sdof(1.0, 1.0, 0.0),
            1.0 / STEPS_PER_PERIOD,
            [0.0],
            [0.0],
            [0.0],
        )
        .unwrap();
        let f = fallback_sdof();
        assert!((n.keff_inv[0][0] - f.keff_inv[0][0]).abs() < 1e-15);
    }
}
