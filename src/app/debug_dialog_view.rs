use gpui::{Context, MouseButton, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::theme::{MONO_FONT, color, size as ui_size};

use super::{Workspace, command_palette::Overlay};

impl Workspace {
    pub(super) fn render_debug_dialog(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        (self.overlay == Overlay::Debug).then(|| {
            let copied = self.debug_dialog.copied;
            div()
                .absolute()
                .top(px(ui_size::TITLEBAR + 16.0))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .id("debug-dialog")
                        .w(px(760.0))
                        .max_h(px(680.0))
                        .flex()
                        .flex_col()
                        .bg(rgb(color::ELEVATED))
                        .border_1()
                        .border_color(rgb(color::BORDER))
                        .shadow_lg()
                        .font_family(MONO_FONT)
                        .text_color(rgb(color::TEXT))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_, _, _, cx| cx.stop_propagation()),
                        )
                        .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                        .child(
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
                                        .child("Debug info"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(color::TEXT_MUTED))
                                        .child("esc"),
                                ),
                        )
                        .child(
                            div()
                                .id("debug-dialog-content")
                                .min_h_0()
                                .p_3()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .overflow_y_scroll()
                                .children(self.debug_dialog.entries.iter().map(debug_row)),
                        )
                        .child(
                            div()
                                .h(px(42.0))
                                .px_3()
                                .flex()
                                .items_center()
                                .justify_between()
                                .border_t_1()
                                .border_color(rgb(color::BORDER))
                                .text_xs()
                                .child(
                                    div()
                                        .text_color(rgb(color::TEXT_MUTED))
                                        .child("Share this when reporting an issue."),
                                )
                                .child(
                                    div()
                                        .id("copy-debug-info")
                                        .px_2()
                                        .py_1()
                                        .cursor_pointer()
                                        .rounded_sm()
                                        .text_color(rgb(if copied {
                                            color::GREEN
                                        } else {
                                            color::TEXT
                                        }))
                                        .hover(|item| item.bg(rgb(color::HOVER)))
                                        .on_click(cx.listener(|workspace, _, _, cx| {
                                            workspace.copy_debug_info_click(cx);
                                        }))
                                        .child(if copied { "copied" } else { "copy  enter" }),
                                ),
                        ),
                )
                .into_any_element()
        })
    }
}

fn debug_row(entry: &super::debug_dialog::DebugEntry) -> gpui::AnyElement {
    div()
        .flex()
        .gap_3()
        .text_xs()
        .child(
            div()
                .w(px(124.0))
                .flex_none()
                .text_color(rgb(color::TEXT_MUTED))
                .child(SharedString::from(entry.label)),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .text_color(rgb(color::TEXT))
                .child(entry.value.clone()),
        )
        .into_any_element()
}
