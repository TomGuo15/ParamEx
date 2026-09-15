//! Transfer parser label vocabulary and matching policy.
//!
//! A labeled parse is attempted before the numeric fallback. This module only
//! locates candidate Vg/Id columns and delegates filtering and integrity checks
//! to [`super::build_curve`].

use std::path::PathBuf;

use crate::shared::grid_headers::{find_label_indices, next_row_has_numerics};
use crate::shared::grid_ingest::{coerce_numeric, Grid};
use crate::transfer::types::ParsedCurve;

use super::curve::build_curve;
use super::sheet::normalized_row;
use super::HEADER_SCAN_LIMIT;

/// Gate-voltage header vocabulary.
pub(super) const VG_LABELS: [&str; 7] = [
    "vg",
    "vgs",
    "v_g",
    "gate",
    "gatev",
    "gate voltage",
    "gate_voltage",
];

/// Drain-current header vocabulary.
pub(super) const ID_LABELS: [&str; 9] = [
    "id",
    "ids",
    "i_d",
    "drain",
    "draini",
    "drain current",
    "drain_current",
    "abs_id",
    "absid",
];

/// Drain-voltage header vocabulary.
pub(super) const VD_LABELS: [&str; 6] = [
    "vd",
    "vds",
    "v_d",
    "drainv",
    "drain voltage",
    "drain_voltage",
];

/// Find a labeled Vg/Id header and build the curve from the rows below it.
/// Tries every `(vg, id)` candidate pair in label order until one yields a
/// usable curve.
pub(super) fn parse_labeled_columns(
    grid: &Grid,
    name: &str,
    source_path: Option<PathBuf>,
) -> Option<ParsedCurve> {
    let scan_rows = HEADER_SCAN_LIMIT.min(grid.len());
    for row_idx in 0..scan_rows {
        let row = normalized_row(grid, row_idx);
        let vg_cols = find_label_indices(&row, &VG_LABELS);
        let id_cols = find_label_indices(&row, &ID_LABELS);
        if vg_cols.is_empty() || id_cols.is_empty() {
            continue;
        }

        for &vg_col in &vg_cols {
            for &id_col in &id_cols {
                if vg_col == id_col {
                    continue;
                }
                if !next_row_has_numerics(grid, row_idx, &[vg_col, id_col]) {
                    continue;
                }
                // data = rows below the header; coerce the two columns.
                let mut vg_vals: Vec<f64> = Vec::new();
                let mut id_vals: Vec<f64> = Vec::new();
                for data_row in grid.iter().skip(row_idx + 1) {
                    vg_vals.push(coerce_numeric(
                        data_row.get(vg_col).map_or("", |s| s.as_str()),
                    ));
                    id_vals.push(coerce_numeric(
                        data_row.get(id_col).map_or("", |s| s.as_str()),
                    ));
                }
                if let Some(curve) = build_curve(name, source_path.clone(), &vg_vals, &id_vals) {
                    return Some(curve);
                }
            }
        }
    }
    None
}
