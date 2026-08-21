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
    tool_label: Option<SharedString>,
    raw: SharedString,
    input: Option<SharedString>,
    output: Option<SharedString>,
    metadata: Option<SharedString>,
    diff_lines: Option<Vec<super::diff_view::DiffLine>>,
}

impl PreparedPart {
    fn build(part: &Part) -> Self {
        let raw = serde_json::to_string_pretty(part)
            .unwrap_or_else(|error| format!("serialization error: {error}"));
        let state = part.data.get("state");
        let tool = part.data.get("tool").and_then(serde_json::Value::as_str);
        let diff = state
            .and_then(|state| state.get("metadata"))
            .and_then(|metadata| metadata.get("diff"))
            .and_then(serde_json::Value::as_str);
        Self {
            kind: part.kind.clone().into(),
            tool: tool.map(|tool| SharedString::from(tool.to_owned())),
            tool_label: tool.map(|tool| super::part_format::tool_display_name(tool).into()),
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
            diff_lines: diff.map(super::diff_view::parse_diff),
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
        let Some(directory) = self.active_directory().map(str::to_owned) else {
            return;
        };
        if let Some(tab) = self.active_tab_mut() {
            tab.selected_part = Some(selection.clone());
        }
        self.prepare_part_detail(&directory, selection, part, cx);
    }

    pub(super) fn prepare_part_detail(
        &mut self,
        directory: &str,
        selection: PartSelection,
        part: Part,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        let cache_current = tab.detail_cache.get(&selection).is_some_and(|prepared| {
            !super::part_format::produces_diff(&part) || prepared.diff_lines.is_some()
        });
        if cache_current || !tab.preparing_parts.insert(selection.clone()) {
            cx.notify();
            return;
        }
        let owner_directory = directory.to_owned();
        let task_directory = owner_directory.clone();

        let preparation = cx.background_spawn(async move { Arc::new(PreparedPart::build(&part)) });
        let task = cx.spawn(async move |workspace, cx| {
            let prepared = preparation.await;
            let _ = workspace.update(cx, |workspace, cx| {
                let Some(tab) = workspace
                    .tabs
                    .iter_mut()
                    .find(|tab| tab.directory == task_directory)
                else {
                    return;
                };
                tab.preparing_parts.remove(&selection);
                tab.detail_cache.insert(selection, prepared);
                cx.notify();
            });
        });
        if let Some(tab) = self
            .tabs
            .iter_mut()
            .find(|tab| tab.directory == owner_directory)
        {
            tab.detail_tasks.push(task);
        }
        cx.notify();
    }

    pub(super) fn render_inspector(&self, width: gpui::Pixels) -> gpui::AnyElement {
        let selection = self.active_tab().and_then(|tab| tab.selected_part.as_ref());
        let part = selection.and_then(|selection| self.find_part(selection));
        let prepared = selection.and_then(|selection| {
            self.active_tab()
                .and_then(|tab| tab.detail_cache.get(selection))
        });
        let detail = selection.map(|_| render_part_detail(prepared.map(Arc::as_ref), true));

        div()
            .w(width)
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(rgb(color::SURFACE))
            .border_l_1()
            .border_color(rgb(color::BORDER))
            .child(super::chrome::pane_header(
                "session".into(),
                part.map_or_else(|| "--".to_owned(), |part| part.kind.clone()),
            ))
            .child(
                div()
                    .id("inspector-body")
                    .min_h_0()
                    .flex_1()
                    .overflow_y_scroll()
                    .child(self.render_session_context())
                    .children(detail),
            )
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
        prepared.diff_lines.as_ref().map_or_else(
            || detail_section("diff", "preparing patch...".into()),
            |lines| super::diff_view::render_diff(lines),
        )
    } else if tool == Some("bash") {
        detail_section(
            "output",
            prepared
                .output
                .clone()
                .unwrap_or_else(|| "(no output)".into()),
        )
    } else if prepared.tool.is_some() {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(detail_section(
                "tool",
                prepared
                    .tool_label
                    .clone()
                    .unwrap_or_else(|| "unknown".into()),
            ))
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
        .when(inspector, gpui::Styled::p_3)
        .when(!inspector, |detail| {
            detail.pl(px(ui_size::TOOL_CONTENT_X)).pr_3().pt_2().pb_3()
        })
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
