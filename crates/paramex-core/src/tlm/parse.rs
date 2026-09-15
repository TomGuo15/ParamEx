//! TLM workbook ingest.
//! `group/length_um/*.xlsx` trees -> `TlmCurve`. Reuses grid_ingest + coerce_numeric.

mod io;
mod paths;
mod sheets;
mod workbook;

pub(super) use paths::{
    dir_group_length, discover_workbooks, path_group_length, rel_os, UnreadableDir,
    WorkbookDiscovery,
};
pub use workbook::parse_workbook;
