//! `table_kit::fit_fill_widths` contract: a vertical-only fill table
//! must never cut text at the card edge — when the ideal padded/floored widths
//! overflow the card, the decorations (floors, padding) yield before the text.
//!
//! The "tight" numbers below are real SELECTED dual-sweep measurements:
//! galleys [91.3, 64.0, 64.0], mins [110, 56, 56], item_spacing.x 8. A card_w
//! of 243 (the pill-row-derived inner width; the live call site passes ~252)
//! sits far below the 278px ideal widths, so the tight path engages and the
//! Backward column must still show its whole value ("1.00e-16 A" keeps its
//! unit).

use paramex_gui::table_kit::{
    fit_fill_widths, fit_yielding_widths, group_separator_stroke, header_rule_stroke,
    table_measure_text_color, CELL_PAD_X, GROUP_SEPARATOR_ALPHA,
};

const GAP: f32 = 8.0;

fn total(widths: &[f32], gap: f32) -> f32 {
    widths.iter().sum::<f32>() + (widths.len().saturating_sub(1)) as f32 * gap
}

#[test]
fn loose_content_scales_up_to_fill_the_card() {
    let w = fit_fill_widths(&[40.0, 20.0], &[56.0, 56.0], 300.0, GAP);
    assert!(
        (total(&w, GAP) - 300.0).abs() < 0.01,
        "narrow content must span the full card: {w:?}"
    );
    // Proportional scaling keeps the floored widths' ratio (here equal floors).
    assert!((w[0] - w[1]).abs() < 0.01, "{w:?}");
}

#[test]
fn tight_content_keeps_every_value_readable_inside_the_card() {
    // The real SELECTED numbers that produced the overflow.
    let galleys = [91.3, 64.0, 64.0];
    let w = fit_fill_widths(&galleys, &[110.0, 56.0, 56.0], 243.0, GAP);
    assert!(
        total(&w, GAP) <= 243.01,
        "tight table must fit the card: {w:?} -> {}",
        total(&w, GAP)
    );
    for (w, g) in w.iter().zip(galleys) {
        assert!(
            *w >= g + 2.0 - 0.01,
            "every column must hold its text + the 2px pad floor: {w} < {g}+2"
        );
    }
}

#[test]
fn oversized_floors_yield_instead_of_overflowing() {
    // Floors alone would overflow (200+200 > 292) while the text is tiny: the
    // floors drop, the pad caps at CELL_PAD_X, and the table fills the card.
    let w = fit_fill_widths(&[10.0, 10.0], &[200.0, 200.0], 300.0, GAP);
    assert!(
        (total(&w, GAP) - 300.0).abs() < 0.01,
        "content fits, so the table must fill the card exactly: {w:?}"
    );
}

#[test]
fn hopeless_overflow_falls_back_to_horizontal_scroll() {
    // Even the 2px pad floor cannot fit 400px of text in a 100px card: widths
    // keep the text readable and the caller's h-scroll takes over.
    let galleys = [200.0, 200.0];
    let w = fit_fill_widths(&galleys, &[56.0, 56.0], 100.0, GAP);
    for (w, g) in w.iter().zip(galleys) {
        assert!(
            *w >= g + 2.0 - 0.01,
            "text must never be squeezed: {w} < {g}+2"
        );
    }
}

#[test]
fn empty_and_single_column_edge_cases() {
    assert!(fit_fill_widths(&[], &[], 243.0, GAP).is_empty());
    let w = fit_fill_widths(&[50.0], &[56.0], 243.0, GAP);
    assert_eq!(w.len(), 1);
    assert!(
        (w[0] - 243.0).abs() < 0.01,
        "single column fills the card: {w:?}"
    );
    // Pad never exceeds the loose CELL_PAD_X even with huge slack.
    let tight = fit_fill_widths(&[300.0, 10.0], &[56.0, 56.0], 243.0, GAP);
    assert!(tight[1] <= 10.0 + CELL_PAD_X + 0.01, "{tight:?}");
}

#[test]
fn yielding_columns_shrink_before_the_table_scrolls() {
    let mut widths = vec![100.0, 200.0];
    fit_yielding_widths(&mut widths, &[80.0, 50.0], &[false, true], 220.0, GAP);

    assert!(
        (total(&widths, GAP) - 220.0).abs() < 0.01,
        "yielding column should shrink to the leftover card width: {widths:?}"
    );
    assert!((widths[0] - 100.0).abs() < 0.01, "{widths:?}");
    assert!((widths[1] - 112.0).abs() < 0.01, "{widths:?}");
}

#[test]
fn yielding_columns_keep_their_floor_when_fixed_columns_already_fill_the_card() {
    let mut widths = vec![180.0, 200.0];
    fit_yielding_widths(&mut widths, &[80.0, 50.0], &[false, true], 220.0, GAP);

    assert!(
        total(&widths, GAP) > 220.0,
        "unavoidable overflow should be left for horizontal scroll: {widths:?}"
    );
    assert!((widths[0] - 180.0).abs() < 0.01, "{widths:?}");
    assert!((widths[1] - 50.0).abs() < 0.01, "{widths:?}");
}

#[test]
fn non_yielding_tables_still_fill_available_width() {
    let mut widths = vec![40.0, 40.0];
    fit_yielding_widths(&mut widths, &[20.0, 20.0], &[false, false], 200.0, GAP);

    assert!(
        (total(&widths, GAP) - 200.0).abs() < 0.01,
        "non-yielding narrow tables should still fill the card: {widths:?}"
    );
}

#[test]
fn table_helpers_use_palette_tokens_for_measurement_and_rules() {
    let t = paramex_gui::theme::tokens();
    assert_eq!(table_measure_text_color(), t.ink);
    assert_eq!(header_rule_stroke().color, t.border);
    assert_eq!(header_rule_stroke().width, 1.0);
    assert_eq!(
        group_separator_stroke().color,
        paramex_gui::theme::token_alpha(t.border, GROUP_SEPARATOR_ALPHA)
    );
    assert_eq!(group_separator_stroke().width, 1.0);
}
