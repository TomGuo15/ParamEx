//! Hand-computed expectations for the collinear corpus groups, so the oracle is
//! checked against arithmetic rather than only against its own past output.
//!
//! The generator writes `R_total = |V_D| / I` at `L = [50, 80, 120, 160] µm`:
//! `process_b` uses `[140k, 200k, 280k, 360k] Ω` and `process_d` uses
//! `[180k, 270k, 390k, 510k] Ω`. Both are exactly linear in `L`.

use crate::common::{assert_close, tlm_corpus_dir};
use paramex_core::tlm::{analyze_dataset, load_dataset, GroupAnalysis};

const RTOL: f64 = 1e-9;

struct Expected {
    group: &'static str,
    slope_ohm_per_um: f64,
    intercept_ohm: f64,
}

/// process_b: consecutive differences 60k/30, 80k/40, 80k/40 = 2000 Ω/µm;
/// intercept 140k - 2000 * 50 = 40 000 Ω.
/// process_d: 90k/30, 120k/40, 120k/40 = 3000 Ω/µm; intercept 180k - 3000 * 50
/// = 30 000 Ω.
const EXPECTED: [Expected; 2] = [
    Expected {
        group: "process_b",
        slope_ohm_per_um: 2000.0,
        intercept_ohm: 40_000.0,
    },
    Expected {
        group: "process_d",
        slope_ohm_per_um: 3000.0,
        intercept_ohm: 30_000.0,
    },
];

fn assert_collinear_fit(fit: &GroupAnalysis, expected: &Expected) {
    let label = expected.group;
    assert_eq!(fit.points.len(), 4, "{label}: four channel lengths");
    assert_close(
        fit.fit.slope_ohm_per_um,
        expected.slope_ohm_per_um,
        RTOL,
        0.0,
    );
    assert_close(fit.fit.intercept_ohm, expected.intercept_ohm, RTOL, 0.0);
    assert_close(
        fit.fit.rc_per_contact_ohm,
        expected.intercept_ohm / 2.0,
        RTOL,
        0.0,
    );
    assert!(
        (fit.fit.r_squared - 1.0).abs() <= RTOL,
        "{label}: r² = {} should be 1",
        fit.fit.r_squared
    );
    assert!(
        fit.warnings.is_empty(),
        "{label}: collinear group must not warn: {:?}",
        fit.warnings
    );
    // Each length point reproduces its designed R_total through |V_D| / I.
    for (point, designed) in fit.points.iter().zip([50.0, 80.0, 120.0, 160.0]) {
        assert_eq!(point.length_um, designed, "{label}: length order");
        let r_total = expected.intercept_ohm + expected.slope_ohm_per_um * designed;
        assert_close(point.rtotal_ohm, r_total, RTOL, 0.0);
    }
}

#[test]
fn collinear_corpus_groups_fit_their_designed_lines() {
    let ds = load_dataset(&tlm_corpus_dir(), None).expect("loads corpus");
    let res = analyze_dataset(&ds, None);
    // The generator peaks every device current at V_G = -40 V, so the default
    // (strongest median current) selection lands there.
    assert_eq!(res.selected_vg, -40.0);

    for expected in &EXPECTED {
        let fit = res
            .group(expected.group)
            .unwrap_or_else(|| panic!("{} present", expected.group));
        assert_collinear_fit(fit, expected);
    }
}
