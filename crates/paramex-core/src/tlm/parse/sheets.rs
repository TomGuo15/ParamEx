//! TLM Setup/List sheet extraction policy.

use crate::shared::grid_ingest::{coerce_numeric, Grid};
use crate::tlm::types::{valid_vd, TlmParseError, TlmSample};

/// V_D from a Setup grid: the `Measurement.Bias.Source` value under the `vd`
/// channel, falling back to the third value when channel names are absent.
pub(super) fn parse_vd_bias(setup: &Grid) -> Result<f64, TlmParseError> {
    // Resolve the channel-name header first so the vd-channel column is used
    // whether Channel.VName appears before or after Measurement.Bias.Source in the
    // sheet. A single forward pass took the fixed-index fallback for the reversed
    // order, silently reading the wrong drain bias (scaling every R_total).
    let channel_names: Option<Vec<String>> = setup.iter().find_map(|row| {
        let label = row.first().map(|s| s.trim()).unwrap_or("");
        (label == "Channel.VName").then(|| {
            row.iter()
                .skip(1)
                .map(|v| v.trim().to_lowercase())
                .collect()
        })
    });
    for row in setup {
        let label = row.first().map(|s| s.trim()).unwrap_or("");
        if label == "Measurement.Bias.Source" {
            let values: Vec<&String> = row.iter().skip(1).collect();
            if let Some(names) = &channel_names {
                if let Some(idx) = names.iter().position(|n| n == "vd") {
                    if idx < values.len() {
                        return valid_vd(coerce_numeric(values[idx]), "Setup drain bias");
                    }
                }
            }
            if values.len() >= 3 {
                return valid_vd(coerce_numeric(values[2]), "Setup drain bias");
            }
        }
    }
    Err(TlmParseError(
        "Setup sheet does not contain drain bias metadata".to_string(),
    ))
}

/// Finite measured samples from a List grid: the header row is the first row
/// naming `vg`, `abs_id`, and `abs_is`.
pub(super) fn parse_list_sheet(list: &Grid) -> Result<Vec<TlmSample>, TlmParseError> {
    let header_index = list.iter().position(|row| {
        let set: std::collections::HashSet<String> =
            row.iter().map(|v| v.trim().to_lowercase()).collect();
        ["vg", "abs_id", "abs_is"].iter().all(|c| set.contains(*c))
    });
    let Some(hi) = header_index else {
        return Err(TlmParseError(
            "List sheet does not contain vg, abs_id, and abs_is columns".to_string(),
        ));
    };
    let headers: Vec<String> = list[hi].iter().map(|v| v.trim().to_lowercase()).collect();
    let col = |name: &str| headers.iter().position(|h| h == name);
    let (Some(cvg), Some(cid), Some(cis)) = (col("vg"), col("abs_id"), col("abs_is")) else {
        return Err(TlmParseError(
            "List sheet is missing required TLM columns".to_string(),
        ));
    };

    let mut samples = Vec::new();
    for row in &list[hi + 1..] {
        let get = |c: usize| row.get(c).map(|s| coerce_numeric(s)).unwrap_or(f64::NAN);
        let (v, d, s) = (get(cvg), get(cid), get(cis));
        // Drop the row when any required cell is non-numeric. `TlmSample`
        // normalizes signed current exports to magnitudes.
        if let Ok(sample) = TlmSample::try_new(v, d, s) {
            samples.push(sample);
        }
    }
    if samples.is_empty() {
        return Err(TlmParseError(
            "List sheet contains no numeric TLM rows".to_string(),
        ));
    }
    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: &[&[&str]]) -> Grid {
        rows.iter()
            .map(|r| r.iter().map(|s| s.to_string()).collect())
            .collect()
    }

    #[test]
    fn vd_bias_resolves_by_channel_name_regardless_of_row_order() {
        // Measurement.Bias.Source BEFORE Channel.VName, with vd NOT at the fixed
        // fallback slot (index 2). V_D must still be read from the vd channel
        // column (0.5), not the fallback (values[2] = 9.9).
        let setup = grid(&[
            &["Measurement.Bias.Source", "0.5", "0.0", "9.9"],
            &["Channel.VName", "Vd", "Vs", "Vg"],
        ]);
        let vd = parse_vd_bias(&setup).expect("vd present");
        assert!(
            (vd - 0.5).abs() < 1e-12,
            "got {vd}, expected 0.5 (vd channel)"
        );
    }

    #[test]
    fn vd_bias_normal_order_reads_vd_channel() {
        let setup = grid(&[
            &["Channel.VName", "Vd", "Vs", "Vg"],
            &["Measurement.Bias.Source", "0.5", "0.0", "9.9"],
        ]);
        let vd = parse_vd_bias(&setup).expect("vd present");
        assert!((vd - 0.5).abs() < 1e-12);
    }

    #[test]
    fn list_sheet_drops_rows_with_any_non_finite_required_value() {
        let list = grid(&[
            &["vg", "abs_id", "abs_is"],
            &["2", "20", "200"],
            &["3", "-30", "-300"],
            &["bad", "10", "100"],
            &["1", "bad", "100"],
            &["1", "10", "bad"],
            &["1", "10", "100"],
        ]);

        let samples = parse_list_sheet(&list).expect("finite rows survive");

        assert_eq!(samples.len(), 3);
        assert_eq!(samples[0].vg(), 2.0);
        assert_eq!(samples[1].abs_id(), 30.0);
        assert_eq!(samples[1].abs_is(), 300.0);
        assert_eq!(samples[2].vg(), 1.0);
    }
}
