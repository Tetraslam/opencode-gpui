use std::sync::Arc;

use gpui::{Context, CursorStyle, MouseButton, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    model::Part,
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{PartSelection, TimelineState, Workspace, chrome::centered_message};

pub(crate) struct PreparedPart {
    kind: SharedString,
    tool: Option<SharedString>,
    raw: SharedString,
    input: Option<SharedString>,
    output: Option<SharedString>,
    metadata: Option<SharedString>,
    diff: Option<SharedString>,
}

impl PreparedPart {
    fn build(part: &Part) -> Self {
        let raw = serde_json::to_string_pretty(part)
            .unwrap_or_else(|error| format!("serialization error: {error}"));
        let state = part.data.get("state");
        Self {
            kind: part.kind.clone().into(),
            tool: part
                .data
                .get("tool")
                .and_then(serde_json::Value::as_str)
                .map(|tool| SharedString::from(tool.to_owned())),
            raw: raw.into(),
            input: state
                .and_then(|state| state.get("input"))
                .map(|value| pretty_json(value).into()),
            output: state
                .and_then(|state| state.get("output"))
                .and_then(serde_json::Value::as_str)
                .map(|output| output.to_owned().into()),
            metadata: state
                .and_then(|state| state.get("metadata"))
                .map(|value| pretty_json(value).into()),
            diff: state
                .and_then(|state| state.get("metadata"))
                .and_then(|metadata| metadata.get("diff"))
                .and_then(serde_json::Value::as_str)
                .map(|diff| SharedString::from(diff.to_owned())),
        }
    }
}

impl Workspace {
    pub(super) fn select_part(
        &mut self,
        selection: PartSelection,
        part: Part,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.active_tab_mut() else {
            return;
        };
        tab.selected_part = Some(selection.clone());
        if tab.detail_cache.contains_key(&selection)
            || !tab.preparing_parts.insert(selection.clone())
        {
            cx.notify();
            return;
        }
        let directory = tab.directory.clone();

        let preparation = cx.background_spawn(async move { Arc::new(PreparedPart::build(&part)) });
        let task = cx.spawn(async move |workspace, cx| {
            let prepared = preparation.await;
            let _ = workspace.update(cx, |workspace, cx| {
                let Some(tab) = workspace
                    .tabs
                    .iter_mut()
                    .find(|tab| tab.directory == directory)
                else {
                    return;
                };
                tab.preparing_parts.remove(&selection);
                if tab.expanded_parts.contains(&selection) && !tab.follow_tail {
                    tab.pending_detail_anchor = Some(tab.timeline_scroll.max_offset().height);
                }
                tab.detail_cache.insert(selection, prepared);
                cx.notify();
            });
        });
        if let Some(tab) = self.active_tab_mut() {
            tab.detail_tasks.push(task);
        }
        cx.notify();
    }

    pub(super) fn render_inspector(&self) -> gpui::AnyElement {
        let selection = self.active_tab().and_then(|tab| tab.selected_part.as_ref());
        let part = selection.and_then(|selection| self.find_part(selection));
        let prepared = selection.and_then(|selection| {
            self.active_tab()
                .and_then(|tab| tab.detail_cache.get(selection))
        });
        let body = if selection.is_none() {
            centered_message("select a part")
        } else {
            render_part_detail(prepared.map(Arc::as_ref), true)
        };

        div()
            .w(self.inspector_width)
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(rgb(color::BORDER))
            .child(
                div()
                    .h(px(ui_size::PANE_HEADER))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .bg(rgb(color::SURFACE))
                    .border_b_1()
                    .border_color(rgb(color::BORDER))
                    .font_family(MONO_FONT)
                    .text_xs()
                    .text_color(rgb(color::TEXT_DIM))
                    .child("inspector")
                    .child(part.map_or_else(
                        || SharedString::from("--"),
                        |part| SharedString::from(part.kind.clone()),
                    )),
            )
            .child(div().min_h_0().flex_1().child(body))
            .into_any_element()
    }

    pub(super) fn render_inspector_resize_handle(cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .id("inspector-resize")
            .w(px(5.0))
            .h_full()
            .flex_none()
            .cursor(CursorStyle::ResizeLeftRight)
            .hover(|handle| handle.bg(rgb(color::BORDER)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|workspace, _, _, cx| {
                    workspace.pane_resize = super::pane_resize::PaneResize::Inspector;
                    cx.notify();
                }),
            )
            .into_any_element()
    }

    fn find_part(&self, selection: &PartSelection) -> Option<&Part> {
        let TimelineState::Ready { messages, .. } = &self.active_tab()?.timeline else {
            return None;
        };
        messages
            .iter()
            .find(|message| message.info.id() == selection.message_id)
            .and_then(|message| {
                message
                    .parts
                    .iter()
                    .find(|part| part.id == selection.part_id)
            })
    }
}

pub(super) fn render_part_detail(
    prepared: Option<&PreparedPart>,
    inspector: bool,
) -> gpui::AnyElement {
    let Some(prepared) = prepared else {
        return centered_message("preparing detail...");
    };
    let tool = prepared.tool.as_ref().map(AsRef::as_ref);
    let patch = matches!(tool, Some("apply_patch" | "patch"));
    let content = if patch {
        detail_section(
            "diff",
            prepared
                .diff
                .clone()
                .unwrap_or_else(|| "preparing patch...".into()),
        )
    } else if tool == Some("bash") {
        detail_section(
            "output",
            prepared
                .output
                .clone()
                .unwrap_or_else(|| "(no output)".into()),
        )
    } else if prepared.kind.as_ref() == "tool" {
        div()
            .child(detail_section(
                "input",
                prepared.input.clone().unwrap_or_else(|| "{}".into()),
            ))
            .child(detail_section(
                "output",
                prepared
                    .output
                    .clone()
                    .unwrap_or_else(|| "(no output)".into()),
            ))
            .child(detail_section(
                "metadata",
                prepared.metadata.clone().unwrap_or_else(|| "{}".into()),
            ))
            .into_any_element()
    } else {
        detail_section("raw", prepared.raw.clone())
    };

    div()
        .id(SharedString::from(format!(
            "detail-{}-{inspector}",
            prepared.kind
        )))
        .when(inspector, |element| element.size_full().overflow_scroll())
        .when(inspector, gpui::Styled::px_3)
        .when(!inspector, |detail| {
            detail.pl(px(ui_size::TOOL_CONTENT_X)).pr_3()
        })
        .pb_3()
        .bg(rgb(if inspector {
            color::SURFACE
        } else {
            color::ELEVATED
        }))
        .child(content)
        .into_any_element()
}

fn detail_section(label: &'static str, content: SharedString) -> gpui::AnyElement {
    div()
        .mt_2()
        .p_3()
        .overflow_hidden()
        .rounded_sm()
        .bg(rgb(color::ELEVATED))
        .border_1()
        .border_color(rgb(color::BORDER_SUBTLE))
        .child(
            div()
                .mb_1()
                .font_family(MONO_FONT)
                .text_xs()
                .text_color(rgb(color::TEXT_MUTED))
                .child(label),
        )
        .child(
            div()
                .font_family(MONO_FONT)
                .text_xs()
                .line_height(px(17.0))
                .whitespace_normal()
                .overflow_hidden()
                .text_color(rgb(color::TEXT_DIM))
                .child(content),
        )
        .into_any_element()
}

fn pretty_json(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}
