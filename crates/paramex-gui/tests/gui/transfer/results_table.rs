use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use paramex_core::transfer::{
    AttachOutputOutcome, OutputCurve, OutputDataset, ParsedCurve, ResultsTableColumn, Session,
};
use paramex_gui::workspaces::transfer::panels::results_table;
use paramex_gui::workspaces::transfer::state::{TransferResultsView, TransferUiState};
use paramex_gui::workspaces::transfer::TransferWorkspace;

use crate::{attach_output, partial_output_dataset, transfer_curve as curve};

fn output_dataset(name: &str, scale: f64) -> OutputDataset {
    OutputDataset {
        name: name.to_string(),
        curves: vec![OutputCurve {
            vg: 5.0,
            vd: vec![0.0, 1.0, 2.0, 3.0],
            id: vec![0.0, scale * 1.0e-6, scale * 1.7e-6, scale * 2.5e-6],
        }],
        source_path: Some(std::path::PathBuf::from(name)),
    }
}

fn scattered_output_dataset(name: &str) -> OutputDataset {
    OutputDataset {
        name: name.to_string(),
        curves: vec![
            OutputCurve {
                vg: 1.0,
                vd: vec![0.0, 1.0, 2.0, 3.0],
                id: vec![0.0, 1.0e-6, 2.0e-6, 3.0e-6],
            },
            OutputCurve {
                vg: 2.0,
                vd: vec![0.0, 1.0, 2.0, 3.0],
                id: vec![99.0e-6, 100.0e-6, 101.0e-6, 102.0e-6],
            },
        ],
        source_path: Some(std::path::PathBuf::from(name)),
    }
}

#[test]
fn results_table_keeps_primary_metrics_inside_the_center_column() {
    let width = paramex_gui::workspaces::transfer::panels::results_table::table_min_width();
    assert!(
        width <= 590.0,
        "GUI results summary should fit its primary metrics in the center card, got {width}"
    );
}

#[test]
fn sweep_column_uses_compact_gui_header_and_width_floor() {
    use paramex_gui::workspaces::transfer::panels::results_table::{
        col_min_width, gui_column_specs, gui_header_label_html,
    };

    let specs = gui_column_specs();
    let sweep = specs
        .iter()
        .find(|spec| spec.column == ResultsTableColumn::Sweep)
        .expect("GUI table keeps a sweep direction column");

    assert_eq!(gui_header_label_html(sweep), "Dir");
    assert!(
        col_min_width(sweep) <= 36.0,
        "F/B/S display is pointless if the GUI sweep column keeps a wide floor"
    );
}

#[test]
fn gui_results_order_prioritizes_scientific_metrics_without_changing_core_schema() {
    let keys: Vec<_> = paramex_gui::workspaces::transfer::panels::results_table::gui_column_specs()
        .iter()
        .map(|spec| spec.column.key())
        .collect();

    let expected = [
        "filename",
        "sweep",
        "Vth",
        "mu_sat",
        "SS_mV_dec",
        "Ion",
        "Ioff",
        "Ion_Ioff",
    ];
    assert_eq!(keys, expected);

    let core_keys: Vec<_> = ResultsTableColumn::ALL
        .iter()
        .map(|column| column.key())
        .collect();
    assert_ne!(
        keys, core_keys,
        "GUI order should be independent from core CSV order"
    );
}

#[test]
fn ratio_column_reserves_the_complete_overall_log_label() {
    use paramex_gui::workspaces::transfer::panels::results_table::{
        col_min_width, gui_column_specs,
    };

    let ratio = gui_column_specs()
        .iter()
        .find(|spec| spec.column == ResultsTableColumn::OnOffRatio)
        .expect("GUI table keeps the ratio column");

    assert!(
        col_min_width(ratio) >= 68.0,
        "the right-aligned log10 Overall value must not cross the cell's left clip edge"
    );
}

#[test]
fn report_bytes_keep_the_canonical_csv_contract() {
    let mut session = Session::new();
    session.add_curve(curve("alpha.csv"));
    session.add_curve(curve("beta.csv"));

    let report = session.report_bytes();
    assert_eq!(&report[..3], &[0xEF, 0xBB, 0xBF]);
    let csv = String::from_utf8(report).unwrap();
    assert!(csv.contains("Forward Results"));
    assert!(csv.contains("alpha.csv"));
    assert!(csv.contains("beta.csv"));
}

#[test]
fn report_bytes_empty_session_is_empty() {
    let session = Session::new();
    assert!(session.report_bytes().is_empty());
}

#[test]
fn output_report_bytes_exports_family_and_line_fits() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));
    assert!(attach_output(&mut session, output_dataset("device_a_output.csv", 1.0)).is_none());

    let bytes = session.output_report_bytes();
    let csv = String::from_utf8(bytes).unwrap();
    assert!(csv.starts_with('\u{feff}'));
    assert!(csv.contains(
        "device,output_file,fit,status,Vg,Idsat,gds,ro,Early voltage,lambda,Vds fit min,Vds fit max,R2"
    ));
    assert!(csv.contains("device_a.csv,device_a_output.csv,Family,ok"));
    assert!(csv.contains("device_a.csv,device_a_output.csv,Line,ok,5.000000e0"));
}

#[test]
fn output_report_bytes_blanks_family_early_voltage_when_line_fits_disagree() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));
    assert!(attach_output(
        &mut session,
        scattered_output_dataset("device_a_output.csv")
    )
    .is_none());

    let csv = String::from_utf8(session.output_report_bytes()).unwrap();
    let family = csv
        .lines()
        .find(|line| line.contains(",Family,"))
        .expect("family row");
    let fields: Vec<_> = family.trim_start_matches('\u{feff}').split(',').collect();
    assert_eq!(fields[8], "");
    assert_eq!(fields[9], "");
    assert!(csv.contains("device_a.csv,device_a_output.csv,Line,ok,1.000000e0"));
    assert!(csv.contains("device_a.csv,device_a_output.csv,Line,ok,2.000000e0"));
}

#[test]
fn output_report_bytes_empty_session_is_empty() {
    let session = Session::new();
    assert!(session.output_report_bytes().is_empty());
}

#[test]
fn transfer_results_view_defaults_to_transfer_and_indexes() {
    let mut ui = TransferUiState::default();

    assert_eq!(ui.results_view(), TransferResultsView::Transfer);
    assert_eq!(TransferResultsView::Transfer.index(), 0);
    assert_eq!(TransferResultsView::Output.index(), 1);
    assert_eq!(
        TransferResultsView::from_index(99),
        TransferResultsView::Transfer
    );

    ui.set_results_view(TransferResultsView::Output);
    assert_eq!(ui.results_view(), TransferResultsView::Output);
}

struct ResultsHarnessApp {
    workspace: TransferWorkspace,
}

impl ResultsHarnessApp {
    fn new(session: Session) -> Self {
        Self {
            workspace: TransferWorkspace::from_session(session),
        }
    }

    fn with_view(mut self, view: TransferResultsView) -> Self {
        self.workspace.set_results_view(view);
        self
    }
}

impl eframe::App for ResultsHarnessApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.allocate_ui(egui::Vec2::new(564.0, 220.0), |ui| {
            let ctx = ui.ctx().clone();
            results_table::show(ui, &ctx, &mut self.workspace);
        });
    }
}

#[test]
fn empty_results_export_action_and_tabs_are_disabled() {
    let state = ResultsHarnessApp::new(Session::new());

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    assert!(
        harness
            .get_by_label("Export CSV")
            .accesskit_node()
            .is_disabled(),
        "empty-state results export should stay visible but disabled"
    );
    assert!(
        harness
            .get_by_label("Transfer Fit")
            .accesskit_node()
            .is_disabled(),
        "empty results card should keep disabled result-view tabs"
    );
    assert!(
        harness
            .get_by_label("Output Fit")
            .accesskit_node()
            .is_disabled(),
        "empty results card should keep disabled result-view tabs"
    );
    harness.get_by_label("No transfer fit rows.");
}

#[test]
fn loaded_results_export_action_is_enabled_when_idle() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));
    let state = ResultsHarnessApp::new(session);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    assert!(
        !harness
            .get_by_label("Export CSV")
            .accesskit_node()
            .is_disabled(),
        "loaded idle Transfer results should keep Export CSV enabled"
    );
}

#[test]
fn loaded_results_use_the_shared_engineering_ratio() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv")).unwrap();
    let result = session
        .selected_file_metrics_projection()
        .expect("selected result")
        .result;
    let expected = paramex_gui::format_ui::fmt_ratio(result.on_off_ratio_forward);
    assert!(!expected.contains("× 10"));
    let state = ResultsHarnessApp::new(session);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label(&expected);
}

#[test]
fn results_view_tabs_share_the_title_row() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));
    assert!(attach_output(&mut session, output_dataset("device_a_output.csv", 1.0)).is_none());
    let state = ResultsHarnessApp::new(session);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let title = harness.get_by_label("RESULTS").rect();
    let transfer = harness.get_by_label("Transfer Fit").rect();
    let output = harness.get_by_label("Output Fit").rect();
    let export = harness.get_by_label("Export CSV").rect();
    let title_and_tabs_share_row =
        title.top() <= transfer.bottom() && title.bottom() >= transfer.top();
    let tabs_and_export_share_row =
        transfer.top() <= export.bottom() && transfer.bottom() >= export.top();

    crate::common::assert_raster_centers_aligned(
        "Transfer and Output result-tab baseline",
        transfer.center().y,
        output.center().y,
        harness.ctx.pixels_per_point(),
    );
    assert!(
        title_and_tabs_share_row
            && tabs_and_export_share_row
            && transfer.left() > title.right()
            && export.left() > output.right(),
        "result view tabs should sit beside RESULTS like the app workspace switcher, with export still pinned right: title={title:?}, transfer={transfer:?}, output={output:?}, export={export:?}"
    );
}

#[test]
fn transfer_and_output_tables_start_on_the_same_raster_row() {
    for (pixels_per_point, expected_gap) in crate::common::RASTER_TEST_SCALES
        .into_iter()
        .zip([13, 17, 21, 23, 27])
    {
        let mut session = Session::new();
        session.add_curve(curve("device_a.csv"));
        assert!(attach_output(&mut session, output_dataset("device_a_output.csv", 1.0)).is_none());

        let mut transfer = Harness::builder()
            .with_size(egui::Vec2::new(590.0, 250.0))
            .with_pixels_per_point(pixels_per_point)
            .build_eframe(|cc| {
                paramex_gui::theme::install(&cc.egui_ctx);
                ResultsHarnessApp::new(session.clone())
            });
        transfer.run();
        transfer.run();

        let mut output = Harness::builder()
            .with_size(egui::Vec2::new(590.0, 250.0))
            .with_pixels_per_point(pixels_per_point)
            .build_eframe(|cc| {
                paramex_gui::theme::install(&cc.egui_ctx);
                ResultsHarnessApp::new(session).with_view(TransferResultsView::Output)
            });
        output.run();
        output.run();

        let transfer_title = transfer.get_by_label("RESULTS").rect();
        let transfer_header = transfer.get_by_label("File").rect();
        let output_title = output.get_by_label("RESULTS").rect();
        let output_header = output.get_by_label("Device").rect();

        crate::common::assert_same_raster_edge(
            &format!("Transfer/Output table-header top at {pixels_per_point} ppp"),
            transfer_header.top(),
            output_header.top(),
            pixels_per_point,
        );
        for (view, title, header) in [
            ("Transfer", transfer_title, transfer_header),
            ("Output", output_title, output_header),
        ] {
            let gap = crate::common::raster_pixel(header.top())
                - crate::common::raster_pixel(title.bottom());
            assert_eq!(
                gap, expected_gap,
                "{view} Results title-to-table gap changed at {pixels_per_point} ppp"
            );
        }
    }
}

#[test]
fn empty_output_results_export_action_is_disabled() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));
    let state = ResultsHarnessApp::new(session).with_view(TransferResultsView::Output);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    assert!(
        harness
            .get_by_label("Export CSV")
            .accesskit_node()
            .is_disabled(),
        "output export should stay visible but disabled when the active output view has no rows"
    );
    assert!(
        harness
            .query_by_label("Load matching output data to see output results.")
            .is_none(),
        "output results should stay quiet until output rows exist"
    );
    harness.get_by_label("No output fit rows.");
}

#[test]
fn output_view_renders_output_headers_and_rows() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));
    assert!(attach_output(&mut session, output_dataset("device_a_output.csv", 1.0)).is_none());
    let state = ResultsHarnessApp::new(session).with_view(TransferResultsView::Output);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();
    harness.run();

    for label in [
        "Device",
        "Output file",
        "Fit",
        "Family",
        "Line",
        "ID,sat (A)",
        "gds (S)",
        "ro (Ω)",
        "VD0 / VA (V)",
        "λ (V-1)",
        "VG (V)",
        "R2",
        "device_a.csv",
        "device_a_output.csv",
    ] {
        harness.get_by_label(label);
    }
}

#[test]
fn output_view_keeps_partial_family_and_failed_line_visible() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));
    assert!(attach_output(&mut session, partial_output_dataset("device_a_output.csv")).is_none());
    let state = ResultsHarnessApp::new(session).with_view(TransferResultsView::Output);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();
    harness.run();

    harness.get_by_label("Partial");
    harness.get_by_label("Line");
    harness.get_by_label("Failed");
    assert!(
        !harness
            .get_by_label("Export CSV")
            .accesskit_node()
            .is_disabled(),
        "partial output results must remain exportable"
    );
}

#[test]
fn output_group_labels_center_across_family_and_line_rows() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));
    assert!(attach_output(&mut session, output_dataset("device_a_output.csv", 1.0)).is_none());
    let state = ResultsHarnessApp::new(session).with_view(TransferResultsView::Output);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();
    harness.run();

    let device = harness.get_by_label("device_a.csv").rect();
    let output = harness.get_by_label("device_a_output.csv").rect();
    let family = harness.get_by_label("Family").rect();
    let line = harness.get_by_label("Line").rect();
    let span_mid = (family.center().y + line.center().y) / 2.0;

    crate::common::assert_raster_centers_aligned(
        "device label across Output Family/Line rows",
        device.center().y,
        span_mid,
        harness.ctx.pixels_per_point(),
    );
    crate::common::assert_raster_centers_aligned(
        "output label across Output Family/Line rows",
        output.center().y,
        span_mid,
        harness.ctx.pixels_per_point(),
    );
}

#[test]
fn output_table_cache_follows_output_attach_replacement_and_removal_between_frames() {
    let mut session = Session::new();
    let alpha_id = session.add_curve(curve("alpha.csv")).expect("alpha added");
    assert!(attach_output(&mut session, output_dataset("alpha_output.csv", 1.0)).is_none());

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            ResultsHarnessApp::new(session).with_view(TransferResultsView::Output)
        });
    harness.run();
    harness.run();
    harness.get_by_label("alpha_output.csv");

    assert!(
        matches!(
            harness
                .state_mut()
                .workspace
                .attach_output(output_dataset("alpha_id-vd.csv", 2.0)),
            AttachOutputOutcome::Attached {
                displaced: Some(_),
                ..
            }
        ),
        "different-source replacement should surface the displaced fixture"
    );
    harness.run();
    harness.run();
    assert!(
        harness.query_by_label("alpha_output.csv").is_none(),
        "replaced output rows must leave the rendered output table"
    );
    harness.get_by_label("alpha_id-vd.csv");

    let workspace = &mut harness.state_mut().workspace;
    assert!(workspace.select_file(&alpha_id));
    assert_eq!(workspace.remove_selected_or_checked(), 1);
    harness.run();
    harness.run();
    assert!(
        harness.query_by_label("alpha_id-vd.csv").is_none(),
        "removed transfer file must remove its rendered output row"
    );
    harness.get_by_label("RESULTS");
    assert!(
        harness
            .query_by_label("Load matching output data to see output results.")
            .is_none(),
        "empty output guidance should not render after the transfer file is gone"
    );
}

#[test]
fn segmented_control_switches_from_transfer_to_output_view() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));
    assert!(attach_output(&mut session, output_dataset("device_a_output.csv", 1.0)).is_none());

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            ResultsHarnessApp::new(session)
        });
    harness.run();
    assert_eq!(
        harness.state().workspace.results_view(),
        TransferResultsView::Transfer
    );

    harness.get_by_label("Output Fit").click();
    harness.run();
    harness.run();

    assert_eq!(
        harness.state().workspace.results_view(),
        TransferResultsView::Output
    );
    harness.get_by_label("Output file");
    harness.get_by_label("device_a_output.csv");
}

#[test]
fn metric_columns_right_align_identity_columns_left() {
    use paramex_gui::workspaces::transfer::panels::results_table::{
        col_right_aligned, gui_column_specs,
    };
    for spec in gui_column_specs() {
        let expect = !matches!(
            spec.column,
            ResultsTableColumn::Filename | ResultsTableColumn::Sweep
        );
        assert_eq!(col_right_aligned(spec), expect, "{}", spec.column.key());
    }
}

/// A double-sweep curve (vg rises then falls) so the results table gets a
/// two-row file group (F + B) with a rowspan leader.
fn double_curve(name: &str) -> ParsedCurve {
    let up: Vec<f64> = (0..30).map(|i| -2.0 + i as f64 * 0.2).collect();
    let down: Vec<f64> = up.iter().rev().copied().collect();
    let id = |vg: f64| 1e-12 * 10f64.powf((vg + 2.0) * 1.5);
    ParsedCurve {
        name: name.to_string(),
        vg: up.iter().chain(down.iter()).copied().collect(),
        id_abs: up.iter().chain(down.iter()).map(|&v| id(v)).collect(),
        source_path: Some(std::path::PathBuf::from(name)),
    }
}

#[test]
fn group_label_centers_across_its_merged_span() {
    // The file label renders once, centered across the F/B group.
    let mut session = Session::new();
    session.add_curve(double_curve("dual.csv"));

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            ResultsHarnessApp::new(session)
        });
    harness.run();
    harness.run();

    let file_label = harness.get_by_label("dual.csv").rect();
    let mut sweeps: Vec<egui::Rect> = harness
        .get_all_by_label("F")
        .chain(harness.get_all_by_label("B"))
        .map(|node| node.rect())
        .collect();
    sweeps.sort_by(|a, b| a.top().total_cmp(&b.top()));
    let (first, second) = (sweeps[0], sweeps[1]);
    let span_mid = (first.center().y + second.center().y) / 2.0;
    crate::common::assert_raster_centers_aligned(
        "group label across merged sweep span",
        file_label.center().y,
        span_mid,
        harness.ctx.pixels_per_point(),
    );
}

#[test]
fn transfer_result_column_headers_do_not_shift_when_rows_load() {
    let header_rects = |session: Session| {
        let mut harness = Harness::builder()
            .with_size(egui::Vec2::new(590.0, 250.0))
            .build_eframe(|cc| {
                paramex_gui::theme::install(&cc.egui_ctx);
                ResultsHarnessApp::new(session)
            });
        harness.run();
        harness.run();
        (
            harness.ctx.pixels_per_point(),
            ["File", "Dir", "Ion (A)", "Ioff (A)", "Ion/Ioff"]
                .map(|label| (label, harness.get_by_label(label).rect())),
        )
    };

    let (pixels_per_point, empty) = header_rects(Session::new());
    let mut loaded_session = Session::new();
    loaded_session.add_curve(double_curve("dual.csv"));
    let (loaded_pixels_per_point, loaded) = header_rects(loaded_session);
    assert_eq!(pixels_per_point, loaded_pixels_per_point);

    for ((label, empty_rect), (_, loaded_rect)) in empty.into_iter().zip(loaded) {
        crate::common::assert_same_raster_rect(
            &format!("{label} header moved between empty and loaded table"),
            empty_rect,
            loaded_rect,
            pixels_per_point,
        );
    }
}

#[test]
fn output_result_column_headers_do_not_shift_when_rows_load() {
    let header_rects = |session: Session| {
        let mut harness = Harness::builder()
            .with_size(egui::Vec2::new(590.0, 250.0))
            .build_eframe(|cc| {
                paramex_gui::theme::install(&cc.egui_ctx);
                ResultsHarnessApp::new(session).with_view(TransferResultsView::Output)
            });
        harness.run();
        harness.run();
        (
            harness.ctx.pixels_per_point(),
            [
                "Device",
                "Output file",
                "Fit",
                "VG (V)",
                "ID,sat (A)",
                "gds (S)",
                "ro (Ω)",
                "VD0 / VA (V)",
                "λ (V-1)",
                "R2",
            ]
            .map(|label| (label, harness.get_by_label(label).rect())),
        )
    };

    let mut empty_session = Session::new();
    empty_session.add_curve(curve("device_a.csv"));
    let (pixels_per_point, empty) = header_rects(empty_session);

    let mut loaded_session = Session::new();
    let id = loaded_session
        .add_curve(curve("very_long_device_name_for_width.csv"))
        .unwrap();
    assert!(loaded_session
        .replace_output(
            &id,
            output_dataset("very_long_output_file_name_for_width.csv", 1.0)
        )
        .is_ok());
    let (loaded_pixels_per_point, loaded) = header_rects(loaded_session);
    assert_eq!(pixels_per_point, loaded_pixels_per_point);

    for ((label, empty_rect), (_, loaded_rect)) in empty.into_iter().zip(loaded) {
        crate::common::assert_same_raster_rect(
            &format!("{label} output header moved between empty and loaded table"),
            empty_rect,
            loaded_rect,
            pixels_per_point,
        );
    }
}

#[test]
fn header_stays_visible_after_vertical_scroll() {
    let mut session = Session::new();
    for idx in 0..8 {
        session.add_curve(curve(&format!("device_{idx:02}.csv")));
    }

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            ResultsHarnessApp::new(session)
        });
    harness.run();
    harness.run();

    let header_before = harness.get_by_label("File").rect();

    // Scroll the body down to a late row; the header must stay pinned (sticky).
    let late_single = harness
        .get_all_by_label("S")
        .max_by(|left, right| left.rect().top().total_cmp(&right.rect().top()))
        .expect("at least one late Single row");
    late_single.scroll_to_me();
    harness.run();
    harness.run();

    let header_after = harness.get_by_label("File").rect();
    crate::common::assert_same_raster_edge(
        "sticky result-header top after vertical scroll",
        header_after.top(),
        header_before.top(),
        harness.ctx.pixels_per_point(),
    );
    assert!(
        header_after.top() >= 0.0 && header_after.bottom() <= 250.0,
        "header scrolled out of the viewport: {header_after:?}"
    );
}

#[test]
fn results_table_vertical_scroll_reaches_late_sweep_row() {
    let mut session = Session::new();
    for idx in 0..8 {
        session.add_curve(curve(&format!("device_{idx:02}.csv")));
    }

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            ResultsHarnessApp::new(session)
        });
    harness.run();
    harness.run();

    let late_single = harness
        .get_all_by_label("S")
        .max_by(|left, right| left.rect().top().total_cmp(&right.rect().top()))
        .expect("at least one late Single row");
    let before = late_single.rect();
    assert!(
        before.top() > 220.0,
        "Late Single row should start below the short results viewport before scrolling: {before:?}"
    );

    late_single.scroll_to_me();
    harness.run();
    harness.run();

    let visible_single = harness
        .get_all_by_label("S")
        .map(|node| node.rect())
        .any(|rect| rect.top() >= 0.0 && rect.bottom() <= 220.0);
    assert!(
        visible_single,
        "Late Single row was not vertically reachable inside Results card"
    );
}

#[test]
fn cached_rows_follow_workspace_mutations_between_frames() {
    // Mutating the session between frames must invalidate the results cache so
    // rendered labels follow the current row set.
    let mut session = Session::new();
    let alpha_id = session.add_curve(curve("alpha.csv")).expect("alpha added");

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(590.0, 250.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            ResultsHarnessApp::new(session)
        });
    harness.run();
    harness.run();
    harness.get_by_label("alpha.csv"); // baseline: the first build renders alpha

    harness
        .state_mut()
        .workspace
        .add_curve(curve("beta.csv"))
        .expect("beta added");
    harness.run();
    harness.run();
    harness.get_by_label("beta.csv"); // a stale cache would still show only alpha

    let workspace = &mut harness.state_mut().workspace;
    assert!(workspace.select_file(&alpha_id));
    assert_eq!(workspace.remove_selected_or_checked(), 1);
    harness.run();
    harness.run();
    assert!(
        harness.query_by_label("alpha.csv").is_none(),
        "removed file must leave the rendered table"
    );
    harness.get_by_label("beta.csv"); // the survivor stays rendered
}
