//! In-memory TLM session: the loaded dataset, its analyses, the active
//! selection, and the committed fallback drain bias. `Session` is the single
//! owner of TLM session state; a renderer keeps only presentation state
//! (table rows, tabs, error rows) around it.

use crate::tlm::report::{result_csv, sweep_csv};
use crate::tlm::service::{analyze_dataset, analyze_sweep};
use crate::tlm::types::{
    valid_vd, GroupAnalysis, TlmAnalysisResult, TlmDataset, TlmParseError, TlmSweepResult,
};

/// Fallback drain bias applied to workbooks without a `Setup(*)` sheet when
/// the user has not entered one.
pub const DEFAULT_FALLBACK_VD: f64 = -0.5;

/// A dataset together with both of its analyses.
///
/// Folder ingest computes this off the UI thread so installing a load into a
/// [`Session`] is a plain move. Later V_G or removal updates re-analyze the
/// resident dataset inside the session.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzedDataset {
    dataset: TlmDataset,
    result: TlmAnalysisResult,
    sweep: TlmSweepResult,
}

impl AnalyzedDataset {
    /// Analyze `dataset` at the engine-default V_G and across the full sweep.
    pub fn analyze(dataset: TlmDataset) -> Self {
        let result = analyze_dataset(&dataset, None);
        let sweep = analyze_sweep(&dataset);
        Self {
            dataset,
            result,
            sweep,
        }
    }

    /// Discovered workbooks, including those that failed to parse.
    pub fn workbook_count(&self) -> usize {
        self.dataset.statuses().len()
    }

    /// Process groups with at least one parsed curve.
    pub fn group_count(&self) -> usize {
        self.result.groups.len()
    }
}

/// In-memory TLM session. Single owner of the loaded dataset, the single-V_G
/// and full-sweep analyses derived from it, the selected group and V_G, and
/// the committed fallback V_D.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    dataset: Option<TlmDataset>,
    /// Single-V_G analysis at `selected_vg`.
    result: Option<TlmAnalysisResult>,
    /// Full V_G sweep; independent of `selected_vg`.
    sweep: Option<TlmSweepResult>,
    /// A group name present in `result`.
    selected_group: Option<String>,
    /// A measured voltage present in `result.vg_values`.
    selected_vg: Option<f64>,
    /// Applies at the next load; survives `clear`.
    fallback_vd: f64,
    /// Monotonic display-cache key: bumped whenever `result` or `sweep` may
    /// have changed, so a renderer can rebuild derived rows behind it.
    /// Selection-only changes do not bump because they do not alter the
    /// analyses.
    generation: u64,
}

impl Default for Session {
    fn default() -> Self {
        Session {
            dataset: None,
            result: None,
            sweep: None,
            selected_group: None,
            selected_vg: None,
            fallback_vd: DEFAULT_FALLBACK_VD,
            generation: 0,
        }
    }
}

impl Session {
    /// New empty session with the default fallback V_D.
    pub fn new() -> Self {
        Session::default()
    }

    /// True when a dataset is loaded.
    pub fn has_dataset(&self) -> bool {
        self.dataset.is_some()
    }

    /// The loaded dataset, if any.
    pub fn dataset(&self) -> Option<&TlmDataset> {
        self.dataset.as_ref()
    }

    /// The single-V_G analysis at the selected V_G, if a dataset is loaded.
    pub fn result(&self) -> Option<&TlmAnalysisResult> {
        self.result.as_ref()
    }

    /// The full-sweep analysis, if a dataset is loaded.
    pub fn sweep(&self) -> Option<&TlmSweepResult> {
        self.sweep.as_ref()
    }

    /// The selected process group, if a dataset is loaded.
    pub fn selected_group_name(&self) -> Option<&str> {
        self.selected_group.as_deref()
    }

    /// The selected measured V_G, if a dataset is loaded.
    pub fn selected_vg(&self) -> Option<f64> {
        self.selected_vg
    }

    /// The selected group's fit at the selected V_G.
    pub fn selected_group_analysis(&self) -> Option<&GroupAnalysis> {
        let result = self.result.as_ref()?;
        let name = self.selected_group.as_ref()?;
        result.group(name)
    }

    /// Committed fallback V_D in volts; finite and nonzero.
    pub fn fallback_vd(&self) -> f64 {
        self.fallback_vd
    }

    /// The current display-cache generation (see the field doc): equal values
    /// across two reads guarantee `result` and `sweep` are unchanged.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Selected-V_G result rows as CSV bytes.
    pub fn result_csv_bytes(&self) -> Option<Vec<u8>> {
        self.result.as_ref().map(result_csv)
    }

    /// Full-sweep rows as CSV bytes.
    pub fn sweep_csv_bytes(&self) -> Option<Vec<u8>> {
        self.sweep.as_ref().map(sweep_csv)
    }

    /// Commit a new fallback V_D. An invalid value returns `Err` with nothing
    /// mutated; the value applies at the next load, not to the loaded dataset.
    pub fn set_fallback_vd(&mut self, value: f64) -> Result<(), TlmParseError> {
        self.fallback_vd = valid_vd(value, "Fallback V_D")?;
        Ok(())
    }

    /// Install an analyzed load, selecting the engine-default V_G and the
    /// alphabetically first group.
    pub fn install(&mut self, analyzed: AnalyzedDataset) {
        let AnalyzedDataset {
            dataset,
            result,
            sweep,
        } = analyzed;
        self.selected_vg = Some(result.selected_vg);
        self.selected_group = result.first_group_name().map(str::to_owned);
        self.dataset = Some(dataset);
        self.result = Some(result);
        self.sweep = Some(sweep);
        self.generation += 1;
    }

    /// Drop the dataset, both analyses, and the selection. The fallback V_D is
    /// a committed setting and survives.
    pub fn clear(&mut self) {
        if self.dataset.is_none() {
            return;
        }
        self.dataset = None;
        self.result = None;
        self.sweep = None;
        self.selected_group = None;
        self.selected_vg = None;
        self.generation += 1;
    }

    /// Re-analyze the loaded dataset at `requested_vg`, snapped to the nearest
    /// measured V_G. The sweep is unaffected. No-op when nothing is loaded.
    pub fn recompute_at_vg(&mut self, requested_vg: f64) {
        let Some(dataset) = self.dataset.as_ref() else {
            return;
        };
        let result = analyze_dataset(dataset, Some(requested_vg));
        self.store_result(result);
    }

    /// Remove one workbook by its status-row relative path and re-analyze the
    /// remainder, so an outlier or a failed workbook can be dropped without
    /// reloading the folder. The selected V_G snaps to the nearest remaining
    /// value and the selected group falls back to the first when it
    /// disappears. Removing the last parsed curve clears the session.
    ///
    /// Returns the number of status rows removed, including residual failure
    /// rows dropped with the final curve; zero means nothing matched.
    pub fn remove_workbook(&mut self, relative_file: &str) -> usize {
        let Some(dataset) = self.dataset.take() else {
            return 0;
        };
        let removal = dataset.remove_workbook(relative_file);
        let removed = removal.removed_statuses;
        let Some(dataset) = removal.dataset else {
            // `take` already emptied the dataset slot; finish the clear by
            // hand so the generation bumps exactly once.
            self.result = None;
            self.sweep = None;
            self.selected_group = None;
            self.selected_vg = None;
            self.generation += 1;
            return removed;
        };
        if removed == 0 {
            self.dataset = Some(dataset);
            return 0;
        }
        let result = analyze_dataset(&dataset, self.selected_vg);
        let sweep = analyze_sweep(&dataset);
        self.dataset = Some(dataset);
        self.sweep = Some(sweep);
        self.store_result(result);
        removed
    }

    /// Select an analyzed process group by name. Unknown groups are ignored.
    pub fn select_group(&mut self, name: &str) -> bool {
        let Some(result) = self.result.as_ref() else {
            return false;
        };
        if !result.has_group(name) {
            return false;
        }
        self.selected_group = Some(name.to_owned());
        true
    }

    /// Store a fresh single-V_G result: adopt its snapped V_G, keep the
    /// selected group when it still exists (else the first), and bump the
    /// generation.
    fn store_result(&mut self, result: TlmAnalysisResult) {
        self.selected_vg = Some(result.selected_vg);
        if self
            .selected_group
            .as_deref()
            .is_none_or(|group| !result.has_group(group))
        {
            self.selected_group = result.first_group_name().map(str::to_owned);
        }
        self.result = Some(result);
        self.generation += 1;
    }
}
