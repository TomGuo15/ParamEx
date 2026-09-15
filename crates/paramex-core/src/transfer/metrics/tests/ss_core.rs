use crate::transfer::metrics::ss::{detect_noise_floor_log10, extract_ss};
use crate::transfer::test_support::{assert_close, f64_vec, load_reference_in, opt_win, parse_f64};
use crate::transfer::{SweepData, Transform, WindowedFitter};

#[test]
fn ss_core_matches_reference() {
    let g = load_reference_in("metrics", "ss_core");
    for case in g["cases"].as_array().unwrap() {
        let label = case["label"].as_str().unwrap();
        let vg = f64_vec(&case["vg"]);
        let id_abs = f64_vec(&case["id_abs"]);
        let fr = opt_win(&case["fit_range"]);
        let mp = case["min_points"].as_u64().unwrap() as usize;
        let fraction = parse_f64(&case["fraction"]);

        let ss = extract_ss(&vg, &id_abs, fr, mp);
        let e = &case["ss"];
        assert_close(ss.ss_mv_dec, parse_f64(&e["ss_mv_dec"]), 1e-9, 1e-12);
        assert_close(ss.slope, parse_f64(&e["slope"]), 1e-9, 1e-12);
        assert_close(ss.intercept, parse_f64(&e["intercept"]), 1e-9, 1e-12);
        assert_close(ss.r2, parse_f64(&e["r2"]), 1e-9, 1e-12);
        assert_eq!(
            ss.points,
            e["points"].as_u64().unwrap() as usize,
            "{label}: points"
        );

        let floor = detect_noise_floor_log10(&id_abs, fraction);
        let exp_floor = &case["floor"];
        if exp_floor.is_null() {
            assert!(floor.is_none(), "{label}: expected None floor");
        } else {
            assert_close(floor.unwrap(), parse_f64(exp_floor), 1e-9, 1e-12);
        }
    }
}

#[test]
fn ss_adapter_exactly_matches_the_shared_windowed_fit_engine() {
    let sweep = SweepData {
        vg: vec![3.0, f64::NAN, 1.0, 2.0, 2.0, 4.0],
        id_abs: vec![1.0e-5, 1.0e-9, 1.0e-10, 1.0e-8, 1.0e-7, f64::INFINITY],
    };
    let range = Some((3.0, 1.0));
    let engine = WindowedFitter::new(&sweep, Transform::Log).fit(range);
    let ss = extract_ss(&sweep.vg, &sweep.id_abs, range, 2);

    assert_eq!(ss.slope.to_bits(), engine.slope.to_bits());
    assert_eq!(ss.intercept.to_bits(), engine.intercept.to_bits());
    assert_eq!(ss.r2.to_bits(), engine.r2.to_bits());
    assert_eq!(ss.points, engine.points);
    assert_eq!(
        ss.ss_mv_dec.to_bits(),
        (1000.0 / engine.slope).abs().to_bits()
    );
}
