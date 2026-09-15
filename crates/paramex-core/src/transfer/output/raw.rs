//! Raw long-form `(Vg, Vd, Id)` ingestion for Transfer output attachments.
//!
//! This module interprets file structure only. [`super`] groups, orders, and
//! fits the returned source-order samples.

use crate::shared::grid_headers::find_column_by_label;
use crate::shared::grid_ingest::{
    coerce_numeric, read_grids, split_single_column, HEADER_SCAN_LIMIT,
};

const VG_ALIASES: &[&str] = &[
    "vg",
    "vgs",
    "v_g",
    "gate",
    "gatev",
    "gate voltage",
    "gate_voltage",
];
const VD_ALIASES: &[&str] = &[
    "vd",
    "vds",
    "v_d",
    "drainv",
    "drain voltage",
    "drain_voltage",
];
const SIGNED_ID_ALIASES: &[&str] = &[
    "id",
    "ids",
    "i_d",
    "drain",
    "draini",
    "drain current",
    "drain_current",
];
const MAGNITUDE_ID_ALIASES: &[&str] = &["abs_id", "absid"];

/// One finite long-form output sample, retained in source-row order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RawOutputSample {
    pub(super) vg: f64,
    pub(super) vd: f64,
    /// The source value without sign folding or magnitude conversion.
    pub(super) id: f64,
}

/// One parsed long-form `(Vg, Vd, Id)` measurement.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RawOutputMeasurement {
    pub(super) samples: Vec<RawOutputSample>,
}

/// Structural failures from raw output-measurement ingestion. [`super`] maps
/// each variant onto the user-facing [`crate::transfer::ParseError`] text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RawOutputParseError {
    GridRead(String),
    NoRows,
    MissingColumns,
    NoSamples,
}

/// Parse every usable long-form output measurement from ordered file grids.
///
/// At most one measurement is returned per grid, in grid order. The
/// signed-current vocabulary is searched before the magnitude vocabulary,
/// independent of physical column order. Invalid rows are skipped; retained
/// finite samples remain in their original row order.
pub(super) fn parse_raw_output_measurements(
    content: &[u8],
    suffix: &str,
) -> Result<Vec<RawOutputMeasurement>, RawOutputParseError> {
    let suffix = suffix.to_ascii_lowercase();
    let grids =
        read_grids(content, &suffix).map_err(|error| RawOutputParseError::GridRead(error.0))?;
    let mut saw_rows = false;
    let mut saw_header = false;
    let mut measurements = Vec::new();

    for grid in grids {
        let grid = split_single_column(&grid);
        if grid.is_empty() {
            continue;
        }
        saw_rows = true;

        for header_row in 0..grid.len().min(HEADER_SCAN_LIMIT) {
            let Some((vg_col, vd_col, id_col)) = output_columns(&grid[header_row]) else {
                continue;
            };
            saw_header = true;

            let samples: Vec<RawOutputSample> = grid
                .iter()
                .skip(header_row + 1)
                .filter_map(|row| {
                    let vg = finite_cell(row, vg_col)?;
                    let vd = finite_cell(row, vd_col)?;
                    let id = finite_cell(row, id_col)?;
                    Some(RawOutputSample { vg, vd, id })
                })
                .collect();
            if !samples.is_empty() {
                measurements.push(RawOutputMeasurement { samples });
                break;
            }
        }
    }

    if !measurements.is_empty() {
        Ok(measurements)
    } else if !saw_rows {
        Err(RawOutputParseError::NoRows)
    } else if saw_header {
        Err(RawOutputParseError::NoSamples)
    } else {
        Err(RawOutputParseError::MissingColumns)
    }
}

/// `(vg, vd, id)` column indices of an output header row. A signed `Id` column
/// is preferred over a magnitude column such as `abs_Id`.
fn output_columns(row: &[String]) -> Option<(usize, usize, usize)> {
    let vg = find_column_by_label(row, VG_ALIASES)?;
    let vd = find_column_by_label(row, VD_ALIASES)?;
    let id = find_column_by_label(row, SIGNED_ID_ALIASES)
        .or_else(|| find_column_by_label(row, MAGNITUDE_ID_ALIASES))?;
    (vg != vd && vg != id && vd != id).then_some((vg, vd, id))
}

fn finite_cell(row: &[String], column: usize) -> Option<f64> {
    let value = coerce_numeric(row.get(column)?);
    value.is_finite().then_some(value)
}
