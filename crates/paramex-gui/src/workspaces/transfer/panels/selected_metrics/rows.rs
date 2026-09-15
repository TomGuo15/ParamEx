//! Selected-file metric tile display projection.

use paramex_core::transfer::MetricResult;

use crate::format_ui::{fmt_current, fmt_fixed2, fmt_ratio, DASH};

pub(super) fn device_tiles(r: &MetricResult) -> Vec<(&'static str, String)> {
    vec![
        ("W", format!("{} \u{00B5}m", fmt_fixed2(r.width_um))),
        ("L", format!("{} \u{00B5}m", fmt_fixed2(r.length_um))),
        ("W/L", fmt_fixed2(r.aspect_ratio)),
        (
            "\u{0394}V<sub>TH,hyst</sub>",
            format!("{} V", fmt_fixed2(r.delta_vth_hysteresis)),
        ),
    ]
}

pub(super) fn empty_device_tiles() -> Vec<(&'static str, String)> {
    vec![
        ("W", DASH.to_string()),
        ("L", DASH.to_string()),
        ("W/L", DASH.to_string()),
        ("\u{0394}V<sub>TH,hyst</sub>", DASH.to_string()),
    ]
}

const SWEEP_LABELS: [&str; 6] = [
    "V<sub>TH</sub> (V)",
    "\u{00B5}<sub>sat</sub> (cm<sup>2</sup> V<sup>-1</sup> s<sup>-1</sup>)",
    "SS (mV dec<sup>-1</sup>)",
    "I<sub>on</sub>",
    "I<sub>off</sub>",
    "I<sub>on</sub>/I<sub>off</sub>",
];

pub(super) fn empty_sweep_metric_rows() -> Vec<(&'static str, String, String)> {
    SWEEP_LABELS
        .iter()
        .map(|label| (*label, DASH.to_string(), DASH.to_string()))
        .collect()
}

fn branch_values(
    vt: f64,
    mu_sat: f64,
    ss_mv_dec: f64,
    ion: f64,
    ioff: f64,
    ratio: f64,
) -> [String; 6] {
    [
        fmt_fixed2(vt),
        fmt_fixed2(mu_sat),
        fmt_fixed2(ss_mv_dec),
        fmt_current(ion),
        fmt_current(ioff),
        if ratio.is_finite() && ratio > 0.0 {
            fmt_ratio(ratio)
        } else {
            "NA".to_string()
        },
    ]
}

pub(super) fn sweep_metric_rows(r: &MetricResult) -> (bool, Vec<(&'static str, String, String)>) {
    let forward = branch_values(
        r.vt_forward,
        r.mu_sat_forward,
        r.ss_mv_dec_forward,
        r.ion_forward,
        r.ioff_forward,
        r.on_off_ratio_forward,
    );
    let backward = branch_values(
        r.vt_backward,
        r.mu_sat_backward,
        r.ss_mv_dec_backward,
        r.ion_backward,
        r.ioff_backward,
        r.on_off_ratio_backward,
    );
    let rows = SWEEP_LABELS
        .into_iter()
        .zip(forward)
        .zip(backward)
        .map(|((label, forward), backward)| {
            let backward = if r.has_backward_sweep {
                backward
            } else {
                String::new()
            };
            (label, forward, backward)
        })
        .collect();
    (r.has_backward_sweep, rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use paramex_core::transfer::{ExtractionStatus, GeometrySource};

    fn sample(has_backward: bool) -> MetricResult {
        MetricResult {
            filename: "a.csv".to_string(),
            width_um: 1500.0,
            length_um: 50.0,
            aspect_ratio: 30.0,
            geometry_source: GeometrySource::Default,
            vt: 1.23,
            mu_sat: 12.5,
            ss_mv_dec: 95.0,
            ion: 1.5e-3,
            ioff: 2.0e-12,
            on_off_ratio: 7.5e8,
            delta_vth_hysteresis: 0.12,
            vt_window: Some((1.0, 3.0)),
            ss_window: Some((-0.5, 0.5)),
            vt_window_bwd: None,
            ss_window_bwd: None,
            status: ExtractionStatus::Ok,
            message: String::new(),
            has_backward_sweep: has_backward,
            vt_forward: 1.23,
            mu_sat_forward: 12.5,
            ss_mv_dec_forward: 95.0,
            ion_forward: 1.5e-3,
            ioff_forward: 2.0e-12,
            on_off_ratio_forward: 7.5e8,
            vt_backward: 1.30,
            mu_sat_backward: 11.0,
            ss_mv_dec_backward: 100.0,
            ion_backward: 1.4e-3,
            ioff_backward: 3.0e-12,
            on_off_ratio_backward: 4.0e8,
        }
    }

    #[test]
    fn selected_rows_format_raw_metrics_in_the_gui() {
        assert_eq!(device_tiles(&sample(false))[0].1, "1500.00 \u{00B5}m");
        let (has_backward, rows) = sweep_metric_rows(&sample(true));
        assert!(has_backward);
        assert_eq!(rows[0].1, "1.23");
        assert_eq!(rows[0].2, "1.30");
        assert_eq!(rows[3].1, "1.50 mA");
        assert_eq!(rows[5].1, "750M");
    }
}
