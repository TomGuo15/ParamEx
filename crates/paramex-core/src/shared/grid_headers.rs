//! Shared header-label matching for string grids.

use crate::shared::grid_ingest::coerce_numeric;

/// Normalize a header cell: strip units, lower-case, no internal spaces or
/// dashes. `"Vg (V)"` -> `"vg"`, `"drain current"` -> `"drain_current"`.
pub(crate) fn normalize_label(value: &str) -> String {
    let unitless = value
        .split('(')
        .next()
        .unwrap_or("")
        .split('[')
        .next()
        .unwrap_or("")
        .trim();
    unitless.replace(['-', ' '], "_").to_lowercase()
}

fn label_matches(value: &str, vocab: &[&str]) -> bool {
    let normalized = normalize_label(value);
    let compact = normalized.replace('_', "");
    vocab.contains(&normalized.as_str()) || vocab.contains(&compact.as_str())
}

pub(crate) fn find_column_by_label(values: &[String], vocab: &[&str]) -> Option<usize> {
    values.iter().position(|value| label_matches(value, vocab))
}

pub(crate) fn find_label_indices(row: &[String], labels: &[&str]) -> Vec<usize> {
    row.iter()
        .enumerate()
        .filter(|(_, value)| label_matches(value, labels))
        .map(|(idx, _)| idx)
        .collect()
}

pub(crate) fn next_row_has_numerics(
    grid: &[Vec<String>],
    header_row: usize,
    cols: &[usize],
) -> bool {
    let next_row = header_row + 1;
    if next_row >= grid.len() {
        return false;
    }
    cols.iter().all(|&col| {
        grid[next_row]
            .get(col)
            .is_some_and(|cell| coerce_numeric(cell).is_finite())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_matching_strips_units_and_compacts_underscores() {
        let row = vec![
            "Vg (V)".to_string(),
            "Drain Current [A]".to_string(),
            "ABS-ID".to_string(),
        ];
        assert_eq!(normalize_label(&row[1]), "drain_current");
        assert_eq!(find_column_by_label(&row, &["vg"]), Some(0));
        assert_eq!(find_column_by_label(&row, &["draincurrent"]), Some(1));
        assert_eq!(find_label_indices(&row, &["absid"]), vec![2]);
    }

    #[test]
    fn next_row_has_numerics_checks_bounds_and_finiteness() {
        let grid = vec![
            vec!["Vg".to_string(), "Id".to_string()],
            vec!["1".to_string(), "2e-9".to_string()],
        ];
        assert!(next_row_has_numerics(&grid, 0, &[0, 1]));
        assert!(!next_row_has_numerics(&grid, 0, &[0, 2]));
        assert!(!next_row_has_numerics(&grid, 1, &[0]));
    }
}
