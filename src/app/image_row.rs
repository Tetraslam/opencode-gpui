use std::sync::Arc;

use gpui::{
    ClickEvent, Context, Image, ObjectFit, SharedString, StyledImage, div, img, prelude::*, px, rgb,
};
use opencode_gpui::{
    model::Part,
    theme::{MONO_FONT, color},
};

use super::{
    PartSelection, Workspace, image_attachment::display_image_filename, part_format::markers,
};

impl Workspace {
    pub(super) fn render_image_part(
        part: &Part,
        selection: PartSelection,
        selected: bool,
        image: Arc<Image>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let click_part = part.clone();
        let filename = display_image_filename(
            part.data
                .get("filename")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("image"),
        );
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
                    .child(image_descriptor(filename)),
            )
            .into_any_element()
    }
}

fn image_descriptor(filename: String) -> gpui::AnyElement {
    div()
        .h(px(28.0))
        .px_2()
        .flex()
        .items_center()
        .gap_1()
        .border_t_1()
        .border_color(rgb(color::BORDER_SUBTLE))
        .font_family(MONO_FONT)
        .text_xs()
        .child(
            div()
                .px_1()
                .rounded_sm()
                .bg(rgb(color::ACCENT))
                .text_color(rgb(color::BASE))
                .child("image"),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .px_1()
                .truncate()
                .rounded_sm()
                .bg(rgb(color::BASE))
                .text_color(rgb(color::TEXT))
                .child(filename),
        )
        .into_any_element()
}
