//! Transfer parser byte/file adapters and parse errors.
//!
//! This layer chooses the input format and iterates sheets. Grid inspection
//! and curve construction remain in the sibling parser modules, which keeps
//! I/O concerns out of the parsing policy.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::shared::grid_ingest::{normalized_extension, read_grids};
use crate::transfer::types::ParsedCurve;

use super::{
    is_supported_measurement_path, parse_grid, read_measurement_file, MIN_TRANSFER_POINTS,
};

/// Raised when a file cannot be parsed into a transfer curve. The wrapped
/// message is the exact user-facing text the GUI surfaces.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// The user-facing message for a file with no usable transfer curve; the
/// reference data pins this text byte for byte.
fn no_usable_curve_message(name: &str) -> String {
    format!(
        "No usable transfer curve found in {name}. Check that the file contains \
Vg and Id columns with at least {MIN_TRANSFER_POINTS} valid positive-current rows."
    )
}

/// First grid that yields a usable curve, else the "no usable curve" error.
/// Shared tail of [`parse_transfer_bytes`]/[`parse_transfer_file`]; each caller
/// validates the extension first (keeping the upload-vs-file error order intact).
fn first_usable_curve(
    content: &[u8],
    suffix: &str,
    name: &str,
    source_path: Option<PathBuf>,
) -> Result<ParsedCurve, ParseError> {
    for grid in read_grids(content, suffix).map_err(|e| ParseError(e.0))? {
        if let Some(curve) = parse_grid(&grid, name, source_path.clone()) {
            return Ok(curve);
        }
    }
    Err(ParseError(no_usable_curve_message(name)))
}

/// Parse a transfer-curve upload from memory. Only `name`'s extension is
/// consulted for format; `source_path` is `None`.
pub fn parse_transfer_bytes(name: &str, content: &[u8]) -> Result<ParsedCurve, ParseError> {
    let suffix = normalized_extension(Path::new(name));
    if !is_supported_measurement_path(Path::new(name)) {
        return Err(ParseError(format!("Unsupported file extension: {suffix}")));
    }
    first_usable_curve(content, &suffix, name, None)
}

/// Parse a transfer-curve file from disk. The returned curve's `source_path`
/// is the input path.
pub fn parse_transfer_file(path: &Path) -> Result<ParsedCurve, ParseError> {
    let (name, suffix, content) = read_measurement_file(path)?;
    first_usable_curve(&content, &suffix, &name, Some(path.to_path_buf()))
}
