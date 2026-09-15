//! TLM plot point projection, fit-line endpoints, and pinned-bounds math.

use paramex_core::tlm::GroupAnalysis;

pub(super) struct PlotModel {
    pub scatter_points: Vec<[f64; 2]>,
    pub median_points: Vec<[f64; 2]>,
    pub max_line: Option<[[f64; 2]; 2]>,
    pub median_line: Option<[[f64; 2]; 2]>,
    pub x_bounds: [f64; 2],
    pub y_bounds: [f64; 2],
}

/// Max-current R_total scatter: `[length_um, rtotal_ohm]` per length point.
pub fn scatter_points(g: &GroupAnalysis) -> Vec<[f64; 2]> {
    g.points
        .iter()
        .map(|p| [p.length_um, p.rtotal_ohm])
        .collect()
}

/// Median-current R_total scatter: `[length_um, rtotal_median_ohm]` per length point.
pub fn median_points(g: &GroupAnalysis) -> Vec<[f64; 2]> {
    g.points
        .iter()
        .map(|p| [p.length_um, p.rtotal_median_ohm])
        .collect()
}

/// The padded x-extent past the longest channel length: shared by `show()` (fit
/// lines span `[0, x]`) and `plot_bounds` (axis pins to the same `x`) so the
/// dashed fits always span exactly the visible range.
pub(super) fn max_length_x(scatter: &[[f64; 2]], med: &[[f64; 2]]) -> f64 {
    scatter
        .iter()
        .chain(med.iter())
        .map(|p| p[0])
        .fold(0.0_f64, f64::max)
        .max(1.0)
        * 1.05
}

pub(super) fn plot_model(g: &GroupAnalysis) -> PlotModel {
    let scatter_points = scatter_points(g);
    let median_points = median_points(g);
    // x range from 0 (so the intercept = 2·Rc reads off the y-axis) to a little
    // past the longest channel (the same extent plot_bounds pins the axis to).
    let x_probe = max_length_x(&scatter_points, &median_points);
    let max_line = crate::plot_kit::fit_line_endpoints(
        g.fit.slope_ohm_per_um,
        g.fit.intercept_ohm,
        0.0,
        x_probe,
    );
    let median_line = crate::plot_kit::fit_line_endpoints(
        g.fit.slope_median_ohm_per_um,
        g.fit.intercept_median_ohm,
        0.0,
        x_probe,
    );
    let (x_bounds, y_bounds) =
        plot_bounds(&scatter_points, &median_points, &[max_line, median_line]);

    PlotModel {
        scatter_points,
        median_points,
        max_line,
        median_line,
        x_bounds,
        y_bounds,
    }
}

/// Pinned plot frame: x from 0 (the intercept = 2·R_c reads off the y-axis) to a
/// little past the longest channel; y from 0 (or the data minimum, if negative)
/// to the data/fit maximum with padding. Falls back to a unit box when empty.
pub fn plot_bounds(
    scatter: &[[f64; 2]],
    med: &[[f64; 2]],
    lines: &[Option<[[f64; 2]; 2]>; 2],
) -> ([f64; 2], [f64; 2]) {
    let x_hi = max_length_x(scatter, med);
    let mut ys: Vec<f64> = scatter.iter().chain(med.iter()).map(|p| p[1]).collect();
    for line in lines.iter().flatten() {
        ys.push(line[0][1]);
        ys.push(line[1][1]);
    }
    let finite = ys.iter().copied().filter(|v| v.is_finite());
    let y_max = finite.clone().fold(f64::NEG_INFINITY, f64::max);
    let y_min = finite.fold(f64::INFINITY, f64::min);
    let (y_lo, y_hi) = if y_max.is_finite() {
        (y_min.min(0.0) * 1.05 - 1e-9, y_max.max(0.0) * 1.08 + 1e-9)
    } else {
        (0.0, 1.0)
    };
    ([0.0, x_hi], [y_lo, y_hi])
}
