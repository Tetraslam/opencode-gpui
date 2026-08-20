use gpui::{SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    model::Part,
    theme::{MONO_FONT, color, size as ui_size},
};

pub(super) fn label(text: &str, width: f32, color_value: u32) -> gpui::AnyElement {
    div()
        .w(px(width))
        .flex_none()
        .flex()
        .justify_center()
        .font_family(MONO_FONT)
        .text_xs()
        .text_color(rgb(color_value))
        .child(SharedString::from(text.to_owned()))
        .into_any_element()
}

pub(super) fn markers(
    marker: &str,
    kind: &str,
    marker_color: u32,
    kind_color: u32,
) -> [gpui::AnyElement; 2] {
    [
        label(marker, ui_size::MARKER_COL, marker_color),
        label(kind, ui_size::KIND_COL, kind_color),
    ]
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

pub(super) fn is_tool_part(part: &Part) -> bool {
    part.kind == "tool" || part.data.get("tool").is_some()
}

pub(super) fn produces_diff(part: &Part) -> bool {
    matches!(tool_name(part), "apply_patch" | "patch")
        && part
            .data
            .get("state")
            .and_then(|state| state.get("metadata"))
            .and_then(|metadata| metadata.get("diff"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|diff| !diff.is_empty())
}

pub(super) fn part_label(part: &Part) -> String {
    match part.kind.as_str() {
        "tool" => tool_icon(tool_name(part)).into(),
        "reasoning" => "thought".into(),
        other => other.to_owned(),
    }
}

pub(super) fn part_marker(part: &Part) -> (&'static str, u32) {
    if !is_tool_part(part) {
        return ("›", kind_color(&part.kind));
    }
    match part
        .data
        .get("state")
        .and_then(|state| state.get("status"))
        .and_then(serde_json::Value::as_str)
    {
        Some("running" | "pending") => ("●", color::GREEN),
        Some("error" | "failed") => ("!", color::RED),
        _ => ("›", color::TEXT_MUTED),
    }
}

pub(super) fn one_line_summary(part: &Part, directory: &str) -> String {
    let summary = if is_tool_part(part) {
        tool_summary(part, directory)
    } else {
        part.summary()
            .unwrap_or_else(|| format!("{} event", part.kind))
    };
    let normalized = strip_inline_markers(&summary.replace('\n', " "));
    if is_tool_part(part) {
        return normalized;
    }
    let mut chars = normalized.chars();
    let preview = chars.by_ref().take(180).collect::<String>();
    if chars.next().is_some() {
        format!("{preview}...")
    } else {
        preview
    }
}

fn tool_summary(part: &Part, directory: &str) -> String {
    let state = part.data.get("state");
    let title = state
        .and_then(|state| state.get("title"))
        .and_then(serde_json::Value::as_str);
    let input = state.and_then(|state| state.get("input"));
    let metadata = state.and_then(|state| state.get("metadata"));
    let tool = tool_name(part);
    let value = |key| {
        input
            .and_then(|input| input.get(key))
            .and_then(|value| value.as_str())
    };
    let summary = match tool {
        "bash" => value("command").unwrap_or("writing command...").to_owned(),
        "apply_patch" | "patch" => patch_summary(state, input, directory),
        "read" => format!(
            "read {}",
            display_path(value("filePath").unwrap_or("file"), directory)
        ),
        "grep" => result_count(
            format!(
                "grep \"{}\" in {}",
                value("pattern").unwrap_or(""),
                display_path(value("path").unwrap_or(directory), directory)
            ),
            metadata.and_then(|value| value.get("matches")),
            "match",
        ),
        "glob" => result_count(
            format!("glob \"{}\"", value("pattern").unwrap_or("")),
            metadata.and_then(|value| value.get("count")),
            "file",
        ),
        _ => title.unwrap_or(tool).to_owned(),
    };
    let status = state
        .and_then(|state| state.get("status"))
        .and_then(serde_json::Value::as_str);
    match status {
        Some("running" | "pending") => format!("{summary}  running"),
        Some("error" | "failed") => format!("{summary}  failed"),
        _ => summary,
    }
}

fn result_count(summary: String, count: Option<&serde_json::Value>, noun: &str) -> String {
    let Some(count) = count.and_then(serde_json::Value::as_u64) else {
        return summary;
    };
    let plural = if count == 1 { "" } else { "s" };
    format!("{summary}  {count} {noun}{plural}")
}

pub(super) fn tool_name(part: &Part) -> &str {
    part.data
        .get("tool")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("tool")
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

fn patch_summary(
    state: Option<&serde_json::Value>,
    input: Option<&serde_json::Value>,
    directory: &str,
) -> String {
    let metadata_files = state
        .and_then(|state| state.get("metadata"))
        .and_then(|metadata| metadata.get("files"))
        .and_then(serde_json::Value::as_array);
    let mut paths = metadata_files.map_or_else(Vec::new, |files| {
        files
            .iter()
            .filter_map(|file| file.get("relativePath").and_then(serde_json::Value::as_str))
            .map(ToOwned::to_owned)
            .collect()
    });
    if paths.is_empty()
        && let Some(patch) = input
            .and_then(|input| input.get("patchText"))
            .and_then(serde_json::Value::as_str)
    {
        paths = patch
            .lines()
            .filter_map(patch_path)
            .map(ToOwned::to_owned)
            .collect();
    }
    match paths.as_slice() {
        [] => "preparing patch...".into(),
        [path] => format!("patched {}", display_path(path, directory)),
        [first, second] => format!(
            "patched {}, {}",
            display_path(first, directory),
            display_path(second, directory)
        ),
        [first, rest @ ..] => format!(
            "patched {} +{} files",
            display_path(first, directory),
            rest.len()
        ),
    }
}

fn patch_path(line: &str) -> Option<&str> {
    ["*** Add File: ", "*** Update File: ", "*** Delete File: "]
        .into_iter()
        .find_map(|prefix| line.strip_prefix(prefix))
}

fn display_path<'a>(path: &'a str, directory: &str) -> &'a str {
    path.strip_prefix(directory)
        .and_then(|path| path.strip_prefix('/'))
        .unwrap_or(path)
}

fn strip_inline_markers(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(inner) = trimmed
        .strip_prefix("**")
        .and_then(|text| text.strip_suffix("**"))
    {
        inner.to_owned()
    } else {
        trimmed.to_owned()
    }
}
