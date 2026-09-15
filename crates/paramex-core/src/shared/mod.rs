//! Product-agnostic primitives: numerics, `numpy_compat`, grid ingest,
//! file identity, and CSV writing. Nothing here knows about transfer curves or
//! TLM; the products compose these into their own vocabularies.

pub(crate) mod csv;
mod file_identity;
pub(crate) mod grid_headers;
pub(crate) mod grid_ingest;
pub mod numerics;
pub mod numpy_compat;

pub use file_identity::{normalized_file_stem, same_named_source, same_source_path};
