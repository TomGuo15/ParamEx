//! Transfer fit-window selector transient state.

use super::plot_cache::PlotKind;

/// Per-graph window-selector mode. Auto clears that graph's fwd+bwd pins
/// -> auto-select; Fwd/Bwd route edits to that direction. Fresh file starts Auto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphMode {
    #[default]
    Auto,
    Fwd,
    Bwd,
}

/// Selector transient state for the SELECTED file's two graphs. `last_file` re-derives
/// mode on file-switch; `drag` is the live (uncommitted) window shared by strip + grab.
#[derive(Debug, Clone, Default)]
pub struct SelectorUi {
    vt_mode: GraphMode,
    ss_mode: GraphMode,
    last_file: Option<String>,
    drag: Option<DragState>,
}

impl SelectorUi {
    /// Re-derive selector state for a newly selected file. Returns true when callers
    /// should clear file-scoped edit buffers.
    pub fn sync_file(&mut self, file_id: &str, vt_mode: GraphMode, ss_mode: GraphMode) -> bool {
        if self.last_file.as_deref() == Some(file_id) {
            return false;
        }
        self.vt_mode = vt_mode;
        self.ss_mode = ss_mode;
        self.last_file = Some(file_id.to_string());
        self.drag = None;
        true
    }

    pub fn mode(&self, kind: PlotKind) -> GraphMode {
        match kind {
            PlotKind::Vt => self.vt_mode,
            PlotKind::Ss => self.ss_mode,
        }
    }

    pub fn set_mode(&mut self, kind: PlotKind, mode: GraphMode) {
        match kind {
            PlotKind::Vt => self.vt_mode = mode,
            PlotKind::Ss => self.ss_mode = mode,
        }
    }

    pub fn reset_modes_to_auto(&mut self) {
        self.vt_mode = GraphMode::Auto;
        self.ss_mode = GraphMode::Auto;
    }

    pub fn drag(&self) -> Option<DragState> {
        self.drag
    }

    pub fn live_window(&self, kind: PlotKind) -> Option<(f64, f64)> {
        DragState::window_for_kind(self.drag, kind)
    }

    pub fn drag_for(&self, kind: PlotKind) -> Option<DragState> {
        self.drag.filter(|drag| drag.is_for(kind))
    }

    pub fn start_drag(&mut self, kind: PlotKind, edge: DragEdge, lo: f64, hi: f64) {
        self.drag = Some(DragState::new(kind, edge, lo, hi));
    }

    pub fn set_strip_drag(&mut self, kind: PlotKind, lo: f64, hi: f64) {
        self.start_drag(kind, DragEdge::Whole, lo, hi);
    }

    pub fn update_drag_for<F>(&mut self, kind: PlotKind, update: F)
    where
        F: FnOnce(&mut DragState),
    {
        if let Some(drag) = self.drag.as_mut() {
            if drag.is_for(kind) {
                update(drag);
            }
        }
    }

    /// Take the drag state for this graph. A drag started on the other graph is
    /// restored untouched so one column cannot consume its sibling's release.
    pub fn finish_drag_for(&mut self, kind: PlotKind) -> Option<DragState> {
        let drag = self.drag.take()?;
        if drag.is_for(kind) {
            Some(drag)
        } else {
            self.drag = Some(drag);
            None
        }
    }

    pub fn clear_drag(&mut self) {
        self.drag = None;
    }
}

/// Which part of a selector window is being dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragEdge {
    Lo,
    Hi,
    Whole,
}

/// An in-progress window drag. Panel-only enums stay out of `state`; callers name
/// graphs with the state-owned [`PlotKind`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragState {
    side_is_vt: bool, // true = VT graph, false = SS graph
    edge: DragEdge,
    lo: f64,
    hi: f64,
}

impl DragState {
    pub fn new(kind: PlotKind, edge: DragEdge, lo: f64, hi: f64) -> Self {
        Self {
            side_is_vt: matches!(kind, PlotKind::Vt),
            edge,
            lo,
            hi,
        }
    }

    fn is_for(self, kind: PlotKind) -> bool {
        self.side_is_vt == matches!(kind, PlotKind::Vt)
    }

    pub fn edge(self) -> DragEdge {
        self.edge
    }

    pub fn window(self) -> (f64, f64) {
        (self.lo, self.hi)
    }

    pub fn set_window(&mut self, lo: f64, hi: f64) {
        self.lo = lo;
        self.hi = hi;
    }

    pub fn window_for_kind(drag: Option<Self>, kind: PlotKind) -> Option<(f64, f64)> {
        drag.filter(|drag| drag.is_for(kind)).map(Self::window)
    }
}
