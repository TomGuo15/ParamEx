//! Minimal HTML-subset rich-text facade for the GUI's `<sub>`/`<sup>` markup.

use eframe::egui::{self, text::LayoutJob, Align, Color32, FontId, TextFormat};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Script {
    Normal,
    Sub,
    Sup,
}

const SCRIPT_SCALE: f32 = 0.72;

/// Build a `LayoutJob` from `<sub>`/`<sup>`-marked `markup`. Any other `<...>` tag
/// is consumed: it is removed from the text and leaves the current script level
/// unchanged, so callers must never rely on it surviving. A `<` with no closing
/// `>` is not a tag and is emitted literally.
pub fn layout_sub_sup(markup: &str, base: FontId, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let small = FontId::new(base.size * SCRIPT_SCALE, base.family.clone());
    let mut script = Script::Normal;
    let mut rest = markup;
    while !rest.is_empty() {
        match rest.find('<') {
            Some(idx) => {
                if idx > 0 {
                    append_run(&mut job, &rest[..idx], script, &base, &small, color);
                }
                match rest[idx..].find('>') {
                    Some(rel_end) => {
                        let tag = &rest[idx + 1..idx + rel_end];
                        script = match tag {
                            "sub" => Script::Sub,
                            "sup" => Script::Sup,
                            "/sub" | "/sup" => Script::Normal,
                            _ => script,
                        };
                        rest = &rest[idx + rel_end + 1..];
                    }
                    None => {
                        append_run(&mut job, &rest[idx..], script, &base, &small, color);
                        break;
                    }
                }
            }
            None => {
                append_run(&mut job, rest, script, &base, &small, color);
                break;
            }
        }
    }
    job
}

fn append_run(
    job: &mut LayoutJob,
    text: &str,
    script: Script,
    base: &FontId,
    small: &FontId,
    color: Color32,
) {
    if text.is_empty() {
        return;
    }
    let (font_id, valign) = match script {
        Script::Normal => (base.clone(), Align::Center),
        Script::Sub => (small.clone(), Align::BOTTOM),
        Script::Sup => (small.clone(), Align::TOP),
    };
    job.append(
        text,
        0.0,
        TextFormat {
            font_id,
            color,
            valign,
            ..Default::default()
        },
    );
}

/// The tag-stripped plain text of `markup`, exactly the `LayoutJob::text` the
/// renderer produces.
pub fn strip_markup(markup: &str) -> String {
    layout_sub_sup(markup, FontId::default(), Color32::BLACK).text
}

/// Render `markup` as a `<sub>`/`<sup>`-aware label using the ui's body font + text
/// color. Use for table cells and metric values.
pub fn rich_label(ui: &mut egui::Ui, markup: &str) -> egui::Response {
    let color = ui.visuals().text_color();
    rich_colored(ui, markup, color)
}

/// `rich_label` with an explicit color (e.g. the muted ink for metric labels).
pub fn rich_colored(ui: &mut egui::Ui, markup: &str, color: Color32) -> egui::Response {
    let base = egui::TextStyle::Body.resolve(ui.style());
    ui.label(layout_sub_sup(markup, base, color))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(markup: &str) -> LayoutJob {
        layout_sub_sup(markup, FontId::proportional(14.0), Color32::BLACK)
    }

    #[test]
    fn plain_text_is_one_run() {
        let j = job("Sweep");
        assert_eq!(j.text, "Sweep");
        assert_eq!(j.sections.len(), 1);
    }

    #[test]
    fn subscript_strips_tags_and_shrinks() {
        let j = job("V<sub>TH</sub>");
        assert_eq!(j.text, "VTH");
        assert_eq!(j.sections.len(), 2);
        assert!(j.sections[1].format.font_id.size < j.sections[0].format.font_id.size);
        assert_eq!(j.sections[0].format.valign, Align::Center);
        assert_eq!(j.sections[1].format.valign, Align::BOTTOM);
    }

    #[test]
    fn superscript_is_raised() {
        let j = job("cm<sup>2</sup>");
        assert_eq!(j.text, "cm2");
        assert_eq!(j.sections.len(), 2);
        assert_eq!(j.sections[1].format.valign, Align::TOP);
    }

    #[test]
    fn mixed_units_round_trip() {
        let j = job("µ<sub>sat</sub> (cm<sup>2</sup> V<sup>-1</sup> s<sup>-1</sup>)");
        assert_eq!(j.text, "µsat (cm2 V-1 s-1)");
        assert_eq!(j.sections.len(), 9);
    }

    #[test]
    fn power_of_ten_markup() {
        let j = job("3 × 10<sup>6</sup>");
        assert_eq!(j.text, "3 × 106");
        assert_eq!(j.sections.last().unwrap().format.valign, Align::TOP);
    }

    #[test]
    fn unclosed_tag_is_literal() {
        let j = job("a < b");
        assert_eq!(j.text, "a < b");
    }

    #[test]
    fn unrecognised_tag_is_removed_without_changing_script() {
        let j = job("V<b>TH</b><sub>x</sub>");
        assert_eq!(j.text, "VTHx");
        assert_eq!(j.sections[0].format.valign, Align::Center);
        assert_eq!(j.sections.last().unwrap().format.valign, Align::BOTTOM);
    }
}
