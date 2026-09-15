//! TLM process-group analysis orchestration.

use crate::shared::numpy_compat::nanmedian;
use crate::tlm::format::fmt_g;
use crate::tlm::types::{
    GroupAnalysis, LengthPoint, TlmCurve, TlmFitSummary, VdSource, VG_MATCH_TOLERANCE_V,
};

use super::{polyfit1, r_squared};

/// Fit one process group at one gate voltage. The highest current per length
/// drives the reported fit; the median-current fit is the diagnostic. Warning
/// strings and their order are part of the CSV contract.
pub(in crate::tlm) fn analyze_group(
    group: &str,
    curves: &[&TlmCurve],
    selected_vg: f64,
) -> GroupAnalysis {
    let mut points: Vec<LengthPoint> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    let fallback_count = curves
        .iter()
        .filter(|c| c.vd_source() == VdSource::Fallback)
        .count();
    if fallback_count > 0 {
        warnings.push(format!("{fallback_count} file(s) used fallback V_D"));
    }

    // Unique lengths, ascending.
    let mut lengths: Vec<f64> = curves.iter().map(|c| c.length_um()).collect();
    lengths.sort_by(|a, b| a.total_cmp(b));
    lengths.dedup();

    for length_um in lengths {
        // (current, actual_vg, &curve) candidates with positive finite current.
        let mut candidates: Vec<(f64, f64, &TlmCurve)> = Vec::new();
        for &c in curves.iter().filter(|c| c.length_um() == length_um) {
            let (current, actual_vg) = c.current_at(selected_vg);
            if current.is_finite() && current > 0.0 {
                candidates.push((current, actual_vg, c));
            }
        }
        if candidates.is_empty() {
            warnings.push(format!("No positive current at L={} um", fmt_g(length_um)));
            continue;
        }
        // Highest current wins; the first candidate wins ties (strict `>`).
        let mut best = candidates[0];
        for &cand in &candidates[1..] {
            if cand.0 > best.0 {
                best = cand;
            }
        }
        let (current, actual_vg, selected_curve) = best;
        if (actual_vg - selected_vg).abs() > VG_MATCH_TOLERANCE_V {
            warnings.push(format!(
                "Nearest measured V_G at L={} um is {} V",
                fmt_g(length_um),
                fmt_g(actual_vg)
            ));
        }
        let vd_abs = selected_curve.vd().abs();
        let currents: Vec<f64> = candidates.iter().map(|c| c.0).collect();
        let median_current = nanmedian(&currents);
        points.push(LengthPoint {
            group: group.to_string(),
            length_um,
            selected_vg,
            actual_vg,
            current_a: current,
            rtotal_ohm: vd_abs / current,
            current_median_a: median_current,
            rtotal_median_ohm: vd_abs / median_current,
            device_count: candidates.len(),
            selected_file: file_name(selected_curve.file_path()),
        });
    }

    if points.len() < 2 {
        warnings.push("At least two valid lengths are required for a TLM fit".to_string());
        return nan_group(group, selected_vg, points, warnings);
    }
    if points.len() < 4 {
        warnings.push("Fewer than four channel lengths were available".to_string());
    }

    let x: Vec<f64> = points.iter().map(|p| p.length_um).collect();
    let y: Vec<f64> = points.iter().map(|p| p.rtotal_ohm).collect();
    let (slope, intercept) = polyfit1(&x, &y);
    let predicted: Vec<f64> = x.iter().map(|xi| slope * xi + intercept).collect();
    let mut r2 = r_squared(&y, &predicted);
    if intercept < 0.0 {
        warnings.push("Fit intercept is negative".to_string());
    }
    if r2.is_finite() && r2 < 0.95 {
        warnings.push(format!("Poor TLM fit (R\u{b2} = {r2:.3} < 0.95)"));
    }

    let my: Vec<f64> = points.iter().map(|p| p.rtotal_median_ohm).collect();
    let (mslope, mintercept) = polyfit1(&x, &my);
    let mpredicted: Vec<f64> = x.iter().map(|xi| mslope * xi + mintercept).collect();
    let mut mr2 = r_squared(&my, &mpredicted);

    if points.len() < 3 {
        r2 = f64::NAN;
        mr2 = f64::NAN;
        warnings.push("R\u{b2} is undefined for fewer than three channel lengths".to_string());
    }

    GroupAnalysis {
        group: group.to_string(),
        selected_vg,
        points,
        fit: TlmFitSummary::from_fits(slope, intercept, r2, mslope, mintercept, mr2),
        warnings,
    }
}

fn nan_group(
    group: &str,
    selected_vg: f64,
    points: Vec<LengthPoint>,
    warnings: Vec<String>,
) -> GroupAnalysis {
    GroupAnalysis {
        group: group.to_string(),
        selected_vg,
        points,
        fit: TlmFitSummary::nan(),
        warnings,
    }
}

/// Basename from a path string with either separator.
fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tlm::types::TlmSample;

    fn curve(length_um: f64, current: f64, file_path: &str) -> TlmCurve {
        TlmCurve::try_new(
            file_path.to_owned(),
            "process".to_owned(),
            length_um,
            vec![TlmSample::try_new(-40.0, current, current).unwrap()],
            -0.5,
            VdSource::Setup,
        )
        .unwrap()
    }

    fn refs(curves: &[TlmCurve]) -> Vec<&TlmCurve> {
        curves.iter().collect()
    }

    #[test]
    fn two_lengths_fit_a_line_but_leave_r_squared_undefined() {
        let curves = vec![curve(50.0, 1e-6, "a"), curve(100.0, 5e-7, "b")];

        let result = analyze_group("process", &refs(&curves), -40.0);

        assert!(result.fit.intercept_ohm.abs() < 1e-3);
        assert!(result.fit.rc_per_contact_ohm.abs() < 1e-3);
        assert!(result.fit.r_squared.is_nan());
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.contains("fewer than three")
                || warning.contains("Fewer than four")));
    }

    #[test]
    fn one_length_returns_nan_fit_values() {
        let curves = [curve(50.0, 1e-6, "a")];
        let result = analyze_group("process", &refs(&curves), -40.0);

        assert!(result.fit.intercept_ohm.is_nan());
        assert!(result.fit.r_squared.is_nan());
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.contains("At least two valid lengths")));
    }

    #[test]
    fn each_length_uses_the_highest_current_device() {
        let curves = vec![
            curve(50.0, 1e-6, "low"),
            curve(50.0, 2e-6, "high"),
            curve(100.0, 5e-7, "long"),
        ];

        let result = analyze_group("process", &refs(&curves), -40.0);
        let point = result
            .points
            .iter()
            .find(|point| point.length_um == 50.0)
            .expect("50 µm point");

        assert_eq!(point.selected_file, "high");
        assert_eq!(point.device_count, 2);
        assert_eq!(point.current_a, 2e-6);
    }

    #[test]
    fn nearest_voltage_warning_uses_the_shared_tolerance() {
        let curves = vec![curve(50.0, 1e-6, "a"), curve(100.0, 5e-7, "b")];
        let refs = refs(&curves);

        let within = analyze_group("process", &refs, -40.0 + VG_MATCH_TOLERANCE_V / 2.0);
        assert!(!within
            .warnings
            .iter()
            .any(|warning| warning.starts_with("Nearest measured V_G")));

        let beyond = analyze_group("process", &refs, -40.0 + VG_MATCH_TOLERANCE_V * 2.0);
        assert!(beyond
            .warnings
            .iter()
            .any(|warning| warning.starts_with("Nearest measured V_G")));
    }
}
