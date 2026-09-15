//! Scalar NumPy compatibility helpers.

/// `np.isclose(a, b, rtol, atol)` for scalar f64 with `equal_nan=False`.
///
/// Returns `|a - b| <= atol + rtol * |b|`.
///
/// The tolerance is **asymmetric**: it is keyed to the magnitude of `b`, so
/// `isclose(a, b, ..)` is not in general equal to `isclose(b, a, ..)`.
///
/// NumPy defaults are `rtol = 1e-5`, `atol = 1e-8`; callers may pass custom
/// tolerances such as `rtol = 0.0`, `atol = 1e-12` (pure absolute tolerance).
///
/// Non-finite handling (matching `np.isclose` with `equal_nan=False`):
/// - if either `a` or `b` is NaN, returns `false` (even NaN vs NaN);
/// - `+inf` vs `+inf` and `-inf` vs `-inf` return `true`;
/// - any inf vs a non-equal value (opposite-sign inf, or a finite number)
///   returns `false`.
pub fn isclose(a: f64, b: f64, rtol: f64, atol: f64) -> bool {
    // equal_nan = false: any NaN operand is never close.
    if a.is_nan() || b.is_nan() {
        return false;
    }
    // Infinities: close iff they are the exact same (sign-matching) infinity.
    // The finite tolerance formula would otherwise yield `inf <= inf` (true) for
    // +inf vs finite, which is wrong, so handle infinities explicitly.
    if a.is_infinite() || b.is_infinite() {
        return a == b;
    }
    (a - b).abs() <= atol + rtol * b.abs()
}

/// Round half-to-even ("banker's rounding"), matching `np.round(x)` with
/// `decimals=0`.
///
/// Ties (exactly `.5`) round to the nearest **even** integer:
/// `0.5 -> 0`, `1.5 -> 2`, `2.5 -> 2`, `3.5 -> 4`, `-0.5 -> -0`, `-2.5 -> -2`.
/// Non-half values round to nearest; already-integral values are unchanged.
/// `NaN` and `+/-inf` pass through unchanged (as NumPy does).
///
/// NOTE: this is NOT `f64::round`, which rounds ties *away from zero*
/// (`2.5 -> 3`) and would diverge from NumPy. We use `f64::round_ties_even`,
/// the exact IEEE-754 round-to-nearest-ties-to-even operation NumPy relies on.
///
/// Callers that need an index/count apply the cast *after* rounding, e.g.
/// `banker_round(n as f64 * frac) as usize`, so the tie rule is applied to the
/// real-valued product rather than to a truncated integer.
pub fn banker_round(x: f64) -> f64 {
    x.round_ties_even()
}

/// Peak-to-peak range: `max - min`, matching `numpy.ptp`.
///
/// Assumes all values are finite (the caller is responsible for filtering
/// non-finite values). Empty input yields `f64::NAN`; `np.ptp` raises on a
/// size-0 array, and NaN is the value-typed encoding of that null result.
///
/// # Examples
/// ```
/// use paramex_core::shared::numpy_compat::ptp;
/// assert_eq!(ptp(&[1.0, 5.0, 3.0]), 4.0);
/// assert_eq!(ptp(&[4.0, 4.0, 4.0]), 0.0);
/// assert!(ptp(&[]).is_nan());
/// ```
pub fn ptp(vals: &[f64]) -> f64 {
    if vals.is_empty() {
        return f64::NAN;
    }
    let mut min = vals[0];
    let mut max = vals[0];
    for &v in &vals[1..] {
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }
    max - min
}
