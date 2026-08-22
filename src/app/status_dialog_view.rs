use super::{
    Workspace,
    command_palette::Overlay,
    status_dialog::{self, McpOperation, StatusDialogState},
};
use gpui::{Context, MouseButton, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    api::{McpStatus, StatusSnapshot},
    theme::{MONO_FONT, color, size as ui_size},
};
impl Workspace {
    pub(super) fn render_status_dialog(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        (self.overlay == Overlay::Status).then(|| {
            let state = &self.status_dialog;
            let initial_loading = state.loading && state.snapshot.is_none();
            div()
                .absolute()
                .top(px(ui_size::TITLEBAR + 16.0))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .id("status-dialog")
                        .w(px(680.0))
                        .max_h(px(680.0))
                        .flex()
                        .flex_col()
                        .bg(rgb(color::ELEVATED))
                        .border_1()
                        .border_color(rgb(color::BORDER))
                        .shadow_lg()
                        .font_family(MONO_FONT)
                        .text_color(rgb(color::TEXT))
                        .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| cx.stop_propagation()))
                        .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                        .child(dialog_header())
                        .child(
                            div()
                                .id("status-dialog-content")
                                .min_h(px(160.0))
                                .p_3()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .overflow_y_scroll()
                                .when(initial_loading, |content| content.child(message("Loading status…", color::TEXT_DIM)))
                                .children(state.error.as_ref().map(|error| {
                                    message(
                                        format!("Status refresh failed: {error}. Check the OpenCode server connection and retry /status."),
                                        color::RED,
                                    )
                                }))
                                .children(state.action_error.as_ref().map(|error| {
                                    message(error.clone(), color::RED)
                                }))
                                .children(
                                    state
                                        .snapshot
                                        .as_deref()
                                        .map(|snapshot| {
                                            render_snapshot(
                                                snapshot,
                                                state,
                                                &self.picker_scroll,
                                                cx,
                                            )
                                        }),
                                ),
                        ),
                )
                .into_any_element()
        })
    }
}
fn dialog_header() -> gpui::AnyElement {
    div()
        .h(px(38.0))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(rgb(color::BORDER))
        .child(
            div()
                .text_sm()
                .text_color(rgb(color::TEXT_BRIGHT))
                .child("Status"),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(color::TEXT_MUTED))
                .child("↑↓ select  space/enter toggle  esc close"),
        )
        .into_any_element()
}
fn render_snapshot(
    snapshot: &StatusSnapshot,
    state: &StatusDialogState,
    scroll: &gpui::ScrollHandle,
    cx: &mut Context<Workspace>,
) -> gpui::AnyElement {
    let enabled = snapshot
        .formatters
        .iter()
        .filter(|formatter| formatter.enabled)
        .collect::<Vec<_>>();
    let mut plugins = snapshot
        .config
        .plugins
        .iter()
        .map(status_dialog::plugin_display)
        .collect::<Vec<_>>();
    plugins.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(mcp_section(snapshot, state, scroll, cx))
        .child(section(
            format!("{} LSP Servers", snapshot.lsp.len()),
            snapshot.lsp.is_empty(),
            "No LSP Servers",
            snapshot.lsp.iter().map(|item| {
                status_row(
                    &item.id,
                    format!("{}  ·  {}  ·  {}", item.name, item.root, item.status),
                    if item.status == "connected" {
                        color::GREEN
                    } else {
                        color::RED
                    },
                )
            }),
        ))
        .child(section(
            format!("{} Formatters", enabled.len()),
            enabled.is_empty(),
            "No Formatters",
            enabled
                .into_iter()
                .map(|item| status_row(&item.name, "Enabled".into(), color::GREEN)),
        ))
        .child(section(
            format!("{} Plugins", plugins.len()),
            plugins.is_empty(),
            "No Plugins",
            plugins.iter().map(|item| {
                let detail = match (&item.version, &item.path) {
                    (Some(version), Some(path)) => format!("@{version}  ·  {path}"),
                    (Some(version), None) => format!("@{version}"),
                    (None, Some(path)) => path.clone(),
                    (None, None) => "Configured".into(),
                };
                status_row(&item.name, detail, color::GREEN)
            }),
        ))
        .into_any_element()
}

fn mcp_section(
    snapshot: &StatusSnapshot,
    state: &StatusDialogState,
    scroll: &gpui::ScrollHandle,
    cx: &mut Context<Workspace>,
) -> gpui::AnyElement {
    if state.mcp_names.is_empty() {
        return div().child("No MCP Servers").into_any_element();
    }
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(format!("{} MCP Servers", state.mcp_names.len()))
        .child(
            div()
                .id("status-mcp-results")
                .max_h(px(260.0))
                .overflow_y_scroll()
                .track_scroll(scroll)
                .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                .children(
                    state
                        .mcp_names
                        .iter()
                        .enumerate()
                        .filter_map(|(index, name)| {
                            let status = snapshot.mcp.get(name)?;
                            Some(mcp_row(name, status, index, state, cx))
                        }),
                ),
        )
        .into_any_element()
}

fn section(
    title: String,
    empty: bool,
    empty_label: &'static str,
    rows: impl Iterator<Item = gpui::AnyElement>,
) -> gpui::AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(if empty {
            empty_label.into()
        } else {
            SharedString::from(title)
        })
        .when(!empty, |section| section.children(rows))
        .into_any_element()
}

fn status_row(name: &str, detail: String, marker: u32) -> gpui::AnyElement {
    div()
        .pl_2()
        .flex()
        .gap_2()
        .text_xs()
        .child(div().text_color(rgb(marker)).child("•"))
        .child(
            div()
                .flex_1()
                .child(format!("{name}  "))
                .child(div().text_color(rgb(color::TEXT_DIM)).child(detail)),
        )
        .into_any_element()
}

fn mcp_row(
    name: &str,
    status: &McpStatus,
    index: usize,
    state: &StatusDialogState,
    cx: &mut Context<Workspace>,
) -> gpui::AnyElement {
    let pending = state
        .pending
        .as_ref()
        .filter(|pending| pending.name == name);
    let detail = pending.map_or_else(
        || status_dialog::mcp_status_label(name, status),
        |pending| match pending.operation {
            McpOperation::Connect => "connecting…".into(),
            McpOperation::Disconnect => "disconnecting…".into(),
        },
    );
    let marker = if pending.is_some() {
        color::YELLOW
    } else {
        mcp_color(status)
    };
    let name = name.to_owned();
    div()
        .id(SharedString::from(format!("status-mcp-{index}")))
        .px_2()
        .py_1()
        .flex()
        .gap_2()
        .cursor_pointer()
        .rounded_sm()
        .text_xs()
        .when(index == state.selected, |row| row.bg(rgb(color::SELECTED)))
        .hover(|row| row.bg(rgb(color::HOVER)))
        .on_click(cx.listener(move |workspace, _, _, cx| {
            workspace.select_and_toggle_status_mcp(index, cx);
        }))
        .child(div().text_color(rgb(marker)).child("•"))
        .child(
            div()
                .flex_1()
                .child(format!("{name}  "))
                .child(div().text_color(rgb(color::TEXT_DIM)).child(detail)),
        )
        .into_any_element()
}

fn mcp_color(status: &McpStatus) -> u32 {
    match status {
        McpStatus::Connected => color::GREEN,
        McpStatus::Disabled => color::TEXT_MUTED,
        McpStatus::NeedsAuth => color::YELLOW,
        McpStatus::Failed { .. }
        | McpStatus::NeedsClientRegistration { .. }
        | McpStatus::Unknown { .. } => color::RED,
    }
}

fn message(text: impl Into<SharedString>, value: u32) -> gpui::AnyElement {
    div()
        .text_xs()
        .text_color(rgb(value))
        .child(text.into())
        .into_any_element()
}
