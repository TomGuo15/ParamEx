//! Transfer-curve parsing, fitting, extraction, plotting, and shared types.

mod extract;
mod file_name;
mod fit;
mod metrics;
mod output;
mod parse;
mod plot_helpers;
mod report;
mod session;
mod types;

pub use extract::extract_metrics;
pub use file_name::output_name_hint;
pub use fit::{Transform, WindowedFitter};
pub use metrics::sweep::split_double_sweep;
pub use output::{
    parse_output_bytes, parse_output_file, OutputCurve, OutputDataset, OutputSummary,
};
pub use parse::{
    is_supported_measurement_path, parse_transfer_bytes, parse_transfer_file, ParseError,
    MIN_TRANSFER_POINTS, SUPPORTED_EXTENSIONS,
};
pub use plot_helpers::{
    axis_bounds, clamp_window_to_axis, log_current_axis_range, sqrt_current_axis_range,
};
pub use report::output::{OutputFitKind, OutputFitStatus, OutputReportRow};
pub use report::{
    ResultsTableCell, ResultsTableColumn, ResultsTableProjection, ResultsTableRow,
    ResultsTableRowKind, ResultsTableSweep,
};
pub use session::{
    AttachOutputOutcome, ExpertWindow, FileGeometryRow, FileListRow, SelectedFileMetricsProjection,
    SelectedFitWindowFile, SelectedOutputFile, Session,
};
pub use types::{
    calculate_stack_cox_nf_per_cm2, CoxError, DeviceGeometry, ExpertRanges, ExtractionSettings,
    ExtractionStatus, GeometryError, GeometrySource, MetricResult, ParsedCurve, SweepData,
    WindowedFitResult,
};

#[cfg(test)]
mod test_support;
