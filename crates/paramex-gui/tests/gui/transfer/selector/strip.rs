use paramex_gui::theme::UTILITY_GRAY;
use paramex_gui::ui_kit::{CONTROL_THUMB_RADIUS, CONTROL_THUMB_RING_WIDTH};
use paramex_gui::workspaces::transfer::selector::strip::{
    clamp_pair, frac_to_value, slide_window, thumb_centers, value_to_frac, THUMB_MIN_CENTER_GAP,
};

#[test]
fn rail_uses_approved_utility_gray() {
    // The strip paints its rail via ui_kit::paint_control_rail (CONTROL_RAIL_COLOR).
    assert_eq!(paramex_gui::ui_kit::CONTROL_RAIL_COLOR, UTILITY_GRAY);
}

#[test]
fn tight_range_thumb_rings_overlap_without_merging_hit_targets() {
    let (lo, hi) = thumb_centers(100.0, 104.0, 80.0, 200.0, 15.0);
    assert_eq!((lo.y, hi.y), (15.0, 15.0));
    assert!(
        lo.distance(hi) + 0.001 >= THUMB_MIN_CENTER_GAP,
        "tight range thumb hit targets overlap: lo={lo:?}, hi={hi:?}"
    );
    assert_eq!(THUMB_MIN_CENTER_GAP, CONTROL_THUMB_RADIUS * 2.0 + 1.0);
    assert!(
        lo.distance(hi) > CONTROL_THUMB_RADIUS * 2.0,
        "tight range thumb hit targets share a boundary: lo={lo:?}, hi={hi:?}"
    );
    assert!(
        lo.distance(hi) < CONTROL_THUMB_RADIUS * 2.0 + CONTROL_THUMB_RING_WIDTH,
        "tight range thumb rings leave a visible gap: lo={lo:?}, hi={hi:?}"
    );
    for (true_lo, true_hi) in [(80.0, 84.0), (196.0, 200.0)] {
        let (lo, hi) = thumb_centers(true_lo, true_hi, 80.0, 200.0, 15.0);
        assert!(
            lo.x >= 80.0 && hi.x <= 200.0,
            "thumbs leave rail: {lo:?} {hi:?}"
        );
    }
}

#[test]
fn round_trips_within_axis() {
    let (v_min, v_max) = (-2.0, 3.0);
    for &v in &[-2.0, -1.0, 0.0, 1.25, 3.0] {
        let t = value_to_frac(v, v_min, v_max);
        assert!((0.0..=1.0).contains(&t));
        assert!((frac_to_value(t, v_min, v_max) - v).abs() < 1e-9);
    }
}

#[test]
fn fraction_clamps_outside_axis() {
    assert_eq!(value_to_frac(-5.0, -2.0, 3.0), 0.0);
    assert_eq!(value_to_frac(9.0, -2.0, 3.0), 1.0);
}

#[test]
fn degenerate_axis_is_guarded() {
    // v_min == v_max must not divide by zero.
    assert_eq!(value_to_frac(1.0, 1.0, 1.0), 0.0);
    assert_eq!(frac_to_value(0.5, 1.0, 1.0), 1.0);
}

#[test]
fn clamp_pair_keeps_lo_le_hi() {
    assert_eq!(clamp_pair(2.0, 1.0), (1.0, 1.0)); // lo pushed down to hi
    assert_eq!(clamp_pair(0.5, 4.0), (0.5, 4.0));
}

#[test]
fn slide_window_moves_range_without_changing_width() {
    let moved = slide_window((-10.0, 0.0), (-6.0, -2.0), 1.5);
    assert_eq!(moved, (-4.5, -0.5));
}

#[test]
fn slide_window_clamps_to_axis_edges() {
    assert_eq!(
        slide_window((-10.0, 0.0), (-6.0, -2.0), -10.0),
        (-10.0, -6.0)
    );
    assert_eq!(slide_window((-10.0, 0.0), (-6.0, -2.0), 10.0), (-4.0, 0.0));
}
