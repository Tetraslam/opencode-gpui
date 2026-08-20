use gpui::{ClickEvent, Context, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    model::{MessageRecord, Part},
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{
    PartSelection, TimelineState, Workspace, chrome::centered_message, part_format::label,
    timeline_state::RenderState,
};

impl Workspace {
    pub(super) fn render_timeline(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(tab) = self.active_tab() else {
            return centered_message("open a directory");
        };
        match &tab.timeline {
            TimelineState::Empty => centered_message("select a session"),
            TimelineState::Loading { .. } => centered_message("loading timeline"),
            TimelineState::Failed { error, .. } => div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .child(
                    div()
                        .text_color(rgb(color::TEXT))
                        .child("timeline unavailable"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(color::TEXT_DIM))
                        .child(error.clone()),
                )
                .into_any_element(),
            TimelineState::Ready { messages, .. } if messages.is_empty() => {
                centered_message("no messages")
            }
            TimelineState::Ready { messages, .. } => {
                let scroll_handle = tab.timeline_scroll.clone();
                if tab.follow_tail {
                    scroll_handle.scroll_to_bottom();
                }
                let older = (!tab.history_exhausted).then(|| {
                    div()
                        .id("older-messages")
                        .h(px(26.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .bg(rgb(color::SURFACE))
                        .font_family(MONO_FONT)
                        .text_xs()
                        .text_color(rgb(color::TEXT_DIM))
                        .hover(|element| element.text_color(rgb(color::TEXT)))
                        .on_click(cx.listener(|workspace, _event: &ClickEvent, _window, cx| {
                            workspace.load_older_messages(cx);
                        }))
                        .child(if tab.history_loading {
                            "loading older messages..."
                        } else {
                            "load 16 older messages"
                        })
                });
                let event_handle = scroll_handle.clone();
                let render_state = RenderState {
                    expanded_parts: &tab.expanded_parts,
                    selected_part: tab.selected_part.as_ref(),
                    detail_cache: &tab.detail_cache,
                    markdown_cache: &tab.markdown.documents,
                    markdown_renders: &tab.markdown_renders,
                    image_cache: &tab.images.images,
                    directory: &tab.directory,
                };
                let content = div()
                    .id("timeline")
                    .min_w_0()
                    .h_full()
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&scroll_handle)
                    .on_scroll_wheel(cx.listener(move |workspace, event, window, cx| {
                        workspace.handle_timeline_scroll(event, &event_handle, window, cx);
                    }))
                    .children(older)
                    .children(messages.iter().enumerate().map(|(index, message)| {
                        let show_header =
                            index == 0 || messages[index - 1].info.role() != message.info.role();
                        Self::render_message(message, show_header, &render_state, cx)
                    }));
                div()
                    .size_full()
                    .flex()
                    .child(content)
                    .child(super::timeline_scroll::render_scrollbar(&scroll_handle, cx))
                    .into_any_element()
            }
        }
    }

    fn render_message(
        message: &MessageRecord,
        show_header: bool,
        state: &RenderState<'_>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let role: SharedString = message.info.role().into();
        let detail: SharedString = message.info.detail().into();
        let role_color = if message.info.role() == "you" {
            color::BLUE
        } else {
            color::GREEN
        };

        div()
            .id(SharedString::from(message.info.id().to_owned()))
            .border_b_1()
            .border_color(rgb(color::BORDER_SUBTLE))
            .when(show_header, |element| {
                element.child(
                    div()
                        .h(px(ui_size::MESSAGE_HEADER))
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .bg(rgb(color::SURFACE))
                        .font_family(MONO_FONT)
                        .child(label("·", ui_size::MARKER_COL, role_color))
                        .child(label("", 18.0, role_color))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .gap_2()
                                .text_xs()
                                .text_color(rgb(role_color))
                                .child(role)
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .truncate()
                                        .text_color(rgb(color::TEXT_DIM))
                                        .child(detail),
                                ),
                        ),
                )
            })
            .children(
                message
                    .parts
                    .iter()
                    .filter(|part| part.kind != "patch")
                    .filter_map(|part| Self::render_part(part, state, cx)),
            )
            .into_any_element()
    }

    fn render_part(
        part: &Part,
        state: &RenderState<'_>,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        if matches!(
            part.kind.as_str(),
            "step-start" | "step-finish" | "snapshot" | "compaction"
        ) {
            return None;
        }
        let selection = PartSelection {
            message_id: part.message_id.clone(),
            part_id: part.id.clone(),
        };
        let expanded = state.expanded_parts.contains(&selection);
        let selected = state.selected_part == Some(&selection);
        if part.kind == "text" {
            if part
                .data
                .get("synthetic")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                return None;
            }
            let document = state
                .markdown_cache
                .get(&selection)
                .filter(|cached| cached.source == part.text().unwrap_or_default())
                .map(|cached| cached.document.as_ref());
            return Some(Self::render_text_part(
                part,
                selection,
                selected,
                document,
                state.markdown_renders,
                cx,
            ));
        }
        if part.kind == "file"
            && let Some(image) = state.image_cache.get(&selection).filter(|cached| {
                cached.source
                    == part
                        .data
                        .get("url")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
            })
        {
            return Some(Self::render_image_part(
                part,
                selection,
                selected,
                image.image.clone(),
                cx,
            ));
        }
        if part.kind == "file" {
            return Some(Self::render_file_part(part, selection, selected, cx));
        }
        if super::part_format::is_tool_part(part) {
            return Some(Self::render_tool_part(
                part,
                &selection,
                expanded,
                state.detail_cache,
                state.directory,
                cx,
            ));
        }

        Some(Self::render_event_part(
            part, &selection, expanded, state, cx,
        ))
    }
}
