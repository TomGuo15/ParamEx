//! Transfer selector plot cache: per-file curve derivation for selector render.

use std::collections::HashMap;

use paramex_core::transfer::{
    axis_bounds, log_current_axis_range, split_double_sweep, sqrt_current_axis_range, SweepData,
    Transform, WindowedFitter,
};

const EMPTY_SELECTOR_VG_AXIS: (f64, f64) = (0.0, 5.0);
const EMPTY_SELECTOR_VT_Y: [f64; 2] = [0.0, 0.04];
const EMPTY_SELECTOR_SS_Y: [f64; 2] = [-15.0, -3.0];

/// Which graph a fitter/axis serves. Local Hash enum because `transfer::Transform`
/// is Copy/Eq but NOT Hash -> cannot be a HashMap key; map PlotKind->Transform at fit time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlotKind {
    Vt,
    Ss,
}

impl PlotKind {
    pub fn transform(self) -> Transform {
        match self {
            PlotKind::Vt => Transform::Sqrt,
            PlotKind::Ss => Transform::Log,
        }
    }
}

/// Which split-sweep branch a fitter regresses (cache-key half).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SweepBranch {
    Forward,
    Backward,
}

/// Memoized per-file axis ranges from `core::transfer` (depend only on the curve).
#[derive(Debug, Clone, Copy)]
pub struct AxisRanges {
    vt_y: [f64; 2], // sqrt_current_axis_range(&id_abs)
    ss_y: [f64; 2], // log_current_axis_range(&id_abs) - log10 units
    vg: (f64, f64), // axis_bounds(&vg)
}

impl AxisRanges {
    /// Y-axis bounds for the sqrt-current V_TH graph.
    pub fn vt_y(self) -> [f64; 2] {
        self.vt_y
    }

    /// Y-axis bounds for the log-current SS graph.
    pub fn ss_y(self) -> [f64; 2] {
        self.ss_y
    }

    /// Shared gate-voltage axis bounds.
    pub fn vg(self) -> (f64, f64) {
        self.vg
    }
}

/// Everything the selector derives from one file's raw curve: the split sweeps,
/// the four transformed scatter series, the whole-curve axis ranges, and the four
/// window-independent preview fitters. Built once per file (curves are immutable
/// once loaded) and borrowed immutably by the render path, so nothing
/// here is rebuilt per frame.
pub struct CurveView {
    vt_fwd_scatter: Vec<[f64; 2]>,
    ss_fwd_scatter: Vec<[f64; 2]>,
    vt_bwd_scatter: Vec<[f64; 2]>,
    ss_bwd_scatter: Vec<[f64; 2]>,
    axes: AxisRanges,
    /// `[fwd/Vt, fwd/Ss, bwd/Vt, bwd/Ss]` - addressed only through [`Self::fitter`].
    fitters: [WindowedFitter; 4],
}

impl CurveView {
    /// Derive the full view from the raw curve. Result-INDEPENDENT by design: the
    /// bwd scatters/fitters are built whenever the split yields backward points -
    /// deliberately NOT gated on `has_backward_sweep`, which lives on the committed
    /// result and flips false->true when a recompute lands; drawing stays gated on
    /// it at the call site. All four fitters are built eagerly: `WindowedFitter::new`
    /// is total on an empty `SweepData` (n()==0; `fit` returns NaN/points:0).
    fn build(vg: &[f64], id_abs: &[f64]) -> Self {
        if vg.is_empty() && id_abs.is_empty() {
            return Self::empty_selector_scaffold();
        }
        let (fwd, bwd) = split_double_sweep(vg, id_abs);
        // Scatter transformed in-GUI (the fitter's transformed y is private). NOT
        // masked like the fitter (which drops id_abs <= 0): sqrt/log10 map directly
        // and egui_plot tolerates NaN/-inf - never unify scatter with the fitter.
        let sqrt_scatter = |s: &SweepData| -> Vec<[f64; 2]> {
            s.vg.iter()
                .zip(&s.id_abs)
                .map(|(v, i)| [*v, i.sqrt()])
                .collect()
        };
        let log_scatter = |s: &SweepData| -> Vec<[f64; 2]> {
            s.vg.iter()
                .zip(&s.id_abs)
                .map(|(v, i)| [*v, i.log10()])
                .collect()
        };
        CurveView {
            vt_fwd_scatter: sqrt_scatter(&fwd),
            ss_fwd_scatter: log_scatter(&fwd),
            vt_bwd_scatter: sqrt_scatter(&bwd),
            ss_bwd_scatter: log_scatter(&bwd),
            // Whole-curve axes from the RAW curve (matches the commit clamp axis +
            // the oracle's _axis_bounds(curve.vg)), not the split branches.
            axes: AxisRanges {
                vt_y: sqrt_current_axis_range(id_abs),
                ss_y: log_current_axis_range(id_abs),
                vg: axis_bounds(vg),
            },
            fitters: [
                WindowedFitter::new(&fwd, PlotKind::Vt.transform()),
                WindowedFitter::new(&fwd, PlotKind::Ss.transform()),
                WindowedFitter::new(&bwd, PlotKind::Vt.transform()),
                WindowedFitter::new(&bwd, PlotKind::Ss.transform()),
            ],
        }
    }

    fn empty_selector_scaffold() -> Self {
        let empty = SweepData {
            vg: Vec::new(),
            id_abs: Vec::new(),
        };
        CurveView {
            vt_fwd_scatter: Vec::new(),
            ss_fwd_scatter: Vec::new(),
            vt_bwd_scatter: Vec::new(),
            ss_bwd_scatter: Vec::new(),
            axes: AxisRanges {
                vt_y: EMPTY_SELECTOR_VT_Y,
                ss_y: EMPTY_SELECTOR_SS_Y,
                vg: EMPTY_SELECTOR_VG_AXIS,
            },
            fitters: [
                WindowedFitter::new(&empty, PlotKind::Vt.transform()),
                WindowedFitter::new(&empty, PlotKind::Ss.transform()),
                WindowedFitter::new(&empty, PlotKind::Vt.transform()),
                WindowedFitter::new(&empty, PlotKind::Ss.transform()),
            ],
        }
    }

    pub fn axes(&self) -> AxisRanges {
        self.axes
    }

    /// The preview fitter for (branch, kind). Window-independent - only
    /// `.fit(window)` re-queries.
    pub fn fitter(&self, branch: SweepBranch, kind: PlotKind) -> &WindowedFitter {
        match (branch, kind) {
            (SweepBranch::Forward, PlotKind::Vt) => &self.fitters[0],
            (SweepBranch::Forward, PlotKind::Ss) => &self.fitters[1],
            (SweepBranch::Backward, PlotKind::Vt) => &self.fitters[2],
            (SweepBranch::Backward, PlotKind::Ss) => &self.fitters[3],
        }
    }

    /// The transformed scatter series for (branch, kind).
    pub fn scatter(&self, branch: SweepBranch, kind: PlotKind) -> &[[f64; 2]] {
        match (branch, kind) {
            (SweepBranch::Forward, PlotKind::Vt) => &self.vt_fwd_scatter,
            (SweepBranch::Forward, PlotKind::Ss) => &self.ss_fwd_scatter,
            (SweepBranch::Backward, PlotKind::Vt) => &self.vt_bwd_scatter,
            (SweepBranch::Backward, PlotKind::Ss) => &self.ss_bwd_scatter,
        }
    }
}

/// Per-file [`CurveView`] memo. The render path takes `&CurveView` - immutable -
/// so the cache borrow never collides with the selector's `&mut` state.
#[derive(Default)]
pub struct PlotCache {
    views: HashMap<String, CurveView>,
}

impl PlotCache {
    /// The cached view for `file_id`, built from the raw curve on first request.
    /// Two-step contains/index rather than the `entry()` interface: `entry` would
    /// allocate the `String` key on every call, even on cache HITS.
    /// Empty-state chrome for the selector when no file is selected. Not stored
    /// in the per-file map, so prune does not rebuild four fitters every frame.
    pub fn empty_view(&self) -> &CurveView {
        static EMPTY: std::sync::OnceLock<CurveView> = std::sync::OnceLock::new();
        EMPTY.get_or_init(CurveView::empty_selector_scaffold)
    }

    pub fn view(&mut self, file_id: &str, vg: &[f64], id_abs: &[f64]) -> &CurveView {
        if !self.views.contains_key(file_id) {
            self.views
                .insert(file_id.to_string(), CurveView::build(vg, id_abs));
        }
        &self.views[file_id]
    }

    /// Drop cache entries for files that no longer exist. A window change never
    /// invalidates: entries are window-independent and curves are immutable
    /// once loaded. Called once per frame from the
    /// selector so removed files don't leak view entries. (ids are unique
    /// counter values, so this is a leak guard, not a stale-data fix.)
    pub fn prune_to<F: Fn(&str) -> bool>(&mut self, is_live: F) {
        self.views.retain(|id, _| is_live(id));
    }

    pub fn view_count(&self) -> usize {
        self.views.len()
    }

    pub fn has_view(&self, file_id: &str) -> bool {
        self.views.contains_key(file_id)
    }
}
