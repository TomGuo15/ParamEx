//! General-purpose numeric helpers shared by the metric modules - no knowledge
//! of V_TH, SS, mobility, or any physical metric.
//!
//! `FLOAT_EPSILON` is the project's unified tolerance. `fit` owns the
//! closed-form linear-fit engine.

mod fit;
mod windowed_fit;

pub use fit::linear_fit_with_r2;
pub(crate) use windowed_fit::{WindowedLinearFit, WindowedLinearFitter};

/// The project's unified floating-point tolerance.
pub const FLOAT_EPSILON: f64 = 1e-12;
