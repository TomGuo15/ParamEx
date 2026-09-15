use paramex_core::transfer::{split_double_sweep, SweepData, Transform, WindowedFitter};
use paramex_gui::plot_kit::fit_line_endpoints;
use paramex_gui::workspaces::transfer::selector::graph::{committed_line_gate, preview_gate};

fn linear_sweep() -> SweepData {
    // sqrt(id) ~ linear in vg → a clean, non-degenerate slope.
    let vg: Vec<f64> = (0..30).map(|i| i as f64 * 0.1).collect();
    let id_abs: Vec<f64> = vg.iter().map(|v| (0.2 * v + 0.05).powi(2)).collect();
    let (fwd, _b) = split_double_sweep(&vg, &id_abs);
    fwd
}

#[test]
fn gate_requires_five_points_and_finite_nonzero_slope() {
    let s = linear_sweep();
    let f = WindowedFitter::new(&s, Transform::Sqrt);
    assert!(preview_gate(&f.fit(Some((0.0, 2.9)))));
    assert!(!preview_gate(&f.fit(Some((0.0, 0.25)))));
}

#[test]
fn endpoints_span_the_full_axis_for_vt() {
    let s = linear_sweep();
    let f = WindowedFitter::new(&s, Transform::Sqrt);
    let r = f.fit(Some((0.0, 2.9)));
    let pts = fit_line_endpoints(r.slope, r.intercept, 0.0, 2.9).unwrap();
    assert_eq!(pts[0][0], 0.0);
    assert_eq!(pts[1][0], 2.9);
    assert!((pts[0][1] - (r.slope * 0.0 + r.intercept)).abs() < 1e-12);
    assert!((pts[1][1] - (r.slope * 2.9 + r.intercept)).abs() < 1e-12);
}

#[test]
fn non_finite_endpoints_yield_none() {
    assert!(fit_line_endpoints(f64::NAN, 0.0, 0.0, 1.0).is_none());
}

#[test]
fn committed_gate_is_weaker_than_preview_gate() {
    use paramex_core::transfer::WindowedFitResult;
    // A finite 3-point fit: committed line draws (>=2 pts) but preview does not (<5 pts).
    let r = WindowedFitResult {
        slope: 2.0,
        intercept: 0.1,
        r2: 0.99,
        points: 3,
    };
    assert!(committed_line_gate(&r));
    assert!(!preview_gate(&r));
    // A degenerate (zero-slope) fit draws under neither preview... but committed still draws.
    let flat = WindowedFitResult {
        slope: 0.0,
        intercept: 0.1,
        r2: 0.0,
        points: 9,
    };
    assert!(!preview_gate(&flat));
    assert!(committed_line_gate(&flat));
}
