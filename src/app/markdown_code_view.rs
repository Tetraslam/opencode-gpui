use gpui::{ObjectFit, StyledImage, div, img, prelude::*, px, rgb};
use opencode_gpui::{
    markdown::{RenderKind, RenderRequest},
    theme::{MONO_FONT, color},
};

use super::markdown_render_cache::MarkdownRenderCache;

pub(super) fn render_diagram(
    language: &str,
    content: &str,
    renders: &MarkdownRenderCache,
) -> gpui::AnyElement {
    if language.trim().eq_ignore_ascii_case("mermaid") {
        let request = RenderRequest {
            kind: RenderKind::Mermaid,
            source: content.to_owned(),
        };
        if let Some(asset) = renders.asset(&request) {
            let ratio = asset.width / asset.height.max(1.0);
            let height = (560.0 / ratio).clamp(120.0, 520.0);
            return div()
                .w_full()
                .h(px(height))
                .max_h(px(520.0))
                .overflow_hidden()
                .rounded_sm()
                .border_1()
                .border_color(rgb(color::BORDER))
                .bg(rgb(color::BASE))
                .child(
                    img(asset.image.clone())
                        .size_full()
                        .object_fit(ObjectFit::Contain),
                )
                .into_any_element();
        }
    }
    render_code(language, content, true)
}

pub(super) fn render_math(content: &str, renders: &MarkdownRenderCache) -> gpui::AnyElement {
    let request = RenderRequest {
        kind: RenderKind::MathDisplay,
        source: content.to_owned(),
    };
    if let Some(asset) = renders.asset(&request) {
        return div()
            .w_full()
            .min_h(px(asset.height))
            .flex()
            .justify_center()
            .overflow_hidden()
            .child(
                img(asset.image.clone())
                    .w(px(asset.width))
                    .h(px(asset.height))
                    .max_w_full(),
            )
            .into_any_element();
    }
    render_code("math", &format!("$${content}$$"), false)
}

pub(super) fn render_code(language: &str, content: &str, diagram: bool) -> gpui::AnyElement {
    let title = if language.is_empty() {
        "code".to_owned()
    } else if diagram {
        format!("{language} diagram")
    } else {
        language.to_owned()
    };
    div()
        .overflow_hidden()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if diagram {
            color::ACCENT
        } else {
            color::BORDER
        }))
        .bg(rgb(color::SURFACE))
        .child(
            div()
                .px_3()
                .py_1()
                .border_b_1()
                .border_color(rgb(color::BORDER_SUBTLE))
                .font_family(MONO_FONT)
                .text_xs()
                .text_color(rgb(if diagram {
                    color::ACCENT
                } else {
                    color::TEXT_MUTED
                }))
                .child(title),
        )
        .child(
            div()
                .px_3()
                .py_2()
                .whitespace_normal()
                .font_family(MONO_FONT)
                .text_xs()
                .line_height(px(18.0))
                .text_color(rgb(color::TEXT))
                .child(content.to_owned()),
        )
        .into_any_element()
}
