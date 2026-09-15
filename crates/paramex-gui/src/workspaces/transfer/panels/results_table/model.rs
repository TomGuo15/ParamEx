//! Typed Transfer projection to GUI display rows.

use eframe::egui;
use paramex_core::transfer::{
    ResultsTableCell, ResultsTableColumn, ResultsTableProjection, ResultsTableRowKind,
    ResultsTableSweep, Session,
};

use super::{columns, measure::measure_col_widths, span::GroupRow};
use crate::format_ui::{fmt_compact_current, fmt_fixed2, fmt_ratio};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct DisplayRow {
    pub cells: Vec<String>,
    pub is_overall: bool,
    pub group_span: i64,
}

impl GroupRow for DisplayRow {
    fn group_span(&self) -> i64 {
        self.group_span
    }

    fn cell(&self, col_idx: usize) -> &str {
        self.cells.get(col_idx).map_or("", String::as_str)
    }
}

#[derive(Default)]
pub(crate) struct ResultsTableCache {
    generation: Option<u64>,
    pixels_per_point: f32,
    rows: Vec<DisplayRow>,
    widths: Vec<f32>,
}

impl ResultsTableCache {
    pub(super) fn ensure(&mut self, ui: &egui::Ui, session: &Session) {
        let ppp = ui.ctx().pixels_per_point();
        if self.generation == Some(session.generation()) && self.pixels_per_point == ppp {
            return;
        }
        self.pixels_per_point = ppp;
        self.rows = display_rows(session.results_table());
        self.widths = measure_col_widths(ui, columns::indexed_gui_column_specs());
        self.generation = Some(session.generation());
    }

    pub(super) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub(super) fn rows(&self) -> &[DisplayRow] {
        &self.rows
    }

    pub(super) fn widths(&self) -> &[f32] {
        &self.widths
    }
}

fn display_rows(projection: ResultsTableProjection) -> Vec<DisplayRow> {
    projection
        .rows
        .into_iter()
        .map(|row| {
            let is_overall = matches!(row.kind, ResultsTableRowKind::Overall { .. });
            let cells = projection
                .columns
                .iter()
                .copied()
                .zip(row.cells)
                .map(|(column, cell)| format_cell(column, cell, row.kind))
                .collect();
            DisplayRow {
                cells,
                is_overall,
                group_span: if row.group_position == 0 {
                    row.group_span as i64
                } else {
                    -(row.group_span as i64)
                },
            }
        })
        .collect()
}

fn format_cell(
    column: ResultsTableColumn,
    cell: ResultsTableCell,
    kind: ResultsTableRowKind,
) -> String {
    match cell {
        ResultsTableCell::Missing => {
            if column.is_numeric() {
                "NA".to_string()
            } else {
                String::new()
            }
        }
        ResultsTableCell::Text(text) => text,
        ResultsTableCell::Number(value) if column.is_current() => fmt_compact_current(value),
        ResultsTableCell::Number(value) if column.is_ratio() => {
            if value.is_finite() && value > 0.0 {
                fmt_ratio(value)
            } else {
                "NA".to_string()
            }
        }
        ResultsTableCell::Number(value) => fmt_fixed2(value),
        ResultsTableCell::Sweep(sweep) => {
            let label = match sweep {
                ResultsTableSweep::Single => "S",
                ResultsTableSweep::Forward => "F",
                ResultsTableSweep::Backward => "B",
            };
            match kind {
                ResultsTableRowKind::Measurement => label.to_string(),
                ResultsTableRowKind::Overall { count } => format!("{label} N={count}"),
            }
        }
        ResultsTableCell::Overall => "Overall".to_string(),
        ResultsTableCell::Summary {
            mean,
            sample_std_dev,
        } => format!(
            "{} \u{00B1} {}",
            format_summary_value(column, mean),
            format_summary_value(column, sample_std_dev)
        ),
        ResultsTableCell::Log10Summary {
            mean,
            sample_std_dev,
        } => format!(
            "log\u{2081}\u{2080} {} \u{00B1} {}",
            format_optional_fixed(mean),
            format_optional_fixed(sample_std_dev)
        ),
        ResultsTableCell::SummaryCaption => "mean \u{00B1} std".to_string(),
    }
}

fn format_summary_value(column: ResultsTableColumn, value: Option<f64>) -> String {
    value.map_or_else(
        || "NA".to_string(),
        |value| {
            if column.is_current() {
                fmt_compact_current(value)
            } else {
                fmt_fixed2(value)
            }
        },
    )
}

fn format_optional_fixed(value: Option<f64>) -> String {
    value.map_or_else(|| "NA".to_string(), fmt_fixed2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_cells_render_without_parsing_report_strings() {
        assert_eq!(
            format_cell(
                ResultsTableColumn::OnCurrent,
                ResultsTableCell::Number(1.5e-3),
                ResultsTableRowKind::Measurement,
            ),
            "1.50m"
        );
        assert_eq!(
            format_cell(
                ResultsTableColumn::OnOffRatio,
                ResultsTableCell::Log10Summary {
                    mean: Some(8.25),
                    sample_std_dev: Some(0.5),
                },
                ResultsTableRowKind::Overall { count: 2 },
            ),
            "log\u{2081}\u{2080} 8.25 \u{00B1} 0.50"
        );
    }
}
