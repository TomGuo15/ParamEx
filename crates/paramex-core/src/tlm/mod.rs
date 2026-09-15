//! ParamEx TLM contact-resistance extraction.
//!
//! This module owns the TLM workbook ingest, fit methods, orchestration, the
//! in-memory session, CSV report rows, and domain types. It reuses shared core
//! numerical and grid-ingest primitives, but the TLM implementation vocabulary
//! lives here.

mod format;
mod methods;
mod parse;
mod report;
mod service;
mod session;
mod types;

pub use parse::parse_workbook;
pub use report::{length_points_csv, result_csv, status_csv, sweep_csv};
pub use service::{analyze_dataset, analyze_sweep, load_dataset};
pub use session::{AnalyzedDataset, Session, DEFAULT_FALLBACK_VD};
pub use types::{
    valid_vd, FileStatus, GroupAnalysis, LengthPoint, Status, TlmAnalysisResult, TlmCurve,
    TlmDataset, TlmDatasetRemoval, TlmFitSummary, TlmParseError, TlmSample, TlmSweepResult,
    VdSource, VoltageSweepPoint,
};
