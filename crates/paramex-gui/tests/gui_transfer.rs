use paramex_core::transfer::{
    AttachOutputOutcome, OutputCurve, OutputDataset, ParsedCurve, Session,
};

mod common;

fn transfer_curve(name: &str) -> ParsedCurve {
    ParsedCurve {
        name: name.to_string(),
        vg: (0..12)
            .map(|index| -1.0 + 5.0 * index as f64 / 11.0)
            .collect(),
        id_abs: (0..12)
            .map(|index| 1e-12 * 10f64.powf(9.0 * index as f64 / 11.0))
            .collect(),
        source_path: Some(std::path::PathBuf::from(name)),
    }
}

fn partial_output_dataset(name: &str) -> OutputDataset {
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
                vd: vec![0.0, 1.0, 2.0],
                id: vec![f64::NAN, f64::NAN, f64::NAN],
            },
        ],
        source_path: Some(std::path::PathBuf::from(name)),
    }
}

fn attach_output(session: &mut Session, output: OutputDataset) -> Option<OutputDataset> {
    match session.attach_output(output) {
        AttachOutputOutcome::Attached { displaced, .. } => displaced,
        other => panic!("expected output to attach, got {other:?}"),
    }
}

#[path = "gui/transfer/cox_commit.rs"]
mod cox_commit;
#[path = "gui/transfer/file_list_wiring.rs"]
mod file_list_wiring;
#[path = "gui/transfer/geometry_commit.rs"]
mod geometry_commit;
#[path = "gui/transfer/output_plot.rs"]
mod output_plot;
#[path = "gui/transfer/results_table.rs"]
mod results_table;
#[path = "gui/transfer/selected_metrics.rs"]
mod selected_metrics;
#[path = "gui/transfer/selector/mod.rs"]
mod selector;
