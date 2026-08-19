use std::{collections::HashMap, sync::Arc};

use gpui::{ClickEvent, Context, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    model::Part,
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{
    PartSelection, Workspace,
    inspector::{self, PreparedPart},
    part_format::{label, one_line_summary, part_marker, tool_name},
};

impl Workspace {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_tool_part(
        part: &Part,
        selection: &PartSelection,
        expanded: bool,
        detail_cache: &HashMap<PartSelection, Arc<PreparedPart>>,
        directory: &str,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let click_selection = selection.to_owned();
        let click_part = part.clone();
        let (marker, marker_color) = part_marker(part);
        let tool = tool_name(part);
        let output = tool_output(part);
        let preview = output_preview(tool, output);
        let files = patch_files(part);

        div()
            .id(SharedString::from(part.id.clone()))
            .overflow_hidden()
            .when(expanded, |row| row.bg(rgb(color::SELECTED)))
            .cursor_pointer()
            .hover(|row| row.bg(rgb(color::HOVER)))
            .on_click(cx.listener(move |workspace, _: &ClickEvent, window, cx| {
                workspace.toggle_part(click_selection.clone(), click_part.clone(), window, cx);
            }))
            .child(
                div()
                    .min_h(px(30.0))
                    .px_3()
                    .py_1()
                    .flex()
                    .items_start()
                    .gap_2()
                    .font_family(MONO_FONT)
                    .child(label(
                        if expanded { "v" } else { marker },
                        ui_size::MARKER_COL,
                        marker_color,
                    ))
                    .child(label(tool_icon(tool), 18.0, color::TOOL))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .whitespace_normal()
                            .when(tool == "bash", gpui::Styled::text_xs)
                            .when(tool != "bash", gpui::Styled::text_sm)
                            .line_height(px(20.0))
                            .text_color(rgb(color::TEXT))
                            .child(one_line_summary(part, directory)),
                    ),
            )
            .children(files.map(render_file_list))
            .children(
                (!expanded)
                    .then_some(preview)
                    .flatten()
                    .map(render_output_preview),
            )
            .when(expanded, |row| {
                row.child(inspector::render_part_detail(
                    detail_cache.get(selection).map(Arc::as_ref),
                    false,
                ))
            })
            .into_any_element()
    }
}

fn tool_icon(tool: &str) -> &'static str {
    match tool {
        "bash" => "$",
        "read" => "→",
        "grep" | "glob" => "*",
        "apply_patch" | "patch" | "edit" | "write" => "←",
        "task" => "@",
        _ => "·",
    }
}

fn tool_output(part: &Part) -> Option<&str> {
    let state = part.data.get("state")?;
    state
        .get("output")
        .and_then(serde_json::Value::as_str)
        .or_else(|| state.get("metadata")?.get("output")?.as_str())
        .map(str::trim)
        .filter(|output| !output.is_empty())
}

struct OutputPreview {
    text: String,
    hidden_lines: usize,
}

fn output_preview(tool: &str, output: Option<&str>) -> Option<OutputPreview> {
    if !matches!(tool, "bash") && known_inline_tool(tool) {
        return None;
    }
    let output = output?;
    let lines = output.lines().collect::<Vec<_>>();
    let visible = lines.iter().take(6).copied().collect::<Vec<_>>().join("\n");
    Some(OutputPreview {
        text: visible,
        hidden_lines: lines.len().saturating_sub(6),
    })
}

fn known_inline_tool(tool: &str) -> bool {
    matches!(
        tool,
        "read" | "grep" | "glob" | "apply_patch" | "patch" | "edit" | "write" | "task"
    )
}

fn render_output_preview(preview: OutputPreview) -> gpui::AnyElement {
    div()
        .ml(px(ui_size::TOOL_CONTENT_X))
        .mr_3()
        .mb_2()
        .px_3()
        .py_2()
        .overflow_hidden()
        .bg(rgb(color::ELEVATED))
        .border_l_2()
        .border_color(rgb(color::BORDER))
        .font_family(MONO_FONT)
        .text_xs()
        .line_height(px(18.0))
        .whitespace_normal()
        .text_color(rgb(color::TEXT_DIM))
        .child(preview.text)
        .children((preview.hidden_lines > 0).then(|| {
            div()
                .pt_1()
                .text_color(rgb(color::TEXT_MUTED))
                .child(format!("{} more lines", preview.hidden_lines))
        }))
        .into_any_element()
}

fn patch_files(part: &Part) -> Option<Vec<String>> {
    if !matches!(tool_name(part), "apply_patch" | "patch") {
        return None;
    }
    let files = part
        .data
        .get("state")?
        .get("metadata")?
        .get("files")?
        .as_array()?;
    let paths = files
        .iter()
        .filter_map(|file| file.get("relativePath")?.as_str().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    (paths.len() > 1).then_some(paths)
}

fn render_file_list(files: Vec<String>) -> gpui::AnyElement {
    div()
        .ml(px(ui_size::TOOL_CONTENT_X))
        .mr_3()
        .mb_2()
        .flex()
        .flex_col()
        .font_family(MONO_FONT)
        .text_xs()
        .text_color(rgb(color::TEXT_DIM))
        .children(files.into_iter().map(|file| div().child(file)))
        .into_any_element()
}
