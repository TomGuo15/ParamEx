use paramex_core::transfer::{
    OutputCurve, OutputDataset, ParsedCurve, ResultsTableCell, ResultsTableColumn,
    ResultsTableRowKind, ResultsTableSweep, Session,
};

fn drain_current(vg: f64, vt: f64, scale: f64) -> f64 {
    let on = scale * 1.0e-3 * (vg - vt).max(0.0).powi(2);
    let off = 1.0e-12 * 10f64.powf((vg - vt).min(0.0) / 0.3);
    on + off + 1.0e-13
}

fn transfer_curve(name: &str, vt: f64, round_trip: bool) -> ParsedCurve {
    let branch: Vec<f64> = (0..80)
        .map(|index| -3.0 + 13.0 * index as f64 / 79.0)
        .collect();
    let mut vg = branch.clone();
    let mut id_abs: Vec<f64> = branch
        .iter()
        .map(|&value| drain_current(value, vt, 1.0))
        .collect();
    if round_trip {
        vg.extend(branch.iter().rev().copied());
        id_abs.extend(
            branch
                .iter()
                .rev()
                .map(|&value| drain_current(value, vt + 0.15, 0.95)),
        );
    }
    ParsedCurve {
        name: name.to_string(),
        vg,
        id_abs,
        source_path: None,
    }
}

fn output_dataset(name: &str) -> OutputDataset {
    OutputDataset {
        name: name.to_string(),
        curves: vec![OutputCurve {
            vg: 5.0,
            vd: vec![0.0, 1.0, 2.0, 3.0],
            id: vec![0.0, 1.0e-6, 1.7e-6, 2.5e-6],
        }],
        source_path: None,
    }
}

fn finite_mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let values: Vec<f64> = values.filter(|value| value.is_finite()).collect();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

#[test]
fn results_table_projects_canonical_typed_cells_and_group_metadata() {
    let mut session = Session::new();
    session
        .add_curve(transfer_curve("single.csv", 1.0, false))
        .expect("single curve added");
    session
        .add_curve(transfer_curve("round-trip.csv", 1.4, true))
        .expect("round-trip curve added");

    let projection = session.results_table();
    assert_eq!(projection.columns, &ResultsTableColumn::ALL);
    for (index, &column) in projection.columns.iter().enumerate() {
        assert_eq!(column.index(), index);
    }
    assert_eq!(
        projection
            .columns
            .iter()
            .map(|column| column.key())
            .collect::<Vec<_>>(),
        vec![
            "filename",
            "sweep",
            "W_um",
            "L_um",
            "W_over_L",
            "geometry_source",
            "Vth",
            "mu_sat",
            "SS_mV_dec",
            "Ion",
            "Ioff",
            "Ion_Ioff",
            "DeltaVth_hysteresis",
            "status",
            "message",
        ]
    );
    assert!(ResultsTableColumn::OnCurrent.is_numeric());
    assert!(ResultsTableColumn::OnCurrent.is_current());
    assert!(ResultsTableColumn::OnOffRatio.is_ratio());
    assert!(ResultsTableColumn::ThresholdVoltage.is_sweep_aware());
    assert!(!ResultsTableColumn::Filename.is_numeric());

    assert_eq!(projection.rows.len(), 5);
    let single = &projection.rows[0];
    assert_eq!(single.cells.len(), ResultsTableColumn::ALL.len());
    assert_eq!(single.kind, ResultsTableRowKind::Measurement);
    assert_eq!(single.sweep, ResultsTableSweep::Single);
    assert_eq!((single.group_position, single.group_span), (0, 1));
    assert_eq!(
        single.cells[ResultsTableColumn::Filename.index()],
        ResultsTableCell::Text("single.csv".to_string())
    );
    assert_eq!(
        single.cells[ResultsTableColumn::Sweep.index()],
        ResultsTableCell::Sweep(ResultsTableSweep::Single)
    );
    assert_eq!(
        single.cells[ResultsTableColumn::WidthUm.index()],
        ResultsTableCell::Number(1500.0)
    );
    assert_eq!(
        single.cells[ResultsTableColumn::GeometrySource.index()],
        ResultsTableCell::Text("default".to_string())
    );

    let forward = &projection.rows[1];
    let backward = &projection.rows[2];
    assert_eq!(forward.sweep, ResultsTableSweep::Forward);
    assert_eq!((forward.group_position, forward.group_span), (0, 2));
    assert_eq!(backward.sweep, ResultsTableSweep::Backward);
    assert_eq!((backward.group_position, backward.group_span), (1, 2));

    let forward_values = projection
        .rows
        .iter()
        .filter(|row| {
            row.kind == ResultsTableRowKind::Measurement
                && matches!(
                    row.sweep,
                    ResultsTableSweep::Single | ResultsTableSweep::Forward
                )
        })
        .filter_map(
            |row| match row.cells[ResultsTableColumn::ThresholdVoltage.index()] {
                ResultsTableCell::Number(value) => Some(value),
                _ => None,
            },
        )
        .collect::<Vec<_>>();
    let expected_forward_count = forward_values.len();
    let expected_forward_mean = finite_mean(forward_values.into_iter());
    let overall_forward = &projection.rows[3];
    assert_eq!(
        overall_forward.kind,
        ResultsTableRowKind::Overall {
            count: expected_forward_count
        }
    );
    assert_eq!(overall_forward.sweep, ResultsTableSweep::Forward);
    assert_eq!(
        (overall_forward.group_position, overall_forward.group_span),
        (0, 2)
    );
    assert_eq!(
        overall_forward.cells[ResultsTableColumn::Filename.index()],
        ResultsTableCell::Overall
    );
    assert_eq!(
        overall_forward.cells[ResultsTableColumn::WidthUm.index()],
        ResultsTableCell::Missing
    );
    assert_eq!(
        overall_forward.cells[ResultsTableColumn::Message.index()],
        ResultsTableCell::SummaryCaption
    );
    match &overall_forward.cells[ResultsTableColumn::ThresholdVoltage.index()] {
        ResultsTableCell::Summary { mean, .. } => {
            assert_eq!(*mean, expected_forward_mean);
        }
        other => panic!("expected typed threshold summary, got {other:?}"),
    }
    assert!(matches!(
        overall_forward.cells[ResultsTableColumn::OnOffRatio.index()],
        ResultsTableCell::Log10Summary { .. }
    ));

    let overall_backward = &projection.rows[4];
    assert_eq!(overall_backward.sweep, ResultsTableSweep::Backward);
    assert!(matches!(
        overall_backward.kind,
        ResultsTableRowKind::Overall { .. }
    ));
    assert_eq!(
        (overall_backward.group_position, overall_backward.group_span),
        (1, 2)
    );
}

#[test]
fn selected_output_projection_keeps_attachment_range_and_summary_coherent() {
    let mut session = Session::new();
    assert_eq!(session.selected_output_file(), None);

    let file_id = session
        .add_curve(transfer_curve("device-a.csv", 1.0, false))
        .expect("curve added");
    {
        let selected = session.selected_output_file().expect("selected file");
        assert_eq!(selected.file_id, file_id);
        assert_eq!(selected.filename, "device-a.csv");
        assert_eq!(selected.transfer_vg.len(), 80);
        assert_eq!(selected.transfer_id_abs.len(), 80);
        assert_eq!(selected.output, None);
        assert_eq!(selected.selected_fit_range, None);
        assert_eq!(selected.summary, None);
    }

    assert!(session
        .replace_output(&file_id, output_dataset("device-a-output.csv"))
        .is_ok());
    {
        let selected = session.selected_output_file().expect("selected output");
        let output = selected.output.expect("attached output");
        assert_eq!(output.name, "device-a-output.csv");
        assert_eq!(selected.selected_fit_range, None);
        assert_eq!(
            selected
                .summary
                .as_ref()
                .and_then(|summary| summary.fit_range),
            Some((2.0, 3.0))
        );
    }

    assert!(session.set_output_fit_range(&file_id, Some((0.0, 1.0))));
    let selected = session.selected_output_file().expect("selected output");
    selected.output.expect("attached output");
    assert_eq!(selected.selected_fit_range, Some((0.0, 1.0)));
    let summary = selected.summary.expect("manual-range summary");
    assert_eq!(summary.fit_range, selected.selected_fit_range);

    let family = &session.output_report_rows()[0];
    assert_eq!(family.fit_range, summary.fit_range);
    assert_eq!(family.idsat, summary.idsat);
    assert_eq!(family.gds, summary.gds);
    assert_eq!(family.ro, summary.ro);
    assert_eq!(family.early_voltage, summary.early_voltage);
    assert_eq!(family.lambda.to_bits(), summary.lambda.to_bits());
    assert_eq!(family.r2, summary.r2);
}

#[test]
fn selected_metrics_projection_keeps_filename_and_total_result_coherent() {
    let mut session = Session::new();
    assert_eq!(session.selected_file_metrics_projection(), None);

    session
        .add_curve(transfer_curve("named-device.csv", 1.0, false))
        .expect("curve added");
    let selected = session
        .selected_file_metrics_projection()
        .expect("selected metrics");
    assert_eq!(selected.filename, "named-device.csv");
    assert_eq!(selected.result.filename, selected.filename);
}
