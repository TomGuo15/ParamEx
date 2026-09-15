use crate::common;

use egui_kittest::kittest::Queryable;
use paramex_core::tlm::AnalyzedDataset;
use paramex_core::tlm::{analyze_dataset, analyze_sweep, TlmAnalysisResult, TlmSweepResult};
use paramex_core::tlm::{result_csv, sweep_csv};
use paramex_gui::table_kit;
use paramex_gui::workspaces::tlm::state::TlmState;

fn assert_rows_match_analysis_shape(
    tlm: &TlmState,
    result: &TlmAnalysisResult,
    sweep: &TlmSweepResult,
) {
    let total_points: usize = result.groups.iter().map(|g| g.points.len()).sum();

    assert_eq!(tlm.rows().results().len(), result.groups.len());
    assert_eq!(tlm.rows().sweep().len(), sweep.points.len());
    assert_eq!(tlm.rows().lengths().len(), total_points);
    assert_eq!(tlm.rows().status().len(), result.statuses.len());
}

#[test]
fn result_rows_one_per_group() {
    let ds = common::load_tlm_corpus();
    let result = analyze_dataset(&ds, None);
    let mut tlm = TlmState::default();
    tlm.install_analyzed(AnalyzedDataset::analyze(ds));
    let rows = tlm.rows().results();

    assert_eq!(rows.len(), result.groups.len());
    // each row: group, R_contact, Rc/contact, slope, R², valid_lengths, warnings = 7 cells
    // (no V_G — it is constant across the table; the sweep table carries it instead)
    assert_eq!(rows[0].len(), 7);
}

#[test]
fn sweep_rows_one_per_group_and_vg() {
    let ds = common::load_tlm_corpus();
    let sweep = analyze_sweep(&ds);
    let mut tlm = TlmState::default();
    tlm.install_analyzed(AnalyzedDataset::analyze(ds));

    assert_eq!(tlm.rows().sweep().len(), sweep.points.len());
}

#[test]
fn length_rows_one_per_point() {
    let ds = common::load_tlm_corpus();
    let result = analyze_dataset(&ds, None);
    let mut tlm = TlmState::default();
    tlm.install_analyzed(AnalyzedDataset::analyze(ds));
    let total_points: usize = result.groups.iter().map(|g| g.points.len()).sum();

    assert_eq!(tlm.rows().lengths().len(), total_points);
}

#[test]
fn export_bytes_are_the_engine_bytes() {
    // The Export buttons must write the engine's CSV verbatim.
    let ds = common::load_tlm_corpus();
    let res = analyze_dataset(&ds, None);
    let swp = analyze_sweep(&ds);
    assert!(!result_csv(&res).is_empty());
    assert!(!sweep_csv(&swp).is_empty());

    let mut tlm = TlmState::default();
    tlm.install_analyzed(AnalyzedDataset::analyze(ds));
    assert_eq!(tlm.result_csv_bytes(), Some(result_csv(&res)));
    assert_eq!(tlm.sweep_csv_bytes(), Some(sweep_csv(&swp)));
}

#[test]
fn cells_use_shared_formatters_not_raw_sci() {
    let tlm = common::loaded_tlm_state();
    let all_rows: Vec<&Vec<String>> = tlm
        .rows()
        .results()
        .iter()
        .chain(tlm.rows().sweep())
        .chain(tlm.rows().lengths())
        .collect();

    for row in &all_rows {
        for cell in row.iter() {
            // fmt_eng output like "393.91k" never has a digit immediately
            // followed by 'e' + digit (raw {:.3e} leaked = "3.939e5")
            let raw_sci = cell.as_bytes().windows(3).any(|w| {
                w[0].is_ascii_digit() && w[1] == b'e' && (w[2].is_ascii_digit() || w[2] == b'-')
            });
            assert!(
                !raw_sci,
                "raw scientific notation leaked into cell {cell:?}"
            );
        }
    }

    // Verify at least one cell actually contains a 'k' SI suffix
    // (corpus has R_contact values ~393k, 110k, 29k — the 393k one definitely uses 'k').
    let has_k_suffix = all_rows
        .iter()
        .flat_map(|row| row.iter())
        .any(|cell| cell.contains('k'));
    assert!(
        has_k_suffix,
        "expected at least one 'k' SI suffix in the TLM cells (corpus has ~393k values)"
    );
}

#[test]
fn status_rows_shape_and_contract() {
    let tlm = common::loaded_tlm_state();
    let rows = tlm.rows().status();

    // Every status row: file, status + the message as a TRAILING hover payload
    // (STATUS_COLS has 2 columns; grid_table reads cell index 2 as status-cell
    // hover text instead of rendering it as a column).
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row.len(), 3, "row {i} has {} cells, expected 3", row.len());
    }

    // Cell index 1 is the status cell: engine contract is exactly "ok" | "error"
    // (paramex_core::tlm::Status::as_str). Any other value would be a contract violation.
    for (i, row) in rows.iter().enumerate() {
        let status = &row[1];
        assert!(
            status == "ok" || status == "error",
            "row {i}: status cell is {status:?}, expected \"ok\" or \"error\""
        );
    }

    // Every ERROR row must carry a non-empty hover message (the columns that used
    // to explain a failure are gone — the message is the only diagnostic left).
    for (i, row) in rows.iter().enumerate() {
        if row[1] == "error" {
            assert!(
                !row[2].is_empty(),
                "row {i}: error row has an empty hover message"
            );
        }
    }
}

// ── Reducer row-cache guards: the render path reads `tlm.rows` and never builds
// rows, so a reducer that forgets to rebuild would show STALE tables. These pin
// each reducer's rebuild against the builders run on the same analysis. ──────

#[test]
fn install_analyzed_populates_all_four_row_sets() {
    let ds = common::load_tlm_corpus();
    let result = analyze_dataset(&ds, None);
    let sweep = analyze_sweep(&ds);
    let mut tlm = TlmState::default();
    let gen_before = tlm.rows_generation();
    tlm.install_analyzed(AnalyzedDataset::analyze(ds));

    assert_rows_match_analysis_shape(&tlm, &result, &sweep);
    assert_ne!(
        tlm.rows_generation(),
        gen_before,
        "install must bump the rows generation (the measurement-cache key)"
    );
}

#[test]
fn recompute_at_vg_rebuilds_result_rows_but_not_sweep_rows() {
    let ds = common::load_tlm_corpus();
    let sweep = analyze_sweep(&ds);
    let mut tlm = TlmState::default();
    tlm.install_analyzed(AnalyzedDataset::analyze(ds.clone()));
    let gen_loaded = tlm.rows_generation();
    let results_before = tlm.rows().results().to_vec();
    let sweep_before = tlm.rows().sweep().to_vec();

    // Re-analyze at a DIFFERENT measured V_G than the engine default.
    let picker = tlm.vg_picker().expect("loaded V_G picker");
    let current = picker.selected_vg;
    let other = picker
        .vg_values
        .iter()
        .copied()
        .find(|v| (v - current).abs() > 1e-12)
        .expect("corpus has at least two measured V_G values");
    tlm.recompute_at_vg(other);

    assert_eq!(
        tlm.selected_vg(),
        Some(other),
        "recompute landed on the new V_G"
    );
    // The result-derived row sets mirror the NEW analysis shape...
    let result = analyze_dataset(&ds, Some(other));
    assert_rows_match_analysis_shape(&tlm, &result, &sweep);
    assert_ne!(
        tlm.rows().results(),
        results_before.as_slice(),
        "fits at a different V_G must produce different result rows"
    );
    // ...while the sweep rows are V_G-invariant and stay byte-identical.
    assert_eq!(
        tlm.rows().sweep(),
        sweep_before.as_slice(),
        "sweep rows must survive a V_G recompute untouched"
    );
    assert_ne!(
        tlm.rows_generation(),
        gen_loaded,
        "recompute must bump the rows generation"
    );
}

#[test]
fn clear_empties_the_row_cache() {
    let mut tlm = common::loaded_tlm_state();
    assert!(!tlm.rows().results().is_empty(), "load produced rows");
    let gen_loaded = tlm.rows_generation();

    tlm.clear();
    assert!(tlm.rows().results().is_empty());
    assert!(tlm.rows().sweep().is_empty());
    assert!(tlm.rows().lengths().is_empty());
    assert!(tlm.rows().status().is_empty());
    assert_ne!(
        tlm.rows_generation(),
        gen_loaded,
        "clear must bump the rows generation"
    );
}

#[test]
fn table_min_widths_exceed_naive_64px_columns() {
    use paramex_gui::workspaces::tlm::panels::columns::LENGTH_COLS;
    let min: f32 = LENGTH_COLS.iter().map(|c| c.min_w).sum();
    assert!(
        min > 500.0,
        "length table must declare enough width to force h-scroll"
    );
}

#[test]
fn sweep_warnings_keep_fixed_rows_and_full_diagnostics() {
    let tlm = common::loaded_tlm_state();
    let warning = tlm
        .rows()
        .sweep()
        .iter()
        .find_map(|row| row.get(7).filter(|text| !text.is_empty()))
        .expect("fixture should contain a sweep warning")
        .clone();
    let accessible_warning = format!("Warning: {warning}");
    let mut app = common::empty_workspace_app(paramex_gui::state::Workspace::Tlm);
    app.set_tlm_state(tlm);
    let mut harness = common::app_harness(app);

    harness.get_by_label("Fits vs VG").click_accesskit();
    harness.run();
    harness.run();

    let header = harness.get_by_label("group").rect();
    let table_row = |label: &str| {
        harness
            .get_all_by_label(label)
            .map(|node| node.rect())
            .find(|rect| rect.left() >= header.left() - 2.0 && rect.top() > header.bottom())
            .unwrap_or_else(|| panic!("the compact sweep should expose {label}"))
    };
    let process_a = table_row("process_a");
    let process_b = table_row("process_b");
    assert!(
        process_b.center().y > process_a.center().y,
        "fixture should keep process_a and process_b adjacent in that order: a={process_a:?}, b={process_b:?}"
    );
    common::assert_same_raster_edge(
        "TLM sweep row spacing",
        process_b.center().y - process_a.center().y,
        table_kit::ROW_H + harness.ctx.global_style().spacing.item_spacing.y,
        harness.ctx.pixels_per_point(),
    );

    let warning_badge = harness
        .get_all_by_label(&accessible_warning)
        .find(|node| node.rect().left() >= header.left())
        .expect("the warning badge should expose its full accessible diagnostic");
    assert!(
        harness.query_by_label(&warning).is_none(),
        "the full warning should stay out of the dense table until hover"
    );
    harness.hover_at(warning_badge.rect().center());
    harness.run();
    harness.run();
    harness.get_by_label(&warning);
}
