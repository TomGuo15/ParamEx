//! TLM extraction primitives.

mod fit;
mod group;
mod voltage;

use fit::{polyfit1, r_squared};
pub(super) use group::analyze_group;
pub(super) use voltage::{default_selected_vg, selected_vg_for_dataset};
