//! Linear-fit numeric engines shared by metric selectors.

use super::FLOAT_EPSILON;

/// Linear fit of `y` on `x` with R2 using the closed-form OLS normal equations.
///
/// Returns `(slope, intercept, r2, points)` where `points` is the number of
/// finite `(x, y)` pairs used. Non-finite pairs (NaN/inf in either coordinate)
/// are dropped first. With zero finite pairs the result is
/// `(NaN, NaN, NaN, 0)`; with one, `(NaN, NaN, NaN, 1)`.
///
/// `slope`/`intercept` are computed as
/// `slope = (n * sum(xy) - sum(x) * sum(y)) / (n * sum(x*x) - sum(x)^2)`,
/// `intercept = (sum(y) - slope * sum(x)) / n`.
///
/// The committed reference fits were produced by an SVD least-squares solver;
/// this closed form agrees with them to better than 1e-9 relative on
/// well-conditioned data, the only regime the SS selector feeds it, and does
/// not reproduce the SVD result bit-for-bit.
///
/// R2 is residual-based and has no clamp, unlike the one-pass engine in
/// `super::windowed_fit`: `ss_res = sum((y - y_hat)^2)`,
/// `ss_tot = sum((y - mean_y)^2)`, and `r2 = NaN` when
/// `ss_tot <= FLOAT_EPSILON`, else `1 - ss_res / ss_tot`.
///
/// # Precondition
/// The finite `x` must not be degenerate (all equal). Such input makes the OLS
/// denominator `<= 0`, where a least-norm solver would still return an answer;
/// production never produces it, so it is intentionally unsupported.
pub fn linear_fit_with_r2(x: &[f64], y: &[f64]) -> (f64, f64, f64, usize) {
    let mut xf: Vec<f64> = Vec::new();
    let mut yf: Vec<f64> = Vec::new();
    for (&xi, &yi) in x.iter().zip(y.iter()) {
        if xi.is_finite() && yi.is_finite() {
            xf.push(xi);
            yf.push(yi);
        }
    }
    let n = xf.len();
    if n == 0 {
        return (f64::NAN, f64::NAN, f64::NAN, 0);
    }
    if n < 2 {
        return (f64::NAN, f64::NAN, f64::NAN, n);
    }

    let nf = n as f64;
    let sx: f64 = xf.iter().sum();
    let sy: f64 = yf.iter().sum();
    let sxx: f64 = xf.iter().map(|v| v * v).sum();
    let sxy: f64 = xf.iter().zip(yf.iter()).map(|(a, b)| a * b).sum();

    let denominator = nf * sxx - sx * sx;
    let slope = (nf * sxy - sx * sy) / denominator;
    let intercept = (sy - slope * sx) / nf;

    // Residual-based r2 without a clamp.
    let mean_y = sy / nf;
    let mut ss_res = 0.0_f64;
    let mut ss_tot = 0.0_f64;
    for (&xi, &yi) in xf.iter().zip(yf.iter()) {
        let y_hat = slope * xi + intercept;
        ss_res += (yi - y_hat) * (yi - y_hat);
        ss_tot += (yi - mean_y) * (yi - mean_y);
    }
    let r2 = if ss_tot <= FLOAT_EPSILON {
        f64::NAN
    } else {
        1.0 - (ss_res / ss_tot)
    };

    (slope, intercept, r2, n)
}
