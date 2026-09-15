//! Numeric helpers pinned to documented NumPy/pandas tie-breaks, rounding,
//! and NaN handling.
//!
//! Every function here is pinned by committed reference data under
//! `tests/reference/numpy_compat/` (tests under `tests/shared/numpy_compat/`).
//! Do not replace these with algebraically equivalent rewrites. The reference
//! data pins tie-breaks, rounding, edge attenuation, and NaN handling.

mod order;
mod scalar;
mod series;
mod stats;

pub use order::{argsort, searchsorted, take_by, unique_mean, Side};
pub use scalar::{banker_round, isclose, ptp};
pub use series::{interp, linspace};
pub use stats::{nanargmax, nanargmin, nanmedian, std_sample};
