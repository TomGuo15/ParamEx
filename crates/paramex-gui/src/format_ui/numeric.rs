//! Shared numeric display formatters: engineering/SI notation, fixed-precision
//! result cells, unit-bearing values, plot tick labels, and the inverse
//! SI-suffix parser for numeric inputs. Table cells, metric readouts, plot
//! axes, and settings fields across both workspaces read from here so one
//! value renders the same everywhere.

/// The shared non-finite dash (em dash) for cells whose value is unavailable.
pub const DASH: &str = "\u{2014}";

/// Fixed two-decimal display used by Transfer result cells.
pub fn fmt_fixed2(v: f64) -> String {
    if v.is_finite() {
        format!("{v:.2}")
    } else {
        "NA".to_string()
    }
}

fn transfer_current_parts(v: f64) -> Option<(String, &'static str)> {
    if !v.is_finite() {
        return None;
    }
    let abs = v.abs();
    for (scale, prefix) in [
        (1.0e-3, "m"),
        (1.0e-6, "\u{00B5}"),
        (1.0e-9, "n"),
        (1.0e-12, "p"),
        (1.0e-15, "f"),
    ] {
        if abs >= scale {
            return Some((format!("{:.2}", v / scale), prefix));
        }
    }
    let raw = format!("{v:.2e}");
    let (mantissa, exponent) = raw.split_once('e').expect("Rust scientific format has e");
    let exponent: i32 = exponent.parse().expect("scientific exponent is numeric");
    Some((format!("{mantissa}e{exponent:+03}"), ""))
}

/// Transfer current display with an ampere unit (`"1.50 mA"`).
pub fn fmt_current(v: f64) -> String {
    transfer_current_parts(v)
        .map(|(value, prefix)| format!("{value} {prefix}A"))
        .unwrap_or_else(|| "NA".to_string())
}

/// Compact Transfer-table current display (`"1.50m"`).
pub fn fmt_compact_current(v: f64) -> String {
    transfer_current_parts(v)
        .map(|(value, prefix)| format!("{value}{prefix}"))
        .unwrap_or_else(|| "NA".to_string())
}

/// Shared engineering-notation core: mantissa at `decimals` precision (trailing
/// zeros trimmed) + SI prefix. `large` adds the `G` through `Q` rows and `femto`
/// the `f` row (both for cell values; axis ticks use neither and stop at `M`/`p`).
/// `None` when the exponent has no prefix - the caller owns the fallback format.
fn eng_format(v: f64, decimals: usize, large: bool, femto: bool) -> Option<String> {
    if v == 0.0 {
        return Some("0".to_string());
    }
    let eng_exp = (v.abs().log10().floor() / 3.0).floor() * 3.0;
    let prefix = match eng_exp as i32 {
        -15 if femto => "f",
        -12 => "p",
        -9 => "n",
        -6 => "\u{00B5}",
        -3 => "m",
        0 => "",
        3 => "k",
        6 => "M",
        9 if large => "G",
        12 if large => "T",
        15 if large => "P",
        18 if large => "E",
        21 if large => "Z",
        24 if large => "Y",
        27 if large => "R",
        30 if large => "Q",
        _ => return None,
    };
    let mantissa = v / 10f64.powf(eng_exp);
    let m = format!("{mantissa:.decimals$}");
    let m = m.trim_end_matches('0').trim_end_matches('.');
    Some(format!("{m}{prefix}"))
}

/// Engineering/SI axis tick: `0.02` -> `"20m"`, `0.0003` -> `"300µ"`. 1-decimal
/// mantissa, zeros trimmed; exponents without a prefix fall back to `{:.1e}`.
pub fn eng_tick(v: f64) -> String {
    eng_format(v, 1, false, false).unwrap_or_else(|| format!("{v:.1e}"))
}

/// Engineering/SI cell value: 2-decimal mantissa (zeros trimmed) + an SI prefix
/// from femto through quetta. Non-finite -> em dash; out-of-range exponents fall
/// back to `{:.3e}`.
/// Femto is included so a sub-pico off-current (`I_off` ~ 1e-13) reads `971.6f`
/// instead of a raw `9.716e-13` that breaks the card's SI-prefix rhythm.
pub fn fmt_eng(v: f64) -> String {
    if !v.is_finite() {
        return DASH.to_string();
    }
    eng_format(v, 2, true, true).unwrap_or_else(|| format!("{v:.3e}"))
}

/// Positive ratio display using the shared engineering rhythm without ever
/// falling back to exponent notation. Values outside the named SI range use a
/// compact bound instead of an unbounded decimal or `e` form.
pub fn fmt_ratio(v: f64) -> String {
    if !v.is_finite() || v <= 0.0 {
        return DASH.to_string();
    }
    if v < 1.0e-15 {
        return "<1f".to_string();
    }
    if v >= 1.0e33 {
        return ">999Q".to_string();
    }
    fmt_eng(v)
}

/// Parse a user-entered number that may carry an SI engineering suffix - the inverse of
/// [`fmt_eng`], so a value the card *shows* (`"2.7m"`, `"10.3n"`, `"300µ"`, `"17.75k"`) is
/// also a value the card *accepts*. Plain and scientific notation pass straight through
/// `f64` parsing first (`"5"`, `"-0.353"`, `"2.696e-3"`), so nothing that already worked
/// regresses; a complete scientific literal is parsed before a trailing `E` is considered
/// the exa prefix. The suffix is case-sensitive - `m` = milli, `M` = mega - matching
/// `fmt_eng`. `u` and both micro glyphs (U+00B5 / U+03BC) map to µ.
/// Returns `None` for empty / garbage input or a non-finite result (so `"inf"`/`"nan"` are
/// rejected).
pub fn parse_eng(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(v) = s.parse::<f64>() {
        return v.is_finite().then_some(v);
    }
    let suffix = s.chars().last()?;
    let factor = match suffix {
        'f' => 1e-15,
        'p' => 1e-12,
        'n' => 1e-9,
        '\u{00B5}' | '\u{03BC}' | 'u' => 1e-6,
        'm' => 1e-3,
        'k' => 1e3,
        'M' => 1e6,
        'G' => 1e9,
        'T' => 1e12,
        'P' => 1e15,
        'E' => 1e18,
        'Z' => 1e21,
        'Y' => 1e24,
        'R' => 1e27,
        'Q' => 1e30,
        _ => return None,
    };
    let head = s[..s.len() - suffix.len_utf8()].trim();
    let v = head.parse::<f64>().ok()? * factor;
    v.is_finite().then_some(v)
}

/// Ohmic value with unit: `"17.75k Ω"`. Non-finite -> dash (no dangling unit).
pub fn fmt_ohm(v: f64) -> String {
    if !v.is_finite() {
        DASH.to_string()
    } else {
        format!("{} \u{2126}", fmt_eng(v))
    }
}

/// TLM slope with unit: `"135.2 Ω/µm"`. Non-finite -> dash.
pub fn fmt_slope(v: f64) -> String {
    if !v.is_finite() {
        DASH.to_string()
    } else {
        format!("{} \u{2126}/\u{00B5}m", fmt_eng(v))
    }
}

/// R² display: 2 decimals. Non-finite -> dash.
pub fn fmt_r2(v: f64) -> String {
    if !v.is_finite() {
        DASH.to_string()
    } else {
        format!("{v:.2}")
    }
}

/// Bare number, <=3 decimals with trailing zeros trimmed (V_G / L cells whose unit
/// lives in the column header). Non-finite -> dash.
pub fn fmt_num3(v: f64) -> String {
    if !v.is_finite() {
        return DASH.to_string();
    }
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    // Negative zero or a small negative that rounds to zero (-0.0 / -0.0004 -> "-0.000" ->
    // "-0") must still read "0": cells never show "-0" (guarded by
    // non_finite_and_negative_zero).
    if s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// Gate-voltage label with unit: `"-5 V"` (trimmed). Non-finite -> dash.
pub fn fmt_vg(v: f64) -> String {
    if !v.is_finite() {
        DASH.to_string()
    } else {
        format!("{} V", fmt_num3(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eng_tick_uses_one_decimal_si_prefixes() {
        assert_eq!(eng_tick(0.0), "0");
        assert_eq!(eng_tick(0.02), "20m");
        assert_eq!(eng_tick(0.0003), "300\u{00B5}");
        assert_eq!(eng_tick(1.0e-15), format!("{:.1e}", 1.0e-15)); // out of prefix range
    }

    #[test]
    fn fmt_eng_cells() {
        assert_eq!(fmt_eng(f64::NAN), "\u{2014}");
        assert_eq!(fmt_eng(0.0), "0");
        assert_eq!(fmt_eng(17750.0), "17.75k");
        assert_eq!(fmt_eng(56500.0), "56.5k");
        assert_eq!(fmt_eng(1.76e13), "17.6T");
        assert_eq!(fmt_eng(1.0e15), "1P");
        assert_eq!(fmt_eng(1.0e30), "1Q");
        assert_eq!(fmt_eng(-2300.0), "-2.3k");
        assert_eq!(fmt_eng(0.0003), "300\u{00B5}");
        assert_eq!(fmt_eng(5.0), "5");
        // Femto: a sub-pico off-current reads with the `f` prefix, not raw scientific.
        assert_eq!(fmt_eng(9.716e-13), "971.6f");
        assert_eq!(fmt_eng(1.0e-15), "1f");
    }

    #[test]
    fn transfer_result_formatters_keep_report_precision_without_reparsing() {
        assert_eq!(fmt_fixed2(1.5), "1.50");
        assert_eq!(fmt_fixed2(f64::NAN), "NA");
        assert_eq!(fmt_current(1.5e-3), "1.50 mA");
        assert_eq!(fmt_compact_current(1.5e-3), "1.50m");
        assert_eq!(fmt_current(1.0e-16), "1.00e-16 A");
        assert_eq!(fmt_compact_current(1.0e-16), "1.00e-16");
        assert_eq!(fmt_current(0.0), "0.00e+00 A");
    }

    #[test]
    fn ratios_never_fall_back_to_exponent_notation() {
        assert_eq!(fmt_ratio(56500.0), "56.5k");
        assert_eq!(fmt_ratio(1.76e13), "17.6T");
        assert_eq!(fmt_ratio(1.0e15), "1P");
        assert_eq!(fmt_ratio(1.0e30), "1Q");
        assert_eq!(fmt_ratio(1.0e33), ">999Q");
        assert_eq!(fmt_ratio(1.0e-18), "<1f");
        assert_eq!(fmt_ratio(f64::NAN), DASH);
    }

    #[test]
    fn fmt_units_and_dashes() {
        assert_eq!(fmt_ohm(17750.0), "17.75k \u{2126}");
        assert_eq!(fmt_ohm(f64::NAN), "\u{2014}");
        assert_eq!(fmt_slope(135.2), "135.2 \u{2126}/\u{00B5}m");
        assert_eq!(fmt_r2(f64::NAN), "\u{2014}");
        assert_eq!(fmt_r2(0.99875), "1.00");
        assert_eq!(fmt_r2(0.936), "0.94");
    }

    #[test]
    fn fmt_vg_trims_zeros() {
        assert_eq!(fmt_vg(-5.0), "-5 V");
        assert_eq!(fmt_vg(-2.5), "-2.5 V");
        assert_eq!(fmt_vg(1.125), "1.125 V");
        assert_eq!(fmt_vg(f64::NAN), "\u{2014}");
        assert_eq!(fmt_num3(-5.0), "-5");
    }

    #[test]
    fn parse_eng_round_trips_fmt_eng_and_plain() {
        // Plain / scientific notation parses straight through `f64`.
        assert_eq!(parse_eng("5"), Some(5.0));
        assert_eq!(parse_eng("-0.353"), Some(-0.353));
        assert_eq!(parse_eng("2.696e-3"), Some(2.696e-3));
        assert_eq!(parse_eng("1.5E-12"), Some(1.5e-12));
        // SI suffixes - exactly what fmt_eng shows in the parameter card - round-trip.
        assert_eq!(parse_eng("2.696m"), Some(2.696e-3)); // MU0 as displayed
        assert_eq!(parse_eng("10n"), Some(1e-8));
        assert_eq!(parse_eng("1p"), Some(1e-12));
        assert_eq!(parse_eng("300\u{00B5}"), Some(3e-4)); // micro sign U+00B5
        assert_eq!(parse_eng("300\u{03BC}"), Some(3e-4)); // Greek mu U+03BC
        assert_eq!(parse_eng("300u"), Some(3e-4)); // ASCII 'u'
        assert_eq!(parse_eng("17.75k"), Some(17750.0));
        assert_eq!(parse_eng("-2.3k"), Some(-2300.0));
        assert_eq!(parse_eng("1T"), Some(1e12));
        assert_eq!(parse_eng("1P"), Some(1e15));
        assert_eq!(parse_eng("1E"), Some(1e18));
        assert_eq!(parse_eng("1Q"), Some(1e30));
        assert_eq!(parse_eng("1f"), Some(1e-15));
        // Case matters: m = milli, M = mega (matches fmt_eng).
        assert_eq!(parse_eng("1m"), Some(1e-3));
        assert_eq!(parse_eng("1M"), Some(1e6));
        // Whitespace between mantissa and suffix is tolerated.
        assert_eq!(parse_eng(" 2.7 m "), Some(2.7e-3));
        // Garbage / non-finite / lone suffix are rejected (None -> caller warns).
        assert_eq!(parse_eng(""), None);
        assert_eq!(parse_eng("   "), None);
        assert_eq!(parse_eng("abc"), None);
        assert_eq!(parse_eng("m"), None);
        assert_eq!(parse_eng("inf"), None);
        assert_eq!(parse_eng("nan"), None);
        assert_eq!(parse_eng("1.2.3k"), None);
    }

    #[test]
    fn non_finite_and_negative_zero() {
        assert_eq!(fmt_eng(f64::INFINITY), DASH);
        assert_eq!(fmt_ohm(f64::NEG_INFINITY), DASH);
        assert_eq!(fmt_num3(-0.0), "0");
        assert_eq!(fmt_vg(-0.0), "0 V");
        // Small negatives that round to zero must also read "0", not "-0".
        assert_eq!(fmt_num3(-0.0004), "0");
        assert_eq!(fmt_vg(-0.0004), "0 V");
        // Below femto (1e-15) fmt_eng still falls back to raw scientific.
        assert_eq!(fmt_eng(1.0e-18), format!("{:.3e}", 1.0e-18));
    }
}
