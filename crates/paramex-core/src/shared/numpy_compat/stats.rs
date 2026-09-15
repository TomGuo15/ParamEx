//! NaN-aware and statistical NumPy/pandas compatibility helpers.

/// Index of the maximum value, ignoring NaN — mirrors `np.nanargmax`.
///
/// Returns the index of the largest non-NaN element. On ties, returns the
/// FIRST occurrence (`np.nanargmax` semantics). `+inf`/`-inf` participate as
/// ordinary extreme values; only NaN is skipped.
///
/// Returns `None` for an empty slice or a slice that is entirely NaN.
/// (`np.nanargmax` raises `ValueError` in those cases; we surface the same "no
/// answer" outcome as `None` so callers can handle it without panicking.)
///
/// Note: `Iterator::max_by` returns the LAST maximum on ties, so first-wins is
/// hand-written here by replacing the running best only on a STRICTLY greater
/// value.
pub fn nanargmax(vals: &[f64]) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, &x) in vals.iter().enumerate() {
        if x.is_nan() {
            continue;
        }
        match best {
            // Strictly greater => move; equal keeps the earlier index (first-wins).
            Some((_, b)) if x > b => best = Some((i, x)),
            None => best = Some((i, x)),
            _ => {}
        }
    }
    best.map(|(i, _)| i)
}

/// Index of the minimum value, ignoring NaN — mirrors `np.nanargmin`.
///
/// Returns the index of the smallest non-NaN element. On ties, returns the
/// FIRST occurrence (`np.nanargmin` semantics). `+inf`/`-inf` participate as
/// ordinary extreme values; only NaN is skipped.
///
/// Returns `None` for an empty slice or a slice that is entirely NaN.
/// (`np.nanargmin` raises `ValueError` in those cases; we surface the same "no
/// answer" outcome as `None` so callers can handle it without panicking.)
///
/// Note: `Iterator::min_by` returns the LAST minimum on ties, so first-wins is
/// hand-written here by replacing the running best only on a STRICTLY lesser
/// value.
pub fn nanargmin(vals: &[f64]) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, &x) in vals.iter().enumerate() {
        if x.is_nan() {
            continue;
        }
        match best {
            // Strictly lesser => move; equal keeps the earlier index (first-wins).
            Some((_, b)) if x < b => best = Some((i, x)),
            None => best = Some((i, x)),
            _ => {}
        }
    }
    best.map(|(i, _)| i)
}

/// Median of `vals` ignoring NaN, matching `numpy.nanmedian`.
///
/// NaN values are dropped. The remaining values are sorted ascending; the
/// median is the middle element for an odd count, or the arithmetic mean
/// `(a + b) / 2.0` of the two middle elements for an even count. If every
/// value is NaN, or the slice is empty, the result is `f64::NAN`.
///
/// Infinities participate as ordinary order statistics; e.g. the mean of
/// the two middles may be `+inf`, `-inf`, or `NaN` (for `(+inf + -inf)/2`),
/// matching the IEEE-754 arithmetic of `np.nanmedian`.
pub fn nanmedian(vals: &[f64]) -> f64 {
    let mut finite: Vec<f64> = vals.iter().copied().filter(|x| !x.is_nan()).collect();
    let n = finite.len();
    if n == 0 {
        return f64::NAN;
    }
    // NaNs were filtered above; treating any unexpected comparator gap as equal
    // keeps sorting total without changing finite values.
    finite.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = n / 2;
    if n % 2 == 1 {
        finite[mid]
    } else {
        (finite[mid - 1] + finite[mid]) / 2.0
    }
}

/// `Σ(v - mean(a))^2`, two-pass (mean, then squared-deviation sum) to match the
/// `pandas.Series.std()` accumulation order.
fn sum_sq_deviations(a: &[f64]) -> f64 {
    let mean = a.iter().sum::<f64>() / a.len() as f64;
    a.iter().map(|&v| (v - mean) * (v - mean)).sum::<f64>()
}

/// Sample standard deviation (ddof = 1), matching `pandas.Series.std()` with
/// default arguments: `sqrt(sum((a - mean(a))^2) / (n - 1))`.
///
/// Returns `f64::NAN` for fewer than two elements, as `pandas.Series.std()`
/// does. Two-pass (mean, then mean-squared-deviation) to match its accumulation.
pub fn std_sample(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 2 {
        return f64::NAN;
    }
    (sum_sq_deviations(a) / (n as f64 - 1.0)).sqrt()
}
