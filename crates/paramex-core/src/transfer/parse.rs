//! Transfer-curve file parser: label helpers plus the sheet/grid scan, rejection
//! filters, and curve construction.
//!
//! # The `Grid` boundary
//! All scan/detect/build logic is pure over a [`Grid`] (one sheet:
//! `Vec<Vec<String>>`). The `grid_ingest` module turns file bytes into ordered
//! grids (one per sheet; one for delimited text). This split keeps the parsing
//! rules testable against reference string grids independent of file format.

use std::path::{Path, PathBuf};

use crate::shared::grid_ingest::{normalized_extension, split_single_column, Grid};
use crate::transfer::types::ParsedCurve;

pub(in crate::transfer) use curve::validate_curve_integrity;
pub use io::{parse_transfer_bytes, parse_transfer_file, ParseError};
use labels::parse_labeled_columns;
use numeric::parse_numeric_fallback;
use sheet::looks_like_output_curve;

mod curve;
mod io;
mod labels;
mod numeric;
mod sheet;

#[cfg(test)]
mod tests;

/// File extensions the parser accepts. Compared against the lower-cased
/// extension *including* the leading dot.
pub const SUPPORTED_EXTENSIONS: [&str; 5] = crate::shared::grid_ingest::MEASUREMENT_EXTENSIONS;

/// Whether a path has a supported Transfer measurement-file extension.
pub fn is_supported_measurement_path(path: &Path) -> bool {
    let suffix = normalized_extension(path);
    SUPPORTED_EXTENSIONS.contains(&suffix.as_str())
}

pub(super) fn read_measurement_file(path: &Path) -> Result<(String, String, Vec<u8>), ParseError> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let suffix = normalized_extension(path);
    if !is_supported_measurement_path(path) {
        return Err(ParseError(format!("Unsupported file extension: {suffix}")));
    }
    let content = std::fs::read(path)
        .map_err(|e| ParseError(format!("Could not read {}: {e}", path.display())))?;
    Ok((name, suffix, content))
}

/// Labeled headers can live deep in B1500A files (a real fixture puts one at
/// row 121), so the header scan is generous.
const HEADER_SCAN_LIMIT: usize = crate::shared::grid_ingest::HEADER_SCAN_LIMIT;

/// Minimum finite positive-current rows for a usable transfer curve.
pub const MIN_TRANSFER_POINTS: usize = 12;

/// Parse one sheet's grid into a [`ParsedCurve`]. This is the parser's policy
/// seam: prepare the grid, reject known output-curve sheets, then prefer
/// labeled columns and finally try the numeric fallback. Empty or unusable
/// grids return `None`.
///
/// File/byte entry points live in [`io`]; this function stays independent of
/// the filesystem so the same rules apply to every sheet and test grid.
fn parse_grid(grid: &Grid, name: &str, source_path: Option<PathBuf>) -> Option<ParsedCurve> {
    // A grid with no rows or with only empty rows carries no data.
    if grid.is_empty() || grid.iter().all(|row| row.is_empty()) {
        return None;
    }
    let g = split_single_column(grid);
    if looks_like_output_curve(&g, name) {
        return None;
    }
    if let Some(curve) = parse_labeled_columns(&g, name, source_path.clone()) {
        return Some(curve);
    }
    parse_numeric_fallback(&g, name, source_path)
}
