use gpui::{Context, MouseButton, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::theme::{MONO_FONT, color, size as ui_size};

use super::{
    Workspace,
    command_palette::Overlay,
    timeline_actions::{ACTIONS, MessageAction},
};

impl Workspace {
    pub(super) fn render_message_actions(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        (self.overlay == Overlay::MessageActions).then(|| {
            div()
                .absolute()
                .top(px(ui_size::TITLEBAR + 80.0))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .id("message-actions")
                        .w(px(520.0))
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
                        .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                        .child(
                            div()
                                .px_3()
                                .py_3()
                                .border_b_1()
                                .border_color(rgb(color::BORDER))
                                .text_sm()
                                .child("Message Actions"),
                        )
                        .children(ACTIONS.iter().copied().enumerate().map(|(index, action)| {
                            action_row(index, action, self.overlay_selection, cx)
                        })),
                )
                .into_any_element()
        })
    }
}

fn action_row(
    index: usize,
    action: MessageAction,
    selected: usize,
    cx: &mut Context<Workspace>,
) -> gpui::AnyElement {
    div()
        .id(SharedString::from(format!(
            "message-action-{}",
            action.title()
        )))
        .h(px(48.0))
        .px_3()
        .flex()
        .flex_col()
        .justify_center()
        .cursor_pointer()
        .border_b_1()
        .border_color(rgb(color::BORDER_SUBTLE))
        .when(index == selected, |row| row.bg(rgb(color::SELECTED)))
        .hover(|row| row.bg(rgb(color::HOVER)))
        .on_click(cx.listener(move |workspace, _, _, cx| {
            workspace.overlay_selection = index;
            workspace.execute_message_action(cx);
        }))
        .child(
            div()
                .text_sm()
                .text_color(rgb(color::TEXT))
                .child(action.title()),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(color::TEXT_DIM))
                .child(action.description()),
        )
        .into_any_element()
}
