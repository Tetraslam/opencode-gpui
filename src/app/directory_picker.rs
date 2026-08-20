use gpui::{Context, MouseButton, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::theme::{MONO_FONT, color, size as ui_size};

use super::{Workspace, command_palette::Overlay, directory_path::directory_name};

impl Workspace {
    pub(super) fn render_directory_picker(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        (self.overlay == Overlay::Directory).then(|| {
            let rows =
                self.directory_suggestions
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(index, directory)| {
                        let selected = index == self.overlay_selection;
                        let row_directory = directory.clone();
                        div()
                            .id(SharedString::from(format!("pick-{directory}")))
                            .h(px(42.0))
                            .px_3()
                            .flex_none()
                            .flex()
                            .flex_col()
                            .justify_center()
                            .cursor_pointer()
                            .border_b_1()
                            .border_color(rgb(color::BORDER_SUBTLE))
                            .when(selected, |row| row.bg(rgb(color::SELECTED)))
                            .hover(|row| row.bg(rgb(color::HOVER)))
                            .on_click(cx.listener(move |workspace, _, _, cx| {
                                workspace.create_directory_session(&row_directory, cx);
                            }))
                            .child(
                                div().text_sm().text_color(rgb(color::TEXT)).child(
                                    SharedString::from(directory_name(&directory).to_owned()),
                                ),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_xs()
                                    .text_color(rgb(color::TEXT_DIM))
                                    .child(SharedString::from(directory)),
                            )
                    });
            div()
                .id("directory-picker")
                .absolute()
                .top(px(ui_size::TITLEBAR))
                .left(px(ui_size::ACTIVITY_RAIL))
                .w(px(430.0))
                .max_h(px(460.0))
                .overflow_scroll()
                .track_scroll(&self.picker_scroll)
                .bg(rgb(color::ELEVATED))
                .border_1()
                .border_color(rgb(color::BORDER))
                .shadow_lg()
                .font_family(MONO_FONT)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|_, _, _, cx| {
                        cx.stop_propagation();
                    }),
                )
                .child(
                    div()
                        .h(px(30.0))
                        .px_3()
                        .flex()
                        .items_center()
                        .justify_between()
                        .border_b_1()
                        .border_color(rgb(color::BORDER))
                        .text_xs()
                        .text_color(rgb(color::TEXT_DIM))
                        .child("open directory")
                        .child("up/down  enter  esc"),
                )
                .child(
                    div()
                        .p_2()
                        .border_b_1()
                        .border_color(rgb(color::BORDER))
                        .child(
                            div()
                                .border_1()
                                .border_color(rgb(color::BORDER))
                                .child(self.directory_editor.clone()),
                        )
                        .children(self.directory_error.as_ref().map(|error| {
                            div()
                                .pt_1()
                                .text_xs()
                                .text_color(rgb(color::RED))
                                .child(error.clone())
                        })),
                )
                .children(rows)
                .into_any_element()
        })
    }
}
