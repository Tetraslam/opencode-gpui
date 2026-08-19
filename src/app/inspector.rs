use std::sync::Arc;

use gpui::{Context, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    model::Part,
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{PartSelection, TimelineState, Workspace, chrome::centered_message};

pub(crate) struct PreparedPart {
    kind: SharedString,
    raw: SharedString,
    input: Option<SharedString>,
    output: Option<SharedString>,
    metadata: Option<SharedString>,
}

impl PreparedPart {
    fn build(part: &Part) -> Self {
        let raw = serde_json::to_string_pretty(part)
            .unwrap_or_else(|error| format!("serialization error: {error}"));
        let state = part.data.get("state");
        Self {
            kind: part.kind.clone().into(),
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
        self.selected_part = Some(selection.clone());
        if self.detail_cache.contains_key(&selection)
            || !self.preparing_parts.insert(selection.clone())
        {
            cx.notify();
            return;
        }

        let preparation = cx.background_spawn(async move { Arc::new(PreparedPart::build(&part)) });
        self.detail_tasks.push(cx.spawn(async move |workspace, cx| {
            let prepared = preparation.await;
            let _ = workspace.update(cx, |workspace, cx| {
                workspace.preparing_parts.remove(&selection);
                workspace.detail_cache.insert(selection, prepared);
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub(super) fn render_inspector(&self) -> gpui::AnyElement {
        let selection = self.selected_part.as_ref();
        let part = selection.and_then(|selection| self.find_part(selection));
        let prepared = selection.and_then(|selection| self.detail_cache.get(selection));
        let body = if selection.is_none() {
            centered_message("select a part")
        } else {
            render_part_detail(prepared.map(Arc::as_ref), true)
        };

        div()
            .w(px(ui_size::INSPECTOR))
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
                    .px_2()
                    .bg(rgb(color::SURFACE))
                    .border_b_1()
                    .border_color(rgb(color::BORDER))
                    .font_family(MONO_FONT)
                    .text_xs()
                    .text_color(rgb(color::TEXT_DIM))
                    .child("INSPECTOR")
                    .child(part.map_or("--", |part| part.kind.as_str()).to_uppercase()),
            )
            .child(div().min_h_0().flex_1().child(body))
            .into_any_element()
    }

    fn find_part(&self, selection: &PartSelection) -> Option<&Part> {
        let TimelineState::Ready { messages, .. } = &self.timeline else {
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
    let content = if prepared.kind.as_ref() == "tool" {
        div()
            .child(detail_section(
                "INPUT",
                prepared.input.clone().unwrap_or_else(|| "{}".into()),
            ))
            .child(detail_section(
                "OUTPUT",
                prepared
                    .output
                    .clone()
                    .unwrap_or_else(|| "(no output)".into()),
            ))
            .child(detail_section(
                "METADATA",
                prepared.metadata.clone().unwrap_or_else(|| "{}".into()),
            ))
            .into_any_element()
    } else {
        detail_section("RAW", prepared.raw.clone())
    };

    div()
        .id(SharedString::from(format!(
            "detail-{}-{inspector}",
            prepared.kind
        )))
        .when(inspector, |element| element.size_full().overflow_scroll())
        .px_3()
        .pb_3()
        .bg(rgb(if inspector {
            color::BASE
        } else {
            color::ELEVATED
        }))
        .child(content)
        .into_any_element()
}

fn detail_section(label: &'static str, content: SharedString) -> gpui::AnyElement {
    div()
        .pt_2()
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
                .text_color(rgb(color::TEXT_DIM))
                .child(content),
        )
        .into_any_element()
}

fn pretty_json(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}
