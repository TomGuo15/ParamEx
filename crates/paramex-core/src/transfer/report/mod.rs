//! Result reporting: typed application projection, column schema, aggregate
//! stats, formatters, "Overall" rows, and byte-exact sectioned CSV export.

pub(super) mod csv;
mod format;
pub(super) mod output;
mod projection;
mod schema;
mod stats;
mod table;

pub(in crate::transfer) use projection::project_results_table;
pub use projection::{
    ResultsTableCell, ResultsTableColumn, ResultsTableProjection, ResultsTableRow,
    ResultsTableRowKind, ResultsTableSweep,
};

#[cfg(test)]
mod tests;
