//! V_TH extraction, mobility, and the ELR window selector.

mod elr;
mod window;

#[cfg(test)]
pub(in crate::transfer::metrics) use elr::extract_vth_elr;
pub(in crate::transfer) use elr::{extract_vt_mu, FitGate, VtFitResult};
#[cfg(test)]
pub(in crate::transfer::metrics) use window::auto_select_vt_window;
pub(in crate::transfer) use window::{
    select_elr_vt_window, VtWindowSelector, DEFAULT_VT_R2_LADDER,
};
