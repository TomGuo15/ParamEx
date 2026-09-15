//! ParamEx core: GUI-free TFT characterization.
//!
//! Two products live here. [`transfer`] parses transfer-curve measurements and
//! extracts threshold voltage, subthreshold swing, mobility, and hysteresis;
//! [`tlm`] ingests transmission-line-method workbook trees and fits contact
//! resistance against channel length. [`shared`] admits only product-agnostic
//! primitives (numerics, NumPy parity, grid ingest, file identity, CSV writing)
//! and knows nothing about either product's vocabulary.
//!
//! `shared::numpy_compat` mirrors specific NumPy and pandas semantics
//! (tie-breaks, rounding, NaN handling) rather than mathematically equivalent
//! alternatives; its reference JSON under `tests/reference/numpy_compat/` was
//! generated with NumPy and pins that behavior.

#![deny(unreachable_pub)]
#![warn(missing_docs)]

pub mod shared;
pub mod tlm;
pub mod transfer;
