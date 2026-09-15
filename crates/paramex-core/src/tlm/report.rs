//! TLM CSV row builders + writer.
//!
//! Every export shares the product-wide byte layout from `shared::csv`: a
//! UTF-8 BOM, a header row, then one CRLF-terminated row per record with
//! fields quoted only when they contain a comma, quote, or line break. A
//! report with no rows is an empty file. Floats use Rust's shortest
//! round-trip formatting; NaN becomes an empty cell.

mod fit_tables;
mod length_points;
mod status;

pub use fit_tables::{result_csv, sweep_csv};
pub use length_points::length_points_csv;
pub use status::status_csv;

/// f64 cell: empty for NaN, else Rust shortest repr.
fn fcell(x: f64) -> String {
    if x.is_nan() {
        String::new()
    } else {
        format!("{x}")
    }
}

fn write_csv(headers: &[&str], rows: Vec<Vec<String>>) -> Vec<u8> {
    crate::shared::csv::write_table(headers, &rows)
}
