//! Aggregate statistics over the metric rows.

use crate::shared::numpy_compat::{nanmedian, std_sample};
use crate::transfer::types::MetricResult;

/// One long-format aggregate-stats row. `count` is the non-NaN sample size;
/// the rest are `None` when undefined (empty column, or `std` with count < 2,
/// or a non-finite result).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct StatRow {
    pub(super) scope: String,
    pub(super) metric: String,
    pub(super) count: usize,
    pub(super) mean: Option<f64>,
    pub(super) std: Option<f64>,
    pub(super) min: Option<f64>,
    pub(super) median: Option<f64>,
    pub(super) max: Option<f64>,
}

/// Non-finite → `None`.
fn export_opt(v: f64) -> Option<f64> {
    if v.is_finite() {
        Some(v)
    } else {
        None
    }
}

/// The undefined-stat sentinel.
///
/// The reference CSV/JSON encodes an undefined statistic (empty column, or
/// `std` with count < 2) as `NaN` rather than a missing value, because its
/// numeric stat columns are uniformly floating point. The report therefore
/// emits `Some(NaN)` here so the downstream contract matches.
fn undefined_stat() -> Option<f64> {
    Some(f64::NAN)
}

/// `log10(x)` where `x > 0`, else `NaN`, elementwise.
fn log10_where_positive(values: &[f64]) -> Vec<f64> {
    values
        .iter()
        .map(|&v| if v > 0.0 { v.log10() } else { f64::NAN })
        .collect()
}

/// Compute one `StatRow` from a column that may contain NaN (NaN samples are
/// dropped first).
fn stat_row(scope: &str, metric: &str, column: &[f64]) -> StatRow {
    let v: Vec<f64> = column.iter().copied().filter(|x| !x.is_nan()).collect();
    let count = v.len();
    let (mean, std, min, median, max) = if v.is_empty() {
        (
            undefined_stat(),
            undefined_stat(),
            undefined_stat(),
            undefined_stat(),
            undefined_stat(),
        )
    } else {
        let n = v.len() as f64;
        let mean = export_opt(v.iter().sum::<f64>() / n);
        // A sample standard deviation needs two samples.
        let std = if v.len() < 2 {
            undefined_stat()
        } else {
            export_opt(std_sample(&v))
        };
        let min = export_opt(v.iter().copied().fold(f64::INFINITY, f64::min));
        let max = export_opt(v.iter().copied().fold(f64::NEG_INFINITY, f64::max));
        let median = export_opt(nanmedian(&v));
        (mean, std, min, median, max)
    };
    StatRow {
        scope: scope.to_string(),
        metric: metric.to_string(),
        count,
        mean,
        std,
        min,
        median,
        max,
    }
}

/// Collect a column, mapping non-finite values to the NaN sentinel the column
/// math drops.
fn col_export(values: impl Iterator<Item = f64>) -> Vec<f64> {
    values
        .map(|v| if v.is_finite() { v } else { f64::NAN })
        .collect()
}

/// Long-format aggregate stats: scope "All" over file metrics, then "Forward"
/// and "Backward" over per-sweep metrics.
pub(super) fn results_to_stats(results: &[MetricResult]) -> Vec<StatRow> {
    let mut rows = Vec::new();

    // File scope ("All"): one value per file regardless of sweep count.
    let file_columns: [(&str, Vec<f64>); 4] = [
        ("W_um", col_export(results.iter().map(|r| r.width_um))),
        ("L_um", col_export(results.iter().map(|r| r.length_um))),
        (
            "W_over_L",
            col_export(results.iter().map(|r| r.aspect_ratio)),
        ),
        (
            "DeltaVth_hysteresis",
            col_export(results.iter().map(|r| r.delta_vth_hysteresis)),
        ),
    ];
    for (metric, column) in &file_columns {
        rows.push(stat_row("All", metric, column));
    }

    // Forward scope: every result contributes its forward values.
    rows.extend(sweep_stat_rows("Forward", results));
    // Backward scope: only dual-sweep results contribute.
    rows.extend(sweep_stat_rows("Backward", results));
    rows
}

/// Per-sweep stat rows for one scope. Forward uses every result's `*_forward`
/// values; Backward uses only `has_backward_sweep` results' `*_backward` values.
fn sweep_stat_rows(scope: &str, results: &[MetricResult]) -> Vec<StatRow> {
    let forward = scope == "Forward";
    let selected: Vec<&MetricResult> = if forward {
        results.iter().collect()
    } else {
        results.iter().filter(|r| r.has_backward_sweep).collect()
    };

    let pick =
        |f: fn(&MetricResult) -> f64| -> Vec<f64> { col_export(selected.iter().map(|&r| f(r))) };
    // Pick the forward or backward field accessor once, per `scope`.
    let col = |fwd_fn: fn(&MetricResult) -> f64, bwd_fn: fn(&MetricResult) -> f64| -> Vec<f64> {
        if forward {
            pick(fwd_fn)
        } else {
            pick(bwd_fn)
        }
    };
    let vth = col(|r| r.vt_forward, |r| r.vt_backward);
    let mu = col(|r| r.mu_sat_forward, |r| r.mu_sat_backward);
    let ss = col(|r| r.ss_mv_dec_forward, |r| r.ss_mv_dec_backward);
    let ion = col(|r| r.ion_forward, |r| r.ion_backward);
    let ioff = col(|r| r.ioff_forward, |r| r.ioff_backward);
    let ratio = col(|r| r.on_off_ratio_forward, |r| r.on_off_ratio_backward);
    let log_ion = log10_where_positive(&ion);
    let log_ioff = log10_where_positive(&ioff);
    let log_ratio = log10_where_positive(&ratio);

    let sweep_columns: [(&str, &[f64]); 8] = [
        ("Vth", &vth),
        ("mu_sat", &mu),
        ("SS_mV_dec", &ss),
        ("Ion", &ion),
        ("Ioff", &ioff),
        ("log10_Ion", &log_ion),
        ("log10_Ioff", &log_ioff),
        ("log10_Ion_Ioff", &log_ratio),
    ];
    sweep_columns
        .iter()
        .map(|(metric, column)| stat_row(scope, metric, column))
        .collect()
}
