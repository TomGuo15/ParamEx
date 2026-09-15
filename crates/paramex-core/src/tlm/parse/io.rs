//! TLM workbook sheet-read adapter.

use std::path::Path;

use crate::shared::grid_ingest::{read_named_excel_sheets, Grid};
use crate::tlm::types::TlmParseError;

/// Read the `Setup(*)`/`List(*)` candidate sheets as `(name, Grid)` (Grid = rows
/// of cell strings), in workbook order. Skips materializing the (often many)
/// other sheets of a MATLAB-style workbook, which the caller never reads.
pub(super) fn read_named_sheets(
    path: &Path,
    rel: &str,
) -> Result<Vec<(String, Grid)>, TlmParseError> {
    let bytes =
        std::fs::read(path).map_err(|e| TlmParseError(format!("{rel} could not be read: {e}")))?;
    let sheets = read_named_excel_sheets(&bytes, |name| {
        let lower = name.to_lowercase();
        lower.starts_with("setup") || lower.starts_with("list")
    })
    // Keep the underlying cause so a present-but-unreadable sheet is reported
    // as unreadable rather than as missing.
    .map_err(|e| TlmParseError(format!("{rel}: {}", e.0)))?;
    Ok(sheets
        .into_iter()
        .map(|sheet| (sheet.name, sheet.grid))
        .collect())
}
