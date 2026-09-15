//! The font-size scale. Every hand-set label size in the GUI comes from here so
//! sibling recipes cannot drift apart by a fraction of a point.

/// Muted metadata and help text, quiet-table headers, plot tick labels, and
/// legend labels.
pub const MUTED: f32 = 11.0;

/// The smallest bold label: badge text and header metadata.
pub const LABEL_XS: f32 = 10.0;

/// Compact bold header actions on the card title rail.
pub const LABEL_SM: f32 = 10.5;

/// Card header tabs, the banner help glyph, and monospace guide code.
pub const LABEL_MD: f32 = 12.0;

/// Buttons, card titles, and banner segments.
pub const LABEL_LG: f32 = 12.5;

/// Technical Guide body text.
pub const BODY: f32 = 13.0;

/// Technical Guide term headings.
pub const TITLE: f32 = 13.5;

/// Card header tabs, the banner workspace labels, and monospace guide code.
pub const HEADER_TAB: f32 = LABEL_MD;
/// Buttons, card titles, and banner segments.
pub const CONTROL: f32 = LABEL_LG;
/// Compact bold header actions on the card title rail.
pub const HEADER_ACTION: f32 = LABEL_SM;
/// Badge text and header metadata.
pub const BADGE: f32 = LABEL_XS;
/// Technical Guide term headings.
pub const GUIDE_TERM: f32 = TITLE;
/// Technical Guide body text.
pub const GUIDE_BODY: f32 = BODY;
