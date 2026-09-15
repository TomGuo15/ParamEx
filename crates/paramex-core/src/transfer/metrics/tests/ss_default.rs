use crate::transfer::metrics::ss::{extract_ss, select_ss_window, SsWindowSelector};
use crate::transfer::test_support::{assert_close, f64_vec, load_reference_in, parse_f64};

#[test]
fn ss_default_rule_matches_reference() {
    let g = load_reference_in("metrics", "ss_default");
    for case in g["cases"].as_array().unwrap() {
        let label = case["label"].as_str().unwrap();
        let vg = f64_vec(&case["vg"]);
        let id_abs = f64_vec(&case["id_abs"]);
        let selector = SsWindowSelector {
            max_points: case["max_points"].as_u64().unwrap() as usize,
            min_decades: parse_f64(&case["min_decades"]),
            min_points: case["min_points"].as_u64().unwrap() as usize,
            min_r2: parse_f64(&case["min_r2"]),
            off_guard_decades: parse_f64(&case["off_guard"]),
        };

        let win = select_ss_window(&vg, &id_abs, &selector);
        let exp = &case["window"];
        if exp.is_null() {
            assert!(win.is_none(), "{label}: expected None window, got {win:?}");
        } else {
            let (lo, hi) = win.unwrap_or_else(|| panic!("{label}: expected Some window"));
            let a = exp.as_array().unwrap();
            assert_eq!(lo, parse_f64(&a[0]), "{label}: lo exact");
            assert_eq!(hi, parse_f64(&a[1]), "{label}: hi exact");
        }

        let ss = extract_ss(&vg, &id_abs, win, selector.min_points);
        let e = &case["ss"];
        assert_close(ss.ss_mv_dec, parse_f64(&e["ss_mv_dec"]), 1e-9, 1e-12);
        assert_close(ss.slope, parse_f64(&e["slope"]), 1e-9, 1e-12);
        assert_close(ss.r2, parse_f64(&e["r2"]), 1e-9, 1e-12);
        assert_eq!(
            ss.points,
            e["points"].as_u64().unwrap() as usize,
            "{label}: points"
        );
    }
}
