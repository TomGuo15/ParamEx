//! Column schema + raw-cell extraction from `MetricResult`.

mod rows;

use super::ResultsTableColumn;

pub(super) use rows::{result_sweeps, results_to_rows, value_for_column, Cell};

/// Per-column formatter kind for plain-text report cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Formatter {
    Text,
    Number,
    Current,
    PowerOfTen,
}

/// Per-column report metadata.
#[derive(Debug, Clone, Copy)]
pub(super) struct ColumnSpec {
    pub(super) column: ResultsTableColumn,
    pub(super) label_plain: &'static str,
    pub(super) formatter: Formatter,
}

/// The canonical column list, in [`ResultsTableColumn::ALL`] order so
/// `COLUMNS[column.index()]` is that column's spec.
pub(super) const COLUMNS: &[ColumnSpec] = &[
    ColumnSpec {
        column: ResultsTableColumn::Filename,
        label_plain: "File",
        formatter: Formatter::Text,
    },
    ColumnSpec {
        column: ResultsTableColumn::Sweep,
        label_plain: "Sweep",
        formatter: Formatter::Text,
    },
    ColumnSpec {
        column: ResultsTableColumn::WidthUm,
        label_plain: "W (\u{00B5}m)",
        formatter: Formatter::Number,
    },
    ColumnSpec {
        column: ResultsTableColumn::LengthUm,
        label_plain: "L (\u{00B5}m)",
        formatter: Formatter::Number,
    },
    ColumnSpec {
        column: ResultsTableColumn::AspectRatio,
        label_plain: "W/L",
        formatter: Formatter::Number,
    },
    ColumnSpec {
        column: ResultsTableColumn::GeometrySource,
        label_plain: "Geometry",
        formatter: Formatter::Text,
    },
    ColumnSpec {
        column: ResultsTableColumn::ThresholdVoltage,
        label_plain: "VTH (V)",
        formatter: Formatter::Number,
    },
    ColumnSpec {
        column: ResultsTableColumn::SaturationMobility,
        label_plain: "mu_sat (cm^2 V^-1 s^-1)",
        formatter: Formatter::Number,
    },
    ColumnSpec {
        column: ResultsTableColumn::SubthresholdSwing,
        label_plain: "SS (mV dec^-1)",
        formatter: Formatter::Number,
    },
    ColumnSpec {
        column: ResultsTableColumn::OnCurrent,
        label_plain: "Ion",
        formatter: Formatter::Current,
    },
    ColumnSpec {
        column: ResultsTableColumn::OffCurrent,
        label_plain: "Ioff",
        formatter: Formatter::Current,
    },
    ColumnSpec {
        column: ResultsTableColumn::OnOffRatio,
        label_plain: "Ion/Ioff",
        formatter: Formatter::PowerOfTen,
    },
    ColumnSpec {
        column: ResultsTableColumn::ThresholdHysteresis,
        label_plain: "DeltaVTH,hyst (V)",
        formatter: Formatter::Number,
    },
    ColumnSpec {
        column: ResultsTableColumn::Status,
        label_plain: "Status",
        formatter: Formatter::Text,
    },
    ColumnSpec {
        column: ResultsTableColumn::Message,
        label_plain: "Message",
        formatter: Formatter::Text,
    },
];

/// The spec for `column`.
pub(super) fn column_spec(column: ResultsTableColumn) -> &'static ColumnSpec {
    let spec = &COLUMNS[column.index()];
    debug_assert_eq!(
        spec.column, column,
        "COLUMNS is in ResultsTableColumn::ALL order"
    );
    spec
}

/// Report columns excluding the Sweep column, in order; the human-readable
/// sections carry the sweep in their title instead.
pub(super) fn columns_without_sweep() -> impl Iterator<Item = ResultsTableColumn> {
    ResultsTableColumn::ALL
        .into_iter()
        .filter(|column| *column != ResultsTableColumn::Sweep)
}
