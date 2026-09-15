use crate::transfer::metrics::vth::{
    auto_select_vt_window, extract_vt_mu, select_elr_vt_window, FitGate, VtWindowSelector,
    DEFAULT_VT_R2_LADDER,
};
use crate::transfer::test_support::{
    assert_close, assert_win, f64_vec, load_reference_in, parse_f64,
};
use crate::transfer::types::ExtractionContext;

#[test]
fn vth_selector_finds_window_for_short_measured_p_type_curve() {
    let vg = vec![
        5.0, 4.5, 4.0, 3.5, 3.0, 2.5, 2.0, 1.5, 1.0, 0.5, 0.0, -0.5, -1.0, -1.5, -2.0, -2.5, -3.0,
        -3.5, -4.0, -4.5, -5.0, -5.5, -6.0, -6.5, -7.0, -7.5, -8.0, -8.5, -9.0, -9.5, -10.0,
    ];
    let id_abs = vec![
        1.0353E-11,
        6.897E-12,
        5.841_000_000_000_001E-12,
        5.059E-12,
        4.494E-12,
        6.361_000_000_000_001E-12,
        1.2356000000000001E-11,
        2.7160000000000003E-11,
        6.094_5E-11,
        1.390_42E-10,
        3.515_95E-10,
        1.14193E-9,
        5.118_22E-9,
        2.4752199999999996E-8,
        9.554_259_999_999_999E-8,
        2.513_24E-7,
        5.024_419_999_999_999E-7,
        8.368_409_999_999_999E-7,
        1.2520500000000002E-6,
        1.730_81E-6,
        2.2504100000000004E-6,
        2.8117100000000004E-6,
        3.4048200000000005E-6,
        4.028_490_000_000_001E-6,
        4.673_03E-6,
        5.340_220_000_000_001E-6,
        6.0247300000000005E-6,
        6.7230100000000005E-6,
        7.434_520_000_000_001E-6,
        8.156_55E-6,
        8.892_780_000_000_001E-6,
    ];

    let selector = VtWindowSelector {
        window_size: 30,
        step: 1,
        min_points: 10,
        min_r2: 0.99,
        r2_ladder: &DEFAULT_VT_R2_LADDER,
    };
    let window = select_elr_vt_window(&vg, &id_abs, &selector);

    assert!(
        window.is_some(),
        "31-point measured curves should not require a nearly full-curve ELR window"
    );
}

#[test]
fn vth_selectors_match_reference() {
    let g = load_reference_in("metrics", "vth_select");
    for case in g["cases"].as_array().unwrap() {
        let label = case["label"].as_str().unwrap();
        let vg = f64_vec(&case["vg"]);
        let id_abs = f64_vec(&case["id_abs"]);
        let selector = VtWindowSelector {
            window_size: case["window_size"].as_u64().unwrap() as usize,
            step: case["step"].as_u64().unwrap() as usize,
            min_points: case["min_points"].as_u64().unwrap() as usize,
            min_r2: parse_f64(&case["min_r2"]),
            r2_ladder: &DEFAULT_VT_R2_LADDER,
        };

        let auto = auto_select_vt_window(
            &vg,
            &id_abs,
            selector.window_size,
            selector.step,
            selector.min_points,
            selector.min_r2,
        );
        assert_win(auto, &case["auto"], label);

        let ladder = select_elr_vt_window(&vg, &id_abs, &selector);
        assert_win(ladder, &case["ladder"], label);

        let ctx = ExtractionContext {
            cox_f_per_cm2: parse_f64(&case["cox"]),
            aspect_ratio: parse_f64(&case["aspect"]),
        };
        let gate = FitGate {
            min_points: selector.min_points,
            min_r2: selector.min_r2,
        };
        let mu = extract_vt_mu(&vg, &id_abs, ctx, ladder, gate);
        let exp = &case["vt_mu_auto"];
        assert_close(mu.vt, parse_f64(&exp["vt"]), 1e-9, 1e-12);
        assert_close(mu.mobility, parse_f64(&exp["mobility"]), 1e-9, 1e-12);
        assert_close(mu.slope, parse_f64(&exp["slope"]), 1e-9, 1e-12);
        assert_close(mu.r2, parse_f64(&exp["r2"]), 1e-9, 1e-12);
        assert_eq!(
            mu.points,
            exp["points"].as_u64().unwrap() as usize,
            "{label}: points"
        );
    }
}
