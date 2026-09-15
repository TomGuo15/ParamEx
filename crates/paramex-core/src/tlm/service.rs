//! TLM dataset orchestration, sequential.

mod load;

use std::collections::BTreeMap;

pub use load::load_dataset;

use crate::tlm::methods::{analyze_group, default_selected_vg, selected_vg_for_dataset};
use crate::tlm::types::{
    TlmAnalysisResult, TlmCurve, TlmDataset, TlmSweepResult, VoltageSweepPoint,
};

/// Analyze a loaded dataset at one V_G: `None` selects the voltage with the
/// strongest median current, `Some` snaps to the nearest measured voltage.
pub fn analyze_dataset(dataset: &TlmDataset, selected_vg: Option<f64>) -> TlmAnalysisResult {
    let effective_vg = match selected_vg {
        None => default_selected_vg(dataset.curves()),
        Some(v) => selected_vg_for_dataset(dataset.vg_values(), Some(v)),
    };
    let grouped = group_curves(dataset.curves());
    let groups = grouped
        .iter()
        .map(|(name, curves)| analyze_group(name, curves, effective_vg))
        .collect();
    TlmAnalysisResult {
        root: dataset.root().to_owned(),
        selected_vg: effective_vg,
        vg_values: dataset.vg_values().to_vec(),
        groups,
        statuses: dataset.statuses().to_vec(),
    }
}

/// Analyze every measured V_G. Outer loop V_G, inner loop group.
pub fn analyze_sweep(dataset: &TlmDataset) -> TlmSweepResult {
    let grouped = group_curves(dataset.curves());
    let mut points: Vec<VoltageSweepPoint> = Vec::new();
    for &vg in dataset.vg_values() {
        for (name, curves) in &grouped {
            let group = analyze_group(name, curves, vg);
            points.push(VoltageSweepPoint {
                group: group.group,
                selected_vg: group.selected_vg,
                fit: group.fit,
                valid_lengths: group.points.len(),
                warnings: group.warnings,
            });
        }
    }
    TlmSweepResult {
        root: dataset.root().to_owned(),
        vg_values: dataset.vg_values().to_vec(),
        points,
    }
}

/// Group curves by name; the `BTreeMap` yields groups in alphabetical order.
fn group_curves(curves: &[TlmCurve]) -> BTreeMap<&str, Vec<&TlmCurve>> {
    let mut map: BTreeMap<&str, Vec<&TlmCurve>> = BTreeMap::new();
    for c in curves {
        map.entry(c.group()).or_default().push(c);
    }
    map
}
