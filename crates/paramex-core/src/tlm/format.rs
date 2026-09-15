//! Shared TLM status and warning text formatting.

/// Largest magnitude `fmt_g` prints through the integer path. Every f64 below
/// it converts to `i64` without overflow; the true exactness ceiling is 2^53,
/// above which whole-number f64s are already spaced more than one apart, but
/// such gate voltages and channel lengths never occur in practice.
const EXACT_I64_LIMIT: f64 = 1e16;

/// Compact number text for the value shapes fed to it: whole numbers and short
/// decimals in the warning and length labels in `tlm::methods`, and the
/// "Loaded with fallback V_D=..." status message in `tlm::service`. Integers
/// print without a decimal point; anything else is Rust shortest-roundtrip
/// `Display`, so a long fraction prints in full and |x| >= 1e6 never switches
/// to scientific notation. These strings are user-visible warning text and the
/// oracle corpus does not exercise those shapes, so changing them is a
/// behavior change that needs oracle coverage first.
pub(super) fn fmt_g(x: f64) -> String {
    if x == x.trunc() && x.abs() < EXACT_I64_LIMIT {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}
