//! Transfer extraction rules grouped by measured quantity.
//!
//! This module owns the scientific policies for threshold voltage, mobility,
//! subthreshold swing, on/off current, hysteresis, and sweep splitting. The
//! sibling `fit` module is only the reusable windowed-regression engine; it
//! does not decide which quantity to extract or which quality gates to apply.

use crate::shared::numpy_compat::isclose;

pub(super) mod hysteresis;
pub(super) mod on_off;
pub(super) mod ss;
pub(super) mod sweep;
pub(super) mod vth;

#[cfg(test)]
mod tests;

/// Relative tolerance of [`isclose_default`].
const ISCLOSE_RTOL: f64 = 1e-5;
/// Absolute tolerance of [`isclose_default`].
const ISCLOSE_ATOL: f64 = 1e-8;

/// `isclose` with the default tolerances the reference data was generated
/// with (`rtol = 1e-5`, `atol = 1e-8`); used for every score tie-break.
pub(in crate::transfer) fn isclose_default(a: f64, b: f64) -> bool {
    isclose(a, b, ISCLOSE_RTOL, ISCLOSE_ATOL)
}
