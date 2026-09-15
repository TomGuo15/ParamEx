//! Transfer device geometry and extraction settings.

use std::fmt;

/// Where a file's W/L came from. Unrecognized labels parse as [`Self::Other`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometrySource {
    /// Session default (1500 µm × 50 µm) applied at load.
    Default,
    /// Applied by "Apply W/L to All Files".
    Global,
    /// Edited on that file's geometry row.
    Manual,
    /// An unrecognized fixture or imported label.
    Other(String),
}

impl GeometrySource {
    /// Parse a CSV / fixture cell into a typed source.
    pub fn parse(label: &str) -> Self {
        match label {
            "default" => Self::Default,
            "global" => Self::Global,
            "manual" => Self::Manual,
            other => Self::Other(other.to_owned()),
        }
    }

    /// CSV / display label. Live sources are `"default"`, `"global"`, `"manual"`.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::Global => "global",
            Self::Manual => "manual",
            Self::Other(label) => label,
        }
    }
}

impl fmt::Display for GeometrySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Rejected W/L: a dimension was non-finite or not strictly positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeometryError;

impl GeometryError {
    /// User-facing sentence used by the geometry card and session commands.
    pub const MESSAGE: &'static str = "W and L must be positive.";
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(Self::MESSAGE)
    }
}

impl std::error::Error for GeometryError {}

/// Per-file device geometry for mobility extraction.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceGeometry {
    /// Channel width in µm.
    pub width_um: f64,
    /// Channel length in µm.
    pub length_um: f64,
    /// Where the geometry came from.
    pub source: GeometrySource,
}

impl Default for DeviceGeometry {
    /// `width_um = 1500.0`, `length_um = 50.0`, `source = Default`.
    fn default() -> Self {
        DeviceGeometry {
            width_um: 1500.0,
            length_um: 50.0,
            source: GeometrySource::Default,
        }
    }
}

impl DeviceGeometry {
    /// W/L, or `NaN` when the channel length is non-positive.
    pub fn aspect_ratio(&self) -> f64 {
        if self.length_um > 0.0 {
            self.width_um / self.length_um
        } else {
            f64::NAN
        }
    }
}

/// Session-level capacitance settings shared by every loaded file. Stored Cox
/// is in nF/cm²; [`ExtractionSettings::cox_f_per_cm2`] converts to SI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtractionSettings {
    /// Gate-oxide capacitance per area in nF/cm².
    pub cox_nf_per_cm2: f64,
}

impl Default for ExtractionSettings {
    fn default() -> Self {
        ExtractionSettings {
            cox_nf_per_cm2: 10.0,
        }
    }
}

impl ExtractionSettings {
    /// Cox in farad/cm² (SI), converted from the stored nF/cm².
    pub fn cox_f_per_cm2(&self) -> f64 {
        self.cox_nf_per_cm2 * 1e-9
    }
}
