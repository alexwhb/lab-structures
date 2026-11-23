//! JavaScript's `Math.sin`, `Math.cos`, `Math.atan2` and `Math.pow` bit for bit, as V8 computes
//! them (its `base::ieee754`, a port of fdlibm), so the settling engine reproduces the mockup
//! to the last bit. The platform's libm disagrees with V8 in the last place for a few percent of
//! arguments, which a chaotic collapse turns into a different outcome.
//!
//! V8 is built with clang's default `-ffp-contract=on`, so on arm64 every `a * b + c` written
//! as one C expression is a fused multiply-add; [`fma`] marks exactly those places (clang's
//! rule: the left operand of `+` / `-` if it is a multiply, else the right one).
//!
//! Arguments outside the ranges the builder uses (|x| > 2¹⁹·π/2 for sin / cos) fall back to
//! the platform's functions (a last-bit difference at most).

#![allow(
    clippy::excessive_precision,
    clippy::unreadable_literal,
    clippy::approx_constant,
    clippy::eq_op,
    clippy::needless_late_init
)]

/// `a·b + c` rounded once (a clang-contracted expression).
#[inline]
fn fma(a: f64, b: f64, c: f64) -> f64 {
    if cfg!(target_arch = "aarch64") {
        a.mul_add(b, c)
    } else {
        a * b + c
    }
}

fn hi(x: f64) -> i32 {
    (x.to_bits() >> 32) as u32 as i32
}
fn lo(x: f64) -> u32 {
    x.to_bits() as u32
}
fn with_lo(x: f64, l: u32) -> f64 {
    f64::from_bits((x.to_bits() & 0xffff_ffff_0000_0000) | l as u64)
}
fn with_hi(x: f64, h: i32) -> f64 {
    f64::from_bits((x.to_bits() & 0x0000_0000_ffff_ffff) | ((h as u32 as u64) << 32))
}
fn from_words(h: i32, l: u32) -> f64 {
    f64::from_bits(((h as u32 as u64) << 32) | l as u64)
}

const S1: f64 = -1.66666666666666324348e-01;
const S2: f64 = 8.33333333332248946124e-03;
const S3: f64 = -1.98412698298579493134e-04;
const S4: f64 = 2.75573137070700676789e-06;
const S5: f64 = -2.50507602534068634195e-08;
const S6: f64 = 1.58969099521155010221e-10;

fn k_sin(x: f64, y: f64, iy: i32) -> f64 {
    let ix = hi(x) & 0x7fffffff;
    if ix < 0x3e400000 && x as i32 == 0 {
        return x;
    }
    let z = x * x;
    let v = z * x;
    let r = fma(z, fma(z, fma(z, fma(z, S6, S5), S4), S3), S2);
    if iy == 0 {
        fma(v, fma(z, r, S1), x)
    } else {
        x - fma(-v, S1, fma(z, fma(0.5, y, -(v * r)), -y))
    }
}

const C1: f64 = 4.16666666666666019037e-02;
const C2: f64 = -1.38888888888741095749e-03;
const C3: f64 = 2.48015872894767294178e-05;
const C4: f64 = -2.75573143513906633035e-07;
const C5: f64 = 2.08757232129817482790e-09;
const C6: f64 = -1.13596475577881948265e-11;

fn k_cos(x: f64, y: f64) -> f64 {
    let ix = hi(x) & 0x7fffffff;
    if ix < 0x3e400000 && x as i32 == 0 {
        return 1.0;
    }
    let z = x * x;
    let r = z * fma(z, fma(z, fma(z, fma(z, fma(z, C6, C5), C4), C3), C2), C1);
    if ix < 0x3FD33333 {
        1.0 - fma(0.5, z, -fma(z, r, -(x * y)))
    } else {
        let qx = if ix > 0x3fe90000 {
            0.28125
        } else {
            from_words(ix - 0x00200000, 0)
        };
        let iz = fma(0.5, z, -qx);
        let a = 1.0 - qx;
        a - (iz - fma(z, r, -(x * y)))
    }
}

const NPIO2_HW: [i32; 32] = [
    0x3FF921FB, 0x400921FB, 0x4012D97C, 0x401921FB, 0x401F6A7A, 0x4022D97C, 0x4025FDBB, 0x402921FB,
    0x402C463A, 0x402F6A7A, 0x4031475C, 0x4032D97C, 0x40346B9C, 0x4035FDBB, 0x40378FDB, 0x403921FB,
    0x403AB41B, 0x403C463A, 0x403DD85A, 0x403F6A7A, 0x40407E4C, 0x4041475C, 0x4042106C, 0x4042D97C,
    0x4043A28C, 0x40446B9C, 0x404534AC, 0x4045FDBB, 0x4046C6CB, 0x40478FDB, 0x404858EB, 0x404921FB,
];
const INVPIO2: f64 = 6.36619772367581382433e-01;
const PIO2_1: f64 = 1.57079632673412561417e+00;
const PIO2_1T: f64 = 6.07710050650619224932e-11;
const PIO2_2: f64 = 6.07710050630396597660e-11;
const PIO2_2T: f64 = 2.02226624879595063154e-21;
const PIO2_3: f64 = 2.02226624871116645580e-21;
const PIO2_3T: f64 = 8.47842766036889956997e-32;

/// `__ieee754_rem_pio2` for |x| ≤ 2¹⁹·π/2; `None` beyond.
fn rem_pio2(x: f64) -> Option<(i32, f64, f64)> {
    let hx = hi(x);
    let ix = hx & 0x7fffffff;
    if ix <= 0x3fe921fb {
        return Some((0, x, 0.0));
    }
    if ix < 0x4002d97c {
        return Some(if hx > 0 {
            let mut z = x - PIO2_1;
            if ix != 0x3ff921fb {
                let y0 = z - PIO2_1T;
                (1, y0, (z - y0) - PIO2_1T)
            } else {
                z -= PIO2_2;
                let y0 = z - PIO2_2T;
                (1, y0, (z - y0) - PIO2_2T)
            }
        } else {
            let mut z = x + PIO2_1;
            if ix != 0x3ff921fb {
                let y0 = z + PIO2_1T;
                (-1, y0, (z - y0) + PIO2_1T)
            } else {
                z += PIO2_2;
                let y0 = z + PIO2_2T;
                (-1, y0, (z - y0) + PIO2_2T)
            }
        });
    }
    if ix <= 0x413921fb {
        let t = x.abs();
        let n = fma(t, INVPIO2, 0.5) as i32;
        let fn_ = n as f64;
        let mut r = fma(-fn_, PIO2_1, t);
        let mut w = fn_ * PIO2_1T;
        let y0;
        if n < 32 && ix != NPIO2_HW[(n - 1) as usize] {
            y0 = r - w;
        } else {
            let j = ix >> 20;
            let mut yy = r - w;
            let mut i = j - ((hi(yy) >> 20) & 0x7ff);
            if i > 16 {
                let t = r;
                w = fn_ * PIO2_2;
                r = t - w;
                w = fma(fn_, PIO2_2T, -((t - r) - w));
                yy = r - w;
                i = j - ((hi(yy) >> 20) & 0x7ff);
                if i > 49 {
                    let t = r;
                    w = fn_ * PIO2_3;
                    r = t - w;
                    w = fma(fn_, PIO2_3T, -((t - r) - w));
                    yy = r - w;
                }
            }
            y0 = yy;
        }
        let y1 = (r - y0) - w;
        return Some(if hx < 0 { (-n, -y0, -y1) } else { (n, y0, y1) });
    }
    None
}

/// `Math.sin(x)`.
pub fn sin(x: f64) -> f64 {
    let ix = hi(x) & 0x7fffffff;
    if ix <= 0x3fe921fb {
        return k_sin(x, 0.0, 0);
    }
    if ix >= 0x7ff00000 {
        return x - x;
    }
    match rem_pio2(x) {
        Some((n, y0, y1)) => match n & 3 {
            0 => k_sin(y0, y1, 1),
            1 => k_cos(y0, y1),
            2 => -k_sin(y0, y1, 1),
            _ => -k_cos(y0, y1),
        },
        None => x.sin(),
    }
}

/// `Math.cos(x)`.
pub fn cos(x: f64) -> f64 {
    let ix = hi(x) & 0x7fffffff;
    if ix <= 0x3fe921fb {
        return k_cos(x, 0.0);
    }
    if ix >= 0x7ff00000 {
        return x - x;
    }
    match rem_pio2(x) {
        Some((n, y0, y1)) => match n & 3 {
            0 => k_cos(y0, y1),
            1 => -k_sin(y0, y1, 1),
            2 => -k_cos(y0, y1),
            _ => k_sin(y0, y1, 1),
        },
        None => x.cos(),
    }
}

const ATANHI: [f64; 4] = [
    4.63647609000806093515e-01,
    7.85398163397448278999e-01,
    9.82793723247329054082e-01,
    1.57079632679489655800e+00,
];
const ATANLO: [f64; 4] = [
    2.26987774529616870924e-17,
    3.06161699786838301793e-17,
    1.39033110312309984516e-17,
    6.12323399573676603587e-17,
];
const AT: [f64; 11] = [
    3.33333333333329318027e-01,
    -1.99999999998764832476e-01,
    1.42857142725034663711e-01,
    -1.11111104054623557880e-01,
    9.09088713343650656196e-02,
    -7.69187620504482999495e-02,
    6.66107313738753120669e-02,
    -5.83357013379057348645e-02,
    4.97687799461593236017e-02,
    -3.65315727442169155270e-02,
    1.62858201153657823623e-02,
];

/// `Math.atan(x)`.
pub fn atan(x: f64) -> f64 {
    let hx = hi(x);
    let ix = hx & 0x7fffffff;
    if ix >= 0x44100000 {
        if ix > 0x7ff00000 || (ix == 0x7ff00000 && lo(x) != 0) {
            return x + x;
        }
        return if hx > 0 {
            ATANHI[3] + ATANLO[3]
        } else {
            -ATANHI[3] - ATANLO[3]
        };
    }
    let mut x = x;
    let id: i32;
    if ix < 0x3fdc0000 {
        if ix < 0x3e400000 {
            return x;
        }
        id = -1;
    } else {
        x = x.abs();
        if ix < 0x3ff30000 {
            if ix < 0x3fe60000 {
                id = 0;
                x = fma(2.0, x, -1.0) / (2.0 + x);
            } else {
                id = 1;
                x = (x - 1.0) / (x + 1.0);
            }
        } else if ix < 0x40038000 {
            id = 2;
            x = (x - 1.5) / fma(1.5, x, 1.0);
        } else {
            id = 3;
            x = -1.0 / x;
        }
    }
    let z = x * x;
    let w = z * z;
    let s1 = z * fma(
        w,
        fma(
            w,
            fma(w, fma(w, fma(w, AT[10], AT[8]), AT[6]), AT[4]),
            AT[2],
        ),
        AT[0],
    );
    let s2 = w * fma(w, fma(w, fma(w, fma(w, AT[9], AT[7]), AT[5]), AT[3]), AT[1]);
    if id < 0 {
        fma(-x, s1 + s2, x)
    } else {
        let id = id as usize;
        let z = ATANHI[id] - (fma(x, s1 + s2, -ATANLO[id]) - x);
        if hx < 0 { -z } else { z }
    }
}

const PI_O_4: f64 = 7.8539816339744827900E-01;
const PI_O_2: f64 = 1.5707963267948965580E+00;
const PI: f64 = 3.1415926535897931160E+00;
const PI_LO: f64 = 1.2246467991473531772E-16;
const TINY: f64 = 1.0e-300;

/// `Math.atan2(y, x)`.
pub fn atan2(y: f64, x: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return x + y;
    }
    let (hx, lx) = (hi(x), lo(x));
    let ix = hx & 0x7fffffff;
    let (hy, ly) = (hi(y), lo(y));
    let iy = hy & 0x7fffffff;
    if (hx.wrapping_sub(0x3ff00000) as u32 | lx) == 0 {
        return atan(y);
    }
    let m = ((hy >> 31) & 1) | ((hx >> 30) & 2);
    if (iy as u32 | ly) == 0 {
        return match m {
            0 | 1 => y,
            2 => PI + TINY,
            _ => -PI - TINY,
        };
    }
    if (ix as u32 | lx) == 0 {
        return if hy < 0 {
            -PI_O_2 - TINY
        } else {
            PI_O_2 + TINY
        };
    }
    if ix == 0x7ff00000 {
        if iy == 0x7ff00000 {
            return match m {
                0 => PI_O_4 + TINY,
                1 => -PI_O_4 - TINY,
                2 => 3.0 * PI_O_4 + TINY,
                _ => -3.0 * PI_O_4 - TINY,
            };
        }
        return match m {
            0 => 0.0,
            1 => -0.0,
            2 => PI + TINY,
            _ => -PI - TINY,
        };
    }
    if iy == 0x7ff00000 {
        return if hy < 0 {
            -PI_O_2 - TINY
        } else {
            PI_O_2 + TINY
        };
    }
    let k = (iy - ix) >> 20;
    let mut m = m;
    let z = if k > 60 {
        m &= 1;
        fma(0.5, PI_LO, PI_O_2)
    } else if hx < 0 && k < -60 {
        0.0
    } else {
        atan((y / x).abs())
    };
    match m {
        0 => z,
        1 => -z,
        2 => PI - (z - PI_LO),
        _ => (z - PI_LO) - PI,
    }
}

const BP: [f64; 2] = [1.0, 1.5];
const DP_H: [f64; 2] = [0.0, 5.84962487220764160156e-01];
const DP_L: [f64; 2] = [0.0, 1.35003920212974897128e-08];
const TWO53: f64 = 9007199254740992.0;
const HUGE: f64 = 1.0e300;
const L1: f64 = 5.99999999999994648725e-01;
const L2: f64 = 4.28571428578550184252e-01;
const L3: f64 = 3.33333329818377432918e-01;
const L4: f64 = 2.72728123808534006489e-01;
const L5: f64 = 2.30660745775561754067e-01;
const L6: f64 = 2.06975017800338417784e-01;
const P1: f64 = 1.66666666666666019037e-01;
const P2: f64 = -2.77777777770155933842e-03;
const P3: f64 = 6.61375632143793436117e-05;
const P4: f64 = -1.65339022054652515390e-06;
const P5: f64 = 4.13813679705723846039e-08;
const LG2: f64 = 6.93147180559945286227e-01;
const LG2_H: f64 = 6.93147182464599609375e-01;
const LG2_L: f64 = -1.90465429995776804525e-09;
const OVT: f64 = 8.0085662595372944372e-17;
const CP: f64 = 9.61796693925975554329e-01;
const CP_H: f64 = 9.61796700954437255859e-01;
const CP_L: f64 = -7.02846165095275826516e-09;
const IVLN2: f64 = 1.44269504088896338700e+00;
const IVLN2_H: f64 = 1.44269502162933349609e+00;
const IVLN2_L: f64 = 1.92596299112661746887e-08;

fn scalbn(x: f64, n: i32) -> f64 {
    x * 2f64.powi(n)
}

/// `Math.pow(x, y)` (fdlibm's `__ieee754_pow`).
pub fn pow(x: f64, y: f64) -> f64 {
    let (hx, lx) = (hi(x), lo(x));
    let (hy, ly) = (hi(y), lo(y));
    let mut ix = hx & 0x7fffffff;
    let iy = hy & 0x7fffffff;
    if (iy as u32 | ly) == 0 {
        return 1.0;
    }
    if ix > 0x7ff00000
        || (ix == 0x7ff00000 && lx != 0)
        || iy > 0x7ff00000
        || (iy == 0x7ff00000 && ly != 0)
    {
        return x + y;
    }
    let mut yisint = 0;
    if hx < 0 {
        if iy >= 0x43400000 {
            yisint = 2;
        } else if iy >= 0x3ff00000 {
            let k = (iy >> 20) - 0x3ff;
            if k > 20 {
                let j = ly >> (52 - k);
                if (j << (52 - k)) == ly {
                    yisint = 2 - (j & 1) as i32;
                }
            } else if ly == 0 {
                let j = iy >> (20 - k);
                if (j << (20 - k)) == iy {
                    yisint = 2 - (j & 1);
                }
            }
        }
    }
    if ly == 0 {
        if iy == 0x7ff00000 {
            if (ix.wrapping_sub(0x3ff00000) as u32 | lx) == 0 {
                return f64::NAN;
            } else if ix >= 0x3ff00000 {
                return if hy >= 0 { y } else { 0.0 };
            } else {
                return if hy < 0 { -y } else { 0.0 };
            }
        }
        if iy == 0x3ff00000 {
            return if hy < 0 { 1.0 / x } else { x };
        }
        if hy == 0x40000000 {
            return x * x;
        }
        if hy == 0x3fe00000 && hx >= 0 {
            return x.sqrt();
        }
    }
    let mut ax = x.abs();
    if lx == 0 && (ix == 0x7ff00000 || ix == 0 || ix == 0x3ff00000) {
        let mut z = ax;
        if hy < 0 {
            z = 1.0 / z;
        }
        if hx < 0 {
            if (ix.wrapping_sub(0x3ff00000) | yisint) == 0 {
                z = (z - z) / (z - z);
            } else if yisint == 1 {
                z = -z;
            }
        }
        return z;
    }
    let mut n: i32 = ((hx as u32) >> 31) as i32 - 1;
    if (n | yisint) == 0 {
        return (x - x) / (x - x);
    }
    let mut s = 1.0;
    if (n | (yisint - 1)) == 0 {
        s = -1.0;
    }
    let (t1, t2);
    if iy > 0x41e00000 {
        if iy > 0x43f00000 {
            if ix <= 0x3fefffff {
                return if hy < 0 { HUGE * HUGE } else { TINY * TINY };
            }
            if ix >= 0x3ff00000 {
                return if hy > 0 { HUGE * HUGE } else { TINY * TINY };
            }
        }
        if ix < 0x3fefffff {
            return if hy < 0 {
                s * HUGE * HUGE
            } else {
                s * TINY * TINY
            };
        }
        if ix > 0x3ff00000 {
            return if hy > 0 {
                s * HUGE * HUGE
            } else {
                s * TINY * TINY
            };
        }
        let t = ax - 1.0;
        let w = (t * t) * fma(-t, fma(-t, 0.25, 0.3333333333333333333333), 0.5);
        let u = IVLN2_H * t;
        let v = fma(t, IVLN2_L, -(w * IVLN2));
        let tt = with_lo(u + v, 0);
        t1 = tt;
        t2 = v - (tt - u);
    } else {
        n = 0;
        if ix < 0x00100000 {
            ax *= TWO53;
            n -= 53;
            ix = hi(ax);
        }
        n += (ix >> 20) - 0x3ff;
        let j = ix & 0x000fffff;
        ix = j | 0x3ff00000;
        let k: usize;
        if j <= 0x3988E {
            k = 0;
        } else if j < 0xBB67A {
            k = 1;
        } else {
            k = 0;
            n += 1;
            ix -= 0x00100000;
        }
        ax = with_hi(ax, ix);
        let u = ax - BP[k];
        let v = 1.0 / (ax + BP[k]);
        let ss = u * v;
        let s_h = with_lo(ss, 0);
        let t_h = from_words(
            ((ix >> 1) | 0x20000000) + 0x00080000 + ((k as i32) << 18),
            0,
        );
        let t_l = ax - (t_h - BP[k]);
        let s_l = v * fma(-s_h, t_l, fma(-s_h, t_h, u));
        let mut s2 = ss * ss;
        let mut r = s2
            * s2
            * fma(
                s2,
                fma(s2, fma(s2, fma(s2, fma(s2, L6, L5), L4), L3), L2),
                L1,
            );
        r = fma(s_l, s_h + ss, r);
        s2 = s_h * s_h;
        let t_h = with_lo(3.0 + s2 + r, 0);
        let t_l = r - ((t_h - 3.0) - s2);
        let u = s_h * t_h;
        let v = fma(s_l, t_h, t_l * ss);
        let p_h = with_lo(u + v, 0);
        let p_l = v - (p_h - u);
        let z_h = CP_H * p_h;
        let z_l = fma(CP_L, p_h, p_l * CP) + DP_L[k];
        let t = n as f64;
        let tt = with_lo(((z_h + z_l) + DP_H[k]) + t, 0);
        t1 = tt;
        t2 = z_l - (((tt - t) - DP_H[k]) - z_h);
    }
    let y1 = with_lo(y, 0);
    let p_l = fma(y - y1, t1, y * t2);
    let mut p_h = y1 * t1;
    let mut z = p_l + p_h;
    let (j, i) = (hi(z), lo(z));
    if j >= 0x40900000 {
        if ((j - 0x40900000) as u32 | i) != 0 || p_l + OVT > z - p_h {
            return s * HUGE * HUGE;
        }
    } else if (j & 0x7fffffff) >= 0x4090cc00
        && ((j.wrapping_sub(0xc090cc00u32 as i32)) as u32 | i != 0 || p_l <= z - p_h)
    {
        return s * TINY * TINY;
    }
    let i = j & 0x7fffffff;
    let mut k = (i >> 20) - 0x3ff;
    let mut n = 0;
    if i > 0x3fe00000 {
        n = j + (0x00100000 >> (k + 1));
        k = ((n & 0x7fffffff) >> 20) - 0x3ff;
        let t = from_words(n & !(0x000fffff >> k), 0);
        n = ((n & 0x000fffff) | 0x00100000) >> (20 - k);
        if j < 0 {
            n = -n;
        }
        p_h -= t;
    }
    let t = with_lo(p_l + p_h, 0);
    let u = t * LG2_H;
    let v = fma(p_l - (t - p_h), LG2, t * LG2_L);
    z = u + v;
    let w = v - (z - u);
    let t = z * z;
    let t1 = fma(-t, fma(t, fma(t, fma(t, fma(t, P5, P4), P3), P2), P1), z);
    let r = (z * t1) / (t1 - 2.0) - fma(z, w, w);
    z = 1.0 - (r - z);
    let mut j = hi(z);
    j = j.wrapping_add(n << 20);
    if (j >> 20) <= 0 {
        z = scalbn(z, n);
    } else {
        z = with_hi(z, j);
    }
    s * z
}

/// `Math.pow(x, 0.2)` — the wind profile's exponent. Every call in the builder has x = k / 10
/// for a whole height k in metres, so V8's own results are tabulated for k = 0…400 ([`pow`]
/// elsewhere: V8's `pow` is not plain fdlibm, and differs from it in the last bit for ~3 % of
/// arguments).
pub fn pow02(x: f64) -> f64 {
    let k = (x * 10.0).round();
    if (0.0..=400.0).contains(&k) && k / 10.0 == x {
        f64::from_bits(POW02_TABLE[k as usize])
    } else {
        pow(x, 0.2)
    }
}

/// V8’s `Math.pow(k / 10, 0.2)` for k = 0…=400, as bit patterns (generated by Node 22 /
/// V8 12.4 on arm64; `tests/builder_goldens.mjs` checks it again whenever Node is present).
#[rustfmt::skip]
pub const POW02_TABLE: [u64; 401] = [
    0x0000000000000000, 0x3fe430cd74f6d478, 0x3fe731651f09a68e, 0x3fe926eff16629a5,
    0x3feaa4469fbf1206, 0x3febdb8cdadbe120, 0x3fece4658b654324, 0x3fedcbfae8202d18,
    0x3fee9a708ec1794a, 0x3fef552ef248b71d, 0x3ff0000000000000, 0x3ff04ed3b7325783,
    0x3ff0981d45329b9f, 0x3ff0dcaaeee362d8, 0x3ff11d1fc4f3391c, 0x3ff159ff126bab41,
    0x3ff193b43862cff2, 0x3ff1ca983b603b59, 0x3ff1fef5c7c3f980, 0x3ff2310c295d36ba,
    0x3ff2611186bae675, 0x3ff28f3495d8362f, 0x3ff2bb9deeb2e2ab, 0x3ff2e671153b6431,
    0x3ff30fcd4ca757e3, 0x3ff337ce41260734, 0x3ff35e8c917abf3e, 0x3ff3841e3f8c7922,
    0x3ff3a8970d39372f, 0x3ff3cc08c975def2, 0x3ff3ee8390d43956, 0x3ff4101603d70f0f,
    0x3ff430cd74f6d478, 0x3ff450b60fd40104, 0x3ff46fdafac687a0, 0x3ff48e4673bddd91,
    0x3ff4ac01e936328f, 0x3ff4c9160fe2bab0, 0x3ff4e58af58fd4ae, 0x3ff5016811a8aa50,
    0x3ff51cb453b9536c, 0x3ff537763037858a, 0x3ff551b3abcfaf70, 0x3ff56b72656a7268,
    0x3ff584b79f1646aa, 0x3ff59d8845fa620a, 0x3ff5b5e8f9726985, 0x3ff5cdde116dd68e,
    0x3ff5e56ba42a1c79, 0x3ff5fc958b5b5d03, 0x3ff6135f68d4c0cc, 0x3ff629ccaabf3a49,
    0x3ff63fe08f6b8541, 0x3ff6559e28ca8abe, 0x3ff66b085f95e5b7, 0x3ff68021f6310cf5,
    0x3ff694ed8b4a9b12, 0x3ff6a96d9c4446bc, 0x3ff6bda487675618, 0x3ff6d1948deaab57,
    0x3ff6e53fd5cef2aa, 0x3ff6f8a86b94f706, 0x3ff70bd043d1b12a, 0x3ff71eb93ca34098,
    0x3ff731651f09a68e, 0x3ff743d5a025ce6c, 0x3ff7560c62612ae2, 0x3ff7680af67ff3bb,
    0x3ff779d2dc9fdb40, 0x3ff78b658524e317, 0x3ff79cc45195cf5c, 0x3ff7adf095699156,
    0x3ff7beeb96c6e294, 0x3ff7cfb68f372b98, 0x3ff7e052ac4db73a, 0x3ff7f0c110441c35,
    0x3ff80102d28cb15a, 0x3ff81119005bce07, 0x3ff821049d288760, 0x3ff830c6a3258b8b,
    0x3ff8406003b2ae5c, 0x3ff84fd1a7c7ae6c, 0x3ff85f1c7058ae3a, 0x3ff86e4136b4d2e4,
    0x3ff87d40ccdf709c, 0x3ff88c1bfde424b2, 0x3ff89ad38e263573, 0x3ff8a9683bab8904,
    0x3ff8b7dabe637e2e, 0x3ff8c62bc869ec5c, 0x3ff8d45c06468a88, 0x3ff8e26c1f28f854,
    0x3ff8f05cb5219ff1, 0x3ff8fe2e6557a37e, 0x3ff90be1c83c04e0, 0x3ff9197771ba33ad,
    0x3ff926eff16629a5, 0x3ff9344bd2a83b58, 0x3ff9418b9ce6c001, 0x3ff94eafd3adb123,
    0x3ff95bb8f6d46053, 0x3ff968a782a15f68, 0x3ff9757befecb58b, 0x3ff98236b44079bb,
    0x3ff98ed841f7e9df, 0x3ff99b61085d13df, 0x3ff9a7d173c524eb, 0x3ff9b429edab71d3,
    0x3ff9c06adccb4a06, 0x3ff9cc94a538a5da, 0x3ff9d8a7a877bf95, 0x3ff9e4a44593a5ca,
    0x3ff9f08ad933d4b8, 0x3ff9fc5bbdb0e38d, 0x3ffa08174b285188, 0x3ffa13bdd78f7e7e,
    0x3ffa1f4fb6c5d945, 0x3ffa2acd3aa64e40, 0x3ffa3636b317ff69, 0x3ffa418c6e1e4ed8,
    0x3ffa4cceb7e84435, 0x3ffa57fddadf5507, 0x3ffa631a1fb59757, 0x3ffa6e23cd7365d5,
    0x3ffa791b29847c20, 0x3ffa840077c49188, 0x3ffa8ed3fa8b7857, 0x3ffa9995f2b8c731,
    0x3ffaa4469fbf1206, 0x3ffaaee63faeb797, 0x3ffab9750f40485f, 0x3ffac3f349de8b82,
    0x3fface6129b02600, 0x3ffad8bee7a0e84c, 0x3ffae30cbb6ac642, 0x3ffaed4adb9e7d08,
    0x3ffaf7797dabea80, 0x3ffb0198d5ea198e, 0x3ffb0ba9179f0668, 0x3ffb15aa75071dec,
    0x3ffb1f9d1f5c7ade, 0x3ffb298146dde3d2, 0x3ffb33571ad58c44, 0x3ffb3d1ec99f9b78,
    0x3ffb46d880b07b5e, 0x3ffb50846c9af1d0, 0x3ffb5a22b916063f, 0x3ffb63b39102b5e4,
    0x3ffb6d371e717866, 0x3ffb76ad8aa796c7, 0x3ffb8016fe24566f, 0x3ffb8973a0a5f9f4,
    0x3ffb92c3992e994f, 0x3ffb9c070e08d2fc, 0x3ffba53e24cc5785, 0x3ffbae69026250e1,
    0x3ffbb787cb09a700, 0x3ffbc09aa25b22d2, 0x3ffbc9a1ab4d70fc, 0x3ffbd29d08390572,
    0x3ffbdb8cdadbe120, 0x3ffbe471445d3a9a, 0x3ffbed4a65510afe, 0x3ffbf6185dbb7ffa,
    0x3ffbfedb4d1453e0, 0x3ffc0793524a0cc0, 0x3ffc10408bc52363, 0x3ffc18e3176b1300,
    0x3ffc217b12a15277, 0x3ffc2a089a5037d5, 0x3ffc328bcae5c6ee, 0x3ffc3b04c0586baa,
    0x3ffc43739629a0d5, 0x3ffc4bd8676883fa, 0x3ffc54334eb456ff, 0x3ffc5c84663ef01d,
    0x3ffc64cbc7cf18bb, 0x3ffc6d098cc2dbd0, 0x3ffc753dce11c442, 0x3ffc7d68a44f0bd6,
    0x3ffc858a27abbb2f, 0x3ffc8da26ff8bb48, 0x3ffc95b194a8d8f6, 0x3ffc9db7acd2bad5,
    0x3ffca5b4cf32ca14, 0x3ffcada9122d0e84, 0x3ffcb5948bcefe52, 0x3ffcbd7751d141dd,
    0x3ffcc55179996be8, 0x3ffccd23183ba69f, 0x3ffcd4ec427c55bb, 0x3ffcdcad0cd1ae1c,
    0x3ffce4658b654325, 0x3ffcec15d2158a34, 0x3ffcf3bdf477547b, 0x3ffcfb5e05d73f83,
    0x3ffd02f6193b1ca2, 0x3ffd0a8641634fa5, 0x3ffd120e90cc24fe, 0x3ffd198f19af1f9b,
    0x3ffd2107ee043ebf, 0x3ffd28791f833c0c, 0x3ffd2fe2bfa4c1ee, 0x3ffd3744dfa39abd,
    0x3ffd3e9f907dd8a1, 0x3ffd45f2e2f5f696, 0x3ffd4d3ee793f2a4, 0x3ffd5483aea66189,
    0x3ffd5bc148437c06, 0x3ffd62f7c44a25f2, 0x3ffd6a273262ef48, 0x3ffd714fa2010f5b,
    0x3ffd787122635a4e, 0x3ffd7f8bc2953108, 0x3ffd869f916f6bbc, 0x3ffd8dac9d993f38,
    0x3ffd94b2f5891d0c, 0x3ffd9bb2a7858ec7, 0x3ffda2abc1a60c4f, 0x3ffda99e51d3cd90,
    0x3ffdb08a65ca9787, 0x3ffdb7700b1984e2, 0x3ffdbe4f4f23ca3c, 0x3ffdc5283f21762b,
    0x3ffdcbfae8202d18, 0x3ffdd2c75703e12a, 0x3ffdd98d98878636, 0x3ffde04db93dc1e2,
    0x3ffde707c5919819, 0x3ffdedbbc9c713ce, 0x3ffdf469d1fbec4b, 0x3ffdfb11ea2826fd,
    0x3ffe01b41e1eb5ef, 0x3ffe0850798e12fe, 0x3ffe0ee70800d7d6, 0x3ffe1577d4de52db,
    0x3ffe1c02eb6b18fe, 0x3ffe228856c994a8, 0x3ffe290821fa91b9, 0x3ffe2f8257ddc6b5,
    0x3ffe35f703325b37, 0x3ffe3c662e976bb1, 0x3ffe42cfe48c8a8a, 0x3ffe49342f723eb4,
    0x3ffe4f93198a7fc2, 0x3ffe55ecacf92f87, 0x3ffe5c40f3c4915c, 0x3ffe628ff7d5bf11,
    0x3ffe68d9c2f91b96, 0x3ffe6f1e5edec371, 0x3ffe755dd51afb08, 0x3ffe7b982f269acc,
    0x3ffe81cd765f7959, 0x3ffe87fdb408d388, 0x3ffe8e28f14bb295, 0x3ffe944f37375047,
    0x3ffe9a708ec1794a, 0x3ffea08d00c6ed9f, 0x3ffea6a4960bbf4b, 0x3ffeacb7573baf41,
    0x3ffeb2c54cea8893, 0x3ffeb8ce7f9479f5, 0x3ffebed2f79e6d98, 0x3ffec4d2bd565f74,
    0x3ffecacdd8f3b1f2, 0x3ffed0c452978115, 0x3ffed6b6324cf420, 0x3ffedca380098dc3,
    0x3ffee28c43ad7ad8, 0x3ffee8708503dfb5, 0x3ffeee504bc32424, 0x3ffef42b9f8d3dfa,
    0x3ffefa0287effa5f, 0x3ffeffd50c6545ce, 0x3fff05a3345372d0, 0x3fff0b6d070d7f76,
    0x3fff11328bd359a6, 0x3fff16f3c9d2222d, 0x3fff1cb0c8246eb5, 0x3fff22698dd28a89,
    0x3fff281e21d2b64b, 0x3fff2dce8b096686, 0x3fff337ad0498138, 0x3fff3922f8549a4c,
    0x3fff3ec709db2f0f, 0x3fff44670b7ce0a2, 0x3fff4a0303c8ad72, 0x3fff4f9af93d29bd,
    0x3fff552ef248b71d, 0x3fff5abef549bb2e, 0x3fff604b088ed546, 0x3fff65d332571353,
    0x3fff6b5778d225cd, 0x3fff70d7e22092d7, 0x3fff76547453e889, 0x3fff7bcd356eee65,
    0x3fff81422b65d5ff, 0x3fff86b35c1e6add, 0x3fff8c20cd704191, 0x3fff918a8524e60f,
    0x3fff96f088f80945, 0x3fff9c52de97adfc, 0x3fffa1b18ba45500, 0x3fffa70c95b12890,
    0x3fffac6402442727, 0x3fffb1b7d6d64d91, 0x3fffb70818d3c058, 0x3fffbc54cd9bf490,
    0x3fffc19dfa81d7f9, 0x3fffc6e3a4cbf884, 0x3fffcc25d1b4ab3e, 0x3fffd164866a3298,
    0x3fffd69fc80ee427, 0x3fffdbd79bb94dbd, 0x3fffe10c06745a01, 0x3fffe63d0d3f746e,
    0x3fffeb6ab50eacc9, 0x3ffff09502cada0e, 0x3ffff5bbfb51bcd3, 0x3ffffadfa376212b,
    0x4000000000000000, 0x4000028e8ad64ffb, 0x4000051b74975ae2, 0x400007a6bf974210,
    0x40000a306e24fd93, 0x40000cb8828a6ba8, 0x40000f3eff0c5ff9, 0x400011c3e5eab2a5,
    0x4000144739604f0b, 0x400016c8fba34265, 0x400019492ee4ca28, 0x40001bc7d5516228,
    0x40001e44f110d297, 0x400020c084463dc3, 0x4000233a91102da6, 0x400025b31988a144,
    0x4000282a1fc519db, 0x40002a9fa5d6a7db, 0x40002d13adc9f7b9, 0x40002f8639a75e89,
    0x400031f74b72e673, 0x40003466e52c5af7, 0x400036d508cf5505, 0x40003941b85346e9,
    0x40003bacf5ab880e, 0x40003e16c2c76098, 0x4000407f219214cf, 0x400042e613f2f06c,
    0x4000454b9bcd51af, 0x400047afbb00b460, 0x40004a127368bc98, 0x40004c73c6dd416f,
    0x40004ed3b7325784, 0x4000513246385b58, 0x4000538f75bbfb93, 0x400055eb47864315,
    0x40005845bd5ca2f2, 0x40005a9ed900fc43, 0x40005cf69c31a9dc, 0x40005f4d08a989d8,
    0x400061a22020070e, 0x400063f5e449225e, 0x4000664856d57be3, 0x4000689979725c05,
    0x40006ae94dc9bc68, 0x40006d37d58250c7, 0x40006f85123f8fa4, 0x400071d105a1bae5,
    0x4000741bb145e851, 0x4000766516c609f1, 0x400078ad37b8f651, 0x40007af415b270af,
    0x40007d39b2433102, 0x40007f7e0ef8ebf4, 0x400081c12d5e5aba, 0x400084030efb42d0,
    0x40008643b5547da5, 0x4000888321ec0023, 0x40008ac15640e227, 0x40008cfe53cf65de,
    0x40008f3a1c10ff06, 0x40009174b07c5a22, 0x400093ae12856386, 0x400095e6439d4e61,
    0x4000981d45329b9f, 0x40009a5318b120be, 0x40009c87bf820e8c, 0x40009ebb3b0bf7cd,
    0x4000a0ed8cb2d7ce, 0x4000a31eb5d818e3, 0x4000a54eb7da9ad0, 0x4000a77d9416b91f,
    0x4000a9ab4be65160, 0x4000abd7e0a0c957, 0x4000ae03539b1517, 0x4000b02da627bd07,
    0x4000b256d996e3da, 0x4000b47eef364c6f, 0x4000b6a5e8515fa1, 0x4000b8cbc6313205,
    0x4000baf08a1c8996,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agrees_with_known_values() {
        assert_eq!(sin(0.0), 0.0);
        assert_eq!(cos(0.0), 1.0);
        assert!((sin(1.0) - 0.8414709848078965).abs() < 1e-16);
        assert!((cos(2.0) + 0.4161468365471424).abs() < 1e-16);
        assert!((atan2(1.0, 1.0) - std::f64::consts::FRAC_PI_4).abs() < 1e-16);
        assert!((pow(2.0, 0.5) - std::f64::consts::SQRT_2).abs() < 1e-15);
        assert!((pow(3.0, 0.2) - 1.2457309396155174).abs() < 1e-15);
        assert!(sin(f64::NAN).is_nan() && pow(f64::NAN, 0.2).is_nan());
        assert!(sin(f64::INFINITY).is_nan());
        assert!((sin(1e9) - 1e9f64.sin()).abs() < 1e-9);
        for k in 0..=400 {
            let x = k as f64 / 10.0;
            assert!(
                (pow02(x) - pow(x, 0.2)).abs() <= 2e-16 * pow(x, 0.2).max(1.0),
                "{k}"
            );
        }
        assert_eq!(pow02(0.45), pow(0.45, 0.2));
    }
}
