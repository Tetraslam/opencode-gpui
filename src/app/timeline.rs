use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use gpui::{ClickEvent, Context, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    model::{MessageRecord, Part},
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{
    PartSelection, TimelineState, Workspace,
    chrome::centered_message,
    inspector::{self, PreparedPart},
    part_format::{kind_color, label, one_line_summary},
};

impl Workspace {
    fn toggle_part(&mut self, selection: PartSelection, part: Part, cx: &mut Context<Self>) {
        self.select_part(selection.clone(), part, cx);
        if !self.expanded_parts.remove(&selection) {
            self.expanded_parts.insert(selection);
        }
        cx.notify();
    }

    pub(super) fn render_timeline(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        match &self.timeline {
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
                let older = (!self.history_exhausted).then(|| {
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
                        .child(if self.history_loading {
                            "loading older messages..."
                        } else {
                            "load 16 older messages"
                        })
                });
                div()
                    .id("timeline")
                    .size_full()
                    .overflow_y_scroll()
                    .children(older)
                    .children(messages.iter().enumerate().map(|(index, message)| {
                        let show_header =
                            index == 0 || messages[index - 1].info.role() != message.info.role();
                        Self::render_message(
                            message,
                            show_header,
                            &self.expanded_parts,
                            self.selected_part.as_ref(),
                            &self.detail_cache,
                            cx,
                        )
                    }))
                    .into_any_element()
            }
        }
    }

    fn render_message(
        message: &MessageRecord,
        show_header: bool,
        expanded_parts: &HashSet<PartSelection>,
        selected_part: Option<&PartSelection>,
        detail_cache: &HashMap<PartSelection, Arc<PreparedPart>>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let role: SharedString = message.info.role().to_uppercase().into();
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
                        .child(
                            div()
                                .w(px(68.0))
                                .flex_none()
                                .text_xs()
                                .text_color(rgb(role_color))
                                .child(role),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .truncate()
                                .text_xs()
                                .text_color(rgb(color::TEXT_DIM))
                                .child(detail),
                        ),
                )
            })
            .children(message.parts.iter().filter_map(|part| {
                Self::render_part(part, expanded_parts, selected_part, detail_cache, cx)
            }))
            .into_any_element()
    }

    fn render_part(
        part: &Part,
        expanded_parts: &HashSet<PartSelection>,
        selected_part: Option<&PartSelection>,
        detail_cache: &HashMap<PartSelection, Arc<PreparedPart>>,
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
        let expanded = expanded_parts.contains(&selection);
        let selected = selected_part == Some(&selection);
        if part.kind == "text" {
            return Some(Self::render_text_part(part, selection, selected, cx));
        }

        let click_selection = selection.clone();
        let click_part = part.clone();
        Some(
            div()
                .id(SharedString::from(part.id.clone()))
                .border_l_2()
                .border_color(rgb(if selected {
                    kind_color(&part.kind)
                } else {
                    color::BORDER_SUBTLE
                }))
                .child(
                    div()
                        .id(SharedString::from(format!("head-{}", part.id)))
                        .h(px(26.0))
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .cursor_pointer()
                        .hover(|element| element.bg(rgb(color::HOVER)))
                        .on_click(cx.listener(
                            move |workspace, _event: &ClickEvent, _window, cx| {
                                workspace.toggle_part(
                                    click_selection.clone(),
                                    click_part.clone(),
                                    cx,
                                );
                            },
                        ))
                        .font_family(MONO_FONT)
                        .child(label(
                            if expanded { "v" } else { ">" },
                            10.0,
                            color::TEXT_MUTED,
                        ))
                        .child(label(
                            &part.kind.to_uppercase(),
                            82.0,
                            kind_color(&part.kind),
                        ))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .truncate()
                                .text_xs()
                                .text_color(rgb(color::TEXT_DIM))
                                .child(one_line_summary(part)),
                        ),
                )
                .when(expanded, |element| {
                    element.child(inspector::render_part_detail(
                        detail_cache.get(&selection).map(Arc::as_ref),
                        false,
                    ))
                })
                .into_any_element(),
        )
    }

    fn render_text_part(
        part: &Part,
        selection: PartSelection,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let click_part = part.clone();
        div()
            .id(SharedString::from(part.id.clone()))
            .px_3()
            .py_2()
            .border_l_2()
            .border_color(rgb(if selected {
                color::BLUE
            } else {
                color::BORDER_SUBTLE
            }))
            .cursor_pointer()
            .hover(|element| element.bg(rgb(color::HOVER)))
            .on_click(
                cx.listener(move |workspace, _event: &ClickEvent, _window, cx| {
                    workspace.select_part(selection.clone(), click_part.clone(), cx);
                }),
            )
            .text_sm()
            .line_height(px(20.0))
            .text_color(rgb(color::TEXT))
            .child(SharedString::from(
                part.text().unwrap_or_default().to_owned(),
            ))
            .into_any_element()
    }
}
