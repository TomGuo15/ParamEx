//! Pins the `Session::generation` display-cache contract: the counter moves
//! exactly when the OUTPUT of `all_results()` may have changed. Over-
//! invalidation (recompute with unchanged inputs) is pinned as deliberate;
//! reads and view-state selection/check writes never bump.

use crate::transfer_curve as curve;
use paramex_core::transfer::{ExpertWindow, Session};

#[test]
fn new_session_starts_at_generation_zero() {
    assert_eq!(Session::new().generation(), 0);
}

#[test]
fn reads_do_not_bump() {
    let mut s = Session::new();
    s.add_curve(curve("a.csv", 1.0)).unwrap();
    let g = s.generation();
    let _ = s.results_table();
    let _ = s.selected_file_metrics_projection();
    let _ = s.has_files();
    assert_eq!(s.generation(), g, "reads must never move the counter");
}

#[test]
fn add_curve_bumps_only_when_inserted() {
    let mut s = Session::new();
    let g0 = s.generation();
    s.add_curve(curve("a.csv", 1.0)).expect("inserted");
    let g1 = s.generation();
    assert!(g1 > g0, "an inserted curve changes all_results()");
    assert!(s.add_curve(curve("a.csv", 1.0)).is_none());
    assert_eq!(s.generation(), g1, "a duplicate mutates nothing — no bump");
}

#[test]
fn remove_bumps_only_when_something_was_removed() {
    let mut s = Session::new();
    let a = s.add_curve(curve("a.csv", 0.5)).unwrap();
    s.add_curve(curve("b.csv", 1.0)).unwrap();
    let g = s.generation();

    assert!(!s.select_file("not-a-file-id"));
    assert!(!s.set_file_checked("not-a-file-id", true));
    assert_eq!(s.generation(), g, "empty intersection touches nothing");

    assert!(s.select_file(&a));
    assert_eq!(s.remove_selected_or_checked(), 1);
    assert!(s.generation() > g, "a real removal changes all_results()");
}

#[test]
fn recomputing_command_bumps_even_with_unchanged_inputs() {
    // Over-invalidation is deliberate: a recomputing command can't cheaply
    // prove whether the refreshed MetricResult differs.
    let mut s = Session::new();
    let id = s.add_curve(curve("a.csv", 1.0)).unwrap();
    let g = s.generation();
    assert!(s.set_expert_window(&id, ExpertWindow::FwdVt, None));
    assert!(s.generation() > g);
}

#[test]
fn set_cox_and_set_global_wl_bump_via_recompute() {
    let mut s = Session::new();
    s.add_curve(curve("a.csv", 1.0)).unwrap();
    s.add_curve(curve("b.csv", 1.5)).unwrap();
    let g0 = s.generation();
    s.set_cox(25.0).expect("positive finite Cox commits");
    let g1 = s.generation();
    assert_eq!(g1, g0 + 1, "set_cox is one bulk cache invalidation");
    assert_eq!(s.set_global_wl(100.0, 10.0), Ok(2));
    assert!(
        s.generation() == g1 + 1,
        "a global W·L apply is one bulk cache invalidation"
    );
}

#[test]
fn set_global_wl_err_does_not_bump() {
    let mut s = Session::new();
    s.add_curve(curve("a.csv", 1.0)).unwrap();
    let g = s.generation();
    assert!(s.set_global_wl(-1.0, 10.0).is_err());
    assert_eq!(s.generation(), g, "rejected geometry mutates nothing");
}

#[test]
fn selection_and_check_methods_do_not_bump() {
    // Neither view-state selection nor check state can change all_results(),
    // so neither invalidates the display cache.
    let mut s = Session::new();
    let a = s.add_curve(curve("a.csv", 0.5)).unwrap();
    let b = s.add_curve(curve("b.csv", 1.0)).unwrap();
    let g = s.generation();
    assert!(s.select_file(&a));
    assert!(s.set_file_checked(&b, true));
    assert!(!s.select_file("missing"));
    assert!(!s.set_file_checked("missing", true));
    assert_eq!(s.generation(), g);
    assert_eq!(s.active_file_id(), Some(a.as_str()));
    assert!(s.file_list_row(&b).expect("file row").is_checked);
}
