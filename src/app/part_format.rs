use gpui::{SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{model::Part, theme::color};

pub(super) fn label(text: &str, width: f32, color_value: u32) -> gpui::AnyElement {
    div()
        .w(px(width))
        .flex_none()
        .text_xs()
        .text_color(rgb(color_value))
        .child(SharedString::from(text.to_owned()))
        .into_any_element()
}

pub(super) fn kind_color(kind: &str) -> u32 {
    match kind {
        "tool" => color::TOOL,
        "reasoning" => color::REASONING,
        "file" | "patch" => color::CYAN,
        "subtask" | "agent" => color::BLUE,
        "retry" => color::RED,
        _ => color::TEXT_MUTED,
    }
}

pub(super) fn one_line_summary(part: &Part) -> String {
    let summary = part
        .summary()
        .unwrap_or_else(|| format!("{} event", part.kind));
    let normalized = summary.replace('\n', " ");
    let mut chars = normalized.chars();
    let preview = chars.by_ref().take(180).collect::<String>();
    if chars.next().is_some() {
        format!("{preview}...")
    } else {
        preview
    }
}
