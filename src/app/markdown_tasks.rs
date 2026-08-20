use gpui::{div, prelude::*, px, rgb};
use opencode_gpui::{
    markdown::{Inline, TaskState},
    theme::color,
};

pub(super) fn item(item: &Inline) -> Option<(TaskState, Inline)> {
    let state = item
        .spans
        .iter()
        .find(|span| span.range.start == 0)
        .and_then(|span| span.style.task)?;
    let prefix = item
        .text
        .get(..4)
        .filter(|text| matches!(*text, "[ ] " | "[x] " | "[X] " | "[.] " | "[-] "))?;
    let mut content = Inline {
        text: item.text[prefix.len()..].to_owned(),
        spans: item
            .spans
            .iter()
            .filter(|span| span.range.end > prefix.len() && span.style.task.is_none())
            .cloned()
            .map(|mut span| {
                span.range.start = span.range.start.saturating_sub(prefix.len());
                span.range.end -= prefix.len();
                span
            })
            .collect(),
    };
    content.spans.sort_unstable_by_key(|span| span.range.start);
    Some((state, content))
}

pub(super) const fn marker(state: TaskState) -> &'static str {
    match state {
        TaskState::Checked => "v",
        TaskState::Active => ".",
        TaskState::Cancelled => "-",
        TaskState::Pending => "",
    }
}

pub(super) fn checkbox(state: Option<TaskState>, marker: String) -> gpui::AnyElement {
    let Some(state) = state else {
        return div()
            .w(px(24.0))
            .flex_none()
            .text_right()
            .text_color(rgb(color::TEXT_MUTED))
            .child(marker)
            .into_any_element();
    };
    div()
        .w(px(24.0))
        .h(px(20.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .size(px(14.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .border_1()
                .border_color(rgb(task_color(state)))
                .when(state == TaskState::Checked, |box_| {
                    box_.bg(rgb(color::GREEN)).text_color(rgb(color::BASE))
                })
                .text_xs()
                .text_color(rgb(task_color(state)))
                .child(marker),
        )
        .into_any_element()
}

const fn task_color(state: TaskState) -> u32 {
    match state {
        TaskState::Checked => color::GREEN,
        TaskState::Active => color::YELLOW,
        TaskState::Pending => color::TEXT_DIM,
        TaskState::Cancelled => color::TEXT_MUTED,
    }
}
