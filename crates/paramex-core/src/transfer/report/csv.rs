//! Sectioned CSV export of the transfer results table.

use super::table::results_to_report_sections;
use crate::shared::csv::{write_row, UTF8_BOM};
use crate::transfer::types::MetricResult;

/// Serialize the sectioned report to CSV bytes.
///
/// Byte layout: a leading UTF-8 BOM; CRLF row terminators; minimal quoting;
/// per section a title row, a header row, the data rows, and a single empty
/// row **between** sections only (no trailing blank). Empty results produce
/// empty bytes, not a lone BOM.
pub(in crate::transfer) fn export_results_bytes(results: &[MetricResult]) -> Vec<u8> {
    let sections = results_to_report_sections(results);
    let mut out: Vec<u8> = Vec::new();
    let n = sections.len();
    if n > 0 {
        out.extend_from_slice(&UTF8_BOM);
    }
    for (i, section) in sections.iter().enumerate() {
        write_row(&mut out, std::slice::from_ref(&section.title));
        write_row(&mut out, &section.header);
        for row in &section.rows {
            write_row(&mut out, row);
        }
        if i + 1 < n {
            write_row::<String>(&mut out, &[]); // single empty row between sections
        }
    }
    out
}
