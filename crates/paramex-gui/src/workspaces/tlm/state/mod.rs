//! TLM-page state: the core [`Session`] plus the presentation state around it
//! (pre-formatted table rows, the active results tab, and the persistent
//! load-error rows). All science and session transitions come from
//! `paramex_core::tlm`; load-time analysis runs on the worker thread and
//! arrives as an [`AnalyzedDataset`].

mod rows;

use paramex_core::tlm::{AnalyzedDataset, GroupAnalysis, Session, TlmParseError};

use rows::error_count;
pub use rows::TlmRows;

/// Which table the TLM results card shows. Transient view state. (File statuses
/// are not a tab: they have their own always-visible right-column FILES card.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TlmTab {
    #[default]
    Results,
    Sweep,
    Lengths,
}

impl TlmTab {
    pub fn index(self) -> usize {
        match self {
            TlmTab::Results => 0,
            TlmTab::Sweep => 1,
            TlmTab::Lengths => 2,
        }
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            1 => TlmTab::Sweep,
            2 => TlmTab::Lengths,
            _ => TlmTab::Results,
        }
    }
}

/// Folder/count projection for the DATA card.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TlmFolderSummary<'a> {
    pub root: &'a str,
    pub workbooks: usize,
    pub groups: usize,
}

/// Render-ready projection for the DATA card.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TlmDataCard<'a> {
    pub folder: Option<TlmFolderSummary<'a>>,
    /// Oldest undismissed error, if any. Extra rows stay queued behind it.
    pub load_error: Option<&'a str>,
    pub load_error_count: usize,
    pub fallback_vd: f64,
    pub has_dataset: bool,
}

/// Render-ready projection for the TLM GROUPS card.
pub struct TlmGroupList<'a> {
    pub groups: &'a [GroupAnalysis],
    pub selected: Option<&'a str>,
}

/// Render-ready projection for the TLM ANALYSIS V_G picker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TlmVgPicker<'a> {
    pub vg_values: &'a [f64],
    pub selected_vg: f64,
}

/// Render-ready projection for the RESULTS card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlmResultsCard {
    pub active_tab: TlmTab,
    pub has_result: bool,
    pub has_sweep: bool,
}

/// Render-ready projection for the TLM FILES status card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlmFilesCard {
    pub status_count: usize,
    pub error_count: usize,
}

/// TLM workspace state: the committed core session and the presentation
/// state derived from or displayed beside it.
pub struct TlmState {
    /// Committed session state: dataset, analyses, selection, fallback V_D.
    session: Session,
    /// Arrival-ordered persistent errors. The DATA card shows the oldest;
    /// later rows (for example a CSV export failure after a load failure)
    /// stay queued until that oldest row is dismissed.
    load_errors: Vec<String>,
    /// Which results table tab is active (transient view state).
    results_tab: TlmTab,
    /// Pre-formatted table rows for the session's current analyses. Rebuilt
    /// by [`Self::sync_rows`] whenever the session generation moves, so render
    /// code reads stable rows and never formats engine data per frame.
    rows: TlmRows,
    /// The session generation `rows` was built from; also the render-side
    /// measurement-cache key.
    rows_generation: u64,
}

impl Default for TlmState {
    fn default() -> Self {
        TlmState {
            session: Session::new(),
            load_errors: Vec::new(),
            results_tab: TlmTab::Results,
            rows: TlmRows::default(),
            rows_generation: 0,
        }
    }
}

impl TlmState {
    /// The committed core session.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// The pre-formatted table rows for the current analyses. Read-only: the
    /// commands below rebuild them in lockstep with [`Self::rows_generation`].
    pub fn rows(&self) -> &TlmRows {
        &self.rows
    }

    /// Monotonic id of the current `rows` contents; the key the render side
    /// uses to invalidate its per-table grid measurements.
    pub fn rows_generation(&self) -> u64 {
        self.rows_generation
    }

    /// Render-ready DATA card state.
    pub fn data_card(&self) -> TlmDataCard<'_> {
        let folder = self
            .session
            .dataset()
            .zip(self.session.result())
            .map(|(dataset, result)| TlmFolderSummary {
                root: dataset.root(),
                workbooks: result.statuses.len(),
                groups: result.groups.len(),
            });
        TlmDataCard {
            folder,
            load_error: self.load_errors.first().map(String::as_str),
            load_error_count: self.load_errors.len(),
            fallback_vd: self.session.fallback_vd(),
            has_dataset: self.session.has_dataset(),
        }
    }

    pub fn has_dataset(&self) -> bool {
        self.session.has_dataset()
    }

    pub fn has_load_error(&self) -> bool {
        !self.load_errors.is_empty()
    }

    pub fn fallback_vd(&self) -> f64 {
        self.session.fallback_vd()
    }

    pub fn set_fallback_vd(&mut self, value: f64) -> Result<(), TlmParseError> {
        self.session.set_fallback_vd(value)
    }

    /// Queue a persistent error row behind any already shown.
    pub fn push_load_error(&mut self, message: String) {
        self.load_errors.push(message);
    }

    pub fn dismiss_load_error(&mut self) {
        if !self.load_errors.is_empty() {
            self.load_errors.remove(0);
        }
    }

    /// Render-ready TLM GROUPS card state.
    pub fn group_list(&self) -> Option<TlmGroupList<'_>> {
        Some(TlmGroupList {
            groups: &self.session.result()?.groups,
            selected: self.session.selected_group_name(),
        })
    }

    pub fn selected_group_name(&self) -> Option<&str> {
        self.session.selected_group_name()
    }

    /// Render-ready TLM ANALYSIS V_G picker state.
    pub fn vg_picker(&self) -> Option<TlmVgPicker<'_>> {
        let result = self.session.result()?;
        Some(TlmVgPicker {
            vg_values: &result.vg_values,
            selected_vg: self.session.selected_vg().unwrap_or(result.selected_vg),
        })
    }

    pub fn selected_vg(&self) -> Option<f64> {
        self.session.selected_vg()
    }

    /// Render-ready RESULTS card state.
    pub fn results_card(&self) -> TlmResultsCard {
        TlmResultsCard {
            active_tab: self.results_tab,
            has_result: self.session.result().is_some(),
            has_sweep: self.session.sweep().is_some(),
        }
    }

    pub fn results_tab(&self) -> TlmTab {
        self.results_tab
    }

    pub fn set_results_tab(&mut self, tab: TlmTab) {
        self.results_tab = tab;
    }

    pub fn result_csv_bytes(&self) -> Option<Vec<u8>> {
        self.session.result_csv_bytes()
    }

    pub fn sweep_csv_bytes(&self) -> Option<Vec<u8>> {
        self.session.sweep_csv_bytes()
    }

    /// Render-ready TLM FILES card state.
    pub fn files_card(&self) -> Option<TlmFilesCard> {
        let result = self.session.result()?;
        Some(TlmFilesCard {
            status_count: result.statuses.len(),
            error_count: error_count(result),
        })
    }

    /// The `GroupAnalysis` for the selected group at the current V_G, if any.
    pub fn selected_group_analysis(&self) -> Option<&GroupAnalysis> {
        self.session.selected_group_analysis()
    }

    /// Install a worker-analyzed load. A successful load clears the error rows
    /// and lands on the Results tab; failed workbooks are surfaced by the
    /// always-visible FILES card, so no tab switch is needed.
    pub fn install_analyzed(&mut self, analyzed: AnalyzedDataset) {
        self.session.install(analyzed);
        self.load_errors.clear();
        self.results_tab = TlmTab::Results;
        self.sync_rows();
    }

    /// Drop the loaded dataset, its analyses, and the error rows. The
    /// committed fallback V_D survives.
    pub fn clear(&mut self) {
        self.session.clear();
        self.load_errors.clear();
        self.results_tab = TlmTab::Results;
        self.sync_rows();
    }

    /// Re-analyze at `requested_vg` (snapped to the nearest measured V_G).
    pub fn recompute_at_vg(&mut self, requested_vg: f64) {
        self.session.recompute_at_vg(requested_vg);
        self.sync_rows();
    }

    /// Remove one file by its FILES-table relative path and re-analyze the
    /// remainder. Removing the last curve clears the page like [`Self::clear`].
    /// Returns the number of status rows removed; zero means nothing matched.
    pub fn remove_file(&mut self, file: &str) -> usize {
        let removed = self.session.remove_workbook(file);
        if removed > 0 && !self.session.has_dataset() {
            self.load_errors.clear();
            self.results_tab = TlmTab::Results;
        }
        self.sync_rows();
        removed
    }

    /// Select an analyzed process group by name. Unknown groups are ignored.
    pub fn select_group(&mut self, name: &str) -> bool {
        self.session.select_group(name)
    }

    /// Rebuild the pre-formatted rows when the session's analyses changed.
    fn sync_rows(&mut self) {
        let generation = self.session.generation();
        if generation == self.rows_generation {
            return;
        }
        self.rows = match (self.session.result(), self.session.sweep()) {
            (Some(result), Some(sweep)) => TlmRows::from_analyses(result, sweep),
            _ => TlmRows::default(),
        };
        self.rows_generation = generation;
    }
}
