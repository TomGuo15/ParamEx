use paramex_gui::workspaces::transfer::selector::{backward_display, derive_mode};
use paramex_gui::workspaces::transfer::state::{DragEdge, GraphMode, PlotKind, SelectorUi};

#[test]
fn backward_value_falls_back_to_forward_when_bwd_none() {
    assert_eq!(backward_display(Some((0.0, 1.0)), None), Some((0.0, 1.0)));
    assert_eq!(
        backward_display(Some((0.0, 1.0)), Some((0.2, 0.8))),
        Some((0.2, 0.8))
    );
    assert_eq!(backward_display(None, None), None);
}

#[test]
fn mode_derived_from_pins_on_file_switch() {
    assert_eq!(derive_mode(None, None), GraphMode::Auto);
    assert_eq!(derive_mode(Some((0.0, 1.0)), None), GraphMode::Fwd);
    assert_eq!(derive_mode(None, Some((0.0, 1.0))), GraphMode::Bwd);
    assert_eq!(
        derive_mode(Some((0.0, 1.0)), Some((0.0, 1.0))),
        GraphMode::Fwd
    ); // fwd precedence
}

#[test]
fn plot_kind_field_routing() {
    use paramex_core::transfer::ExpertWindow;
    assert_eq!(
        PlotKind::Vt.window_which(GraphMode::Fwd).unwrap(),
        ExpertWindow::FwdVt
    );
    assert_eq!(
        PlotKind::Ss.window_which(GraphMode::Bwd).unwrap(),
        ExpertWindow::BwdSs
    );
    assert!(PlotKind::Vt.window_which(GraphMode::Auto).is_none()); // Auto edits nothing
}

#[test]
fn selector_state_syncs_file_and_owns_drag_lifecycle() {
    let mut sel = SelectorUi::default();

    assert!(sel.sync_file("a", GraphMode::Fwd, GraphMode::Auto));
    assert_eq!(sel.mode(PlotKind::Vt), GraphMode::Fwd);
    assert_eq!(sel.mode(PlotKind::Ss), GraphMode::Auto);
    assert!(!sel.sync_file("a", GraphMode::Auto, GraphMode::Bwd));
    assert_eq!(sel.mode(PlotKind::Vt), GraphMode::Fwd);
    assert_eq!(sel.mode(PlotKind::Ss), GraphMode::Auto);

    sel.start_drag(PlotKind::Vt, DragEdge::Lo, -1.0, 1.0);
    assert_eq!(sel.live_window(PlotKind::Vt), Some((-1.0, 1.0)));
    assert_eq!(sel.live_window(PlotKind::Ss), None);
    sel.update_drag_for(PlotKind::Ss, |drag| drag.set_window(0.0, 0.5));
    assert_eq!(sel.live_window(PlotKind::Vt), Some((-1.0, 1.0)));
    assert!(sel.finish_drag_for(PlotKind::Ss).is_none());
    assert_eq!(sel.live_window(PlotKind::Vt), Some((-1.0, 1.0)));

    let drag = sel
        .finish_drag_for(PlotKind::Vt)
        .expect("VT drag should finish on the VT graph");
    assert_eq!(drag.edge(), DragEdge::Lo);
    assert_eq!(drag.window(), (-1.0, 1.0));
    assert_eq!(sel.live_window(PlotKind::Vt), None);
}
