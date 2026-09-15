//! Numeric coercion, trace rejection, and numeric-fallback parsing.

use std::path::PathBuf;

use crate::shared::grid_ingest::{coerce_numeric, Grid};
use crate::transfer::types::ParsedCurve;

use super::curve::build_curve;
use super::sheet::normalized_row;
use super::MIN_TRANSFER_POINTS;

/// Detect an oscilloscope trace so it is rejected from the numeric-fallback
/// path. Scans the first 10 rows.
pub(super) fn looks_like_scope_trace(grid: &Grid) -> bool {
    let scan_rows = 10.min(grid.len());
    for row_idx in 0..scan_rows {
        let row = normalized_row(grid, row_idx);
        if row.iter().any(|v| v == "second") && row.iter().any(|v| v == "volt") {
            return true;
        }
        if let Some(first) = row.first() {
            if matches!(first.as_str(), "x-axis" | "time" | "time(s)" | "time (s)") {
                return true;
            }
        }
    }
    false
}

/// Detect an energy-spectrum trace so it is rejected from the numeric-fallback
/// path. Scans the first 50 rows.
pub(super) fn looks_like_spectrum_trace(grid: &Grid) -> bool {
    let scan_rows = 50.min(grid.len());
    for row_idx in 0..scan_rows {
        let row = normalized_row(grid, row_idx);
        if row.iter().any(|v| v == "energy")
            && (row.iter().any(|v| v == "axis") || row.iter().any(|v| v.contains("counts")))
        {
            return true;
        }
        if row.iter().any(|v| v.contains("binding energy")) {
            return true;
        }
    }
    false
}

/// Coerce the whole grid to a rectangular `f64` table (rows x `width`), filling
/// missing cells with NaN. `width` is the maximum row length so ragged rows
/// pad out instead of shifting columns.
fn coerce_grid(grid: &Grid) -> (Vec<Vec<f64>>, usize) {
    let width = grid.iter().map(|r| r.len()).max().unwrap_or(0);
    let table = grid
        .iter()
        .map(|row| {
            (0..width)
                .map(|c| coerce_numeric(row.get(c).map_or("", |s| s.as_str())))
                .collect()
        })
        .collect();
    (table, width)
}

/// Column indices with at least [`MIN_TRANSFER_POINTS`] finite values.
fn numeric_columns(table: &[Vec<f64>], width: usize) -> Vec<usize> {
    (0..width)
        .filter(|&col| {
            table.iter().filter(|row| row[col].is_finite()).count() >= MIN_TRANSFER_POINTS
        })
        .collect()
}

/// Last-resort parse of an unlabeled numeric table. Requires exactly two
/// usable columns and picks the ordered `(vg, id)` pair with the most
/// joint-finite rows (first-max on strict `>`), then builds the curve.
pub(super) fn parse_numeric_fallback(
    grid: &Grid,
    name: &str,
    source_path: Option<PathBuf>,
) -> Option<ParsedCurve> {
    if looks_like_scope_trace(grid) || looks_like_spectrum_trace(grid) {
        return None;
    }
    let (table, width) = coerce_grid(grid);
    let usable = numeric_columns(&table, width);
    if usable.len() != 2 {
        return None;
    }

    let mut best_pair: Option<(usize, usize)> = None;
    let mut best_count: usize = 0;
    for &vg_col in &usable {
        for &id_col in &usable {
            if vg_col == id_col {
                continue;
            }
            let count = table
                .iter()
                .filter(|row| row[vg_col].is_finite() && row[id_col].is_finite())
                .count();
            if count > best_count {
                best_count = count;
                best_pair = Some((vg_col, id_col));
            }
        }
    }

    let (vg_col, id_col) = best_pair?;
    if best_count < MIN_TRANSFER_POINTS {
        return None;
    }
    let vg: Vec<f64> = table.iter().map(|row| row[vg_col]).collect();
    let id: Vec<f64> = table.iter().map(|row| row[id_col]).collect();
    build_curve(name, source_path, &vg, &id)
}
