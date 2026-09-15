//! TLM page layout policy layered on the shared shell/card-stack geometry.

/// DATA card floor (left top). Just tall enough for the summary/error,
/// fallback field, and paired actions without stealing list space.
pub const TLM_DATA_CARD_HEIGHT: f32 = 164.0;
/// Floor for the GROUPS list (left bottom; scrolls internally).
pub const TLM_GROUPS_MIN_HEIGHT: f32 = 140.0;
/// ANALYSIS card height (left middle): the V_G strip row and direct-entry row.
pub const TLM_ANALYSIS_HEIGHT: f32 = 128.0;
