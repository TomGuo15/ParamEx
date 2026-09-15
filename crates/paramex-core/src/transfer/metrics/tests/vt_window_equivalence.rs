use crate::transfer::metrics::vth::{select_elr_vt_window, VtWindowSelector, DEFAULT_VT_R2_LADDER};
use crate::transfer::test_support::{f64_vec, load_reference_in, parse_f64};

#[test]
fn select_elr_vt_window_matches_reference_corpus() {
    let g = load_reference_in("metrics", "vt_window_equivalence");
    let cases = g["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 80, "expected 80 corpus seeds");

    // Production defaults.
    let selector = VtWindowSelector {
        window_size: 30,
        step: 1,
        min_points: 10,
        min_r2: 0.99,
        r2_ladder: &DEFAULT_VT_R2_LADDER,
    };
    let mut non_none = 0usize;
    for case in cases {
        let seed = case["seed"].as_u64().unwrap();
        let vg = f64_vec(&case["vg"]);
        let id_abs = f64_vec(&case["id_abs"]);
        let got = select_elr_vt_window(&vg, &id_abs, &selector);
        let exp = &case["window"];
        if exp.is_null() {
            assert!(got.is_none(), "seed {seed}: expected None, got {got:?}");
        } else {
            let (lo, hi) = got.unwrap_or_else(|| panic!("seed {seed}: expected Some, got None"));
            let a = exp.as_array().unwrap();
            assert_eq!(lo, parse_f64(&a[0]), "seed {seed}: lo exact");
            assert_eq!(hi, parse_f64(&a[1]), "seed {seed}: hi exact");
            non_none += 1;
        }
    }
    // Non-degeneracy guard: every seed must select a real window, so the
    // equivalence check is not vacuous.
    assert_eq!(
        non_none, 80,
        "expected all 80 seeds to select a window, got {non_none}"
    );
}
