//! Transfer-only presentation state: selection, geometry/Cox editors, plot
//! derivations, and persistent pending/error rows. Scientific results remain
//! owned by `paramex_core::transfer::Session`.

use paramex_core::shared::same_named_source;
use paramex_core::transfer::OutputDataset;
use std::sync::atomic::{AtomicU64, Ordering};

mod cox;
mod file_rows;
mod geometry;
mod plot_cache;
mod selector;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingOutputReason {
    NoMatch,
    Ambiguous,
    Detached,
}

impl PendingOutputReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::NoMatch => "No match",
            Self::Ambiguous => "Ambiguous",
            Self::Detached => "Detached",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingOutput {
    id: String,
    dataset: OutputDataset,
    reason: PendingOutputReason,
}

impl PendingOutput {
    pub fn new(dataset: OutputDataset, reason: PendingOutputReason) -> Self {
        static NEXT_PENDING_OUTPUT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_PENDING_OUTPUT_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            id: format!("pending-output-{id}"),
            dataset,
            reason,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.dataset.name
    }

    pub fn reason(&self) -> PendingOutputReason {
        self.reason
    }

    pub(crate) fn dataset(&self) -> &OutputDataset {
        &self.dataset
    }

    pub(crate) fn into_dataset(self) -> OutputDataset {
        self.dataset
    }
}

pub use cox::{parse_or_zero, CoxUi, LayerRow, COX_ESTIMATE_PENDING_LABEL};
pub(crate) use file_rows::{FileRow, FileRows};
pub use geometry::GeometryUi;
pub use plot_cache::{AxisRanges, CurveView, PlotCache, PlotKind, SweepBranch};
pub use selector::{DragEdge, DragState, GraphMode, SelectorUi};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransferResultsView {
    #[default]
    Transfer,
    Output,
}

impl TransferResultsView {
    pub fn index(self) -> usize {
        match self {
            Self::Transfer => 0,
            Self::Output => 1,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Output,
            _ => Self::Transfer,
        }
    }
}

/// Per-frame Transfer UI state that does not round-trip to core Transfer data.
#[derive(Debug, Default)]
pub struct TransferUiState {
    geometry: GeometryUi,
    cox: CoxUi,
    selector: SelectorUi,
    results_view: TransferResultsView,
}

impl TransferUiState {
    pub fn results_view(&self) -> TransferResultsView {
        self.results_view
    }

    pub fn set_results_view(&mut self, view: TransferResultsView) {
        self.results_view = view;
    }

    pub fn selector_mut(&mut self) -> &mut SelectorUi {
        &mut self.selector
    }

    pub fn geometry_mut(&mut self) -> &mut GeometryUi {
        &mut self.geometry
    }

    pub fn cox(&self) -> &CoxUi {
        &self.cox
    }

    pub fn cox_mut(&mut self) -> &mut CoxUi {
        &mut self.cox
    }
}

/// Same-source key for pending-output rows.
pub(crate) fn same_output_source(a: &OutputDataset, b: &OutputDataset) -> bool {
    same_named_source(
        &a.name,
        a.source_path.as_deref(),
        &b.name,
        b.source_path.as_deref(),
    )
}

pub(crate) fn upsert_pending_output(
    pending_outputs: &mut Vec<PendingOutput>,
    dataset: OutputDataset,
    reason: PendingOutputReason,
) {
    let pending = PendingOutput::new(dataset, reason);
    replace_all_matching(pending_outputs, pending, |row, incoming| {
        same_output_source(row.dataset(), incoming.dataset())
    });
}

/// Remove every element matching `incoming`, reinsert `incoming` at the first
/// match's index; push if none matched.
fn replace_all_matching<T>(values: &mut Vec<T>, incoming: T, matches: impl Fn(&T, &T) -> bool) {
    let Some(index) = values.iter().position(|value| matches(value, &incoming)) else {
        values.push(incoming);
        return;
    };
    values.retain(|value| !matches(value, &incoming));
    values.insert(index, incoming);
}

/// Retain an older attached value only when no newer pending generation
/// directly matches it.
pub(crate) fn retain_detached_output(
    pending_outputs: &mut Vec<PendingOutput>,
    dataset: OutputDataset,
) {
    if pending_outputs
        .iter()
        .any(|pending| same_output_source(pending.dataset(), &dataset))
    {
        return;
    }
    pending_outputs.push(PendingOutput::new(dataset, PendingOutputReason::Detached));
}
