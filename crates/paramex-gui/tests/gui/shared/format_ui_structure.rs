use paramex_gui::format_ui::{
    already_loaded, cleared_error_rows, global_wl_message, point_count_label, removed_items,
    status_badge, transfer_output_summary, ATTACHED_PENDING_OUTPUT_MESSAGE, DASH,
    OUTPUT_FIT_FAILED_MESSAGE, OUTPUT_MOVED_TO_PENDING_MESSAGE, REMOVED_OUTPUT_MESSAGE,
    REMOVED_PENDING_OUTPUT_MESSAGE, WL_NUMERIC_MESSAGE,
};

#[test]
fn format_ui_facade_exports_label_and_numeric_contracts() {
    assert_eq!(point_count_label(1), "1 pts");
    assert_eq!(point_count_label(2), "2 pts");
    assert_eq!(status_badge(false), "ok");
    assert_eq!(status_badge(true), "error");
    assert_eq!(global_wl_message(2), "Applied W/L to 2 file(s).");
    assert_eq!(already_loaded("a.csv"), "a.csv is already loaded.");
    assert_eq!(removed_items(2, "file"), "Removed 2 file(s).");
    assert_eq!(cleared_error_rows(), "Cleared error row(s).");
    assert_eq!(
        transfer_output_summary(1, 2, 3, 4, 5, None),
        "Transfer output: 1 attached, 2 unmatched, 3 ambiguous, 4 displaced, 5 error(s)."
    );
    assert_eq!(WL_NUMERIC_MESSAGE, "W and L must be numeric.");
    assert_eq!(OUTPUT_FIT_FAILED_MESSAGE, "Output fit failed.");
    assert_eq!(ATTACHED_PENDING_OUTPUT_MESSAGE, "Attached pending output.");
    assert_eq!(REMOVED_PENDING_OUTPUT_MESSAGE, "Removed pending output.");
    assert_eq!(OUTPUT_MOVED_TO_PENDING_MESSAGE, "Output moved to pending.");
    assert_eq!(REMOVED_OUTPUT_MESSAGE, "Removed output.");
    assert_eq!(DASH, "\u{2014}");
}
