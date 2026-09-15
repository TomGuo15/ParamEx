//! GUI-only display-formatting facade.
//!
//! The implementation is split by interface: reusable row labels/messages and
//! numeric engineering notation live behind this module.

mod labels;
mod numeric;

pub use labels::{
    already_loaded, cleared_error_rows, exported_to, global_wl_message, loaded_files,
    output_partial_fit_message, point_count_label, removed_items, status_badge,
    transfer_output_summary, ATTACHED_PENDING_OUTPUT_MESSAGE, OUTPUT_FIT_FAILED_MESSAGE,
    OUTPUT_MOVED_TO_PENDING_MESSAGE, OUTPUT_NO_FINITE_POINTS_MESSAGE,
    OUTPUT_SUMMARY_UNAVAILABLE_MESSAGE, REMOVED_OUTPUT_MESSAGE, REMOVED_PENDING_OUTPUT_MESSAGE,
    WL_NUMERIC_MESSAGE, WL_POSITIVE_MESSAGE,
};
pub use numeric::{
    eng_tick, fmt_compact_current, fmt_current, fmt_eng, fmt_fixed2, fmt_num3, fmt_ohm, fmt_r2,
    fmt_ratio, fmt_slope, fmt_vg, parse_eng, DASH,
};
