use gpui::{ClickEvent, Context, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    markdown::Document,
    model::Part,
    theme::{color, size as ui_size},
};

use super::{PartSelection, Workspace, part_format::label};

impl Workspace {
    pub(super) fn render_text_part(
        part: &Part,
        selection: PartSelection,
        selected: bool,
        document: Option<&Document>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let click_part = part.clone();
        div()
            .id(SharedString::from(part.id.clone()))
            .px_3()
            .py_2()
            .flex()
            .gap_2()
            .overflow_hidden()
            .when(selected, |row| row.bg(rgb(color::SELECTED)))
            .cursor_pointer()
            .hover(|row| row.bg(rgb(color::HOVER)))
            .on_click(cx.listener(move |workspace, _: &ClickEvent, _, cx| {
                workspace.select_part(selection.clone(), click_part.clone(), cx);
            }))
            .child(label("·", ui_size::MARKER_COL, color::BLUE))
            .child(label("", 18.0, color::BLUE))
            .child(div().min_w_0().flex_1().child(document.map_or_else(
                || {
                    div()
                        .whitespace_normal()
                        .text_sm()
                        .line_height(px(20.0))
                        .text_color(rgb(color::TEXT))
                        .child(SharedString::from(
                            part.text().unwrap_or_default().to_owned(),
                        ))
                        .into_any_element()
                },
                super::markdown_view::render_document,
            )))
            .into_any_element()
    }
}
