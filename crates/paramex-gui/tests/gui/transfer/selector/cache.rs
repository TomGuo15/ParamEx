//! `PlotCache` / `CurveView` contract: the per-file derived curve data (split
//! sweeps, scatters, axes, fitters) is built ONCE per file, matches what the old
//! per-frame path derived from core, and is result-independent (bwd scatters are
//! cached even before a recompute lands `has_backward_sweep`).

use paramex_core::transfer::{
    axis_bounds, log_current_axis_range, split_double_sweep, sqrt_current_axis_range, Transform,
    WindowedFitter,
};
use paramex_gui::workspaces::transfer::state::{PlotCache, PlotKind, SweepBranch};

/// A strictly monotonic single sweep: the split yields a 1-POINT apex bwd branch.
fn monotonic_curve() -> (Vec<f64>, Vec<f64>) {
    let vg: Vec<f64> = (0..20).map(|i| i as f64 * 0.1).collect();
    let id_abs: Vec<f64> = vg.iter().map(|v| 1e-9 * 10f64.powf(v * 2.0)).collect();
    (vg, id_abs)
}

/// A round-trip double sweep (20 up + 20 down, duplicated apex): both branches
/// clear the >=12-point `has_backward_sweep` bar. The down branch carries 2x the
/// current (hysteresis) ON PURPOSE: a palindromic fixture made every fwd/bwd
/// assertion branch-symmetric, so a swapped fitter/scatter router passed the
/// whole suite — asymmetry is what lets the routing pins below bite.
fn double_sweep_curve() -> (Vec<f64>, Vec<f64>) {
    let up = (0..20).map(|i| i as f64 * 0.1);
    let down = (0..20).rev().map(|i| i as f64 * 0.1);
    let vg: Vec<f64> = up.chain(down).collect();
    let id_abs: Vec<f64> = vg
        .iter()
        .enumerate()
        .map(|(idx, v)| {
            let base = 1e-9 * 10f64.powf(v * 2.0);
            if idx < 20 {
                base
            } else {
                base * 2.0
            }
        })
        .collect();
    (vg, id_abs)
}

#[test]
fn view_is_memoized_per_file() {
    let mut cache = PlotCache::default();
    let (vg, id_abs) = double_sweep_curve();
    let (s1_fwd_len, s1_bwd_len, n1) = {
        let v = cache.view("f1", &vg, &id_abs);
        (
            v.scatter(SweepBranch::Forward, PlotKind::Vt).len(),
            v.scatter(SweepBranch::Backward, PlotKind::Vt).len(),
            v.fitter(SweepBranch::Forward, PlotKind::Vt).n(),
        )
    };
    // A cache HIT must ignore its curve arguments entirely (ids are immutable
    // per curve): re-query with a DIFFERENT, shorter curve and assert the view
    // still reflects the FIRST build — an every-call rebuild fails here.
    let (other_vg, other_id) = monotonic_curve();
    let v = cache.view("f1", &other_vg, &other_id);
    assert_eq!(
        v.scatter(SweepBranch::Forward, PlotKind::Vt).len(),
        s1_fwd_len
    );
    assert_eq!(
        v.scatter(SweepBranch::Backward, PlotKind::Vt).len(),
        s1_bwd_len
    );
    assert_eq!(v.fitter(SweepBranch::Forward, PlotKind::Vt).n(), n1);
    assert_eq!(cache.view_count(), 1); // repeated calls hit the same entry
}

#[test]
fn axes_match_core_helpers_exactly() {
    let mut cache = PlotCache::default();
    let (vg, id_abs) = monotonic_curve();
    let a = cache.view("f1", &vg, &id_abs).axes();
    assert_eq!(a.vt_y(), sqrt_current_axis_range(&id_abs));
    assert_eq!(a.ss_y(), log_current_axis_range(&id_abs));
    assert_eq!(a.vg(), axis_bounds(&vg));
}

#[test]
fn empty_view_reserves_loaded_plot_chrome_without_fake_points() {
    let cache = PlotCache::default();
    let view = cache.empty_view();

    assert_eq!(view.axes().vg(), (0.0, 5.0));
    assert_eq!(view.axes().vt_y(), [0.0, 0.04]);
    assert_eq!(view.axes().ss_y(), [-15.0, -3.0]);
    assert!(view.scatter(SweepBranch::Forward, PlotKind::Vt).is_empty());
    assert!(view.scatter(SweepBranch::Forward, PlotKind::Ss).is_empty());
}

#[test]
fn bwd_scatter_is_result_independent() {
    let mut cache = PlotCache::default();

    // Double sweep: a full bwd branch is cached (the >=12-point has_backward_sweep
    // gate lives on the COMMITTED result and only controls drawing, never caching).
    let (vg, id_abs) = double_sweep_curve();
    let (fwd, bwd) = split_double_sweep(&vg, &id_abs);
    let v = cache.view("double", &vg, &id_abs);
    assert_eq!(
        v.scatter(SweepBranch::Forward, PlotKind::Vt).len(),
        fwd.vg.len()
    );
    assert_eq!(
        v.scatter(SweepBranch::Forward, PlotKind::Ss).len(),
        fwd.vg.len()
    );
    assert_eq!(
        v.scatter(SweepBranch::Backward, PlotKind::Vt).len(),
        bwd.vg.len()
    );
    assert_eq!(
        v.scatter(SweepBranch::Backward, PlotKind::Ss).len(),
        bwd.vg.len()
    );
    assert!(v.scatter(SweepBranch::Backward, PlotKind::Vt).len() >= 12);

    // Monotonic single sweep: the split's 1-point apex bwd branch is cached as-is
    // (harmless — drawing is gated on has_bwd at the call site).
    let (vg, id_abs) = monotonic_curve();
    let v = cache.view("mono", &vg, &id_abs);
    assert_eq!(v.scatter(SweepBranch::Backward, PlotKind::Vt).len(), 1);
    assert_eq!(v.scatter(SweepBranch::Backward, PlotKind::Ss).len(), 1);
    assert_eq!(v.scatter(SweepBranch::Backward, PlotKind::Vt).len(), 1);
}

#[test]
fn fwd_fitter_matches_direct_construction() {
    let mut cache = PlotCache::default();
    let (vg, id_abs) = double_sweep_curve();
    let (fwd, bwd) = split_double_sweep(&vg, &id_abs);
    let direct_vt = WindowedFitter::new(&fwd, Transform::Sqrt);
    let direct_ss = WindowedFitter::new(&fwd, Transform::Log);
    let direct_bwd_vt = WindowedFitter::new(&bwd, Transform::Sqrt);
    let v = cache.view("f1", &vg, &id_abs);
    assert_eq!(
        v.fitter(SweepBranch::Forward, PlotKind::Vt).n(),
        direct_vt.n()
    );
    assert_eq!(
        v.fitter(SweepBranch::Forward, PlotKind::Ss).n(),
        direct_ss.n()
    );
    assert_eq!(
        v.fitter(SweepBranch::Backward, PlotKind::Vt).n(),
        direct_bwd_vt.n()
    );
    assert_eq!(
        v.fitter(SweepBranch::Forward, PlotKind::Vt).x(),
        direct_vt.x()
    );
    // Branch-routing pins on the hysteretic fixture (the down branch carries 2x
    // the current, so the two branches fit DIFFERENT slopes — a swapped router
    // cannot pass these, where n()/x() comparisons could):
    let fwd_slope = v.fitter(SweepBranch::Forward, PlotKind::Vt).fit(None).slope;
    let bwd_slope = v
        .fitter(SweepBranch::Backward, PlotKind::Vt)
        .fit(None)
        .slope;
    assert_eq!(fwd_slope, direct_vt.fit(None).slope);
    assert_eq!(bwd_slope, direct_bwd_vt.fit(None).slope);
    assert_ne!(fwd_slope, bwd_slope, "hysteretic branches must fit apart");
}

#[test]
fn scatter_routing_matches_branch_and_transform() {
    // Value-level pins for the scatter router: y must be sqrt(|I_D|) on the VT
    // graph and log10(|I_D|) on the SS graph, from the matching SPLIT branch
    // (the hysteretic fixture keeps fwd/bwd distinguishable by value).
    let mut cache = PlotCache::default();
    let (vg, id_abs) = double_sweep_curve();
    let (fwd, bwd) = split_double_sweep(&vg, &id_abs);
    let v = cache.view("f1", &vg, &id_abs);
    let k = 3;
    assert_eq!(
        v.scatter(SweepBranch::Forward, PlotKind::Vt)[k],
        [fwd.vg[k], fwd.id_abs[k].sqrt()]
    );
    assert_eq!(
        v.scatter(SweepBranch::Forward, PlotKind::Ss)[k],
        [fwd.vg[k], fwd.id_abs[k].log10()]
    );
    assert_eq!(
        v.scatter(SweepBranch::Backward, PlotKind::Vt)[k],
        [bwd.vg[k], bwd.id_abs[k].sqrt()]
    );
    assert_eq!(
        v.scatter(SweepBranch::Backward, PlotKind::Ss)[k],
        [bwd.vg[k], bwd.id_abs[k].log10()]
    );
    // The accessor routes to the same arrays.
    assert_eq!(
        v.scatter(SweepBranch::Backward, PlotKind::Ss)[k],
        [bwd.vg[k], bwd.id_abs[k].log10()]
    );
    assert_ne!(
        v.scatter(SweepBranch::Forward, PlotKind::Vt)[k][1],
        v.scatter(SweepBranch::Backward, PlotKind::Vt)[k][1],
        "hysteretic branches must scatter apart"
    );
}

#[test]
fn prune_to_drops_absent_files() {
    use std::collections::HashSet;
    let mut cache = PlotCache::default();
    let (vg, id_abs) = monotonic_curve();
    cache.view("keep", &vg, &id_abs);
    cache.view("drop", &vg, &id_abs);
    assert_eq!(cache.view_count(), 2);
    let live: HashSet<&str> = ["keep"].into_iter().collect();
    cache.prune_to(|id| live.contains(id));
    assert_eq!(cache.view_count(), 1); // only "keep" remains
    assert!(cache.has_view("keep"));
    assert!(!cache.has_view("drop"));
}
