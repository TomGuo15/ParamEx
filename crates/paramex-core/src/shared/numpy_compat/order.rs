//! Ordering, search, and duplicate-collapse NumPy compatibility helpers.

/// Side selector for [`searchsorted`], matching the `np.searchsorted` `side` argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// `np.searchsorted(..., side="left")`: first `i` with `a[i] >= v`.
    Left,
    /// `np.searchsorted(..., side="right")`: first `i` with `a[i] > v`.
    Right,
}

/// Find the insertion index that keeps the ascending slice `a` sorted when `v`
/// is inserted, matching `numpy.searchsorted`.
///
/// `a` must be ascending (non-decreasing). The returned index is in
/// `0..=a.len()`:
/// - [`Side::Left`] returns the first `i` such that `a[i] >= v`
///   (so all `a[..i] < v`).
/// - [`Side::Right`] returns the first `i` such that `a[i] > v`
///   (so all `a[..i] <= v`).
///
/// For a value below every element this is `0`; for a value above every element
/// it is `a.len()`. Querying a value equal to a knot returns the start of the
/// run of equal elements under `Left` and one-past-the-end of that run under
/// `Right`, so `Right - Left` is the multiplicity of `v` in `a`.
///
/// Comparisons use the IEEE-754 ordering on `f64`; `-inf`/`+inf` behave as
/// ordinary ordered endpoints, consistent with `np.searchsorted`. `NaN` in `a`
/// or `v` is not a supported input (`np.searchsorted` itself yields unspecified
/// results for unsorted or NaN-containing input).
pub fn searchsorted(a: &[f64], v: f64, side: Side) -> usize {
    // Binary search over the half-open range [lo, hi).
    let mut lo = 0usize;
    let mut hi = a.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        // Left:  go right while a[mid] <  v  -> first a[i] >= v.
        // Right: go right while a[mid] <= v  -> first a[i] >  v.
        let go_right = match side {
            Side::Left => a[mid] < v,
            Side::Right => a[mid] <= v,
        };
        if go_right {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Merge duplicate `x` points by averaging `y` at each `x`.
///
/// Mirrors the `np.unique(x, return_inverse=True)` + `np.add.at` idiom: the
/// distinct `x` values ascending, each `y` scatter-added into its group, and
/// the sums divided by `np.maximum(counts, 1.0)`.
///
/// Groups `x` by exact equality and returns the distinct values in ascending
/// order (matching `np.unique`). For each group, `y` is accumulated by a
/// scatter-add that walks `x` in its **original order** (so floating-point
/// summation order matches `np.add.at`), then divided by the group count. The
/// `maximum(counts, 1.0)` guard is kept: an empty input yields empty outputs,
/// and no realized group can have count 0, so it never alters a real mean.
///
/// IEEE corner: `-0.0` and `+0.0` form a single group (they compare equal), and
/// the representative carried into `unique_x` is `-0.0` (the sort minimum),
/// matching `np.unique`. `+inf`/`-inf` order cleanly as the max/min and do not
/// collapse.
///
/// NOT replicated: `np.unique` collapses *all* NaNs into one group sorted last,
/// despite `nan != nan`. The production x-axis never carries NaN, so that
/// behavior is intentionally out of scope here; callers must not pass NaN `x`.
///
/// # Panics
/// Debug-asserts `x.len() == y.len()` (`np.add.at` assumes aligned arrays).
pub fn unique_mean(x: &[f64], y: &[f64]) -> (Vec<f64>, Vec<f64>) {
    debug_assert_eq!(x.len(), y.len(), "x and y must have equal length");

    if x.is_empty() {
        return (Vec::new(), Vec::new());
    }

    // Distinct ascending x via total order (NaN-free by contract). total_cmp
    // sorts `-0.0` before `+0.0`; we then dedup on IEEE equality so the two
    // zeros merge into one group whose representative is the first seen, i.e.
    // `-0.0`. This reproduces np.unique's ordering and signed-zero handling.
    let mut order: Vec<usize> = (0..x.len()).collect();
    order.sort_by(|&a, &b| x[a].total_cmp(&x[b]));

    let mut unique_x: Vec<f64> = Vec::new();
    // group index per original position, used as the scatter-add target.
    let mut inverse: Vec<usize> = vec![0usize; x.len()];
    for &idx in &order {
        let v = x[idx];
        // New group when empty or v differs (IEEE ==) from the current rep.
        // `-0.0 == 0.0` is true, so the +0.0 entries fold into the -0.0 group.
        let new_group = match unique_x.last() {
            None => true,
            Some(&last) => v != last,
        };
        if new_group {
            unique_x.push(v);
        }
        inverse[idx] = unique_x.len() - 1;
    }

    let n_groups = unique_x.len();
    let mut sums = vec![0.0f64; n_groups];
    let mut counts = vec![0.0f64; n_groups];
    // Scatter-add in ORIGINAL order to match the `np.add.at` accumulation order.
    for i in 0..x.len() {
        let g = inverse[i];
        sums[g] += y[i];
        counts[g] += 1.0;
    }

    let mean_y: Vec<f64> = sums
        .iter()
        .zip(counts.iter())
        .map(|(&s, &c)| s / c.max(1.0))
        .collect();

    (unique_x, mean_y)
}

/// Indices that would sort `vals` ascending, matching `numpy.argsort` on
/// distinct keys.
///
/// This is a STABLE sort by IEEE total order (`f64::total_cmp`): equal keys keep
/// their original relative order (first-occurrence-wins). The default
/// `np.argsort` is *unstable* (introsort/quicksort), so for arrays with
/// duplicate keys the index permutation can differ, but every ParamEx consumer
/// either collapses duplicate x immediately afterwards (`unique_mean`)
/// or runs on strictly unique gate voltages, where all sort kinds agree.
/// `total_cmp` orders `-0.0` before `+0.0` and treats `+/-inf` as ordinary
/// extremes.
pub fn argsort(vals: &[f64]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..vals.len()).collect();
    order.sort_by(|&a, &b| vals[a].total_cmp(&vals[b]));
    order
}

/// Gather `vals` into the order given by an [`argsort`] permutation
/// (`out[k] = vals[order[k]]`). Reorders an array parallel to a sort key without
/// re-typing the index-map idiom at every call site.
pub fn take_by(vals: &[f64], order: &[usize]) -> Vec<f64> {
    order.iter().map(|&i| vals[i]).collect()
}
