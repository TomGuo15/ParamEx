//! Source-grep lints that enforce the GUI contract across the crate: production
//! panels must route widgets, text voices, colors, and shared copy through the
//! kits (`ui_kit`, `table_kit`, `theme`, `format_ui`) instead of reaching for
//! egui primitives or ad hoc values.

use std::path::Path;

use crate::common::{crate_file, read_crate_file, visit_rs_files};

fn is_ui_kit_source(src: &Path, path: &Path) -> bool {
    let rel = path.strip_prefix(src).unwrap_or(path);
    rel == Path::new("ui_kit.rs") || rel.starts_with(Path::new("ui_kit"))
}

/// Visit every production file that renders panel content (the brand bar and
/// the workspace panels/selector), excluding the kits themselves.
fn visit_production_panel_files(mut f: impl FnMut(&Path, &str)) {
    for root in [
        "src/app/brand_bar.rs",
        "src/workspaces/transfer/panels",
        "src/workspaces/transfer/selector",
        "src/workspaces/tlm/panels",
    ] {
        let path = crate_file(root);
        if path.is_file() {
            let text = read_crate_file(root);
            f(&path, &text);
        } else {
            visit_rs_files(&path, |path, text| f(path, text));
        }
    }
}

#[test]
fn production_buttons_use_ui_kit_entry_points() {
    let src = crate_file("src");
    let mut violations = Vec::new();
    visit_rs_files(&src, |path, text| {
        if is_ui_kit_source(&src, path) {
            return;
        }
        for (idx, line) in text.lines().enumerate() {
            if line.contains("egui::Button::new") || line.contains(".button(") {
                violations.push(format!("{}:{}: {}", path.display(), idx + 1, line.trim()));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "production buttons must route through ui_kit for consistent color, type, height, and state:\n{}",
        violations.join("\n")
    );
}

#[test]
fn production_text_inputs_use_ui_kit_entry_point() {
    let src = crate_file("src");
    let mut violations = Vec::new();
    visit_rs_files(&src, |path, text| {
        if is_ui_kit_source(&src, path) {
            return;
        }
        if text.contains("TextEdit::singleline") {
            violations.push(path.display().to_string());
        }
    });

    assert!(
        violations.is_empty(),
        "production text inputs must route through ui_kit::singleline_edit for consistent field styling:\n{}",
        violations.join("\n")
    );
}

#[test]
fn production_panel_markup_labels_use_shared_typography_recipes() {
    let mut violations = Vec::new();
    visit_production_panel_files(|path, text| {
        if text.contains("richtext::rich_label") {
            violations.push(path.display().to_string());
        }
    });

    assert!(
        violations.is_empty(),
        "panel markup labels must route through ui_kit or table_kit recipes instead of body-default richtext:\n{}",
        violations.join("\n")
    );
}

#[test]
fn production_panels_do_not_wrap_ui_kit_text_with_local_labels() {
    let mut violations = Vec::new();
    visit_production_panel_files(|path, text| {
        for (idx, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.contains("ui.label(ui_kit::") {
                violations.push(format!("{}:{}: {line}", path.display(), idx + 1));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "panel text should use ui_kit label helpers rather than locally wrapping ui_kit RichText:\n{}",
        violations.join("\n")
    );
}

#[test]
fn production_panels_use_shared_right_aligned_layout() {
    let mut violations = Vec::new();
    visit_production_panel_files(|path, text| {
        for (idx, line) in text.lines().enumerate() {
            if line.contains("with_layout(egui::Layout::right_to_left(egui::Align::Center)") {
                violations.push(format!("{}:{}: {}", path.display(), idx + 1, line.trim()));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "production panels must use ui_kit::right_aligned for right-pinned row layout:\n{}",
        violations.join("\n")
    );
}

#[test]
fn utility_alpha_variants_are_centralized_in_theme() {
    let mut violations = Vec::new();
    let src = crate_file("src");
    visit_rs_files(&src, |path, text| {
        if path.file_name().is_some_and(|name| name == "theme.rs") {
            return;
        }
        for forbidden in ["from_white_alpha", "from_black_alpha"] {
            if text.contains(forbidden) {
                violations.push(format!("{}: {forbidden}", path.display()));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "Utility white/black alpha variants should route through theme helpers:\n{}",
        violations.join("\n")
    );
}

#[test]
fn source_mentions_only_approved_palette_hex_values() {
    let approved = [
        "#000F30", "#003CFF", "#CDDEE5", "#F8E7B9", "#DF83A7", "#9BE5D4", "#B7B9FA", "#FFFFFF",
        "#000000", "#606060", "#888888",
    ];
    let mut violations = Vec::new();
    let src = crate_file("src");
    visit_rs_files(&src, |path, text| {
        for hex in six_digit_hex_mentions(text) {
            if !approved.contains(&hex.as_str()) {
                violations.push(format!("{}: {hex}", path.display()));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "off-palette hex mention(s):\n{}",
        violations.join("\n")
    );
}

#[test]
fn source_uses_palette_tokens_instead_of_ad_hoc_color_helpers() {
    let mut violations = Vec::new();
    let src = crate_file("src");
    visit_rs_files(&src, |path, text| {
        for forbidden in ["gamma_multiply", "linear_multiply", "shade(", "from_gray("] {
            if text.contains(forbidden) {
                violations.push(format!("{}: {forbidden}", path.display()));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "runtime color states should use palette tokens or approved alpha variants, not RGB/gray ad hoc helpers:\n{}",
        violations.join("\n")
    );
}

fn six_digit_hex_mentions(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 7 <= bytes.len() {
        if bytes[i] == b'#'
            && bytes[i + 1..i + 7].iter().all(u8::is_ascii_hexdigit)
            && bytes.get(i + 7).is_none_or(|b| !b.is_ascii_hexdigit())
        {
            out.push(text[i..i + 7].to_ascii_uppercase());
            i += 7;
        } else {
            i += 1;
        }
    }
    out
}

/// Headless GPU work must go through the locked helpers in `tests/common`
/// (`render`, `snapshot`) or sit inside a test that holds `wgpu_guard()` for
/// its whole body. A bare harness render in a parallel test binary crashes the
/// process intermittently, which is far harder to diagnose than a lint.
#[test]
fn gpu_renders_route_through_the_locked_test_helpers() {
    let tests = crate_file("tests");
    let mut violations = Vec::new();
    visit_rs_files(&tests, |path, text| {
        let rel = path.strip_prefix(&tests).unwrap_or(path);
        if rel == Path::new("common").join("mod.rs") {
            return;
        }
        if rel == Path::new("app_snapshot.rs") {
            // The snapshot binary holds the guard per test instead, so its
            // multi-frame snapshot pairs cannot interleave with each other.
            for (idx, body) in text.split("#[test]").enumerate().skip(1) {
                if !body.contains("wgpu_guard()") {
                    violations.push(format!(
                        "{}: test #{idx} does not hold wgpu_guard()",
                        path.display()
                    ));
                }
            }
            return;
        }
        // Built at runtime so this file does not match its own needles.
        let needles = [format!(".{}()", "render"), format!(".{}(", "snapshot")];
        for (idx, line) in text.lines().enumerate() {
            if needles.iter().any(|needle| line.contains(needle.as_str())) {
                violations.push(format!("{}:{}: {}", path.display(), idx + 1, line.trim()));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "GPU renders must use common::render / common::snapshot (or hold wgpu_guard):\n{}",
        violations.join("\n")
    );
}

#[test]
fn shared_ui_copy_is_not_duplicated_outside_format_ui() {
    let needles = [
        "W and L must be numeric.",
        "Output fit failed.",
        "Attached pending output.",
        "Removed pending output.",
        "Output moved to pending.",
        "Removed output.",
        "Cleared error row(s).",
        "Transfer output: {attached} attached,",
        "Removed {count} {noun}(s).",
    ];
    let mut violations = Vec::new();

    visit_rs_files(crate_file("src"), |path, source| {
        let normalized = path.to_string_lossy().replace('\\', "/");
        if normalized.ends_with("/src/format_ui.rs") || normalized.contains("/src/format_ui/") {
            return;
        }
        for (line_idx, line) in source.lines().enumerate() {
            for needle in needles {
                if line.contains(needle) {
                    violations.push(format!(
                        "{}:{} duplicates shared UI copy `{needle}`",
                        path.display(),
                        line_idx + 1
                    ));
                }
            }
        }
    });

    assert!(
        violations.is_empty(),
        "shared user-facing copy belongs in format_ui, independent of caller layout:\n{}",
        violations.join("\n")
    );
}
