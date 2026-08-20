use gpui::{FontWeight, div, prelude::*, px, rgb};
use opencode_gpui::{
    model::{Message, MessageRecord},
    theme::{MONO_FONT, color, size as ui_size},
};

pub(super) fn render(
    message: &MessageRecord,
    messages: &[MessageRecord],
    last: bool,
) -> Option<gpui::AnyElement> {
    let Message::Assistant(assistant) = &message.info else {
        return None;
    };
    let final_message = assistant
        .finish
        .as_deref()
        .is_some_and(|finish| !matches!(finish, "tool-calls" | "unknown"));
    if !last && !final_message {
        return None;
    }
    let duration = assistant.time.completed.and_then(|completed| {
        messages
            .iter()
            .find(|candidate| candidate.info.id() == assistant.parent_id)
            .and_then(|parent| match &parent.info {
                Message::User(user) => Some(completed.saturating_sub(user.time.created)),
                Message::Assistant(_) => None,
            })
    });
    Some(
        div()
            .pl(px(ui_size::TOOL_CONTENT_X))
            .pr_3()
            .pt_2()
            .pb_3()
            .flex()
            .items_center()
            .gap_2()
            .font_family(MONO_FONT)
            .text_xs()
            .text_color(rgb(color::TEXT_MUTED))
            .child(
                div()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(color::GREEN))
                    .child("▣"),
            )
            .child(
                div()
                    .text_color(rgb(color::TEXT))
                    .child(title_case(&assistant.mode)),
            )
            .child(format!(
                "· {}/{}{}",
                assistant.provider_id,
                assistant.model_id,
                duration.map_or_else(String::new, |value| format!(
                    " · {}",
                    format_duration(value)
                ))
            ))
            .into_any_element(),
    )
}

fn title_case(value: &str) -> String {
    let mut characters = value.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(characters).collect()
    })
}

fn format_duration(milliseconds: u64) -> String {
    let seconds = milliseconds / 1_000;
    if seconds < 60 {
        return format!("{seconds}s");
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes}m {}s", seconds % 60);
    }
    format!("{}h {}m", minutes / 60, minutes % 60)
}
