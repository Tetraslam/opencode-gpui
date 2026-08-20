use gpui::{ClickEvent, Context, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    model::Part,
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{PartSelection, Workspace, part_format::label};

impl Workspace {
    pub(super) fn render_file_part(
        part: &Part,
        selection: PartSelection,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let click_part = part.clone();
        let directory = part.data.get("mime").and_then(serde_json::Value::as_str)
            == Some("application/x-directory");
        let filename = part
            .data
            .get("filename")
            .and_then(serde_json::Value::as_str)
            .or_else(|| part.data.get("url").and_then(serde_json::Value::as_str))
            .unwrap_or("attachment")
            .to_owned();
        div()
            .id(SharedString::from(part.id.clone()))
            .px_3()
            .py_1()
            .flex()
            .items_center()
            .gap_2()
            .when(selected, |row| row.bg(rgb(color::SELECTED)))
            .cursor_pointer()
            .hover(|row| row.bg(rgb(color::HOVER)))
            .on_click(cx.listener(move |workspace, _: &ClickEvent, _, cx| {
                workspace.select_part(selection.clone(), click_part.clone(), cx);
            }))
            .child(label("·", ui_size::MARKER_COL, color::CYAN))
            .child(label("", 18.0, color::CYAN))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .font_family(MONO_FONT)
                    .text_xs()
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_l_sm()
                            .bg(rgb(color::ACCENT))
                            .text_color(rgb(color::BASE))
                            .child(if directory { "directory" } else { "file" }),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .max_w(px(520.0))
                            .px_2()
                            .py_1()
                            .truncate()
                            .rounded_r_sm()
                            .bg(rgb(color::ELEVATED))
                            .text_color(rgb(color::TEXT))
                            .child(filename),
                    ),
            )
            .into_any_element()
    }
}
