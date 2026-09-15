//! The corner-radius scale (in points, the `CornerRadius::same` unit). Larger
//! surfaces take larger radii so nested chrome reads as concentric.

/// Cards, and the popup/menu/tooltip chrome that floats over them.
pub const CARD: u8 = 8;

/// Buttons, text fields, the banner segment track, and list rows.
pub const WIDGET: u8 = 7;

/// Inner fills nested inside a widget: hover washes on icon buttons, banner
/// segments, focus rings, and equation panels.
pub const INNER: u8 = 5;

/// Compact chrome: header actions, badges, and code blocks.
pub const COMPACT: u8 = 4;

/// Flat card header tabs.
pub const TAB: u8 = 3;

/// Compact chrome: header actions, badges, and code blocks.
pub const CHIP: u8 = COMPACT;
/// Inner fills nested inside a widget: hover washes, banner segments, focus rings.
pub const SEGMENT: u8 = INNER;
