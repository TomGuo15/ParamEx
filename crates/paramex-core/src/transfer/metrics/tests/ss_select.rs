use crate::transfer::metrics::ss::{
    auto_select_subthreshold_window, select_local_subthreshold_window, SsWindowSelector,
};
use crate::transfer::test_support::{assert_win, f64_vec, load_reference_in, parse_f64};

#[test]
fn ss_selectors_match_reference() {
    let g = load_reference_in("metrics", "ss_select");
    for case in g["cases"].as_array().unwrap() {
        let label = case["label"].as_str().unwrap();
        let vg = f64_vec(&case["vg"]);
        let id_abs = f64_vec(&case["id_abs"]);
        let got = match case["kind"].as_str().unwrap() {
            "local" => select_local_subthreshold_window(
                &vg,
                &id_abs,
                parse_f64(&case["min_decades"]),
                case["points"].as_u64().unwrap() as usize,
                parse_f64(&case["min_r2"]),
            ),
            "auto" => auto_select_subthreshold_window(
                &vg,
                &id_abs,
                &SsWindowSelector {
                    max_points: case["max_points"].as_u64().unwrap() as usize,
                    min_decades: parse_f64(&case["min_decades"]),
                    min_points: case["min_points"].as_u64().unwrap() as usize,
                    min_r2: parse_f64(&case["min_r2"]),
                    off_guard_decades: parse_f64(&case["off_guard"]),
                },
            ),
            other => panic!("unknown kind {other}"),
        };
        assert_win(got, &case["window"], label);
    }
}
