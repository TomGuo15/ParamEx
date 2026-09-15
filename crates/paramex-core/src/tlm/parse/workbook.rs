//! TLM workbook parsing orchestration.

use std::path::Path;

use crate::tlm::types::{valid_vd, TlmCurve, TlmParseError, VdSource};

use super::io::read_named_sheets;
use super::paths::{rel_os, workbook_group_length};
use super::sheets::{parse_list_sheet, parse_vd_bias};

/// Parse one MATLAB-style TLM workbook: `Setup(*)` supplies `V_D` (or the
/// caller's `fallback_vd` when absent) and `List(*)` supplies the samples.
pub fn parse_workbook(
    path: &Path,
    root: &Path,
    fallback_vd: Option<f64>,
) -> Result<TlmCurve, TlmParseError> {
    let (group, length_um) = workbook_group_length(path, root)?;

    let sheets = read_named_sheets(path, &rel_os(path, root))?;
    let setup = sheets
        .iter()
        .find(|(name, _)| name.to_lowercase().starts_with("setup"));
    let list = sheets
        .iter()
        .find(|(name, _)| name.to_lowercase().starts_with("list"));

    let Some((_, list_grid)) = list else {
        return Err(TlmParseError(format!(
            "{} must contain a List(*) sheet",
            rel_os(path, root)
        )));
    };

    let (vd, vd_source) = match setup {
        Some((_, setup_grid)) => (parse_vd_bias(setup_grid)?, VdSource::Setup),
        None => match fallback_vd {
            Some(fb) => (valid_vd(fb, "Fallback V_D")?, VdSource::Fallback),
            None => {
                return Err(TlmParseError(format!(
                    "{} has no Setup(*) sheet; enter fallback V_D to load this workbook",
                    rel_os(path, root)
                )))
            }
        },
    };

    let samples = parse_list_sheet(list_grid)?;

    TlmCurve::try_new(
        path.display().to_string(),
        group,
        length_um,
        samples,
        vd,
        vd_source,
    )
}
