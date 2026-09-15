use crate::transfer::metrics::sweep::{has_backward_sweep, split_double_sweep, MIN_SWEEP_POINTS};
use crate::transfer::test_support::{f64_vec, load_reference_in};

fn assert_exact(actual: &[f64], expected: &[f64], what: &str) {
    assert_eq!(actual.len(), expected.len(), "{what}: length");
    for (i, (&a, &e)) in actual.iter().zip(expected.iter()).enumerate() {
        if e.is_nan() {
            assert!(a.is_nan(), "{what}[{i}]: expected NaN got {a}");
        } else {
            assert_eq!(a, e, "{what}[{i}]: exact mismatch");
        }
    }
}

#[test]
fn min_sweep_points_is_12() {
    assert_eq!(MIN_SWEEP_POINTS, 12);
}

#[test]
fn split_double_sweep_matches_reference() {
    let g = load_reference_in("metrics", "sweep");
    for case in g["cases"].as_array().unwrap() {
        let vg = f64_vec(&case["vg"]);
        let id_abs = f64_vec(&case["id_abs"]);
        let (f, b) = split_double_sweep(&vg, &id_abs);
        assert_exact(&f.vg, &f64_vec(&case["forward_vg"]), "forward_vg");
        assert_exact(&f.id_abs, &f64_vec(&case["forward_id"]), "forward_id");
        assert_exact(&b.vg, &f64_vec(&case["backward_vg"]), "backward_vg");
        assert_exact(&b.id_abs, &f64_vec(&case["backward_id"]), "backward_id");
        assert_eq!(
            has_backward_sweep(&f, &b),
            case["has_backward"].as_bool().unwrap(),
            "has_backward for {}",
            case["label"]
        );
    }
}
