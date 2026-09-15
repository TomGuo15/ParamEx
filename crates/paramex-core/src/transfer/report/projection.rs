//! Typed application projection for the Transfer results table.
//!
//! Report serialization keeps its byte-exact formatting implementation, while
//! this projection exposes domain values for callers that own presentation.

use super::schema::{result_sweeps, value_for_column, Cell};
use super::stats::{results_to_stats, StatRow};
use crate::transfer::types::MetricResult;

/// Canonical Transfer result columns, in report order.
///
/// The enum is the stable column identity. Callers may choose a narrower
/// display order without depending on report-schema implementation types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum ResultsTableColumn {
    /// Source file name.
    Filename,
    /// Which sweep the row describes.
    Sweep,
    /// Channel width in µm.
    WidthUm,
    /// Channel length in µm.
    LengthUm,
    /// W/L.
    AspectRatio,
    /// Where the geometry came from.
    GeometrySource,
    /// Threshold voltage in V.
    ThresholdVoltage,
    /// Saturation mobility in cm² V⁻¹ s⁻¹.
    SaturationMobility,
    /// Subthreshold swing in mV/dec.
    SubthresholdSwing,
    /// On current in A.
    OnCurrent,
    /// Off current in A.
    OffCurrent,
    /// Unitless on/off current ratio.
    OnOffRatio,
    /// Forward-to-backward threshold shift in V.
    ThresholdHysteresis,
    /// Extraction status text.
    Status,
    /// Extraction message text.
    Message,
}

impl ResultsTableColumn {
    /// Every canonical column in report order.
    pub const ALL: [Self; 15] = [
        Self::Filename,
        Self::Sweep,
        Self::WidthUm,
        Self::LengthUm,
        Self::AspectRatio,
        Self::GeometrySource,
        Self::ThresholdVoltage,
        Self::SaturationMobility,
        Self::SubthresholdSwing,
        Self::OnCurrent,
        Self::OffCurrent,
        Self::OnOffRatio,
        Self::ThresholdHysteresis,
        Self::Status,
        Self::Message,
    ];

    /// Stable report key shared with canonical CSV serialization.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Filename => "filename",
            Self::Sweep => "sweep",
            Self::WidthUm => "W_um",
            Self::LengthUm => "L_um",
            Self::AspectRatio => "W_over_L",
            Self::GeometrySource => "geometry_source",
            Self::ThresholdVoltage => "Vth",
            Self::SaturationMobility => "mu_sat",
            Self::SubthresholdSwing => "SS_mV_dec",
            Self::OnCurrent => "Ion",
            Self::OffCurrent => "Ioff",
            Self::OnOffRatio => "Ion_Ioff",
            Self::ThresholdHysteresis => "DeltaVth_hysteresis",
            Self::Status => "status",
            Self::Message => "message",
        }
    }

    /// Index of this column in [`Self::ALL`] and every projected row.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Whether ordinary measurement cells in this column are numeric.
    pub const fn is_numeric(self) -> bool {
        !matches!(
            self,
            Self::Filename | Self::Sweep | Self::GeometrySource | Self::Status | Self::Message
        )
    }

    /// Whether numeric measurement cells represent current in amperes.
    pub const fn is_current(self) -> bool {
        matches!(self, Self::OnCurrent | Self::OffCurrent)
    }

    /// Whether numeric measurement cells represent the unitless on/off ratio.
    pub const fn is_ratio(self) -> bool {
        matches!(self, Self::OnOffRatio)
    }

    /// Whether the value can differ between forward and backward sweeps.
    pub const fn is_sweep_aware(self) -> bool {
        matches!(
            self,
            Self::Sweep
                | Self::ThresholdVoltage
                | Self::SaturationMobility
                | Self::SubthresholdSwing
                | Self::OnCurrent
                | Self::OffCurrent
                | Self::OnOffRatio
        )
    }
}

/// Sweep represented by one measurement or aggregate row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultsTableSweep {
    /// The whole curve of a file without a usable backward branch.
    Single,
    /// The forward branch of a round-trip sweep.
    Forward,
    /// The backward branch of a round-trip sweep.
    Backward,
}

impl ResultsTableSweep {
    /// The sweep name the report and its CSV encode for this row.
    pub(super) const fn report_name(self) -> &'static str {
        match self {
            Self::Single => "Single",
            Self::Forward => "Forward",
            Self::Backward => "Backward",
        }
    }
}

/// One typed result-table cell.
///
/// `Number` is always finite. Summary options contain only finite values;
/// undefined means and sample standard deviations are `None`.
#[derive(Debug, Clone, PartialEq)]
pub enum ResultsTableCell {
    /// No value (a failed extraction or a column that does not apply).
    Missing,
    /// A text value such as a file name or status.
    Text(String),
    /// A finite numeric value in the column's native units.
    Number(f64),
    /// The sweep marker of a measurement row.
    Sweep(ResultsTableSweep),
    /// Canonical marker for the aggregate rows' filename cell.
    Overall,
    /// Mean and sample standard deviation in the column's native units.
    Summary {
        /// Mean over the finite samples; `None` when there are none.
        mean: Option<f64>,
        /// Sample standard deviation; `None` below two samples.
        sample_std_dev: Option<f64>,
    },
    /// Mean and sample standard deviation after a base-10 logarithm.
    Log10Summary {
        /// Mean of `log10(value)` over the positive samples.
        mean: Option<f64>,
        /// Sample standard deviation of `log10(value)`; `None` below two samples.
        sample_std_dev: Option<f64>,
    },
    /// Canonical marker for the aggregate rows' explanatory message cell.
    SummaryCaption,
}

/// Whether a projected row is a measured sweep or an aggregate summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultsTableRowKind {
    /// One measured sweep of one file.
    Measurement,
    /// Aggregate count for the row's sweep scope.
    Overall {
        /// Number of sweeps with a finite threshold voltage in this scope.
        count: usize,
    },
}

/// One canonical typed row.
///
/// `cells` is aligned with [`ResultsTableColumn::ALL`]. Consecutive rows that
/// share a filename (including the two overall rows) share `group_span`;
/// `group_position == 0` identifies the leader.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultsTableRow {
    /// Cells aligned with [`ResultsTableColumn::ALL`].
    pub cells: Vec<ResultsTableCell>,
    /// Sweep this row describes.
    pub sweep: ResultsTableSweep,
    /// Measurement or aggregate.
    pub kind: ResultsTableRowKind,
    /// Zero-based position within the row's filename group.
    pub group_position: usize,
    /// Number of rows in the row's filename group.
    pub group_span: usize,
}

/// Session-owned snapshot of canonical Transfer results-table semantics.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultsTableProjection {
    /// Column order of every row; always [`ResultsTableColumn::ALL`].
    pub columns: &'static [ResultsTableColumn],
    /// Measurement rows in session order followed by the two overall rows.
    pub rows: Vec<ResultsTableRow>,
}

fn typed_cell(cell: Cell) -> ResultsTableCell {
    match cell {
        Cell::Float(value) if value.is_finite() => ResultsTableCell::Number(value),
        Cell::Float(_) | Cell::Null => ResultsTableCell::Missing,
        Cell::Text(value) => ResultsTableCell::Text(value),
    }
}

fn measurement_row(
    result: &MetricResult,
    sweep: ResultsTableSweep,
    group_position: usize,
    group_span: usize,
) -> ResultsTableRow {
    let cells = ResultsTableColumn::ALL
        .into_iter()
        .map(|column| match column {
            ResultsTableColumn::Sweep => ResultsTableCell::Sweep(sweep),
            _ => typed_cell(value_for_column(column, result, sweep)),
        })
        .collect();
    ResultsTableRow {
        cells,
        sweep,
        kind: ResultsTableRowKind::Measurement,
        group_position,
        group_span,
    }
}

fn stat<'a>(stats: &'a [StatRow], scope: &str, metric: &str) -> Option<&'a StatRow> {
    stats
        .iter()
        .find(|row| row.scope == scope && row.metric == metric)
}

fn finite(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite())
}

fn summary_cell(stats: &[StatRow], scope: &str, metric: &str) -> ResultsTableCell {
    let row = stat(stats, scope, metric);
    ResultsTableCell::Summary {
        mean: row.and_then(|row| finite(row.mean)),
        sample_std_dev: row.and_then(|row| finite(row.std)),
    }
}

fn log10_summary_cell(stats: &[StatRow], scope: &str, metric: &str) -> ResultsTableCell {
    let row = stat(stats, scope, metric);
    ResultsTableCell::Log10Summary {
        mean: row.and_then(|row| finite(row.mean)),
        sample_std_dev: row.and_then(|row| finite(row.std)),
    }
}

fn overall_row(
    stats: &[StatRow],
    sweep: ResultsTableSweep,
    group_position: usize,
) -> ResultsTableRow {
    let scope = sweep.report_name();
    let count = stat(stats, scope, "Vth").map_or(0, |row| row.count);
    let cells = ResultsTableColumn::ALL
        .into_iter()
        .map(|column| match column {
            ResultsTableColumn::Filename => ResultsTableCell::Overall,
            ResultsTableColumn::Sweep => ResultsTableCell::Sweep(sweep),
            ResultsTableColumn::WidthUm
            | ResultsTableColumn::LengthUm
            | ResultsTableColumn::AspectRatio
            | ResultsTableColumn::GeometrySource
            | ResultsTableColumn::Status => ResultsTableCell::Missing,
            ResultsTableColumn::ThresholdVoltage => summary_cell(stats, scope, "Vth"),
            ResultsTableColumn::SaturationMobility => summary_cell(stats, scope, "mu_sat"),
            ResultsTableColumn::SubthresholdSwing => summary_cell(stats, scope, "SS_mV_dec"),
            ResultsTableColumn::OnCurrent => summary_cell(stats, scope, "Ion"),
            ResultsTableColumn::OffCurrent => summary_cell(stats, scope, "Ioff"),
            ResultsTableColumn::OnOffRatio => log10_summary_cell(stats, scope, "log10_Ion_Ioff"),
            ResultsTableColumn::ThresholdHysteresis => {
                summary_cell(stats, "All", "DeltaVth_hysteresis")
            }
            ResultsTableColumn::Message => ResultsTableCell::SummaryCaption,
        })
        .collect();
    ResultsTableRow {
        cells,
        sweep,
        kind: ResultsTableRowKind::Overall { count },
        group_position,
        group_span: 2,
    }
}

pub(in crate::transfer) fn project_results_table(
    results: &[MetricResult],
) -> ResultsTableProjection {
    let mut rows = Vec::new();
    for result in results {
        let sweeps = result_sweeps(result);
        for (group_position, &sweep) in sweeps.iter().enumerate() {
            rows.push(measurement_row(result, sweep, group_position, sweeps.len()));
        }
    }

    if !rows.is_empty() {
        let stats = results_to_stats(results);
        rows.push(overall_row(&stats, ResultsTableSweep::Forward, 0));
        rows.push(overall_row(&stats, ResultsTableSweep::Backward, 1));
    }

    ResultsTableProjection {
        columns: &ResultsTableColumn::ALL,
        rows,
    }
}
