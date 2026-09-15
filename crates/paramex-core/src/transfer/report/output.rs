//! Canonical Transfer output-fit report projection and CSV serialization.

use crate::shared::csv::{write_row, UTF8_BOM};
use crate::transfer::output::{
    extract_output_line_outcomes, summarize_output_line_fits, OutputDataset,
};

/// Whether one output-report row summarizes the whole gate family or one
/// gate-voltage line fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFitKind {
    /// Aggregate over every fitted gate line of one output file.
    Family,
    /// One gate-voltage line fit.
    Line,
}

impl OutputFitKind {
    /// The text the output table and CSV show for this row kind.
    pub fn label(self) -> &'static str {
        match self {
            Self::Family => "Family",
            Self::Line => "Line",
        }
    }
}

/// Whether the scientific values in one output-report row are complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFitStatus {
    /// Every value in the row is available.
    Ok,
    /// A family row whose lines did not all fit.
    Partial,
    /// No fit was possible; numeric values are `NaN`.
    Unavailable,
}

impl OutputFitStatus {
    /// The text the output table and CSV show for this status.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Partial => "partial",
            Self::Unavailable => "unavailable",
        }
    }
}

/// One ordered scientific row shared by the output results table and CSV.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputReportRow {
    /// Transfer file the output is attached to.
    pub device: String,
    /// Output file name.
    pub output_file: String,
    /// Family aggregate or single line.
    pub fit: OutputFitKind,
    /// Completeness of the row's values.
    pub status: OutputFitStatus,
    /// Gate voltage in V (the median over fitted lines for a family row).
    pub vg: f64,
    /// Saturation drain current in A.
    pub idsat: f64,
    /// Output conductance in S.
    pub gds: f64,
    /// Output resistance in Ω.
    pub ro: f64,
    /// Early voltage in V.
    pub early_voltage: f64,
    /// Channel-length modulation in V⁻¹.
    pub lambda: f64,
    /// V_D fit range in V, as selected or as chosen automatically.
    pub fit_range: Option<(f64, f64)>,
    /// Coefficient of determination of the fit.
    pub r2: f64,
}

/// Project one output attachment into its family row followed by ascending-V_G
/// line rows.
pub(in crate::transfer) fn project_output_report_rows(
    device: &str,
    output: &OutputDataset,
    fit_range: Option<(f64, f64)>,
) -> Vec<OutputReportRow> {
    let outcomes = extract_output_line_outcomes(output, fit_range);
    let summary = summarize_output_line_fits(&output.name, fit_range, &outcomes);
    let fitted_lines = summary.as_ref().map_or(0, |summary| summary.fitted_lines);
    let family_status = if fitted_lines == 0 {
        OutputFitStatus::Unavailable
    } else if fitted_lines < outcomes.len() {
        OutputFitStatus::Partial
    } else {
        OutputFitStatus::Ok
    };
    let mut rows = vec![OutputReportRow {
        device: device.to_owned(),
        output_file: output.name.clone(),
        fit: OutputFitKind::Family,
        status: family_status,
        vg: summary.as_ref().map_or(f64::NAN, |summary| summary.vg_used),
        idsat: summary.as_ref().map_or(f64::NAN, |summary| summary.idsat),
        gds: summary.as_ref().map_or(f64::NAN, |summary| summary.gds),
        ro: summary.as_ref().map_or(f64::NAN, |summary| summary.ro),
        early_voltage: summary
            .as_ref()
            .map_or(f64::NAN, |summary| summary.early_voltage),
        lambda: summary.as_ref().map_or(f64::NAN, |summary| summary.lambda),
        fit_range: summary
            .as_ref()
            .and_then(|summary| summary.fit_range)
            .or(fit_range),
        r2: summary.as_ref().map_or(f64::NAN, |summary| summary.r2),
    }];
    rows.extend(outcomes.into_iter().map(|outcome| {
        let status = if outcome.fit.is_some() {
            OutputFitStatus::Ok
        } else {
            OutputFitStatus::Unavailable
        };
        OutputReportRow {
            device: device.to_owned(),
            output_file: output.name.clone(),
            fit: OutputFitKind::Line,
            status,
            vg: outcome.vg,
            idsat: outcome.fit.as_ref().map_or(f64::NAN, |fit| fit.idsat),
            gds: outcome.fit.as_ref().map_or(f64::NAN, |fit| fit.gds),
            ro: outcome.fit.as_ref().map_or(f64::NAN, |fit| fit.ro),
            early_voltage: outcome
                .fit
                .as_ref()
                .map_or(f64::NAN, |fit| fit.early_voltage),
            lambda: outcome.fit.as_ref().map_or(f64::NAN, |fit| fit.lambda),
            fit_range: outcome.fit.as_ref().map(|fit| fit.fit_range).or(fit_range),
            r2: outcome.fit.as_ref().map_or(f64::NAN, |fit| fit.r2),
        }
    }));
    rows
}

/// Serialize the canonical output report to UTF-8-BOM/CRLF CSV bytes.
///
/// Empty rows produce empty bytes. Non-finite numeric values are blank and fit
/// bounds are ordered numerically in the exported columns.
pub(in crate::transfer) fn export_output_report_bytes(rows: &[OutputReportRow]) -> Vec<u8> {
    if rows.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    out.extend_from_slice(&UTF8_BOM);
    out.extend_from_slice(
        b"device,output_file,fit,status,Vg,Idsat,gds,ro,Early voltage,lambda,Vds fit min,Vds fit max,R2\r\n",
    );
    for row in rows {
        let (fit_min, fit_max) = fit_range_fields(row.fit_range);
        write_row(
            &mut out,
            &[
                row.device.clone(),
                row.output_file.clone(),
                row.fit.label().to_owned(),
                row.status.label().to_owned(),
                format_float(row.vg),
                format_float(row.idsat),
                format_float(row.gds),
                format_float(row.ro),
                format_float(row.early_voltage),
                format_float(row.lambda),
                fit_min,
                fit_max,
                format_float(row.r2),
            ],
        );
    }
    out
}

fn fit_range_fields(range: Option<(f64, f64)>) -> (String, String) {
    let Some((a, b)) = range else {
        return (String::new(), String::new());
    };
    if !a.is_finite() || !b.is_finite() {
        return (String::new(), String::new());
    }
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    (format_float(lo), format_float(hi))
}

fn format_float(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.6e}")
    } else {
        String::new()
    }
}
