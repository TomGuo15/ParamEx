//! TLM result.csv and sweep.csv report shapes.

use crate::tlm::types::{
    GroupAnalysis, TlmAnalysisResult, TlmFitSummary, TlmSweepResult, VoltageSweepPoint,
};

use super::{fcell, write_csv};

/// Eight max+median fit cells shared by result + sweep rows.
fn fit_cells(fit: &TlmFitSummary) -> Vec<String> {
    vec![
        fcell(fit.intercept_ohm), // legacy Rcontact_script_ohm header
        fcell(fit.rc_per_contact_ohm),
        fcell(fit.slope_ohm_per_um),
        fcell(fit.r_squared),
        fcell(fit.intercept_median_ohm), // legacy Rcontact_median_ohm header
        fcell(fit.rc_per_contact_median_ohm),
        fcell(fit.slope_median_ohm_per_um),
        fcell(fit.r_squared_median),
    ]
}

const FIT_HEADERS: [&str; 8] = [
    "Rcontact_script_ohm",
    "Rc_per_contact_ohm",
    "slope_ohm_per_um",
    "r_squared",
    "Rcontact_median_ohm",
    "Rc_per_contact_median_ohm",
    "slope_median_ohm_per_um",
    "r_squared_median",
];

fn group_fit_cells(g: &GroupAnalysis) -> Vec<String> {
    fit_cells(&g.fit)
}

fn point_fit_cells(p: &VoltageSweepPoint) -> Vec<String> {
    fit_cells(&p.fit)
}

/// result.csv: one row per group.
pub fn result_csv(result: &TlmAnalysisResult) -> Vec<u8> {
    let mut headers = vec!["group", "selected_vg"];
    headers.extend(FIT_HEADERS);
    headers.extend(["valid_lengths", "warnings"]);
    let rows = result
        .groups
        .iter()
        .map(|g| {
            let mut row = vec![g.group.clone(), fcell(g.selected_vg)];
            row.extend(group_fit_cells(g));
            row.push(g.points.len().to_string());
            row.push(g.warnings.join("; "));
            row
        })
        .collect();
    write_csv(&headers, rows)
}

/// sweep.csv: one row per (group, V_G).
pub fn sweep_csv(result: &TlmSweepResult) -> Vec<u8> {
    let mut headers = vec!["group", "selected_vg"];
    headers.extend(FIT_HEADERS);
    headers.extend(["valid_lengths", "warnings"]);
    let rows = result
        .points
        .iter()
        .map(|p| {
            let mut row = vec![p.group.clone(), fcell(p.selected_vg)];
            row.extend(point_fit_cells(p));
            row.push(p.valid_lengths.to_string());
            row.push(p.warnings.join("; "));
            row
        })
        .collect();
    write_csv(&headers, rows)
}
