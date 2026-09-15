//! Sheet-grid preparation and transfer-file rejection policy.

use crate::shared::grid_headers::find_column_by_label;
use crate::shared::grid_ingest::Grid;
use crate::transfer::file_name::output_name_hint;

use super::labels::{ID_LABELS, VD_LABELS, VG_LABELS};

/// Output-curve markers live in the metadata preamble (first ~20 rows) by
/// B1500A convention; a wider scan dominated parse time.
pub(super) const OUTPUT_CURVE_SCAN_LIMIT: usize = 20;

/// Lower-cased, trimmed cells of grid row `row_idx`. The grid cells are
/// already the source cell text, so this is just `trim().to_lowercase()`.
/// `row_idx` is assumed in-bounds (callers scan `0..min(limit, len)`).
pub(super) fn normalized_row(grid: &Grid, row_idx: usize) -> Vec<String> {
    grid[row_idx]
        .iter()
        .map(|cell| cell.trim().to_lowercase())
        .collect()
}

/// Detect a B1500A output-curve (`Id-Vd`) file so it is rejected from the
/// transfer path. An explicit output-axis declaration is
/// checked anywhere in the grid; other content evidence stays within the
/// scanned [`OUTPUT_CURVE_SCAN_LIMIT`] rows (a load-bearing perf property).
/// Output evidence rejects, and a transfer-shaped header row (Vg + Id, no Vd)
/// clears a filename that merely *looks* like an output convention. The name
/// hint (`id-vd` / `-output` / digit+`o` stems) decides only when content finds
/// neither kind of evidence.
pub(super) fn looks_like_output_curve(grid: &Grid, name: &str) -> bool {
    let name_hint = output_name_hint(name);

    // Instrument exports may place this explicit axis declaration after the
    // metadata preamble. It is authoritative and cheap to recognize without
    // widening the bounded structural scan below.
    if grid.iter().any(|row| {
        let row = row
            .iter()
            .map(|cell| cell.trim().to_lowercase())
            .collect::<Vec<_>>();
        row.first()
            .is_some_and(|cell| cell.contains("output.graph.xaxis.data"))
            && row
                .get(1..)
                .is_some_and(|rest| find_column_by_label(rest, &VD_LABELS).is_some())
    }) {
        return true;
    }

    let scan_rows = OUTPUT_CURVE_SCAN_LIMIT.min(grid.len());
    for row_idx in 0..scan_rows {
        let row = normalized_row(grid, row_idx);
        if row.is_empty() {
            continue;
        }
        let first_cell = &row[0];
        let vd_col = find_column_by_label(&row, &VD_LABELS);
        if (first_cell.contains("setup title") || first_cell.contains("setup name"))
            && row.iter().any(|v| v.replace('_', "-").contains("id-vd"))
        {
            return true;
        }
        if first_cell.contains("output.graph.xaxis.data") && vd_col.is_some() {
            return true;
        }
        let id_col = find_column_by_label(&row, &ID_LABELS);
        let vg_col = find_column_by_label(&row, &VG_LABELS);
        if vd_col.is_some() && id_col.is_some() && vg_col.is_none() {
            return true;
        }
        if name_hint && vg_col.is_some() && id_col.is_some() && vd_col.is_none() {
            return false;
        }
    }
    name_hint
}
