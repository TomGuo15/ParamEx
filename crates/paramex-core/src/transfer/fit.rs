//! Windowed linear-regression engine used by Transfer extraction.
//!
//! A [`WindowedFitter`] is built once for a `(sweep, transform)` pair and then
//! answers many windowed `.fit`/`.fit_indices` queries in O(1) each, via prefix
//! sums precomputed by the shared fitter. Callers apply their
//! own R² / min-points gating afterwards. Scientific extraction policies live
//! under `transfer::metrics`.

use crate::shared::numerics::{WindowedLinearFit, WindowedLinearFitter};
use crate::transfer::types::{SweepData, WindowedFitResult};

/// Which transform of `|Id|` is regressed against `Vg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transform {
    /// `sqrt(|Id|)` vs `Vg` — the V_TH ELR fit.
    Sqrt,
    /// `log10(|Id|)` vs `Vg` — the SS fit.
    Log,
}

/// Windowed linear-regression engine over a single transformed sweep.
pub struct WindowedFitter {
    inner: WindowedLinearFitter,
}

impl WindowedFitter {
    /// Build the fitter from a sweep; see [`Self::from_slices`].
    pub fn new(sweep: &SweepData, transform: Transform) -> Self {
        Self::from_slices(&sweep.vg, &sweep.id_abs, transform)
    }

    /// Build the fitter from paired `vg`/`id_abs` slices without copying them
    /// into a [`SweepData`].
    ///
    /// Samples are kept where `vg` and `id_abs` are finite and `id_abs > 0`;
    /// `y` is `sqrt(|Id|)` or `log10(|Id|)` per `transform`; the kept samples are
    /// sorted ascending by `vg`. Prefix sums of `x`, `y`, `x²`, `y²`, `xy` are
    /// then precomputed.
    ///
    /// The VT window selector relies on this mask, transform, and sort being
    /// identical to its own candidate preparation (guarded by
    /// `select_elr_vt_window_matches_reference_corpus`). The sort is a stable
    /// sort by `total_cmp`; the reference corpus has unique Vg values, so
    /// duplicate-Vg tie order is not exercised there. Fit *results* are
    /// unaffected by tie order (the prefix sums commute within any window).
    pub fn from_slices(vg: &[f64], id_abs: &[f64], transform: Transform) -> Self {
        let samples = vg
            .iter()
            .copied()
            .zip(id_abs.iter().copied())
            .filter(|(voltage, current)| {
                voltage.is_finite() && current.is_finite() && *current > 0.0
            })
            .map(|(voltage, current)| {
                let current = current.abs();
                let value = match transform {
                    Transform::Sqrt => current.sqrt(),
                    Transform::Log => current.log10(),
                };
                (voltage, value)
            });
        Self {
            inner: WindowedLinearFitter::new(samples),
        }
    }

    /// The finite, positive, Vg-sorted sample abscissae (read-only).
    pub fn x(&self) -> &[f64] {
        self.inner.x()
    }

    /// Number of usable samples after masking.
    pub fn n(&self) -> usize {
        self.inner.len()
    }

    /// Linear fit over the half-open positional window `[start, end)`.
    /// O(1) via the prefix sums. `start`/`end`
    /// index into [`Self::x`]. Use this for window *searches*. Public callers
    /// may pass stale/out-of-range indices; they are clamped to the available
    /// samples so malformed input yields a NaN fit instead of panicking.
    pub fn fit_indices(&self, start: usize, end: usize) -> WindowedFitResult {
        into_transfer_result(self.inner.fit_indices(start, end))
    }

    /// Fit the samples whose Vg lies in `fit_range`.
    ///
    /// `None` means "use every sample." Bounds are inclusive on both ends; they
    /// are sorted ascending first. The window is located with
    /// `searchsorted(x, lo, "left")` .. `searchsorted(x, hi, "right")`, then fed
    /// to [`Self::fit_indices`].
    pub fn fit(&self, fit_range: Option<(f64, f64)>) -> WindowedFitResult {
        into_transfer_result(self.inner.fit(fit_range))
    }
}

fn into_transfer_result(fit: WindowedLinearFit) -> WindowedFitResult {
    WindowedFitResult {
        slope: fit.slope,
        intercept: fit.intercept,
        r2: fit.r2,
        points: fit.points,
    }
}
