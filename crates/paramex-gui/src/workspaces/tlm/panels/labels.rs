//! Canonical TLM metric-tile labels.
//!
//! Table headers stay in `columns.rs`; selected-group tile labels live here so
//! the metrics card does not depend on table-column schema to name quantities.

/// The reported max-current fit uses the same qualifier scheme as the median
/// diagnostics: if one block carries a qualifier, both do.
pub const TILE_RCONTACT: &str = "intercept (2R<sub>c</sub>) (max)";
pub const TILE_RC_PER_CONTACT: &str = "R<sub>c</sub>/contact (max)";
pub const TILE_SLOPE: &str = "slope (max)";
pub const TILE_R2: &str = "R<sup>2</sup> (max)";
pub const TILE_RCONTACT_MED: &str = "intercept (2R<sub>c</sub>) (median)";
pub const TILE_RC_PER_CONTACT_MED: &str = "R<sub>c</sub>/contact (median)";
pub const TILE_SLOPE_MED: &str = "slope (median)";
pub const TILE_R2_MED: &str = "R<sup>2</sup> (median)";

pub const TILE_LABELS: [&str; 8] = [
    TILE_RCONTACT,
    TILE_RC_PER_CONTACT,
    TILE_SLOPE,
    TILE_R2,
    TILE_RCONTACT_MED,
    TILE_RC_PER_CONTACT_MED,
    TILE_SLOPE_MED,
    TILE_R2_MED,
];
