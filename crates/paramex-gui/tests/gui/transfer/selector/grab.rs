use paramex_gui::workspaces::transfer::selector::bands::{
    classify_grab, cursor_for_grab, snap_to_nearest_x, Grab,
};

#[test]
fn classify_grab_picks_edge_or_whole_or_none() {
    // window [1.0, 3.0], edge tolerance 0.1 in data units
    assert_eq!(classify_grab(1.02, 1.0, 3.0, 0.1), Some(Grab::EdgeLo));
    assert_eq!(classify_grab(2.95, 1.0, 3.0, 0.1), Some(Grab::EdgeHi));
    assert_eq!(classify_grab(2.0, 1.0, 3.0, 0.1), Some(Grab::Whole));
    assert_eq!(classify_grab(5.0, 1.0, 3.0, 0.1), None);
}

#[test]
fn cursor_telegraphs_resize_on_edges_and_grab_inside() {
    use egui::CursorIcon;
    // Edges resize regardless of drag state.
    assert_eq!(
        cursor_for_grab(Grab::EdgeLo, false),
        CursorIcon::ResizeHorizontal
    );
    assert_eq!(
        cursor_for_grab(Grab::EdgeHi, true),
        CursorIcon::ResizeHorizontal
    );
    // The interior offers a grab at rest and shows the closed hand mid-drag.
    assert_eq!(cursor_for_grab(Grab::Whole, false), CursorIcon::Grab);
    assert_eq!(cursor_for_grab(Grab::Whole, true), CursorIcon::Grabbing);
}

#[test]
fn snap_picks_nearest_sorted_x() {
    let xs = [0.0, 0.5, 1.0, 1.5, 2.0];
    assert_eq!(snap_to_nearest_x(&xs, 1.2), 1.0);
    assert_eq!(snap_to_nearest_x(&xs, 1.3), 1.5);
    assert_eq!(snap_to_nearest_x(&xs, -9.0), 0.0); // clamps to ends
    assert_eq!(snap_to_nearest_x(&xs, 9.0), 2.0);
}
