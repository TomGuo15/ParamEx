//! Overall-row composition, report sections, and plain-text cell formatting.

use super::format::{fmt, fmt_engineering_current, fmt_power_of_ten, format_count};
use super::schema::{column_spec, columns_without_sweep, results_to_rows, Cell, Formatter};
use super::stats::{results_to_stats, StatRow};
use super::{ResultsTableColumn, ResultsTableSweep};
use crate::transfer::types::MetricResult;

/// A finished report section: a title row, a header row (no-sweep plain labels),
/// and plain-string data rows. Consumed by the CSV writer (`transfer::report::csv`).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ReportSection {
    pub(super) title: String,
    pub(super) header: Vec<String>,
    pub(super) rows: Vec<Vec<String>>,
}

/// Format one raw cell for display. `Text` passes through unchanged (so
/// pre-formatted Overall cells survive). For numeric columns `Null`/non-finite
/// render the formatter's NA sentinel.
pub(super) fn format_cell(column: ResultsTableColumn, value: &Cell) -> String {
    if let Cell::Text(s) = value {
        return s.clone();
    }
    let opt = match value {
        Cell::Float(v) => Some(*v),
        Cell::Null | Cell::Text(_) => None,
    };
    match column_spec(column).formatter {
        Formatter::Text => match opt {
            None => String::new(),
            // Unreachable in practice (text columns carry Text/Null), kept total.
            Some(v) => v.to_string(),
        },
        Formatter::Current => fmt_engineering_current(opt),
        Formatter::PowerOfTen => fmt_power_of_ten(opt),
        Formatter::Number => fmt(opt),
    }
}

/// How [`lookup_stat`] formats a statistic (and which sentinel a missing/`None`
/// value gets): integer count (`"0"`), engineering current, or fixed number
/// (both `"NA"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatFmt {
    /// Integer count: finite → `(v as i64)`, else `"0"`; None sentinel `"0"`.
    Count,
    /// Engineering-current format; None sentinel `"NA"`.
    Current,
    /// Fixed-number format; None sentinel `"NA"`.
    Number,
}

/// Which statistic of a [`StatRow`] to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Statistic {
    Count,
    Mean,
    Std,
}

/// Look up one statistic and format it. Missing row or `None`/non-finite value
/// → `"0"` ([`StatFmt::Count`]) or `"NA"`.
fn lookup_stat(
    stats: &[StatRow],
    scope: &str,
    metric: &str,
    statistic: Statistic,
    fmt_kind: StatFmt,
) -> String {
    let row = stats
        .iter()
        .find(|r| r.scope == scope && r.metric == metric);
    let value: Option<f64> = row.and_then(|r| match statistic {
        Statistic::Count => Some(r.count as f64),
        Statistic::Mean => r.mean,
        Statistic::Std => r.std,
    });
    match value {
        None => match fmt_kind {
            StatFmt::Count => format_count(None),
            StatFmt::Current | StatFmt::Number => "NA".to_string(),
        },
        Some(v) => match fmt_kind {
            StatFmt::Count => format_count(Some(v)),
            StatFmt::Current => fmt_engineering_current(Some(v)),
            StatFmt::Number => fmt(Some(v)),
        },
    }
}

/// `"{mean} ± {std}"`. `±` is U+00B1.
fn mean_std(stats: &[StatRow], scope: &str, metric: &str, fmt_kind: StatFmt) -> String {
    let mean = lookup_stat(stats, scope, metric, Statistic::Mean, fmt_kind);
    let std = lookup_stat(stats, scope, metric, Statistic::Std, fmt_kind);
    format!("{} \u{00B1} {}", mean, std)
}

/// Build one "Overall (sweep)" row as cells aligned to
/// [`ResultsTableColumn::ALL`]. The on/off column reports the mean ± std of
/// `log10(Ion/Ioff)`.
pub(super) fn overall_row(stats: &[StatRow], sweep: ResultsTableSweep) -> Vec<Cell> {
    let scope = sweep.report_name();
    let count = lookup_stat(stats, scope, "Vth", Statistic::Count, StatFmt::Count);
    ResultsTableColumn::ALL
        .into_iter()
        .map(|column| match column {
            ResultsTableColumn::Filename => Cell::Text("Overall".to_string()),
            ResultsTableColumn::Sweep => Cell::Text(format!("{} N={}", scope, count)),
            ResultsTableColumn::WidthUm
            | ResultsTableColumn::LengthUm
            | ResultsTableColumn::AspectRatio => Cell::Float(f64::NAN),
            ResultsTableColumn::GeometrySource | ResultsTableColumn::Status => {
                Cell::Text(String::new())
            }
            ResultsTableColumn::ThresholdVoltage => {
                Cell::Text(mean_std(stats, scope, "Vth", StatFmt::Number))
            }
            ResultsTableColumn::SaturationMobility => {
                Cell::Text(mean_std(stats, scope, "mu_sat", StatFmt::Number))
            }
            ResultsTableColumn::SubthresholdSwing => {
                Cell::Text(mean_std(stats, scope, "SS_mV_dec", StatFmt::Number))
            }
            ResultsTableColumn::OnCurrent => {
                Cell::Text(mean_std(stats, scope, "Ion", StatFmt::Current))
            }
            ResultsTableColumn::OffCurrent => {
                Cell::Text(mean_std(stats, scope, "Ioff", StatFmt::Current))
            }
            ResultsTableColumn::OnOffRatio => Cell::Text(format!(
                "log10 {}",
                mean_std(stats, scope, "log10_Ion_Ioff", StatFmt::Number)
            )),
            ResultsTableColumn::ThresholdHysteresis => Cell::Text(mean_std(
                stats,
                "All",
                "DeltaVth_hysteresis",
                StatFmt::Number,
            )),
            ResultsTableColumn::Message => Cell::Text("mean \u{00B1} std".to_string()),
        })
        .collect()
}

/// Forward/Backward sections for the human-readable CSV report. Empty when
/// there are no results. Each section masks the raw rows by sweep, appends its
/// scoped Overall row, formats every cell to plain text, and drops the Sweep
/// column.
pub(super) fn results_to_report_sections(results: &[MetricResult]) -> Vec<ReportSection> {
    if results.is_empty() {
        return Vec::new();
    }
    let raw_rows = results_to_rows(results);
    let stats = results_to_stats(results);
    let sweep_idx = ResultsTableColumn::Sweep.index();
    let no_sweep: Vec<ResultsTableColumn> = columns_without_sweep().collect();
    let header: Vec<String> = no_sweep
        .iter()
        .map(|&column| column_spec(column).label_plain.to_string())
        .collect();

    type SectionSpec = (&'static str, ResultsTableSweep, fn(&str) -> bool);
    let specs: [SectionSpec; 2] = [
        ("Forward Results", ResultsTableSweep::Forward, |s| {
            s == ResultsTableSweep::Single.report_name()
                || s == ResultsTableSweep::Forward.report_name()
        }),
        ("Backward Results", ResultsTableSweep::Backward, |s| {
            s == ResultsTableSweep::Backward.report_name()
        }),
    ];

    specs
        .iter()
        .map(|&(title, sweep, keep)| {
            let mut section_rows: Vec<Vec<Cell>> = raw_rows
                .iter()
                .filter(|row| match &row[sweep_idx] {
                    Cell::Text(s) => keep(s),
                    _ => false,
                })
                .cloned()
                .collect();
            section_rows.push(overall_row(&stats, sweep));
            let rows: Vec<Vec<String>> = section_rows
                .iter()
                .map(|row| {
                    no_sweep
                        .iter()
                        .map(|&column| format_cell(column, &row[column.index()]))
                        .collect()
                })
                .collect();
            ReportSection {
                title: title.to_string(),
                header: header.clone(),
                rows,
            }
        })
        .collect()
}
