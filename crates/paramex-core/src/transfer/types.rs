//! Public data types for the transfer-extraction pipeline: sweep/fit/metric
//! DTOs plus geometry/settings and Cox helper facades.

use std::path::PathBuf;

mod cox;
mod geometry;

pub(in crate::transfer) use cox::validate_cox_nf_per_cm2;
pub use cox::{calculate_stack_cox_nf_per_cm2, CoxError};
pub use geometry::{DeviceGeometry, ExtractionSettings, GeometryError, GeometrySource};

/// A single transfer sweep: gate voltage `vg` paired index-for-index with the
/// absolute drain current `id_abs`.
#[derive(Debug, Clone, PartialEq)]
pub struct SweepData {
    /// Gate voltage in V.
    pub vg: Vec<f64>,
    /// Absolute drain current `|I_D|` in A.
    pub id_abs: Vec<f64>,
}

/// Plain windowed linear-regression result; no metric-specific fields. Returned
/// by [`crate::transfer::WindowedFitter`]. Higher-level metric functions wrap this
/// after applying their own R² / min-points gating.
///
/// `slope`, `intercept`, and `r2` are `NaN` when the window has fewer than two
/// samples or the regression is degenerate. `points` is the sample count in the
/// window (which may be 0 or 1 in the NaN cases).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowedFitResult {
    /// Regression slope in transformed-current units per V.
    pub slope: f64,
    /// Regression intercept in transformed-current units.
    pub intercept: f64,
    /// Coefficient of determination, clamped to at most `1.0`.
    pub r2: f64,
    /// Number of samples inside the fit window.
    pub points: usize,
}

/// A normalized transfer curve loaded from a user file.
///
/// `vg` and `id_abs` are index-for-index paired, already masked to finite,
/// strictly-positive `|Id|` samples by the parser. `source_path` is `None` for
/// in-memory (uploaded-bytes) parses. Fields remain public as a transport value;
/// [`crate::transfer::Session`] revalidates the parser contract before
/// persistent admission.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCurve {
    /// Display name, normally the source file name.
    pub name: String,
    /// Gate voltage in V.
    pub vg: Vec<f64>,
    /// Absolute drain current `|I_D|` in A.
    pub id_abs: Vec<f64>,
    /// Source file path; `None` for in-memory parses.
    pub source_path: Option<PathBuf>,
}

/// Physics constants required for V_TH / mobility extraction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::transfer) struct ExtractionContext {
    pub(in crate::transfer) cox_f_per_cm2: f64,
    pub(in crate::transfer) aspect_ratio: f64,
}

/// Per-sweep extraction output. Float fields are `NaN` when their metric could
/// not be extracted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::transfer) struct SweepExtractionResult {
    pub(in crate::transfer) vt: f64,
    pub(in crate::transfer) mobility: f64,
    pub(in crate::transfer) ss_mv_dec: f64,
    pub(in crate::transfer) ion: f64,
    pub(in crate::transfer) ioff: f64,
    pub(in crate::transfer) on_off_ratio: f64,
}

/// Optional user-pinned extraction windows. Each is `None` when the user has
/// not pinned that window (auto-select runs).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ExpertRanges {
    /// Primary/forward V_TH window in V.
    pub vt_range: Option<(f64, f64)>,
    /// Primary/forward SS window in V.
    pub ss_range: Option<(f64, f64)>,
    /// Backward V_TH window in V.
    pub vt_range_bwd: Option<(f64, f64)>,
    /// Backward SS window in V.
    pub ss_range_bwd: Option<(f64, f64)>,
}

/// One display/export row for a parsed transfer curve. Float fields are `NaN`
/// when their metric could not be extracted; window fields are `None` when not
/// selected.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricResult {
    /// Display name of the source curve.
    pub filename: String,
    /// Channel width in µm.
    pub width_um: f64,
    /// Channel length in µm.
    pub length_um: f64,
    /// W/L, or `NaN` for a non-positive length.
    pub aspect_ratio: f64,
    /// Where the geometry came from.
    pub geometry_source: GeometrySource,
    /// Primary-sweep threshold voltage in V.
    pub vt: f64,
    /// Primary-sweep saturation mobility in cm² V⁻¹ s⁻¹.
    pub mu_sat: f64,
    /// Primary-sweep subthreshold swing in mV/dec.
    pub ss_mv_dec: f64,
    /// Maximum `|I_D|` over the whole curve in A.
    pub ion: f64,
    /// Minimum positive `|I_D|` over the whole curve in A.
    pub ioff: f64,
    /// `ion / ioff` (unitless).
    pub on_off_ratio: f64,
    /// Forward-to-backward V_G shift at equal current in V.
    pub delta_vth_hysteresis: f64,
    /// Primary-sweep V_TH fit window in V.
    pub vt_window: Option<(f64, f64)>,
    /// Primary-sweep SS fit window in V.
    pub ss_window: Option<(f64, f64)>,
    /// Backward-sweep V_TH fit window in V.
    pub vt_window_bwd: Option<(f64, f64)>,
    /// Backward-sweep SS fit window in V.
    pub ss_window_bwd: Option<(f64, f64)>,
    /// Both primary fits finite, or at least one missing.
    pub status: ExtractionStatus,
    /// Human-readable note; empty when [`ExtractionStatus::Ok`].
    pub message: String,
    /// Whether the curve contained a usable backward branch.
    pub has_backward_sweep: bool,
    /// Forward-sweep threshold voltage in V.
    pub vt_forward: f64,
    /// Forward-sweep saturation mobility in cm² V⁻¹ s⁻¹.
    pub mu_sat_forward: f64,
    /// Forward-sweep subthreshold swing in mV/dec.
    pub ss_mv_dec_forward: f64,
    /// Forward-sweep maximum `|I_D|` in A.
    pub ion_forward: f64,
    /// Forward-sweep minimum positive `|I_D|` in A.
    pub ioff_forward: f64,
    /// Forward-sweep `ion / ioff` (unitless).
    pub on_off_ratio_forward: f64,
    /// Backward-sweep threshold voltage in V.
    pub vt_backward: f64,
    /// Backward-sweep saturation mobility in cm² V⁻¹ s⁻¹.
    pub mu_sat_backward: f64,
    /// Backward-sweep subthreshold swing in mV/dec.
    pub ss_mv_dec_backward: f64,
    /// Backward-sweep maximum `|I_D|` in A.
    pub ion_backward: f64,
    /// Backward-sweep minimum positive `|I_D|` in A.
    pub ioff_backward: f64,
    /// Backward-sweep `ion / ioff` (unitless).
    pub on_off_ratio_backward: f64,
}

/// Outcome of the primary V_TH + SS extraction. Strings appear only at the
/// CSV / table boundary via [`ExtractionStatus::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionStatus {
    /// Both primary V_TH and SS fits are finite.
    Ok,
    /// At least one primary fit is missing.
    Partial,
}

impl ExtractionStatus {
    /// CSV / table cell: `"ok"` or `"partial"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Partial => "partial",
        }
    }

    /// Parse a reference / CSV cell.
    pub fn parse(label: &str) -> Self {
        match label {
            "ok" => Self::Ok,
            _ => Self::Partial,
        }
    }
}

impl std::fmt::Display for ExtractionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
