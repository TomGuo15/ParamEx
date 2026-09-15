use paramex_core::transfer::{AttachOutputOutcome, OutputDataset, ParsedCurve};

pub(crate) fn transfer_curve(name: &str, vt: f64) -> ParsedCurve {
    let n = 160usize;
    let mut vg = Vec::with_capacity(n);
    let mut id_abs = Vec::with_capacity(n);
    for i in 0..n {
        let v = -3.0 + 13.0 * (i as f64) / ((n - 1) as f64);
        vg.push(v);
        let on = 1e-3 * (v - vt).max(0.0).powi(2);
        let off = 1e-12 * 10f64.powf((v - vt).min(0.0) / 0.3);
        id_abs.push((on + off).abs() + 1e-13);
    }
    ParsedCurve {
        name: name.to_string(),
        vg,
        id_abs,
        source_path: None,
    }
}

pub(crate) fn expect_attached(
    outcome: AttachOutputOutcome,
    expected_file_id: &str,
) -> Option<OutputDataset> {
    match outcome {
        AttachOutputOutcome::Attached { file_id, displaced } => {
            assert_eq!(file_id, expected_file_id);
            displaced
        }
        other => panic!("expected attached output, got {other:?}"),
    }
}
