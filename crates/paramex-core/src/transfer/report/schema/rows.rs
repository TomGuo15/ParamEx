//! Raw report-cell projection from `MetricResult`.

use crate::transfer::types::MetricResult;

use super::super::{ResultsTableColumn, ResultsTableSweep};

/// A report cell: a finite float, a blank, or a string. Pre-formatted
/// "Overall" cells are `Text`; per-file numeric cells are `Float`/`Null`.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::transfer::report) enum Cell {
    Float(f64),
    Null,
    Text(String),
}

/// Finite -> `Float`, else `Null`.
pub(in crate::transfer::report) fn export_float(value: f64) -> Cell {
    if value.is_finite() {
        Cell::Float(value)
    } else {
        Cell::Null
    }
}

/// Per-(file, sweep) raw cell for `column`.
pub(in crate::transfer::report) fn value_for_column(
    column: ResultsTableColumn,
    result: &MetricResult,
    sweep: ResultsTableSweep,
) -> Cell {
    match column {
        ResultsTableColumn::Filename => Cell::Text(result.filename.clone()),
        ResultsTableColumn::Sweep => Cell::Text(sweep.report_name().to_string()),
        ResultsTableColumn::WidthUm => export_float(result.width_um),
        ResultsTableColumn::LengthUm => export_float(result.length_um),
        ResultsTableColumn::AspectRatio => export_float(result.aspect_ratio),
        ResultsTableColumn::GeometrySource => Cell::Text(result.geometry_source.to_string()),
        ResultsTableColumn::ThresholdHysteresis => export_float(result.delta_vth_hysteresis),
        ResultsTableColumn::Status => Cell::Text(result.status.to_string()),
        ResultsTableColumn::Message => Cell::Text(result.message.clone()),
        ResultsTableColumn::ThresholdVoltage => export_float(sweep_pick(
            sweep,
            result.vt_forward,
            result.vt_backward,
            result.vt,
        )),
        ResultsTableColumn::SaturationMobility => export_float(sweep_pick(
            sweep,
            result.mu_sat_forward,
            result.mu_sat_backward,
            result.mu_sat,
        )),
        ResultsTableColumn::SubthresholdSwing => export_float(sweep_pick(
            sweep,
            result.ss_mv_dec_forward,
            result.ss_mv_dec_backward,
            result.ss_mv_dec,
        )),
        ResultsTableColumn::OnCurrent => export_float(sweep_pick(
            sweep,
            result.ion_forward,
            result.ion_backward,
            result.ion,
        )),
        ResultsTableColumn::OffCurrent => export_float(sweep_pick(
            sweep,
            result.ioff_forward,
            result.ioff_backward,
            result.ioff,
        )),
        ResultsTableColumn::OnOffRatio => export_float(sweep_pick(
            sweep,
            result.on_off_ratio_forward,
            result.on_off_ratio_backward,
            result.on_off_ratio,
        )),
    }
}

/// Pick the forward/backward/single attribute for `sweep`.
fn sweep_pick(sweep: ResultsTableSweep, forward: f64, backward: f64, single: f64) -> f64 {
    match sweep {
        ResultsTableSweep::Forward => forward,
        ResultsTableSweep::Backward => backward,
        ResultsTableSweep::Single => single,
    }
}

/// The sweeps a result contributes rows for: `Forward` then `Backward` for a
/// double-sweep file, one `Single` otherwise.
pub(in crate::transfer::report) fn result_sweeps(
    result: &MetricResult,
) -> &'static [ResultsTableSweep] {
    if result.has_backward_sweep {
        &[ResultsTableSweep::Forward, ResultsTableSweep::Backward]
    } else {
        &[ResultsTableSweep::Single]
    }
}

/// One raw row per (file, sweep), cells aligned to [`ResultsTableColumn::ALL`].
pub(in crate::transfer::report) fn results_to_rows(results: &[MetricResult]) -> Vec<Vec<Cell>> {
    let mut rows = Vec::new();
    for result in results {
        for &sweep in result_sweeps(result) {
            rows.push(
                ResultsTableColumn::ALL
                    .into_iter()
                    .map(|column| value_for_column(column, result, sweep))
                    .collect(),
            );
        }
    }
    rows
}
