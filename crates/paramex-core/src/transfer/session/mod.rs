//! In-memory session: files, selection, settings, and the state transitions
//! that act on them. `Session` is the single owner of session state and exposes
//! projections plus commands for its callers.

mod file_set;

#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use indexmap::IndexMap;

use self::file_set::{curve_loaded, source_path_loaded};
use crate::shared::{normalized_file_stem, same_named_source};
use crate::transfer::extract::extract_metrics;
use crate::transfer::file_name::output_match_key;
use crate::transfer::output::{extract_output_summary, OutputDataset, OutputSummary};
use crate::transfer::parse::validate_curve_integrity;
use crate::transfer::report::csv::export_results_bytes;
use crate::transfer::report::output::{
    export_output_report_bytes, project_output_report_rows, OutputReportRow,
};
use crate::transfer::report::{project_results_table, ResultsTableProjection};
use crate::transfer::types::{
    validate_cox_nf_per_cm2, CoxError, DeviceGeometry, ExpertRanges, ExtractionSettings,
    GeometryError, GeometrySource, MetricResult, ParsedCurve,
};
use crate::transfer::{axis_bounds, clamp_window_to_axis};

/// One loaded transfer-curve file. Plot caches derived from the curve belong
/// to the GUI layer, not here.
#[derive(Debug, Clone, PartialEq)]
struct LoadedFile {
    curve: ParsedCurve,
    geometry: DeviceGeometry,
    expert_ranges: ExpertRanges,
    result: MetricResult,
    output: Option<OutputAttachment>,
    is_checked: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct OutputAttachment {
    dataset: OutputDataset,
    fit_range: Option<(f64, f64)>,
}

impl LoadedFile {
    fn new(curve: ParsedCurve, settings: &ExtractionSettings) -> Self {
        let geometry = DeviceGeometry::default();
        let expert_ranges = ExpertRanges::default();
        let result = extract_metrics(&curve, settings, &expert_ranges, &geometry);
        LoadedFile {
            curve,
            geometry,
            expert_ranges,
            result,
            output: None,
            is_checked: false,
        }
    }
}

/// Extract metrics for `item` with `settings`. Extraction is total.
fn extract_into(item: &mut LoadedFile, settings: &ExtractionSettings) {
    item.result = extract_metrics(&item.curve, settings, &item.expert_ranges, &item.geometry);
}

fn dimensions_are_valid(width_um: f64, length_um: f64) -> bool {
    width_um.is_finite() && length_um.is_finite() && width_um > 0.0 && length_um > 0.0
}

/// In-memory session: ordered files, the active selection, and shared settings.
/// Single owner of all session state. `IndexMap` preserves file insertion
/// order.
#[derive(Debug, Clone, Default)]
pub struct Session {
    files: IndexMap<String, LoadedFile>,
    selected: Option<String>,
    settings: ExtractionSettings,
    /// Monotonic display-cache key: bumped whenever [`Session::results_table`]
    /// or [`Session::output_report_rows`] may have changed, so a renderer can
    /// cache derived rows behind it. Over-invalidation (a recompute whose
    /// inputs didn't change) is deliberate and harmless — the cache just
    /// rebuilds once. Direct writes to `selected` / `is_checked` don't affect
    /// result rows, so they don't bump.
    generation: u64,
}

/// Result of automatically matching an output-curve file to a loaded transfer file.
///
/// The command is lossless: an unattached input is returned to the caller, and
/// replacing a different source returns the displaced dataset for recovery.
#[must_use = "unmatched or displaced output data must be handled"]
#[derive(Debug, Clone, PartialEq)]
pub enum AttachOutputOutcome {
    /// Exactly one transfer file matched and now owns the output.
    Attached {
        /// The matched transfer file.
        file_id: String,
        /// A previously attached dataset from a different source, if any.
        displaced: Option<OutputDataset>,
    },
    /// No loaded transfer file matches the output name.
    NoMatch {
        /// The unattached input, returned to the caller.
        output: OutputDataset,
    },
    /// More than one loaded transfer file matches the output name.
    Ambiguous {
        /// The unattached input, returned to the caller.
        output: OutputDataset,
    },
}

/// One user-pinned extraction window field on a loaded file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpertWindow {
    /// Forward (primary) V_TH window.
    FwdVt,
    /// Forward (primary) SS window.
    FwdSs,
    /// Backward V_TH window.
    BwdVt,
    /// Backward SS window.
    BwdSs,
}

/// Display-ready geometry for one loaded file, in session insertion order.
#[derive(Debug, Clone, PartialEq)]
pub struct FileGeometryRow {
    /// Opaque loaded-file id.
    pub file_id: String,
    /// Display name of the curve.
    pub name: String,
    /// Channel width in µm.
    pub width_um: f64,
    /// Channel length in µm.
    pub length_um: f64,
    /// Where the geometry came from.
    pub source: GeometrySource,
}

/// Display-ready file-list row for one loaded file.
#[derive(Debug, Clone, PartialEq)]
pub struct FileListRow {
    /// Opaque loaded-file id.
    pub file_id: String,
    /// Display name of the curve.
    pub name: String,
    /// Number of parsed samples.
    pub point_count: usize,
    /// Whether the file is checked for bulk actions.
    pub is_checked: bool,
    /// Whether the file is the current selection.
    pub is_selected: bool,
    /// Whether any extraction window is user-pinned.
    pub manual_ranges: bool,
    /// Name of the attached output file, if any.
    pub output_name: Option<String>,
}

/// Read-only selected-metrics projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectedFileMetricsProjection<'a> {
    /// Display name of the selected curve.
    pub filename: &'a str,
    /// Its current extraction result.
    pub result: &'a MetricResult,
}

/// Selected file data needed by the fit-window selector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectedFitWindowFile<'a> {
    /// Opaque loaded-file id.
    pub file_id: &'a str,
    /// User-pinned windows.
    pub expert_ranges: ExpertRanges,
    /// Whether the curve has a usable backward branch.
    pub has_backward_sweep: bool,
    /// Effective primary V_TH window in V.
    pub vt_window: Option<(f64, f64)>,
    /// Effective primary SS window in V.
    pub ss_window: Option<(f64, f64)>,
    /// Effective backward V_TH window in V.
    pub vt_window_bwd: Option<(f64, f64)>,
    /// Effective backward SS window in V.
    pub ss_window_bwd: Option<(f64, f64)>,
    /// Gate voltage samples in V.
    pub vg: &'a [f64],
    /// Absolute drain current samples in A.
    pub id_abs: &'a [f64],
}

/// Selected file data needed by the Transfer/output plot.
///
/// `selected_fit_range` is the optional user selection. `summary` is computed
/// from the attached output with that exact selection; without a selection its
/// `fit_range` records the automatic range.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectedOutputFile<'a> {
    /// Opaque loaded-file id.
    pub file_id: &'a str,
    /// Display name of the selected transfer curve.
    pub filename: &'a str,
    /// Transfer gate voltage samples in V.
    pub transfer_vg: &'a [f64],
    /// Transfer absolute drain current samples in A.
    pub transfer_id_abs: &'a [f64],
    /// Attached output dataset, if any.
    pub output: Option<&'a OutputDataset>,
    /// User-selected V_D fit range in V.
    pub selected_fit_range: Option<(f64, f64)>,
    /// Family fit of the attached output under `selected_fit_range`.
    pub summary: Option<OutputSummary>,
}

impl Session {
    /// New empty session with default settings.
    pub fn new() -> Self {
        Session::default()
    }

    /// True when at least one file is loaded.
    pub fn has_files(&self) -> bool {
        !self.files.is_empty()
    }

    /// Current committed Cox value in nF/cm2.
    pub fn cox_nf_per_cm2(&self) -> f64 {
        self.settings.cox_nf_per_cm2
    }

    /// Number of loaded files.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Loaded file ids in insertion order.
    pub fn file_ids(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// True when a loaded file id is still present.
    pub fn has_file(&self, id: &str) -> bool {
        self.files.contains_key(id)
    }

    /// Per-file geometry rows in insertion order.
    pub fn file_geometry_rows(&self) -> Vec<FileGeometryRow> {
        self.files
            .iter()
            .map(|(id, file)| FileGeometryRow {
                file_id: id.clone(),
                name: file.curve.name.clone(),
                width_um: file.geometry.width_um,
                length_um: file.geometry.length_um,
                source: file.geometry.source.clone(),
            })
            .collect()
    }

    /// Display-ready file-list row by id.
    pub fn file_list_row(&self, id: &str) -> Option<FileListRow> {
        let file = self.files.get(id)?;
        let ranges = &file.expert_ranges;
        Some(FileListRow {
            file_id: id.to_string(),
            name: file.curve.name.clone(),
            point_count: file.curve.vg.len(),
            is_checked: file.is_checked,
            is_selected: self.is_selected_file(id),
            manual_ranges: ranges.vt_range.is_some()
                || ranges.ss_range.is_some()
                || ranges.vt_range_bwd.is_some()
                || ranges.ss_range_bwd.is_some(),
            output_name: file
                .output
                .as_ref()
                .map(|attachment| attachment.dataset.name.clone()),
        })
    }

    /// Selected filename and total metric result as one coherent projection.
    pub fn selected_file_metrics_projection(&self) -> Option<SelectedFileMetricsProjection<'_>> {
        let file = self.selected_file()?;
        Some(SelectedFileMetricsProjection {
            filename: &file.curve.name,
            result: &file.result,
        })
    }

    /// Selected-file input for the fit-window selector.
    pub fn selected_fit_window_file(&self) -> Option<SelectedFitWindowFile<'_>> {
        let (id, file) = self.selected_file_entry()?;
        Some(SelectedFitWindowFile {
            file_id: id,
            expert_ranges: file.expert_ranges,
            has_backward_sweep: file.result.has_backward_sweep,
            vt_window: file.result.vt_window,
            ss_window: file.result.ss_window,
            vt_window_bwd: file.result.vt_window_bwd,
            ss_window_bwd: file.result.ss_window_bwd,
            vg: &file.curve.vg,
            id_abs: &file.curve.id_abs,
        })
    }

    /// Selected transfer samples plus its optional output attachment and fit.
    pub fn selected_output_file(&self) -> Option<SelectedOutputFile<'_>> {
        let (file_id, file) = self.selected_file_entry()?;
        let summary = file.output.as_ref().and_then(|attachment| {
            extract_output_summary(&attachment.dataset, attachment.fit_range)
        });
        Some(SelectedOutputFile {
            file_id,
            filename: &file.curve.name,
            transfer_vg: &file.curve.vg,
            transfer_id_abs: &file.curve.id_abs,
            output: file.output.as_ref().map(|attachment| &attachment.dataset),
            selected_fit_range: file
                .output
                .as_ref()
                .and_then(|attachment| attachment.fit_range),
            summary,
        })
    }

    /// True when any loaded curve was sourced from `path`.
    pub fn source_path_loaded(&self, path: &Path) -> bool {
        source_path_loaded(self.files.values().map(|file| &file.curve), path)
    }

    /// True when `id` is the current selected loaded file.
    fn is_selected_file(&self, id: &str) -> bool {
        self.selected.as_deref() == Some(id) && self.has_file(id)
    }

    /// True when at least one loaded file is checked.
    pub fn has_checked_files(&self) -> bool {
        self.files.values().any(|file| file.is_checked)
    }

    /// True when at least one loaded file is unchecked.
    pub fn has_unchecked_files(&self) -> bool {
        self.files.values().any(|file| !file.is_checked)
    }

    /// The currently selected file, if any.
    fn selected_file(&self) -> Option<&LoadedFile> {
        self.selected.as_ref().and_then(|id| self.files.get(id))
    }

    /// True when the current selection points at a loaded file.
    pub fn has_selected_file(&self) -> bool {
        self.selected_file().is_some()
    }

    /// The current loaded file id, if the selection is still loaded.
    pub fn active_file_id(&self) -> Option<&str> {
        let id = self.selected.as_deref()?;
        self.files.contains_key(id).then_some(id)
    }

    /// The currently selected file id and file, if the selection is still
    /// loaded.
    fn selected_file_entry(&self) -> Option<(&str, &LoadedFile)> {
        let id = self.selected.as_deref()?;
        self.files.get(id).map(|file| (id, file))
    }

    /// Select a loaded file by id. Unknown ids are ignored.
    pub fn select_file(&mut self, id: &str) -> bool {
        if !self.files.contains_key(id) {
            return false;
        }
        self.selected = Some(id.to_string());
        true
    }

    /// Set one loaded file's checked state. Unknown ids are ignored.
    pub fn set_file_checked(&mut self, id: &str, checked: bool) -> bool {
        let Some(item) = self.files.get_mut(id) else {
            return false;
        };
        item.is_checked = checked;
        true
    }

    /// Apply one manual per-file W/L edit and recompute that file. Unknown ids
    /// are ignored; invalid dimensions return the validation error unchanged.
    pub fn set_manual_geometry(
        &mut self,
        id: &str,
        width: Option<f64>,
        length: Option<f64>,
    ) -> Result<bool, GeometryError> {
        {
            let Some(item) = self.files.get_mut(id) else {
                return Ok(false);
            };
            let width_um = width.unwrap_or(item.geometry.width_um);
            let length_um = length.unwrap_or(item.geometry.length_um);
            if !dimensions_are_valid(width_um, length_um) {
                return Err(GeometryError);
            }
            item.geometry = DeviceGeometry {
                width_um,
                length_um,
                source: GeometrySource::Manual,
            };
        }
        self.recompute(id);
        Ok(true)
    }

    /// Set one user-pinned extraction window after clamping it to the file's
    /// V_G axis, then recompute that file. Unknown ids and non-finite endpoints
    /// are rejected.
    pub fn set_expert_window(
        &mut self,
        id: &str,
        which: ExpertWindow,
        window: Option<(f64, f64)>,
    ) -> bool {
        if window.is_some_and(|(lo, hi)| !lo.is_finite() || !hi.is_finite()) {
            return false;
        }
        {
            let Some(item) = self.files.get_mut(id) else {
                return false;
            };
            let clamped = clamp_window_to_axis(window, axis_bounds(&item.curve.vg));
            match which {
                ExpertWindow::FwdVt => item.expert_ranges.vt_range = clamped,
                ExpertWindow::FwdSs => item.expert_ranges.ss_range = clamped,
                ExpertWindow::BwdVt => item.expert_ranges.vt_range_bwd = clamped,
                ExpertWindow::BwdSs => item.expert_ranges.ss_range_bwd = clamped,
            }
        }
        self.recompute(id);
        true
    }

    /// Clear all user-pinned extraction windows and recompute the file once.
    /// Unknown ids are ignored.
    pub fn clear_expert_windows(&mut self, id: &str) -> bool {
        {
            let Some(item) = self.files.get_mut(id) else {
                return false;
            };
            item.expert_ranges = ExpertRanges::default();
        }
        self.recompute(id);
        true
    }

    /// Extraction results for every file, in insertion order.
    fn all_results(&self) -> Vec<&MetricResult> {
        self.files.values().map(|file| &file.result).collect()
    }

    /// Canonical typed results-table projection for application rendering.
    pub fn results_table(&self) -> ResultsTableProjection {
        let results: Vec<MetricResult> = self.all_results().into_iter().cloned().collect();
        project_results_table(&results)
    }

    /// Canonical Transfer-fit report bytes for the current session.
    pub fn report_bytes(&self) -> Vec<u8> {
        let results: Vec<MetricResult> = self.all_results().into_iter().cloned().collect();
        export_results_bytes(&results)
    }

    /// Canonical family + line output-report rows in transfer-file insertion
    /// order. Each family is followed by its line fits in ascending V_G.
    pub fn output_report_rows(&self) -> Vec<OutputReportRow> {
        self.files
            .values()
            .filter_map(|file| {
                file.output.as_ref().map(|attachment| {
                    project_output_report_rows(
                        &file.curve.name,
                        &attachment.dataset,
                        attachment.fit_range,
                    )
                })
            })
            .flatten()
            .collect()
    }

    /// Canonical output report bytes for the current session.
    pub fn output_report_bytes(&self) -> Vec<u8> {
        export_output_report_bytes(&self.output_report_rows())
    }

    /// The current display-cache generation (see the field doc): equal values
    /// across two reads guarantee result-row output is unchanged.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Run total extraction for one file and store the new result.
    fn recompute(&mut self, id: &str) {
        // Bump unconditionally: recompute can't tell whether the refreshed
        // MetricResult differs, and over-invalidation only costs one cache
        // rebuild.
        self.generation += 1;
        let settings = self.settings;
        if let Some(item) = self.files.get_mut(id) {
            extract_into(item, &settings);
        }
    }

    /// Recompute every file as one bulk invalidation.
    fn recompute_all(&mut self) {
        if self.files.is_empty() {
            return;
        }
        self.generation += 1;
        let settings = self.settings;
        for item in self.files.values_mut() {
            extract_into(item, &settings);
        }
    }

    /// Monotonic source of opaque loaded-file ids. In-memory only (never
    /// persisted), so a process-local counter is sufficient.
    fn next_id() -> String {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        NEXT_ID.fetch_add(1, Ordering::Relaxed).to_string()
    }

    /// Add a parser-valid curve unless already loaded. Mints an id, extracts
    /// immediately, appends, and selects it.
    /// Returns the new id, or `None` for invalid data or a duplicate.
    pub fn add_curve(&mut self, curve: ParsedCurve) -> Option<String> {
        if !validate_curve_integrity(&curve)
            || curve_loaded(self.files.values().map(|f| &f.curve), &curve)
        {
            return None;
        }
        let id = Self::next_id();
        let settings = self.settings;
        let item = LoadedFile::new(curve, &settings);
        self.files.insert(id.clone(), item);
        self.selected = Some(id.clone());
        // Inserted path only: the duplicate early-return above mutates nothing,
        // so it must not invalidate display caches.
        self.generation += 1;
        Some(id)
    }

    /// Attach an output-curve dataset to its unique normalized transfer-file match.
    pub fn attach_output(&mut self, output: OutputDataset) -> AttachOutputOutcome {
        let output_key = output_match_key(&output.name);
        let matches: Vec<String> = self
            .files
            .iter()
            .filter(|(_, file)| normalized_file_stem(&file.curve.name) == output_key)
            .map(|(id, _)| id.clone())
            .collect();
        match matches.as_slice() {
            [id] => {
                let id = id.clone();
                let displaced = self
                    .replace_output(&id, output)
                    .expect("a uniquely matched transfer file remains loaded");
                AttachOutputOutcome::Attached {
                    file_id: id,
                    displaced,
                }
            }
            [] => AttachOutputOutcome::NoMatch { output },
            _ => AttachOutputOutcome::Ambiguous { output },
        }
    }

    /// Attach an output-curve dataset to a specific loaded transfer file.
    ///
    /// A replacement resets the hand-tuned V_D fit range except when it comes
    /// from the same source file (a folder re-scan / repeated Load Output
    /// re-parses every output on disk). The returned dataset is therefore only
    /// a different-source displacement that a caller may need to retain; an
    /// initial attachment or same-source reload returns `Ok(None)`.
    pub fn replace_output(
        &mut self,
        file_id: &str,
        output: OutputDataset,
    ) -> Result<Option<OutputDataset>, OutputDataset> {
        let Some(file) = self.files.get_mut(file_id) else {
            return Err(output);
        };
        let same_source = file.output.as_ref().is_some_and(|previous| {
            same_named_source(
                &previous.dataset.name,
                previous.dataset.source_path.as_deref(),
                &output.name,
                output.source_path.as_deref(),
            )
        });
        let preserved_range = same_source
            .then(|| file.output.as_ref().and_then(|previous| previous.fit_range))
            .flatten();
        let displaced = file
            .output
            .replace(OutputAttachment {
                dataset: output,
                fit_range: preserved_range,
            })
            .and_then(|attachment| (!same_source).then_some(attachment.dataset));
        self.generation += 1;
        Ok(displaced)
    }

    /// Take the output-curve dataset attached to one transfer file.
    pub fn take_output(&mut self, file_id: &str) -> Option<OutputDataset> {
        let file = self.files.get_mut(file_id)?;
        let attachment = file.output.take()?;
        self.generation += 1;
        Some(attachment.dataset)
    }

    /// Set or clear the output-curve V_D fit range for a loaded transfer file.
    pub fn set_output_fit_range(&mut self, file_id: &str, range: Option<(f64, f64)>) -> bool {
        let Some(file) = self.files.get_mut(file_id) else {
            return false;
        };
        if let Some((lo, hi)) = range {
            if !lo.is_finite() || !hi.is_finite() || lo == hi {
                return false;
            }
        }
        let Some(attachment) = file.output.as_mut() else {
            return false;
        };
        attachment.fit_range = range;
        self.generation += 1;
        true
    }

    /// Remove a set of ids, preserving the order of survivors; when the
    /// selection was removed, the first survivor becomes selected. Returns the
    /// count removed.
    fn remove_file_ids(&mut self, ids: &HashSet<String>) -> usize {
        let to_remove: Vec<String> = self
            .files
            .keys()
            .filter(|k| ids.contains(*k))
            .cloned()
            .collect();
        if to_remove.is_empty() {
            return 0;
        }
        let mut removed = 0;
        for id in &to_remove {
            if self.files.shift_remove(id).is_some() {
                removed += 1;
            }
        }
        if self
            .selected
            .as_ref()
            .is_none_or(|selected| !self.files.contains_key(selected))
        {
            self.selected = self.files.keys().next().cloned();
        }
        // Only reached with a non-empty intersection (the early return above
        // covers removed == 0), so files actually left `all_results()`.
        self.generation += 1;
        removed
    }

    /// Remove checked files, or the selected file when no files are checked.
    pub fn remove_selected_or_checked(&mut self) -> usize {
        let mut ids: HashSet<String> = self
            .files
            .iter()
            .filter(|(_, file)| file.is_checked)
            .map(|(id, _)| id.clone())
            .collect();
        if ids.is_empty() {
            if let Some(selected) = &self.selected {
                ids.insert(selected.clone());
            }
        }
        self.remove_file_ids(&ids)
    }

    /// Keep checked files by removing unchecked files. Returns `None` when no
    /// file is checked, so callers can keep the "nothing to keep" UI distinct
    /// from a zero-removal no-op.
    pub fn keep_checked_files(&mut self) -> Option<usize> {
        if !self.has_checked_files() {
            return None;
        }
        let ids = self
            .files
            .iter()
            .filter(|(_, file)| !file.is_checked)
            .map(|(id, _)| id.clone())
            .collect();
        Some(self.remove_file_ids(&ids))
    }

    /// Remove every loaded file.
    pub fn clear_files(&mut self) -> usize {
        let ids: HashSet<String> = self.files.keys().cloned().collect();
        self.remove_file_ids(&ids)
    }

    /// Apply a new Cox value into settings and recompute all. An invalid value
    /// returns `Err` with nothing mutated and no recompute.
    pub fn set_cox(&mut self, cox_nf_per_cm2: f64) -> Result<(), CoxError> {
        let cox_nf_per_cm2 = validate_cox_nf_per_cm2(cox_nf_per_cm2)?;
        self.settings = ExtractionSettings { cox_nf_per_cm2 };
        self.recompute_all();
        Ok(())
    }

    /// Apply one global W·L to every file and recompute. Like `set_cox`, a
    /// non-positive or non-finite dimension returns `Err` with nothing mutated
    /// and no recompute; on success every file gets `GeometrySource::Global`, is
    /// recomputed, and the updated count is returned.
    pub fn set_global_wl(&mut self, width_um: f64, length_um: f64) -> Result<usize, GeometryError> {
        if !dimensions_are_valid(width_um, length_um) {
            return Err(GeometryError);
        }
        let count = self.files.len();
        for file in self.files.values_mut() {
            file.geometry = DeviceGeometry {
                width_um,
                length_um,
                source: GeometrySource::Global,
            };
        }
        self.recompute_all();
        Ok(count)
    }
}
