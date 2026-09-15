//! Shared prefix-sum windowed linear regression.

use crate::shared::numpy_compat::{argsort, searchsorted, take_by, Side};

use super::FLOAT_EPSILON;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WindowedLinearFit {
    pub(crate) slope: f64,
    pub(crate) intercept: f64,
    pub(crate) r2: f64,
    pub(crate) points: usize,
}

/// Cached one-pass OLS over sorted `(x, y)` samples.
pub(crate) struct WindowedLinearFitter {
    x: Vec<f64>,
    sx: Vec<f64>,
    sy: Vec<f64>,
    sxx: Vec<f64>,
    syy: Vec<f64>,
    sxy: Vec<f64>,
}

impl WindowedLinearFitter {
    /// Build from already validated and transformed finite `(x, y)` samples.
    pub(crate) fn new(samples: impl IntoIterator<Item = (f64, f64)>) -> Self {
        let (x, y): (Vec<_>, Vec<_>) = samples.into_iter().unzip();
        let order = argsort(&x);
        let x = take_by(&x, &order);
        let y = take_by(&y, &order);
        let xx: Vec<_> = x.iter().map(|value| value * value).collect();
        let yy: Vec<_> = y.iter().map(|value| value * value).collect();
        let xy: Vec<_> = x.iter().zip(&y).map(|(a, b)| a * b).collect();

        Self {
            sx: prefix_sum(&x),
            sy: prefix_sum(&y),
            sxx: prefix_sum(&xx),
            syy: prefix_sum(&yy),
            sxy: prefix_sum(&xy),
            x,
        }
    }

    pub(crate) fn x(&self) -> &[f64] {
        &self.x
    }

    pub(crate) fn len(&self) -> usize {
        self.x.len()
    }

    pub(crate) fn fit_indices(&self, start: usize, end: usize) -> WindowedLinearFit {
        let start = start.min(self.x.len());
        let end = end.min(self.x.len());
        let points = end.saturating_sub(start);
        if points < 2 {
            return nan_fit(points);
        }

        fit_from_sums(
            points,
            range_sum(&self.sx, start, end),
            range_sum(&self.sy, start, end),
            range_sum(&self.sxx, start, end),
            range_sum(&self.syy, start, end),
            range_sum(&self.sxy, start, end),
        )
    }

    pub(crate) fn fit(&self, fit_range: Option<(f64, f64)>) -> WindowedLinearFit {
        let (start, end) = match fit_range {
            None => (0, self.x.len()),
            Some((a, b)) => {
                let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                (
                    searchsorted(&self.x, lo, Side::Left),
                    searchsorted(&self.x, hi, Side::Right),
                )
            }
        };
        self.fit_indices(start, end)
    }
}

fn fit_from_sums(
    points: usize,
    sx: f64,
    sy: f64,
    sxx: f64,
    syy: f64,
    sxy: f64,
) -> WindowedLinearFit {
    let n = points as f64;
    let denominator = n * sxx - sx * sx;
    if denominator.abs() <= FLOAT_EPSILON {
        return nan_fit(points);
    }

    let slope = (n * sxy - sx * sy) / denominator;
    let intercept = (sy - slope * sx) / n;
    let mut ss_res = syy - 2.0 * slope * sxy - 2.0 * intercept * sy + slope * slope * sxx;
    ss_res += 2.0 * slope * intercept * sx + n * intercept * intercept;
    let ss_tot = syy - sy * sy / n;
    let mut r2 = if ss_tot <= FLOAT_EPSILON {
        f64::NAN
    } else {
        1.0 - ss_res / ss_tot
    };
    if r2.is_finite() {
        r2 = r2.min(1.0);
    }
    WindowedLinearFit {
        slope,
        intercept,
        r2,
        points,
    }
}

fn nan_fit(points: usize) -> WindowedLinearFit {
    WindowedLinearFit {
        slope: f64::NAN,
        intercept: f64::NAN,
        r2: f64::NAN,
        points,
    }
}

fn prefix_sum(values: &[f64]) -> Vec<f64> {
    let mut out = Vec::with_capacity(values.len() + 1);
    out.push(0.0);
    let mut accumulator = 0.0;
    for &value in values {
        accumulator += value;
        out.push(accumulator);
    }
    out
}

fn range_sum(prefix: &[f64], start: usize, end: usize) -> f64 {
    prefix[end] - prefix[start]
}
