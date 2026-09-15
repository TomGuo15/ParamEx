//! TLM (contact-resistance) page panels — thin views over `paramex_core::tlm`.
//! Left = inputs (data folder, group pick, V_G analysis), center = plot + tabbed
//! results, right = outputs (selected-group fit + per-file status).
pub mod analysis;
pub mod columns;
pub mod data;
pub mod groups;
pub mod labels;
pub mod metrics;
pub mod plot;
pub mod tables;
