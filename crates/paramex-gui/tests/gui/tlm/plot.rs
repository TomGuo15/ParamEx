use paramex_core::tlm::{GroupAnalysis, LengthPoint, TlmFitSummary};
use paramex_gui::plot_kit::fit_line_endpoints;
use paramex_gui::workspaces::tlm::panels::plot::{plot_bounds, scatter_points};

fn point(length_um: f64, rtotal: f64, rtotal_median: f64) -> LengthPoint {
    LengthPoint {
        group: "g".into(),
        length_um,
        selected_vg: 0.0,
        actual_vg: 0.0,
        current_a: 1.0,
        rtotal_ohm: rtotal,
        current_median_a: 1.0,
        rtotal_median_ohm: rtotal_median,
        device_count: 1,
        selected_file: "f".into(),
    }
}

fn group() -> GroupAnalysis {
    GroupAnalysis {
        group: "g".into(),
        selected_vg: 0.0,
        points: vec![point(5.0, 110.0, 115.0), point(25.0, 150.0, 160.0)],
        fit: TlmFitSummary {
            intercept_ohm: 100.0,
            rc_per_contact_ohm: 50.0,
            slope_ohm_per_um: 2.0,
            r_squared: 0.99,
            intercept_median_ohm: 100.0,
            rc_per_contact_median_ohm: 50.0,
            slope_median_ohm_per_um: 2.2,
            r_squared_median: 0.98,
        },
        warnings: vec![],
    }
}

#[test]
fn scatter_points_map_length_to_rtotal() {
    let g = group();
    assert_eq!(scatter_points(&g), vec![[5.0, 110.0], [25.0, 150.0]]);
}

#[test]
fn fit_endpoints_compute_a_line() {
    // y = 2x + 100 over [0, 25] => [[0,100],[25,150]]
    assert_eq!(
        fit_line_endpoints(2.0, 100.0, 0.0, 25.0),
        Some([[0.0, 100.0], [25.0, 150.0]])
    );
}

#[test]
fn fit_endpoints_none_for_nan_slope() {
    assert_eq!(fit_line_endpoints(f64::NAN, 100.0, 0.0, 25.0), None);
}

#[test]
fn bounds_pin_x_to_zero_and_pad_y() {
    let scatter = vec![[5.0, 100.0], [50.0, 900.0]];
    let med = vec![[5.0, 120.0]];
    let lines = [Some([[0.0, 40.0], [52.5, 950.0]]), None];
    let (x, y) = plot_bounds(&scatter, &med, &lines);
    assert_eq!(x[0], 0.0);
    assert!(x[1] > 50.0);
    assert!(y[0] <= 0.0); // y starts at 0 (intercept readable) unless data dips below
    assert!(y[1] >= 950.0);
}

#[test]
fn bounds_survive_empty_data() {
    let (x, y) = plot_bounds(&[], &[], &[None, None]);
    assert!(x[1] > x[0]);
    assert!(y[1] > y[0]);
}

#[test]
fn bounds_pad_factors_and_negative_minimum() {
    // negative y_min: padding moves DOWN (more negative)
    let scatter = vec![[10.0, -100.0], [20.0, 1000.0]];
    let (x, y) = plot_bounds(&scatter, &[], &[None, None]);
    assert!(y[0] <= -105.0); // -100 * 1.05
    assert!(y[1] >= 1080.0); // 1000 * 1.08
    assert!((x[1] - 21.0).abs() < 1e-9); // 20 * 1.05
}
