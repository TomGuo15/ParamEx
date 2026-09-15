//! TLM length_points.csv report shape.

use crate::tlm::types::{LengthPoint, TlmAnalysisResult};

use super::{fcell, write_csv};

/// length_points.csv: one row per (group, length).
pub fn length_points_csv(result: &TlmAnalysisResult) -> Vec<u8> {
    let headers = [
        "group",
        "length_um",
        "selected_vg",
        "actual_vg",
        "current_a",
        "Rtotal_ohm",
        "current_median_a",
        "Rtotal_median_ohm",
        "device_count",
        "selected_file",
    ];
    let mut rows = Vec::new();
    for g in &result.groups {
        for p in &g.points {
            rows.push(point_row(p));
        }
    }
    write_csv(&headers, rows)
}

fn point_row(p: &LengthPoint) -> Vec<String> {
    vec![
        p.group.clone(),
        fcell(p.length_um),
        fcell(p.selected_vg),
        fcell(p.actual_vg),
        fcell(p.current_a),
        fcell(p.rtotal_ohm),
        fcell(p.current_median_a),
        fcell(p.rtotal_median_ohm),
        p.device_count.to_string(),
        p.selected_file.clone(),
    ]
}
