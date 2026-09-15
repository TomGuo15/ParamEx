//! Pure-logic windowed-fitter invariants that need no numeric reference file
//! (comparison cases live in the construct/range/indices references).
use paramex_core::transfer::{SweepData, Transform, WindowedFitter};

fn sqrt_fitter() -> (WindowedFitter, Vec<f64>) {
    // Strictly increasing Vg, strictly positive Id -> nothing masked or
    // reordered, so the fitter's sorted x equals vg.
    let n = 80usize;
    let vg: Vec<f64> = (0..n)
        .map(|i| 6.0 * (i as f64) / ((n - 1) as f64))
        .collect();
    let id_abs: Vec<f64> = vg
        .iter()
        .map(|&v| (0.015 * (v - 1.2)).powi(2) + 5e-13)
        .collect();
    let fitter = WindowedFitter::new(
        &SweepData {
            vg: vg.clone(),
            id_abs,
        },
        Transform::Sqrt,
    );
    (fitter, vg)
}

#[test]
fn fit_indices_exposes_size_and_sorted_x() {
    let (fitter, vg) = sqrt_fitter();
    assert_eq!(fitter.n(), vg.len());
    assert_eq!(fitter.x(), vg.as_slice());
}

#[test]
fn fit_indices_full_range_equals_fit_none() {
    let (fitter, _) = sqrt_fitter();
    let whole = fitter.fit(None);
    let idx = fitter.fit_indices(0, fitter.n());
    assert_eq!(idx.slope, whole.slope);
    assert_eq!(idx.intercept, whole.intercept);
    assert_eq!(idx.r2, whole.r2);
    assert_eq!(idx.points, whole.points);
}

#[test]
fn fit_indices_too_few_points_is_nan() {
    let (fitter, _) = sqrt_fitter();
    let res = fitter.fit_indices(5, 6); // one point
    assert_eq!(res.points, 1);
    assert!(res.slope.is_nan() && res.r2.is_nan());
}

#[test]
fn r2_never_exceeds_one_on_near_perfect_fit() {
    // sqrt(|Id|) is an exact linear function of Vg -> the one-pass SSE can cancel
    // slightly negative and report r2 > 1 without the clamp. Build Vg with the
    // validated numpy_compat::linspace so the samples are bit-identical to
    // linspace(0, 5, 200): there the window [10,190) has an unclamped
    // r2 of 1.0000000000000002, so these `<= 1.0` assertions genuinely fail if the
    // upper clamp is removed (a hand-rolled `(5*i)/199` Vg differs by ~1 ULP and
    // does NOT overshoot, which would make this guard vacuous).
    let vg = paramex_core::shared::numpy_compat::linspace(0.0, 5.0, 200);
    let id_abs: Vec<f64> = vg.iter().map(|&v| (0.01 * (v + 0.1)).powi(2)).collect();
    let fitter = WindowedFitter::new(&SweepData { vg, id_abs }, Transform::Sqrt);
    assert!(fitter.fit(None).r2 <= 1.0);
    assert!(fitter.fit_indices(10, 190).r2 <= 1.0);
}
