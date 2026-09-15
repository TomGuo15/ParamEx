//! Pre-formatted TLM table row projections.
//!
//! `TlmState` rebuilds these once per session generation so render code reads
//! stable rows instead of formatting engine data every frame.

use paramex_core::tlm::{Status, TlmAnalysisResult, TlmSweepResult};

// Layering: state may reach the leaf `format_ui` util (pure string formatters).
use crate::format_ui::{fmt_eng, fmt_num3, fmt_r2};

/// The shared fit cells (intercept (2R_c), R_c/contact, slope, R²) for a result/sweep row.
fn fit_cells(intercept: f64, rc_per: f64, slope: f64, r2: f64) -> Vec<String> {
    vec![
        fmt_eng(intercept),
        fmt_eng(rc_per),
        fmt_eng(slope),
        fmt_r2(r2),
    ]
}

/// One Results row per group: group, intercept (2R_c), R_c/contact, slope, R², N,
/// warnings. (No V_G cell: it is constant across the table; see `RESULT_COLS`.)
fn result_rows(result: &TlmAnalysisResult) -> Vec<Vec<String>> {
    result
        .groups
        .iter()
        .map(|g| {
            let mut row = vec![g.group.clone()];
            row.extend(fit_cells(
                g.fit.intercept_ohm,
                g.fit.rc_per_contact_ohm,
                g.fit.slope_ohm_per_um,
                g.fit.r_squared,
            ));
            row.push(g.points.len().to_string());
            row.push(g.warnings.join("; "));
            row
        })
        .collect()
}

/// One Voltage-Sweep row per (group, V_G).
fn sweep_rows(sweep: &TlmSweepResult) -> Vec<Vec<String>> {
    sweep
        .points
        .iter()
        .map(|p| {
            let mut row = vec![p.group.clone(), fmt_num3(p.selected_vg)];
            row.extend(fit_cells(
                p.fit.intercept_ohm,
                p.fit.rc_per_contact_ohm,
                p.fit.slope_ohm_per_um,
                p.fit.r_squared,
            ));
            row.push(p.valid_lengths.to_string());
            row.push(p.warnings.join("; "));
            row
        })
        .collect()
}

/// One Length-Points row per (group, length): group, L, the point's actual V_G,
/// I, R_total, I(median), R_total(median), devices, file. The table-wide selected
/// V_G is not a column: it lives in the ANALYSIS and SELECTED cards.
fn length_rows(result: &TlmAnalysisResult) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for g in &result.groups {
        for p in &g.points {
            rows.push(vec![
                p.group.clone(),
                fmt_num3(p.length_um),
                fmt_num3(p.actual_vg),
                fmt_eng(p.current_a),
                fmt_eng(p.rtotal_ohm),
                fmt_eng(p.current_median_a),
                fmt_eng(p.rtotal_median_ohm),
                p.device_count.to_string(),
                p.selected_file.clone(),
            ]);
        }
    }
    rows
}

/// One File-Status row per file: file, status, plus the parse message as a
/// trailing cell beyond `STATUS_COLS`. The TLM grid renderer surfaces that
/// trailing value as hover text on the status cell.
fn status_rows(result: &TlmAnalysisResult) -> Vec<Vec<String>> {
    result
        .statuses
        .iter()
        .map(|s| {
            vec![
                s.file.clone(),
                s.status.as_str().to_string(),
                s.message.clone(),
            ]
        })
        .collect()
}

/// Pre-formatted display rows for the four TLM tables. Rebuilt only by
/// [`super::TlmState`] when the session's analyses change.
#[derive(Debug, Clone, Default)]
pub struct TlmRows {
    results: Vec<Vec<String>>,
    sweep: Vec<Vec<String>>,
    lengths: Vec<Vec<String>>,
    status: Vec<Vec<String>>,
}

impl TlmRows {
    pub fn results(&self) -> &[Vec<String>] {
        &self.results
    }

    pub fn sweep(&self) -> &[Vec<String>] {
        &self.sweep
    }

    pub fn lengths(&self) -> &[Vec<String>] {
        &self.lengths
    }

    pub fn status(&self) -> &[Vec<String>] {
        &self.status
    }

    /// Build all cached rows from the session's current analyses.
    pub(super) fn from_analyses(result: &TlmAnalysisResult, sweep: &TlmSweepResult) -> Self {
        Self {
            results: result_rows(result),
            sweep: sweep_rows(sweep),
            lengths: length_rows(result),
            status: status_rows(result),
        }
    }
}

/// How many workbooks failed to parse; drives the FILES card's header pill.
pub(super) fn error_count(result: &TlmAnalysisResult) -> usize {
    result
        .statuses
        .iter()
        .filter(|s| s.status != Status::Ok)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use paramex_core::tlm::{
        FileStatus, GroupAnalysis, LengthPoint, TlmAnalysisResult, TlmFitSummary, TlmSweepResult,
        VdSource, VoltageSweepPoint,
    };

    fn analysis() -> TlmAnalysisResult {
        TlmAnalysisResult {
            root: "root".to_string(),
            selected_vg: 1.0,
            vg_values: vec![1.0],
            groups: vec![GroupAnalysis {
                group: "G1".to_string(),
                selected_vg: 1.0,
                points: vec![LengthPoint {
                    group: "G1".to_string(),
                    length_um: 10.0,
                    selected_vg: 1.0,
                    actual_vg: 1.0,
                    current_a: 1e-6,
                    rtotal_ohm: 393_910.0,
                    current_median_a: 8e-7,
                    rtotal_median_ohm: 500_000.0,
                    device_count: 2,
                    selected_file: "device.xlsx".to_string(),
                }],
                fit: TlmFitSummary {
                    intercept_ohm: 393_910.0,
                    rc_per_contact_ohm: 196_955.0,
                    slope_ohm_per_um: 12_345.0,
                    r_squared: 0.9876,
                    intercept_median_ohm: 500_000.0,
                    rc_per_contact_median_ohm: 250_000.0,
                    slope_median_ohm_per_um: 20_000.0,
                    r_squared_median: 0.9,
                },
                warnings: vec!["checked".to_string()],
            }],
            statuses: vec![
                FileStatus {
                    file: "ok.xlsx".to_string(),
                    group: "G1".to_string(),
                    length_um: Some(10.0),
                    status: Status::Ok,
                    message: String::new(),
                    vd_source: VdSource::Setup,
                },
                FileStatus {
                    file: "bad.xlsx".to_string(),
                    group: "G1".to_string(),
                    length_um: None,
                    status: Status::Error,
                    message: "parse failed".to_string(),
                    vd_source: VdSource::Unread,
                },
            ],
        }
    }

    fn sweep() -> TlmSweepResult {
        TlmSweepResult {
            root: "root".to_string(),
            vg_values: vec![1.0],
            points: vec![VoltageSweepPoint {
                group: "G1".to_string(),
                selected_vg: 1.0,
                fit: TlmFitSummary {
                    intercept_ohm: 393_910.0,
                    rc_per_contact_ohm: 196_955.0,
                    slope_ohm_per_um: 12_345.0,
                    r_squared: 0.9876,
                    intercept_median_ohm: 500_000.0,
                    rc_per_contact_median_ohm: 250_000.0,
                    slope_median_ohm_per_um: 20_000.0,
                    r_squared_median: 0.9,
                },
                valid_lengths: 3,
                warnings: vec!["checked".to_string()],
            }],
        }
    }

    #[test]
    fn private_result_rows_shape_and_format_cells() {
        let rows = result_rows(&analysis());

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].len(), 7);
        assert_eq!(rows[0][0], "G1");
        assert_eq!(rows[0][5], "1");
        assert_eq!(rows[0][6], "checked");
        assert!(
            rows[0].iter().any(|cell| cell.contains('k')),
            "fit cells should use shared engineering formatting"
        );
    }

    #[test]
    fn private_sweep_and_length_rows_shape() {
        let analysis = analysis();
        let sweep = sweep();

        assert_eq!(sweep_rows(&sweep).len(), sweep.points.len());
        assert_eq!(
            length_rows(&analysis).len(),
            analysis.groups[0].points.len()
        );
        assert_eq!(length_rows(&analysis)[0][8], "device.xlsx");
    }

    #[test]
    fn private_status_rows_keep_hover_payload_and_error_count() {
        let analysis = analysis();
        let rows = status_rows(&analysis);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["ok.xlsx", "ok", ""]);
        assert_eq!(rows[1], vec!["bad.xlsx", "error", "parse failed"]);
        assert_eq!(error_count(&analysis), 1);
    }
}
