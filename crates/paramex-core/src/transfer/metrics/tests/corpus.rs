use crate::transfer::metrics::{
    hysteresis::extract_delta_vth_hysteresis_curve_shift,
    on_off::on_off_ratio,
    ss::{extract_ss, select_ss_window, SsWindowSelector},
    vth::{extract_vt_mu, select_elr_vt_window, FitGate, VtWindowSelector, DEFAULT_VT_R2_LADDER},
};
use crate::transfer::split_double_sweep;
use crate::transfer::test_support::{assert_close, f64_vec, load_reference_in, parse_f64};
use crate::transfer::types::ExtractionContext;

/// The production V_TH selector defaults the corpus was generated with.
const VT_SELECTOR: VtWindowSelector = VtWindowSelector {
    window_size: 30,
    step: 1,
    min_points: 10,
    min_r2: 0.99,
    r2_ladder: &DEFAULT_VT_R2_LADDER,
};
/// The production SS selector defaults the corpus was generated with.
const SS_SELECTOR: SsWindowSelector = SsWindowSelector {
    max_points: 30,
    min_decades: 1.0,
    min_points: 5,
    min_r2: 0.9,
    off_guard_decades: 0.3,
};

#[test]
fn metrics_corpus_full_auto_paths_match_reference() {
    let g = load_reference_in("metrics", "metrics_corpus");
    let cases = g["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 8, "expected 8 corpus seeds");
    for case in cases {
        let seed = case["seed"].as_u64().unwrap();
        let vg = f64_vec(&case["vg"]);
        let id_abs = f64_vec(&case["id_abs"]);
        let ctx = ExtractionContext {
            cox_f_per_cm2: parse_f64(&case["cox"]),
            aspect_ratio: parse_f64(&case["aspect"]),
        };

        let vt_window = select_elr_vt_window(&vg, &id_abs, &VT_SELECTOR);
        let vt_gate = FitGate {
            min_points: 10,
            min_r2: 0.99,
        };
        let vtmu = extract_vt_mu(&vg, &id_abs, ctx, vt_window, vt_gate);
        let e = &case["vt_mu_auto"];
        assert_close(vtmu.vt, parse_f64(&e["vt"]), 1e-9, 1e-12);
        assert_close(vtmu.mobility, parse_f64(&e["mobility"]), 1e-9, 1e-12);
        assert_close(vtmu.slope, parse_f64(&e["slope"]), 1e-9, 1e-12);
        assert_close(vtmu.r2, parse_f64(&e["r2"]), 1e-9, 1e-12);
        assert_eq!(
            vtmu.points,
            e["points"].as_u64().unwrap() as usize,
            "seed {seed}: vt points"
        );

        let ss_window = select_ss_window(&vg, &id_abs, &SS_SELECTOR);
        let ss = extract_ss(&vg, &id_abs, ss_window, 5);
        let e = &case["ss_auto"];
        assert_close(ss.ss_mv_dec, parse_f64(&e["ss_mv_dec"]), 1e-9, 1e-12);
        assert_close(ss.slope, parse_f64(&e["slope"]), 1e-9, 1e-12);
        assert_close(ss.r2, parse_f64(&e["r2"]), 1e-9, 1e-12);
        assert_eq!(
            ss.points,
            e["points"].as_u64().unwrap() as usize,
            "seed {seed}: ss points"
        );

        let (ion, ioff, ratio) = on_off_ratio(&id_abs);
        let e = &case["on_off"];
        assert_close(ion, parse_f64(&e["ion"]), 1e-9, 1e-12);
        assert_close(ioff, parse_f64(&e["ioff"]), 1e-9, 1e-12);
        assert_close(ratio, parse_f64(&e["ratio"]), 1e-9, 1e-12);

        let vg_rt = f64_vec(&case["vg_rt"]);
        let id_rt = f64_vec(&case["id_rt"]);
        let (forward, backward) = split_double_sweep(&vg_rt, &id_rt);
        let hy = extract_delta_vth_hysteresis_curve_shift(&forward, &backward, 0.2, 12);
        let e = &case["hysteresis"];
        assert_close(hy, parse_f64(&e["delta_vt"]), 1e-9, 1e-12);
        assert_close(hy.abs(), parse_f64(&e["abs_delta_vt"]), 1e-9, 1e-12);
    }
}
