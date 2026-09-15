//! TLM table-card facade.
//!
//! `results.rs` owns the center tabbed Results / V_G sweep / Length points
//! card. `files.rs` owns the right-column FILES status card. `grid.rs` owns the
//! repeated analytical table renderer and measurement cache used by both cards.

mod files;
mod grid;
mod results;

pub use files::show_files;
pub(crate) use grid::TlmGridCache;
pub(crate) use results::show_results;
