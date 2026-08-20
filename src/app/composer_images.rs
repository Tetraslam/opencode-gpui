use gpui::{
    ClickEvent, Context, ObjectFit, SharedString, StyledImage, div, img, prelude::*, px, rgb,
};
use opencode_gpui::theme::{MONO_FONT, color, size as ui_size};

use super::{Workspace, image_attachment::PromptImage};

impl Workspace {
    pub(super) fn render_prompt_images(
        images: &[PromptImage],
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        div()
            .id("prompt-images")
            .h(px(ui_size::ATTACHMENT_HEIGHT + ui_size::GAP))
            .w_full()
            .px_2()
            .pt_2()
            .flex()
            .gap_2()
            .overflow_x_scroll()
            .children(
                images
                    .iter()
                    .map(|image| Self::render_prompt_image(image, cx)),
            )
            .into_any_element()
    }

    fn render_prompt_image(image: &PromptImage, cx: &mut Context<Self>) -> gpui::AnyElement {
        let remove_id = image.id.clone();
        let ready = image.data_url.is_some();
        div()
            .id(SharedString::from(format!("prompt-image-{}", image.id)))
            .w(px(ui_size::ATTACHMENT_WIDTH))
            .h(px(ui_size::ATTACHMENT_HEIGHT))
            .flex_none()
            .overflow_hidden()
            .rounded_sm()
            .border_1()
            .border_color(rgb(if ready { color::BORDER } else { color::ACCENT }))
            .bg(rgb(color::ELEVATED))
            .child(
                div()
                    .h(px(ui_size::ATTACHMENT_PREVIEW))
                    .w_full()
                    .bg(rgb(color::BASE))
                    .child(
                        img(image.image.clone())
                            .size_full()
                            .object_fit(ObjectFit::Cover),
                    ),
            )
            .child(
                div()
                    .h(px(ui_size::ATTACHMENT_HEIGHT - ui_size::ATTACHMENT_PREVIEW))
                    .px_2()
                    .flex()
                    .items_center()
                    .gap_1()
                    .border_t_1()
                    .border_color(rgb(color::BORDER_SUBTLE))
                    .font_family(MONO_FONT)
                    .text_xs()
                    .child(div().text_color(rgb(color::ACCENT)).child("img"))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .truncate()
                            .text_color(rgb(if ready { color::TEXT } else { color::YELLOW }))
                            .child(if ready {
                                image.filename.clone()
                            } else {
                                "processing...".into()
                            }),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("remove-{}", image.id)))
                            .size(px(18.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .cursor_pointer()
                            .text_color(rgb(color::TEXT_MUTED))
                            .hover(|button| {
                                button.bg(rgb(color::HOVER)).text_color(rgb(color::RED))
                            })
                            .child("x")
                            .on_click(cx.listener(move |workspace, _: &ClickEvent, _, cx| {
                                workspace.remove_prompt_image(&remove_id, cx);
                            })),
                    ),
            )
            .into_any_element()
    }
}
