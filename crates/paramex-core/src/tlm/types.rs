//! TLM domain types: validated samples, curves, datasets, and analysis results.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};

use crate::shared::numpy_compat::banker_round;

/// Raised when a TLM workbook cannot be interpreted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlmParseError(pub String);

impl fmt::Display for TlmParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for TlmParseError {}

/// Two measured gate voltages closer than this are the same `V_G`. It is the
/// resolution `available_vg_values` rounds to, and the slack allowed before a
/// nearest-`V_G` substitution is reported as a warning.
pub(super) const VG_MATCH_TOLERANCE_V: f64 = 1e-9;

/// Reciprocal of `VG_MATCH_TOLERANCE_V`, spelled as a literal so the rounding
/// scale is exact in f64.
const VG_ROUND_SCALE: f64 = 1e9;

/// Per-workbook ingest outcome. The string form is the `status.csv` cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The workbook parsed into a curve.
    Ok,
    /// The workbook could not be parsed; the row carries the reason.
    Error,
}
impl Status {
    /// Lower-case CSV cell text.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Error => "error",
        }
    }
}
impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where a curve's drain bias came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VdSource {
    /// Read from the workbook's `Setup(*)` sheet.
    Setup,
    /// The workbook has no `Setup(*)` sheet; the user-entered fallback applies.
    Fallback,
    /// No drain bias was read because the workbook failed to parse.
    Unread,
}
impl VdSource {
    /// Lower-case CSV cell text.
    pub fn as_str(self) -> &'static str {
        match self {
            VdSource::Setup => "setup",
            VdSource::Fallback => "fallback",
            VdSource::Unread => "unread",
        }
    }
}
impl fmt::Display for VdSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One finite measured row from a TLM workbook.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TlmSample {
    /// Gate voltage in volts.
    vg: f64,
    /// Drain-current magnitude in amperes; non-negative.
    abs_id: f64,
    /// Source-current magnitude in amperes; non-negative.
    abs_is: f64,
}

impl TlmSample {
    /// Validate one measured row and store terminal currents as magnitudes.
    /// Some instruments export the `abs_*` columns with a sign, so finite input
    /// is normalized here before the rest of TLM uses the magnitude contract.
    pub fn try_new(vg: f64, abs_id: f64, abs_is: f64) -> Result<Self, TlmParseError> {
        if !vg.is_finite() {
            return Err(TlmParseError(
                "TLM sample V_G must be a finite number".to_string(),
            ));
        }
        if !abs_id.is_finite() {
            return Err(TlmParseError(
                "TLM sample abs_id must be a finite number".to_string(),
            ));
        }
        if !abs_is.is_finite() {
            return Err(TlmParseError(
                "TLM sample abs_is must be a finite number".to_string(),
            ));
        }
        Ok(Self {
            vg,
            abs_id: abs_id.abs(),
            abs_is: abs_is.abs(),
        })
    }

    /// Gate voltage in volts.
    pub fn vg(self) -> f64 {
        self.vg
    }

    /// Drain-current magnitude in amperes.
    pub fn abs_id(self) -> f64 {
        self.abs_id
    }

    /// Source-current magnitude in amperes.
    pub fn abs_is(self) -> f64 {
        self.abs_is
    }
}

/// One parsed workbook with non-empty samples sorted by ascending `V_G`.
#[derive(Debug, Clone, PartialEq)]
pub struct TlmCurve {
    file_path: String,
    group: String,
    length_um: f64,
    device_id: String,
    samples: Vec<TlmSample>,
    vd: f64,
    vd_source: VdSource,
}

impl TlmCurve {
    /// Validate a parsed workbook and sort its samples by ascending `V_G`.
    /// The sort is stable so repeated measurements at one `V_G` keep file order.
    pub fn try_new(
        file_path: String,
        group: String,
        length_um: f64,
        mut samples: Vec<TlmSample>,
        vd: f64,
        vd_source: VdSource,
    ) -> Result<Self, TlmParseError> {
        if file_path.trim().is_empty() {
            return Err(TlmParseError(
                "TLM curve file path must not be empty".to_string(),
            ));
        }
        if group.trim().is_empty() {
            return Err(TlmParseError(
                "TLM curve group must not be empty".to_string(),
            ));
        }
        if !length_um.is_finite() {
            return Err(TlmParseError(
                "TLM curve channel length must be a finite number".to_string(),
            ));
        }
        if samples.is_empty() {
            return Err(TlmParseError(
                "TLM curve must contain at least one sample".to_string(),
            ));
        }
        let vd = valid_vd(vd, "TLM curve V_D")?;
        if vd_source == VdSource::Unread {
            return Err(TlmParseError(
                "TLM curve V_D source must be setup or fallback".to_string(),
            ));
        }
        let device_id = Path::new(&file_path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.trim().is_empty())
            .ok_or_else(|| TlmParseError("TLM curve file path must identify a device".to_string()))?
            .to_string();
        samples.sort_by(|left, right| {
            left.vg
                .partial_cmp(&right.vg)
                .expect("validated TLM sample voltages are finite")
        });

        Ok(Self {
            file_path,
            group,
            length_um,
            device_id,
            samples,
            vd,
            vd_source,
        })
    }

    /// Full path of the source workbook as given to the parser.
    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    /// Process-group name derived from the folder layout.
    pub fn group(&self) -> &str {
        &self.group
    }

    /// Channel length in micrometers derived from the folder layout.
    pub fn length_um(&self) -> f64 {
        self.length_um
    }

    /// Workbook file stem, used as the device label.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Measured rows sorted by ascending `V_G`.
    pub fn samples(&self) -> &[TlmSample] {
        &self.samples
    }

    /// Drain bias in volts; finite and nonzero.
    pub fn vd(&self) -> f64 {
        self.vd
    }

    /// Whether `vd` came from the workbook or the user fallback.
    pub fn vd_source(&self) -> VdSource {
        self.vd_source
    }

    /// Channel current at the gate voltage nearest `selected_vg`, as
    /// `(current, actual_vg)`. The current is `min(abs_id, abs_is)`: the smaller
    /// terminal current is the one least inflated by gate leakage.
    pub fn current_at(&self, selected_vg: f64) -> (f64, f64) {
        let size = self.samples.len();
        let pos = self
            .samples
            .partition_point(|sample| sample.vg < selected_vg);
        let index = if pos == 0 {
            0
        } else if pos == size {
            size - 1
        } else {
            let left = self.samples[pos - 1].vg;
            let right = self.samples[pos].vg;
            if (right - selected_vg).abs() < (selected_vg - left).abs() {
                pos
            } else {
                pos - 1
            }
        };
        // Duplicate V_G rows represent repeated measurements. The first row
        // wins on ties so the answer does not depend on the search path.
        let vg = self.samples[index].vg;
        let index = self.samples[..=index]
            .iter()
            .position(|sample| sample.vg == vg)
            .unwrap_or(index);
        let sample = self.samples[index];
        let current = sample.abs_id.min(sample.abs_is);
        (current, sample.vg)
    }
}

/// One `status.csv` row: the ingest outcome for a single workbook.
#[derive(Debug, Clone, PartialEq)]
pub struct FileStatus {
    /// Path relative to the dataset root, with OS separators.
    pub file: String,
    /// Process-group name, or the best guess from the path on failure.
    pub group: String,
    /// Channel length when the path yields one.
    pub length_um: Option<f64>,
    /// Whether the workbook produced a curve.
    pub status: Status,
    /// Human-readable outcome: "Loaded", the fallback note, or the error.
    pub message: String,
    /// Where the drain bias came from; `Unread` for failed workbooks.
    pub vd_source: VdSource,
}

/// Bundle of parsed workbooks plus the per-file statuses, with the measured
/// gate-voltage set derived once at construction.
#[derive(Debug, Clone, PartialEq)]
pub struct TlmDataset {
    root: String,
    curves: Vec<TlmCurve>,
    statuses: Vec<FileStatus>,
    vg_values: Vec<f64>,
}

/// Result of removing one workbook from an admitted TLM dataset.
///
/// `dataset` is `None` when the removal consumed the final successfully parsed
/// curve, so callers cannot retain an error-only aggregate.
#[must_use = "workbook removal must retain the returned dataset or handle its terminal state"]
#[derive(Debug, Clone, PartialEq)]
pub struct TlmDatasetRemoval {
    /// The surviving dataset, or `None` when no curve remains.
    pub dataset: Option<TlmDataset>,
    /// Status rows dropped by this removal, including residual failure rows
    /// when the removal was terminal.
    pub removed_statuses: usize,
}

impl TlmDataset {
    /// Admit a coherent workbook aggregate and derive its measured gate-voltage set.
    pub fn try_new(
        root: String,
        curves: Vec<TlmCurve>,
        statuses: Vec<FileStatus>,
    ) -> Result<Self, TlmParseError> {
        if root.trim().is_empty() {
            return Err(TlmParseError(
                "TLM dataset root must not be empty".to_string(),
            ));
        }
        if curves.is_empty() {
            return Err(TlmParseError(
                "No valid TLM workbooks were found.".to_string(),
            ));
        }

        let root_path = Path::new(&root);
        let mut curves_by_file = BTreeMap::<PathBuf, &TlmCurve>::new();
        for curve in &curves {
            let relative = Path::new(curve.file_path())
                .strip_prefix(root_path)
                .map_err(|_| {
                    TlmParseError(format!(
                        "TLM curve {} is not under dataset root {root}",
                        curve.file_path()
                    ))
                })?;
            if !is_clean_relative_path(relative) {
                return Err(TlmParseError(format!(
                    "TLM curve {} has an invalid workbook identity",
                    curve.file_path()
                )));
            }
            if curves_by_file
                .insert(relative.to_path_buf(), curve)
                .is_some()
            {
                return Err(TlmParseError(format!(
                    "TLM dataset contains duplicate curve identity {}",
                    relative.display()
                )));
            }
        }

        let mut statuses_by_file = BTreeMap::<PathBuf, &FileStatus>::new();
        for status in &statuses {
            let relative = Path::new(&status.file);
            if !is_clean_relative_path(relative) {
                return Err(TlmParseError(format!(
                    "TLM status has an invalid workbook identity {}",
                    status.file
                )));
            }
            if statuses_by_file
                .insert(relative.to_path_buf(), status)
                .is_some()
            {
                return Err(TlmParseError(format!(
                    "TLM dataset contains duplicate status identity {}",
                    status.file
                )));
            }

            match status.status {
                Status::Ok => {
                    let curve = curves_by_file.get(relative).ok_or_else(|| {
                        TlmParseError(format!(
                            "Successful TLM status {} has no matching curve",
                            status.file
                        ))
                    })?;
                    if status.group != curve.group()
                        || status.length_um != Some(curve.length_um())
                        || status.vd_source != curve.vd_source()
                    {
                        return Err(TlmParseError(format!(
                            "TLM status metadata does not match curve {}",
                            status.file
                        )));
                    }
                }
                Status::Error => {
                    if curves_by_file.contains_key(relative) {
                        return Err(TlmParseError(format!(
                            "Failed TLM status {} must not have a curve",
                            status.file
                        )));
                    }
                }
            }
        }

        for relative in curves_by_file.keys() {
            if !statuses_by_file
                .get(relative)
                .is_some_and(|status| status.status == Status::Ok)
            {
                return Err(TlmParseError(format!(
                    "TLM curve {} has no matching successful status",
                    relative.display()
                )));
            }
        }

        let vg_values = available_vg_values(&curves);
        Ok(Self {
            root,
            curves,
            statuses,
            vg_values,
        })
    }

    /// Dataset root folder as given to the loader.
    pub fn root(&self) -> &str {
        &self.root
    }

    /// Successfully parsed workbooks in discovery order.
    pub fn curves(&self) -> &[TlmCurve] {
        &self.curves
    }

    /// One status row per discovered workbook (and per unreadable folder).
    pub fn statuses(&self) -> &[FileStatus] {
        &self.statuses
    }

    /// Sorted, unique gate voltages measured across all curves.
    pub fn vg_values(&self) -> &[f64] {
        &self.vg_values
    }

    /// Consume this aggregate to remove one exact relative workbook identity.
    /// The final successful-curve removal is terminal: residual failed-workbook
    /// statuses are included in the count and no empty aggregate is returned.
    pub fn remove_workbook(mut self, relative_file: &str) -> TlmDatasetRemoval {
        let removed = self
            .statuses
            .iter()
            .filter(|status| status.file == relative_file)
            .count();
        if removed == 0 {
            return TlmDatasetRemoval {
                dataset: Some(self),
                removed_statuses: 0,
            };
        }

        self.statuses.retain(|status| status.file != relative_file);

        let relative_file = Path::new(relative_file);
        self.curves.retain(|curve| {
            !Path::new(curve.file_path())
                .strip_prefix(Path::new(&self.root))
                .is_ok_and(|relative| relative == relative_file)
        });
        if self.curves.is_empty() {
            return TlmDatasetRemoval {
                dataset: None,
                removed_statuses: removed + self.statuses.len(),
            };
        }
        self.vg_values = available_vg_values(&self.curves);
        TlmDatasetRemoval {
            dataset: Some(self),
            removed_statuses: removed,
        }
    }
}

fn is_clean_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// Round half-to-even to the `VG_MATCH_TOLERANCE_V` resolution (nine decimals).
fn round9(x: f64) -> f64 {
    let scaled = x * VG_ROUND_SCALE;
    if scaled.is_finite() {
        banker_round(scaled) / VG_ROUND_SCALE
    } else {
        // At this magnitude an f64 cannot represent any fractional decimal
        // place, so rounding to nine decimals is already the identity.
        x
    }
}

/// Sorted, unique gate voltages across `curves`, rounded to the match
/// tolerance so instrument jitter below it collapses to one `V_G`.
pub(super) fn available_vg_values(curves: &[TlmCurve]) -> Vec<f64> {
    let mut vals: Vec<f64> = curves
        .iter()
        .flat_map(|curve| curve.samples().iter().map(|sample| round9(sample.vg())))
        .collect();
    vals.sort_by(|a, b| {
        a.partial_cmp(b)
            .expect("validated TLM gate voltages remain finite after rounding")
    });
    vals.dedup();
    vals
}

/// One `length_points.csv` row: the per-length device selection behind a fit.
/// Max-policy fields first, then the median diagnostic.
#[derive(Debug, Clone, PartialEq)]
pub struct LengthPoint {
    /// Process-group name.
    pub group: String,
    /// Channel length in micrometers.
    pub length_um: f64,
    /// Gate voltage requested for the analysis.
    pub selected_vg: f64,
    /// Nearest measured gate voltage actually used.
    pub actual_vg: f64,
    /// Highest channel current among the devices at this length (A).
    pub current_a: f64,
    /// `|V_D| / current_a` (ohm); the value the reported fit uses.
    pub rtotal_ohm: f64,
    /// Median channel current across the devices at this length (A).
    pub current_median_a: f64,
    /// `|V_D| / current_median_a` (ohm); the value the diagnostic fit uses.
    pub rtotal_median_ohm: f64,
    /// Devices with positive finite current at this length.
    pub device_count: usize,
    /// File name of the device that supplied `current_a`.
    pub selected_file: String,
}

/// Shared max-current + median-current TLM fit numbers. [`GroupAnalysis`] and
/// [`VoltageSweepPoint`] both carry this shape.
#[derive(Debug, Clone, PartialEq)]
pub struct TlmFitSummary {
    /// Fitted intercept (ohm), which is `2 R_c`.
    pub intercept_ohm: f64,
    /// `intercept_ohm / 2` (ohm).
    pub rc_per_contact_ohm: f64,
    /// Fitted slope (ohm per micrometer).
    pub slope_ohm_per_um: f64,
    /// Coefficient of determination of the reported fit; NaN when undefined.
    pub r_squared: f64,
    /// Intercept of the median-current diagnostic fit (ohm).
    pub intercept_median_ohm: f64,
    /// `intercept_median_ohm / 2` (ohm).
    pub rc_per_contact_median_ohm: f64,
    /// Slope of the median-current diagnostic fit (ohm per micrometer).
    pub slope_median_ohm_per_um: f64,
    /// Coefficient of determination of the diagnostic fit; NaN when undefined.
    pub r_squared_median: f64,
}

impl TlmFitSummary {
    /// Build from the two linear fits. Contact resistances are `intercept / 2`.
    pub fn from_fits(
        slope: f64,
        intercept: f64,
        r_squared: f64,
        median_slope: f64,
        median_intercept: f64,
        r_squared_median: f64,
    ) -> Self {
        Self {
            intercept_ohm: intercept,
            rc_per_contact_ohm: intercept / 2.0,
            slope_ohm_per_um: slope,
            r_squared,
            intercept_median_ohm: median_intercept,
            rc_per_contact_median_ohm: median_intercept / 2.0,
            slope_median_ohm_per_um: median_slope,
            r_squared_median,
        }
    }

    /// Every field `NaN`: used when fewer than two lengths are available.
    pub fn nan() -> Self {
        Self::from_fits(f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN)
    }
}

/// One group's TLM fit at one `V_G`.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupAnalysis {
    /// Process-group name.
    pub group: String,
    /// Gate voltage the fit was evaluated at.
    pub selected_vg: f64,
    /// Per-length points that fed the fit.
    pub points: Vec<LengthPoint>,
    /// Reported and diagnostic fit numbers.
    pub fit: TlmFitSummary,
    /// Human-readable fit caveats, in the order they were detected.
    pub warnings: Vec<String>,
}

/// Single-`V_G` analysis result over every group in a dataset.
#[derive(Debug, Clone, PartialEq)]
pub struct TlmAnalysisResult {
    /// Dataset root folder.
    pub root: String,
    /// Gate voltage the analysis was evaluated at.
    pub selected_vg: f64,
    /// Sorted, unique gate voltages measured across the dataset.
    pub vg_values: Vec<f64>,
    /// One fit per process group, alphabetical by name.
    pub groups: Vec<GroupAnalysis>,
    /// Per-workbook ingest outcomes.
    pub statuses: Vec<FileStatus>,
}
impl TlmAnalysisResult {
    /// Name of the alphabetically first group, if any group was analyzed.
    pub fn first_group_name(&self) -> Option<&str> {
        self.groups.first().map(|g| g.group.as_str())
    }

    /// True when a group with this exact name was analyzed.
    pub fn has_group(&self, name: &str) -> bool {
        self.groups.iter().any(|g| g.group == name)
    }

    /// The analyzed group with this exact name.
    pub fn group(&self, name: &str) -> Option<&GroupAnalysis> {
        self.groups.iter().find(|g| g.group == name)
    }
}

/// One `sweep.csv` row: the same fit shape as `GroupAnalysis` without points.
#[derive(Debug, Clone, PartialEq)]
pub struct VoltageSweepPoint {
    /// Process-group name.
    pub group: String,
    /// Gate voltage the fit was evaluated at.
    pub selected_vg: f64,
    /// Reported and diagnostic fit numbers.
    pub fit: TlmFitSummary,
    /// Channel lengths with a usable current at this `V_G`.
    pub valid_lengths: usize,
    /// Human-readable fit caveats, in the order they were detected.
    pub warnings: Vec<String>,
}

/// Full-sweep result: one fit per (group, measured `V_G`).
#[derive(Debug, Clone, PartialEq)]
pub struct TlmSweepResult {
    /// Dataset root folder.
    pub root: String,
    /// Sorted, unique gate voltages measured across the dataset.
    pub vg_values: Vec<f64>,
    /// Fits in `V_G`-major, then alphabetical-group order.
    pub points: Vec<VoltageSweepPoint>,
}

/// Coerce a candidate drain bias to a finite, nonzero f64. Zero is rejected
/// because `R_total = |V_D| / I` would collapse to zero for every device.
pub fn valid_vd(value: f64, label: &str) -> Result<f64, TlmParseError> {
    if !value.is_finite() {
        return Err(TlmParseError(format!("{label} must be a finite number")));
    }
    if value == 0.0 {
        return Err(TlmParseError(format!("{label} must be nonzero")));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_scale_is_the_reciprocal_of_the_match_tolerance() {
        assert_eq!(VG_ROUND_SCALE * VG_MATCH_TOLERANCE_V, 1.0);
    }

    #[test]
    fn sample_normalizes_negative_current_magnitudes() {
        let sample = TlmSample::try_new(0.0, -1e-6, -2e-6).expect("finite signed currents load");
        assert_eq!(sample.abs_id(), 1e-6);
        assert_eq!(sample.abs_is(), 2e-6);
        assert!(TlmSample::try_new(0.0, 0.0, -0.0).is_ok());
    }

    #[test]
    fn status_and_vd_source_display_as_their_csv_cells() {
        assert_eq!(Status::Ok.to_string(), "ok");
        assert_eq!(Status::Error.to_string(), "error");
        assert_eq!(VdSource::Setup.to_string(), "setup");
        assert_eq!(VdSource::Fallback.to_string(), "fallback");
        assert_eq!(VdSource::Unread.to_string(), "unread");
    }
}
