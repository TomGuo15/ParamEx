//! Number formatters for the plain-text report.
//!
//! All take `Option<f64>` where `None` is a missing or non-numeric input. NaN,
//! ±Inf, and `None` collapse to each formatter's sentinel (`"NA"`, or `"0"`
//! for counts).

/// Fixed two-decimal number, or `"NA"`.
pub(super) fn fmt(value: Option<f64>) -> String {
    match value {
        Some(v) if v.is_finite() => format!("{v:.2}"),
        _ => "NA".to_string(),
    }
}

/// Engineering-notation current (mA→fA), or `"NA"`. Below 1 fA, falls back to
/// [`format_sci_2e`] + `" A"`. The unit `µ` is U+00B5 (MICRO SIGN), which the
/// reference CSV encodes.
pub(super) fn fmt_engineering_current(value: Option<f64>) -> String {
    let number = match value {
        Some(v) if v.is_finite() => v,
        _ => return "NA".to_string(),
    };
    let abs_value = number.abs();
    const UNITS: [(f64, &str); 5] = [
        (1e-3, "mA"),
        (1e-6, "\u{00B5}A"),
        (1e-9, "nA"),
        (1e-12, "pA"),
        (1e-15, "fA"),
    ];
    for (scale, unit) in UNITS {
        if abs_value >= scale {
            return format!("{:.2} {}", number / scale, unit);
        }
    }
    format!("{} A", format_sci_2e(number))
}

/// Plain-text power-of-ten `m.mm × 10^exp`. `"NA"` for `None`/non-finite/
/// `x ≤ 0`. Exponent = `floor(log10(x))` (floating-point edge case: near exact
/// powers the mantissa may render `10.00 × 10ⁿ`). `×` is U+00D7.
pub(super) fn fmt_power_of_ten(value: Option<f64>) -> String {
    let number = match value {
        Some(v) if v.is_finite() && v > 0.0 => v,
        _ => return "NA".to_string(),
    };
    let exponent = number.log10().floor() as i64;
    let mantissa = number / 10f64.powi(exponent as i32);
    format!("{:.2} \u{00D7} 10^{}", mantissa, exponent)
}

/// Integer count string, `"0"` for `None`/non-finite. The value truncates
/// toward zero (`value as i64`).
pub(super) fn format_count(value: Option<f64>) -> String {
    match value {
        Some(v) if v.is_finite() => (v as i64).to_string(),
        _ => "0".to_string(),
    }
}

/// Two-decimal scientific notation with a lowercase `e`, an explicit exponent
/// sign, and an exponent zero-padded to at least two digits — the form the
/// reference CSV encodes. Rust's `{:.2e}` mantissa is kept and only the
/// exponent is reformatted. Examples: `0.0 → "0.00e+00"`,
/// `1e-15 → "1.00e-15"`, `-0.125 → "-1.25e-01"`. Non-finite input returns
/// `"nan"` / `"inf"` / `"-inf"`.
pub(super) fn format_sci_2e(n: f64) -> String {
    if !n.is_finite() {
        // Rust formats non-finite floats with no exponent, so the 'e' split
        // below would panic; the reference encodes these lowercase words.
        return if n.is_nan() {
            "nan".to_string()
        } else if n > 0.0 {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }
    let raw = format!("{:.2e}", n); // e.g. "1.25e-1", "0.00e0", "-1.25e-1"
    let (mantissa, exp) = raw.split_once('e').expect("rust {:e} always has 'e'");
    let exp_i: i32 = exp.parse().expect("exponent is an integer");
    let sign = if exp_i < 0 { '-' } else { '+' };
    format!("{}e{}{:02}", mantissa, sign, exp_i.abs())
}
