use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{ClickEvent, Context, MouseButton, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    editor::TextEditor,
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{Workspace, command_palette::Overlay, timeline_overlay::TimelineEntry};

impl Workspace {
    pub(super) fn render_timeline_overlay(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        (self.overlay == Overlay::Timeline).then(|| {
            let results =
                div()
                    .id("timeline-results")
                    .min_h_0()
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.picker_scroll)
                    .children(self.timeline_suggestions.iter().enumerate().map(
                        |(index, entry)| timeline_row(entry, index, self.overlay_selection, cx),
                    ));
            div()
                .id("timeline-overlay")
                .absolute()
                .top(px(ui_size::TITLEBAR + 16.0))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(overlay_panel("Timeline", self.command_editor.clone(), cx).child(results))
                .into_any_element()
        })
    }
}

fn overlay_panel(
    title: &'static str,
    editor: gpui::Entity<TextEditor>,
    cx: &mut Context<Workspace>,
) -> gpui::Div {
    div()
        .w(px(760.0))
        .max_h(px(650.0))
        .flex()
        .flex_col()
        .bg(rgb(color::ELEVATED))
        .border_1()
        .border_color(rgb(color::BORDER))
        .shadow_lg()
        .font_family(MONO_FONT)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_, _, _, cx| cx.stop_propagation()),
        )
        .child(
            div()
                .px_3()
                .pt_3()
                .pb_2()
                .text_sm()
                .text_color(rgb(color::TEXT))
                .child(title),
        )
        .child(
            div()
                .px_2()
                .pb_2()
                .border_b_1()
                .border_color(rgb(color::BORDER))
                .child(editor),
        )
}

fn timeline_row(
    entry: &TimelineEntry,
    index: usize,
    selected: usize,
    cx: &mut Context<Workspace>,
) -> gpui::AnyElement {
    let message_id = entry.message_id.clone();
    div()
        .id(SharedString::from(format!("timeline-{message_id}")))
        .min_h(px(44.0))
        .px_3()
        .py_2()
        .flex()
        .items_center()
        .justify_between()
        .cursor_pointer()
        .border_b_1()
        .border_color(rgb(color::BORDER_SUBTLE))
        .when(index == selected, |row| row.bg(rgb(color::SELECTED)))
        .hover(|row| row.bg(rgb(color::HOVER)))
        .on_click(cx.listener(move |workspace, _: &ClickEvent, _, cx| {
            workspace.overlay_selection = index;
            workspace.timeline_message = Some(message_id.clone());
            workspace.preview_timeline_selection();
            workspace.open_message_actions(cx);
        }))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .truncate()
                .text_sm()
                .text_color(rgb(color::TEXT))
                .child(entry.title.clone()),
        )
        .child(
            div()
                .ml_3()
                .flex_none()
                .text_xs()
                .text_color(rgb(color::TEXT_DIM))
                .child(relative_time(entry.created)),
        )
        .into_any_element()
}

fn relative_time(created: u64) -> String {
    let now = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(u64::MAX);
    let seconds = now.saturating_sub(created) / 1_000;
    match seconds {
        0..=59 => "just now".into(),
        60..=3_599 => format!("{}m ago", seconds / 60),
        3_600..=86_399 => format!("{}h ago", seconds / 3_600),
        _ => format!("{}d ago", seconds / 86_400),
    }
}
