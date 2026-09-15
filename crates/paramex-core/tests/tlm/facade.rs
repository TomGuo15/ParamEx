use crate::common::tlm_corpus_dir;
use paramex_core::tlm::{
    analyze_dataset, analyze_sweep, load_dataset, result_csv, sweep_csv, GroupAnalysis,
    LengthPoint, TlmAnalysisResult, TlmDataset, TlmSweepResult,
};

#[test]
fn tlm_interface_exports_domain_named_analysis_types() {
    let dataset: TlmDataset = load_dataset(&tlm_corpus_dir(), None).expect("corpus loads");
    let analysis: TlmAnalysisResult = analyze_dataset(&dataset, None);
    let sweep: TlmSweepResult = analyze_sweep(&dataset);

    let _: Option<&GroupAnalysis> = analysis.groups.first();
    let _: Option<&LengthPoint> = analysis.groups.first().and_then(|g| g.points.first());
    assert!(!result_csv(&analysis).is_empty());
    assert!(!sweep_csv(&sweep).is_empty());
}
