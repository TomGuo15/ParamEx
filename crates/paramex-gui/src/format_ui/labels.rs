//! User-facing row labels, badges, and GUI messages.

/// The point-count label, exactly `"{N} pts"`. No thousands separators. Used by
/// the Transfer file rows and the TLM group rows.
pub fn point_count_label(n: usize) -> String {
    format!("{n} pts")
}

/// The status-badge text: `"error"` when the file has an ingestion/extraction
/// error, else `"ok"`.
pub fn status_badge(has_error: bool) -> &'static str {
    if has_error {
        "error"
    } else {
        "ok"
    }
}

/// The global-apply toast. `"file(s)"` is a fixed literal - no dynamic
/// pluralization.
pub fn global_wl_message(count: usize) -> String {
    format!("Applied W/L to {count} file(s).")
}

pub fn exported_to(name: &str) -> String {
    format!("Exported to {name}")
}

pub fn loaded_files(count: usize) -> String {
    format!("Loaded {count} file(s)")
}

pub fn already_loaded(name: &str) -> String {
    format!("{name} is already loaded.")
}

pub fn removed_items(count: usize, noun: &str) -> String {
    format!("Removed {count} {noun}(s).")
}

pub fn cleared_error_rows() -> &'static str {
    "Cleared error row(s)."
}

pub fn transfer_output_summary(
    attached: usize,
    unmatched: usize,
    ambiguous: usize,
    displaced: usize,
    errors: usize,
    first_err: Option<&str>,
) -> String {
    with_first_error(
        format!(
            "Transfer output: {attached} attached, {unmatched} unmatched, \
             {ambiguous} ambiguous, {displaced} displaced, {errors} error(s)."
        ),
        attached,
        errors,
        first_err,
    )
}

pub fn output_partial_fit_message(fitted: usize, total: usize) -> String {
    let unavailable = total.saturating_sub(fitted);
    let noun = if total == 1 { "line" } else { "lines" };
    format!("{unavailable} of {total} output {noun} unavailable")
}

fn with_first_error(
    base: String,
    success_count: usize,
    errors: usize,
    first_err: Option<&str>,
) -> String {
    match first_err {
        Some(e) if success_count == 0 && errors > 0 => format!("{base} First error: {e}"),
        _ => base,
    }
}

pub const WL_POSITIVE_MESSAGE: &str = "W and L must be positive numbers.";
pub const WL_NUMERIC_MESSAGE: &str = "W and L must be numeric.";
pub const OUTPUT_NO_FINITE_POINTS_MESSAGE: &str = "No finite Id-Vd points";
pub const OUTPUT_SUMMARY_UNAVAILABLE_MESSAGE: &str = "Output fit unavailable";
pub const OUTPUT_FIT_FAILED_MESSAGE: &str = "Output fit failed.";
pub const ATTACHED_PENDING_OUTPUT_MESSAGE: &str = "Attached pending output.";
pub const REMOVED_PENDING_OUTPUT_MESSAGE: &str = "Removed pending output.";
pub const OUTPUT_MOVED_TO_PENDING_MESSAGE: &str = "Output moved to pending.";
pub const REMOVED_OUTPUT_MESSAGE: &str = "Removed output.";
