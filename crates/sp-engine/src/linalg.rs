//! Dense linear solves for small structural systems.
//!
//! The stiffness systems here are small (a player's bridge has tens of joints, a netlist a few
//! hundred), so a dense elimination is exact, simple and fast enough. The solver keeps the
//! column order (row pivoting only), so a singular column maps straight back to the degree of
//! freedom — and so to the joint — that can move without straining anything.

/// The matrix is singular: elimination found no usable pivot in `column` (an index into the
/// unknowns, in the order they were given).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Singular {
    /// The first column without a usable pivot.
    pub column: usize,
}

/// Relative pivot tolerance: a pivot no bigger than `PIVOT_TOL × max |A_ii|` counts as zero
/// (the mockups' `maxd * 1e-9`).
pub const PIVOT_TOL: f64 = 1e-9;

/// Solves `A·x = b` for a dense row-major `n × n` matrix by Gaussian elimination with partial
/// (row) pivoting. `a` and `b` are consumed as scratch space.
///
/// Returns [`Singular`] with the first column whose best pivot is `≤ PIVOT_TOL × max |A_ii|`
/// (or not finite). An empty system (`n = 0`) solves to an empty vector.
pub fn solve_dense(n: usize, a: &mut [f64], b: &mut [f64]) -> Result<Vec<f64>, Singular> {
    if a.len() != n * n || b.len() != n {
        return Err(Singular { column: 0 });
    }
    let mut maxd = 0.0f64;
    for i in 0..n {
        maxd = maxd.max(a[i * n + i].abs());
    }
    let tol = maxd * PIVOT_TOL;
    for c in 0..n {
        let mut p = c;
        for r in c + 1..n {
            if a[r * n + c].abs() > a[p * n + c].abs() {
                p = r;
            }
        }
        let piv = a[p * n + c];
        if !piv.is_finite() || piv.abs() <= tol || piv == 0.0 {
            return Err(Singular { column: c });
        }
        if p != c {
            for k in 0..n {
                a.swap(c * n + k, p * n + k);
            }
            b.swap(c, p);
        }
        for r in c + 1..n {
            let f = a[r * n + c] / a[c * n + c];
            if f == 0.0 {
                continue;
            }
            for k in c..n {
                a[r * n + k] -= f * a[c * n + k];
            }
            b[r] -= f * b[c];
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let mut s = b[r];
        for k in r + 1..n {
            s -= a[r * n + k] * x[k];
        }
        x[r] = s / a[r * n + r];
    }
    if x.iter().any(|v| !v.is_finite()) {
        return Err(Singular {
            column: n.saturating_sub(1),
        });
    }
    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_a_small_system() {
        let mut a = vec![2.0, 1.0, 1.0, 3.0];
        let mut b = vec![3.0, 5.0];
        let x = solve_dense(2, &mut a, &mut b).unwrap();
        assert!((x[0] - 0.8).abs() < 1e-12 && (x[1] - 1.4).abs() < 1e-12);
    }

    #[test]
    fn needs_a_pivot() {
        let mut a = vec![0.0, 1.0, 1.0, 0.0];
        let mut b = vec![2.0, 3.0];
        let x = solve_dense(2, &mut a, &mut b).unwrap();
        assert_eq!(x, vec![3.0, 2.0]);
    }

    #[test]
    fn reports_the_singular_column() {
        // second unknown is free
        let mut a = vec![1.0, 0.0, 0.0, 0.0];
        let mut b = vec![1.0, 0.0];
        assert_eq!(solve_dense(2, &mut a, &mut b), Err(Singular { column: 1 }));
    }

    #[test]
    fn bad_shapes_and_nan_do_not_panic() {
        assert!(solve_dense(2, &mut [1.0], &mut [1.0, 2.0]).is_err());
        let mut a = vec![f64::NAN];
        assert!(solve_dense(1, &mut a, &mut [1.0]).is_err());
        assert_eq!(solve_dense(0, &mut [], &mut []), Ok(vec![]));
    }
}
