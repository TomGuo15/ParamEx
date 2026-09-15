//! The Transfer workspace's transient-state accessors as the panels use them:
//! geometry/Cox edit text, selector fit-window drags, and the plot cache.

use paramex_gui::workspaces::transfer::state::{
    CoxUi, DragEdge, DragState, GeometryUi, GraphMode, LayerRow, PlotCache, PlotKind, SelectorUi,
    SweepBranch, COX_ESTIMATE_PENDING_LABEL,
};

#[test]
fn transfer_geometry_and_cox_state_expose_accessors() {
    let mut geometry = GeometryUi::with_global_inputs("300", "20");
    assert_eq!(geometry.global_w(), "300");
    assert_eq!(geometry.global_l(), "20");
    let (global_w, global_l) = geometry.global_wl_mut();
    *global_w = "1500".to_string();
    *global_l = "50".to_string();
    assert_eq!(geometry.parse_global_wl(), Some((1500.0, 50.0)));

    let mut layer = LayerRow::new("3.9", "300");
    let (eps_text, th_text) = layer.texts_mut();
    *eps_text = "7.5".to_string();
    *th_text = "20".to_string();
    assert_eq!(layer.eps_text(), "7.5");
    assert_eq!(layer.th_text(), "20");

    let mut cox = CoxUi::default();
    assert_eq!(cox.estimate_label(), COX_ESTIMATE_PENDING_LABEL);
    assert_eq!(cox.estimate_value(), None);
    cox.add_layer(layer);
    assert!(cox.can_remove_layer());
    assert_eq!(cox.layers().len(), 2);
    assert!(cox.remove_layer(1));
}

#[test]
fn selector_state_exposes_window_drag_contract() {
    let mut selector = SelectorUi::default();
    assert_eq!(selector.mode(PlotKind::Vt), GraphMode::Auto);

    selector.set_mode(PlotKind::Vt, GraphMode::Fwd);
    assert_eq!(selector.mode(PlotKind::Vt), GraphMode::Fwd);

    selector.start_drag(PlotKind::Vt, DragEdge::Lo, 0.0, 1.0);
    assert_eq!(selector.live_window(PlotKind::Vt), Some((0.0, 1.0)));
    selector.update_drag_for(PlotKind::Vt, |drag| drag.set_window(0.25, 1.25));
    let drag = selector
        .finish_drag_for(PlotKind::Vt)
        .expect("drag finished");
    assert_eq!(drag.edge(), DragEdge::Lo);
    assert_eq!(drag.window(), (0.25, 1.25));

    let other = DragState::new(PlotKind::Ss, DragEdge::Whole, 2.0, 4.0);
    assert_eq!(
        DragState::window_for_kind(Some(other), PlotKind::Ss),
        Some((2.0, 4.0))
    );
}

#[test]
fn plot_cache_exposes_view_contract() {
    let mut cache = PlotCache::default();
    let view = cache.view("file-a", &[0.0, 1.0, 2.0], &[1e-12, 1e-9, 1e-6]);

    assert_eq!(view.axes().vg(), (0.0, 2.0));
    assert_eq!(view.scatter(SweepBranch::Forward, PlotKind::Vt).len(), 3);
    assert_eq!(cache.view_count(), 1);
    assert!(cache.has_view("file-a"));
    cache.prune_to(|_| false);
    assert_eq!(cache.view_count(), 0);
}
