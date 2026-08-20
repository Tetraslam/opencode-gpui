use std::sync::Arc;

use gpui::{
    ClickEvent, Context, Image, ObjectFit, SharedString, StyledImage, div, img, prelude::*, px, rgb,
};
use opencode_gpui::{
    model::Part,
    theme::{MONO_FONT, color},
};

use super::{PartSelection, Workspace, part_format::markers};

impl Workspace {
    pub(super) fn render_image_part(
        part: &Part,
        selection: PartSelection,
        selected: bool,
        image: Arc<Image>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let click_part = part.clone();
        let filename = part
            .data
            .get("filename")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("image")
            .to_owned();
        div()
            .id(SharedString::from(part.id.clone()))
            .px_3()
            .py_2()
            .flex()
            .gap_2()
            .when(selected, |row| row.bg(rgb(color::SURFACE)))
            .cursor_pointer()
            .hover(|row| row.bg(rgb(color::HOVER)))
            .on_click(cx.listener(move |workspace, _: &ClickEvent, _, cx| {
                workspace.select_part(selection.clone(), click_part.clone(), cx);
            }))
            .children(markers("·", "", color::CYAN, color::CYAN))
            .child(
                div()
                    .w(px(320.0))
                    .max_w_full()
                    .overflow_hidden()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(color::BORDER))
                    .bg(rgb(color::SURFACE))
                    .child(
                        div()
                            .h(px(190.0))
                            .w_full()
                            .bg(rgb(color::BASE))
                            .child(img(image).size_full().object_fit(ObjectFit::Contain)),
                    )
                    .child(
                        div()
                            .h(px(24.0))
                            .px_2()
                            .flex()
                            .items_center()
                            .border_t_1()
                            .border_color(rgb(color::BORDER_SUBTLE))
                            .font_family(MONO_FONT)
                            .text_xs()
                            .text_color(rgb(color::TEXT_DIM))
                            .child(filename),
                    ),
            )
            .into_any_element()
    }
}
