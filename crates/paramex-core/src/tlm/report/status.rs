//! TLM status.csv report shape.

use crate::tlm::types::TlmAnalysisResult;

use super::{fcell, write_csv};

/// status.csv: one row per discovered workbook (and per unlistable folder).
pub fn status_csv(result: &TlmAnalysisResult) -> Vec<u8> {
    let headers = [
        "file",
        "group",
        "length_um",
        "status",
        "message",
        "vd_source",
    ];
    let rows = result
        .statuses
        .iter()
        .map(|s| {
            vec![
                s.file.clone(),
                s.group.clone(),
                s.length_um.map(fcell).unwrap_or_default(),
                s.status.as_str().to_string(),
                s.message.clone(),
                s.vd_source.as_str().to_string(),
            ]
        })
        .collect();
    write_csv(&headers, rows)
}
