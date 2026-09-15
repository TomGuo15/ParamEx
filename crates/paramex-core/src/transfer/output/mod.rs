//! Output-curve parsing and Early-voltage fitting for Transfer attachments.
//!
//! Output measurements sweep drain voltage (`Vd`) at fixed gate voltages
//! (`Vg`). [`raw`] turns the source grid into paired samples; this module groups
//! those samples into [`OutputCurve`] values and optionally derives
//! [`OutputSummary`] rows. Transfer parsing uses the same files only to reject
//! them from the `Vg`-sweep path.

mod raw;

use std::path::{Path, PathBuf};

use crate::shared::grid_ingest::normalized_extension;
use crate::shared::numerics::{linear_fit_with_r2, FLOAT_EPSILON};
use crate::shared::numpy_compat::nanmedian;
use crate::transfer::parse::{is_supported_measurement_path, read_measurement_file};
use crate::transfer::ParseError;

use raw::{parse_raw_output_measurements, RawOutputMeasurement, RawOutputParseError};

const EARLY_VOLTAGE_MAX_MAGNITUDE_RATIO: f64 = 10.0;

/// One output line: the drain sweep measured at a single gate voltage.
/// `vd` and `id` are paired index-for-index and keep their source signs.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputCurve {
    /// Gate voltage held during the sweep, in V.
    pub vg: f64,
    /// Drain voltage samples in V.
    pub vd: Vec<f64>,
    /// Signed drain current samples in A.
    pub id: Vec<f64>,
}

/// One parsed output-curve file: every measured gate line in source order.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputDataset {
    /// Display name, normally the source file name.
    pub name: String,
    /// Measured lines, one per distinct gate voltage.
    pub curves: Vec<OutputCurve>,
    /// Source file path; `None` for in-memory parses.
    pub source_path: Option<PathBuf>,
}

/// Family-level output fit aggregated over every fitted gate line. Float
/// fields are `NaN` when the quantity is unavailable or the lines disagree.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputSummary {
    /// Display name of the source output file.
    pub filename: String,
    /// Largest-magnitude saturation drain current among the fitted lines, in A.
    pub idsat: f64,
    /// Median output conductance (fit slope) in S.
    pub gds: f64,
    /// Median fit intercept in A.
    pub fit_intercept: f64,
    /// Output resistance `1 / |gds|` in Ω.
    pub ro: f64,
    /// Early voltage `intercept / gds` in V; `NaN` when the lines disagree.
    pub early_voltage: f64,
    /// Channel-length modulation `1 / |early_voltage|` in V⁻¹.
    pub lambda: f64,
    /// Median gate voltage of the fitted lines, in V.
    pub vg_used: f64,
    /// V_D fit range in V; `None` when the lines used different ranges.
    pub fit_range: Option<(f64, f64)>,
    /// Median coefficient of determination of the line fits.
    pub r2: f64,
    /// Number of lines whose fit succeeded.
    pub fitted_lines: usize,
    /// Number of measured lines in the dataset.
    pub total_lines: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct OutputLineFit {
    pub(super) vg: f64,
    pub(super) idsat: f64,
    pub(super) gds: f64,
    pub(super) fit_intercept: f64,
    pub(super) ro: f64,
    pub(super) early_voltage: f64,
    pub(super) lambda: f64,
    pub(super) fit_range: (f64, f64),
    pub(super) r2: f64,
}

struct FitSelection {
    fit_indices: Vec<usize>,
    idsat_indices: Vec<usize>,
    fit_range: (f64, f64),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct OutputLineOutcome {
    pub(super) vg: f64,
    pub(super) fit: Option<OutputLineFit>,
}

fn no_output_message(name: &str) -> String {
    format!(
        "No usable output curve found in {name}. Check that the file contains Vd, Vg, and Id columns."
    )
}

fn finite_sample_indices(curve: &OutputCurve) -> impl Iterator<Item = usize> + '_ {
    let len = curve.vd.len().min(curve.id.len());
    (0..len).filter(|&i| curve.vd[i].is_finite() && curve.id[i].is_finite())
}

fn same_nonzero_sign(a: f64, b: f64) -> bool {
    a != 0.0 && b != 0.0 && a.is_sign_positive() == b.is_sign_positive()
}

fn distinct_vd_indices(curve: &OutputCurve, indices: &[usize]) -> Vec<usize> {
    let mut distinct = Vec::new();
    for &idx in indices {
        if !distinct
            .iter()
            .any(|&existing| curve.vd[existing] == curve.vd[idx])
        {
            distinct.push(idx);
        }
    }
    distinct
}

fn default_fit_indices(curve: &OutputCurve) -> Option<(Vec<usize>, (f64, f64))> {
    let mut indices: Vec<usize> = finite_sample_indices(curve).collect();
    // Anchor ordering: |Vd| first, then signed Vd (so a symmetric bipolar sweep
    // picks the same branch regardless of row order — the same positive-branch
    // preference as `idsat_index`), then row order for exact-duplicate Vd
    // endpoints (first row wins, pinned by the duplicated-endpoint test).
    indices.sort_by(|&a, &b| {
        curve.vd[b]
            .abs()
            .total_cmp(&curve.vd[a].abs())
            .then_with(|| curve.vd[b].total_cmp(&curve.vd[a]))
            .then_with(|| a.cmp(&b))
    });
    let anchor = *indices.first()?;
    let anchor_vd = curve.vd[anchor];
    let next = indices.iter().copied().skip(1).find(|&i| {
        curve.vd[i] != anchor_vd && (anchor_vd == 0.0 || same_nonzero_sign(curve.vd[i], anchor_vd))
    })?;
    let mut indices = vec![anchor, next];
    indices.sort_unstable();
    let range = (curve.vd[indices[0]], curve.vd[*indices.last()?]);
    Some((indices, range))
}

fn ranged_candidate_indices(curve: &OutputCurve, range: (f64, f64)) -> Option<Vec<usize>> {
    if !range.0.is_finite() || !range.1.is_finite() {
        return None;
    }
    let same_sign_range = same_nonzero_sign(range.0, range.1);
    Some(if same_sign_range {
        let min_abs = range.0.abs().min(range.1.abs());
        let max_abs = range.0.abs().max(range.1.abs());
        finite_sample_indices(curve)
            .filter(|&i| {
                same_nonzero_sign(curve.vd[i], range.0) && {
                    let vd_abs = curve.vd[i].abs();
                    vd_abs + FLOAT_EPSILON >= min_abs && vd_abs <= max_abs + FLOAT_EPSILON
                }
            })
            .collect()
    } else {
        let min = range.0.min(range.1);
        let max = range.0.max(range.1);
        finite_sample_indices(curve)
            .filter(|&i| curve.vd[i] + FLOAT_EPSILON >= min && curve.vd[i] <= max + FLOAT_EPSILON)
            .collect()
    })
}

fn finite_nonzero(value: f64) -> bool {
    value.is_finite() && value != 0.0
}

fn fit_indices_for_range(
    curve: &OutputCurve,
    fit_range: Option<(f64, f64)>,
) -> Option<FitSelection> {
    match fit_range {
        Some(range) => {
            let idsat_indices = ranged_candidate_indices(curve, range)?;
            let fit_indices = distinct_vd_indices(curve, &idsat_indices);
            (fit_indices.len() >= 2).then_some(FitSelection {
                fit_indices,
                idsat_indices,
                fit_range: range,
            })
        }
        None => default_fit_indices(curve).map(|(fit_indices, range)| {
            let idsat_indices =
                ranged_candidate_indices(curve, range).unwrap_or_else(|| fit_indices.clone());
            FitSelection {
                fit_indices,
                idsat_indices,
                fit_range: range,
            }
        }),
    }
}

fn output_r2(vd: &[f64], id: &[f64], slope: f64, intercept: f64) -> f64 {
    if !slope.is_finite() || !intercept.is_finite() || id.is_empty() {
        return f64::NAN;
    }
    let mean_id = id.iter().sum::<f64>() / id.len() as f64;
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    for (&vd, &id) in vd.iter().zip(id.iter()) {
        let fitted = slope * vd + intercept;
        ss_res += (id - fitted) * (id - fitted);
        ss_tot += (id - mean_id) * (id - mean_id);
    }
    if ss_tot == 0.0 {
        f64::NAN
    } else {
        1.0 - (ss_res / ss_tot)
    }
}

fn median_finite(values: impl Iterator<Item = f64>) -> f64 {
    let values: Vec<f64> = values.filter(|value| value.is_finite()).collect();
    nanmedian(&values)
}

fn consistent_early_voltage(fits: &[&OutputLineFit]) -> f64 {
    let values: Vec<f64> = fits
        .iter()
        .map(|fit| fit.early_voltage)
        .filter(|value| value.is_finite())
        .collect();
    if values.is_empty() || early_voltage_family_conflicts(&values) {
        return f64::NAN;
    }
    nanmedian(&values)
}

fn early_voltage_family_conflicts(values: &[f64]) -> bool {
    if values.len() < 2 {
        return false;
    }
    let sign = values
        .iter()
        .find(|value| value.abs() > FLOAT_EPSILON)
        .map(|value| value.is_sign_positive());
    if let Some(sign) = sign {
        if values
            .iter()
            .any(|value| value.abs() > FLOAT_EPSILON && value.is_sign_positive() != sign)
        {
            return true;
        }
    }
    let (min_abs, max_abs) = values.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(min_abs, max_abs), value| {
            let mag = value.abs();
            (min_abs.min(mag), max_abs.max(mag))
        },
    );
    max_abs > FLOAT_EPSILON
        && (min_abs <= FLOAT_EPSILON || max_abs / min_abs > EARLY_VOLTAGE_MAX_MAGNITUDE_RATIO)
}

fn idsat_index(curve: &OutputCurve, indices: &[usize]) -> Option<usize> {
    indices.iter().copied().max_by(|&a, &b| {
        curve.vd[a]
            .abs()
            .total_cmp(&curve.vd[b].abs())
            .then_with(|| curve.vd[a].total_cmp(&curve.vd[b]))
            .then_with(|| curve.id[a].abs().total_cmp(&curve.id[b].abs()))
            .then_with(|| curve.id[a].total_cmp(&curve.id[b]))
    })
}

pub(super) fn extract_output_summary(
    dataset: &OutputDataset,
    fit_range: Option<(f64, f64)>,
) -> Option<OutputSummary> {
    let outcomes = extract_output_line_outcomes(dataset, fit_range);
    summarize_output_line_fits(&dataset.name, fit_range, &outcomes)
}

pub(super) fn summarize_output_line_fits(
    filename: &str,
    fit_range: Option<(f64, f64)>,
    outcomes: &[OutputLineOutcome],
) -> Option<OutputSummary> {
    let fits: Vec<&OutputLineFit> = outcomes
        .iter()
        .filter_map(|outcome| outcome.fit.as_ref())
        .collect();
    if fits.is_empty() {
        return None;
    }
    let idsat = fits
        .iter()
        .map(|fit| fit.idsat)
        .max_by(|a, b| a.abs().total_cmp(&b.abs()).then_with(|| a.total_cmp(b)))
        .unwrap_or(f64::NAN);
    let gds = median_finite(fits.iter().map(|fit| fit.gds));
    let early_voltage = consistent_early_voltage(&fits);
    Some(OutputSummary {
        filename: filename.to_owned(),
        idsat,
        gds,
        fit_intercept: median_finite(fits.iter().map(|fit| fit.fit_intercept)),
        ro: if finite_nonzero(gds) {
            1.0 / gds.abs()
        } else {
            f64::NAN
        },
        early_voltage,
        lambda: if finite_nonzero(early_voltage) {
            1.0 / early_voltage.abs()
        } else {
            f64::NAN
        },
        vg_used: median_finite(fits.iter().map(|fit| fit.vg)),
        fit_range: family_fit_range(fit_range, outcomes),
        r2: median_finite(fits.iter().map(|fit| fit.r2)),
        fitted_lines: fits.len(),
        total_lines: outcomes.len(),
    })
}

fn family_fit_range(
    explicit_range: Option<(f64, f64)>,
    outcomes: &[OutputLineOutcome],
) -> Option<(f64, f64)> {
    if explicit_range.is_some() {
        return explicit_range;
    }
    let common = outcomes.first()?.fit.as_ref()?.fit_range;
    outcomes
        .iter()
        .all(|outcome| {
            outcome
                .fit
                .as_ref()
                .is_some_and(|fit| fit.fit_range == common)
        })
        .then_some(common)
}

pub(super) fn extract_output_line_outcomes(
    dataset: &OutputDataset,
    fit_range: Option<(f64, f64)>,
) -> Vec<OutputLineOutcome> {
    let mut outcomes: Vec<OutputLineOutcome> = dataset
        .curves
        .iter()
        .map(|curve| OutputLineOutcome {
            vg: curve.vg,
            fit: fit_output_curve(curve, fit_range),
        })
        .collect();
    outcomes.sort_by(|a, b| a.vg.total_cmp(&b.vg));
    outcomes
}

fn fit_output_curve(curve: &OutputCurve, fit_range: Option<(f64, f64)>) -> Option<OutputLineFit> {
    let selection = fit_indices_for_range(curve, fit_range)?;
    let vd: Vec<f64> = selection.fit_indices.iter().map(|&i| curve.vd[i]).collect();
    let id: Vec<f64> = selection.fit_indices.iter().map(|&i| curve.id[i]).collect();
    let (gds, intercept, _shared_r2, points) = linear_fit_with_r2(&vd, &id);
    if points < 2 {
        return None;
    }
    let idsat_idx = idsat_index(curve, &selection.idsat_indices)?;
    let idsat = curve.id[idsat_idx];
    let ro = if finite_nonzero(gds) {
        1.0 / gds.abs()
    } else {
        f64::NAN
    };
    let early_voltage = if finite_nonzero(gds) {
        intercept / gds
    } else {
        f64::NAN
    };
    let lambda = if finite_nonzero(early_voltage) {
        1.0 / early_voltage.abs()
    } else {
        f64::NAN
    };
    Some(OutputLineFit {
        vg: curve.vg,
        idsat,
        gds,
        fit_intercept: intercept,
        ro,
        early_voltage,
        lambda,
        fit_range: selection.fit_range,
        r2: output_r2(&vd, &id, gds, intercept),
    })
}

fn project_transfer_output(raw: RawOutputMeasurement) -> Vec<OutputCurve> {
    let mut groups: Vec<(f64, Vec<f64>, Vec<f64>)> = Vec::new();
    for sample in raw.samples {
        if let Some((_, vd, id)) = groups.iter_mut().find(|(vg, _, _)| *vg == sample.vg) {
            vd.push(sample.vd);
            id.push(sample.id);
        } else {
            groups.push((sample.vg, vec![sample.vd], vec![sample.id]));
        }
    }

    groups
        .into_iter()
        .filter(|(_, vd, id)| !vd.is_empty() && vd.len() == id.len())
        .map(|(vg, vd, id)| OutputCurve { vg, vd, id })
        .collect()
}

fn parse_output_content(
    name: &str,
    suffix: &str,
    content: &[u8],
    source_path: Option<PathBuf>,
) -> Result<OutputDataset, ParseError> {
    let raw_measurements =
        parse_raw_output_measurements(content, suffix).map_err(|error| match error {
            RawOutputParseError::GridRead(message) => ParseError(message),
            RawOutputParseError::NoRows
            | RawOutputParseError::MissingColumns
            | RawOutputParseError::NoSamples => ParseError(no_output_message(name)),
        })?;
    for raw in raw_measurements {
        let curves = project_transfer_output(raw);
        // A workbook may contain a short preview grid before the real output
        // measurement. Select the first grid with at least three samples, then
        // retain every measured line from that chosen grid so the report can
        // mark weaker siblings unavailable.
        if curves.iter().any(|curve| curve.vd.len() >= 3) {
            return Ok(OutputDataset {
                name: name.to_owned(),
                curves,
                source_path,
            });
        }
    }
    Err(ParseError(no_output_message(name)))
}

/// Parse an output-curve upload from memory. Only `name`'s extension is
/// consulted for format; the returned `source_path` is `None`.
pub fn parse_output_bytes(name: &str, content: &[u8]) -> Result<OutputDataset, ParseError> {
    let suffix = normalized_extension(Path::new(name));
    if !is_supported_measurement_path(Path::new(name)) {
        return Err(ParseError(format!("Unsupported file extension: {suffix}")));
    }
    parse_output_content(name, &suffix, content, None)
}

/// Parse an output-curve file from disk. The returned dataset's `source_path`
/// is the input path.
pub fn parse_output_file(path: &Path) -> Result<OutputDataset, ParseError> {
    let (name, suffix, content) = read_measurement_file(path)?;
    parse_output_content(&name, &suffix, &content, Some(path.to_path_buf()))
}
