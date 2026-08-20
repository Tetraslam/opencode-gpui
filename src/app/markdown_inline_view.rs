use std::ops::Range;

use gpui::{
    FontStyle, FontWeight, HighlightStyle, InteractiveText, StrikethroughStyle, StyledText,
    UnderlineStyle, div, img, prelude::*, px, rgb,
};
use opencode_gpui::{
    markdown::{Inline, RenderKind, RenderRequest, TaskState},
    theme::color,
};

use super::markdown_render_cache::MarkdownRenderCache;

pub(super) fn render_inline(content: &Inline, renders: &MarkdownRenderCache) -> gpui::AnyElement {
    let math = content
        .spans
        .iter()
        .filter(|span| span.style.math.is_some())
        .collect::<Vec<_>>();
    if math.is_empty() {
        return render_text(content, 0..content.text.len(), 0);
    }
    let mut cursor = 0;
    let mut children = Vec::new();
    for span in math {
        if span.range.start > cursor {
            children.push(render_text(content, cursor..span.range.start, cursor));
        }
        let source = span.style.math.as_deref().unwrap_or_default();
        let request = RenderRequest {
            kind: RenderKind::MathInline,
            source: source.to_owned(),
        };
        children.push(renders.asset(&request).map_or_else(
            || render_text(content, span.range.clone(), span.range.start),
            |asset| {
                img(asset.image.clone())
                    .w(px(asset.width))
                    .h(px(asset.height))
                    .max_w_full()
                    .into_any_element()
            },
        ));
        cursor = span.range.end;
    }
    if cursor < content.text.len() {
        children.push(render_text(content, cursor..content.text.len(), cursor));
    }
    div()
        .min_w_0()
        .flex()
        .flex_wrap()
        .items_baseline()
        .children(children)
        .into_any_element()
}

fn render_text(content: &Inline, range: Range<usize>, identity: usize) -> gpui::AnyElement {
    let Some(text) = content.text.get(range.clone()) else {
        return div().into_any_element();
    };
    let highlights = content.spans.iter().filter_map(|span| {
        intersection(&span.range, &range).map(|overlap| {
            (
                overlap.start - range.start..overlap.end - range.start,
                highlight(&span.style),
            )
        })
    });
    let styled = StyledText::new(text.to_owned()).with_highlights(highlights);
    let links = content
        .spans
        .iter()
        .filter_map(|span| {
            let overlap = intersection(&span.range, &range)?;
            Some((
                overlap.start - range.start..overlap.end - range.start,
                span.style.link.clone()?,
            ))
        })
        .collect::<Vec<_>>();
    if links.is_empty() {
        return styled.into_any_element();
    }
    let ranges = links.iter().map(|(range, _)| range.clone()).collect();
    let urls = links.into_iter().map(|(_, url)| url).collect::<Vec<_>>();
    InteractiveText::new(
        (
            "markdown-inline",
            std::ptr::from_ref(content) as usize ^ identity,
        ),
        styled,
    )
    .on_click(ranges, move |index, _, cx| {
        cx.stop_propagation();
        if let Some(url) = urls.get(index) {
            cx.open_url(url);
        }
    })
    .into_any_element()
}

fn intersection(left: &Range<usize>, right: &Range<usize>) -> Option<Range<usize>> {
    let start = left.start.max(right.start);
    let end = left.end.min(right.end);
    (start < end).then_some(start..end)
}

fn highlight(style: &opencode_gpui::markdown::InlineStyle) -> HighlightStyle {
    let semantic_color = style
        .link
        .as_ref()
        .map(|_| color::BLUE)
        .or(style.path.then_some(color::CYAN))
        .or(style.math.as_ref().map(|_| color::CYAN))
        .or(style.code.then_some(color::GREEN))
        .or(style.italic.then_some(color::YELLOW))
        .or(style.task.map(task_color));
    HighlightStyle {
        color: semantic_color.map(|value| rgb(value).into()),
        font_weight: (style.bold || style.kbd).then_some(FontWeight::BOLD),
        font_style: style.italic.then_some(FontStyle::Italic),
        background_color: (style.code || style.kbd || style.path)
            .then_some(rgb(color::ELEVATED).into()),
        underline: style.link.as_ref().map(|_| UnderlineStyle {
            thickness: px(1.0),
            color: Some(rgb(color::BLUE).into()),
            wavy: false,
        }),
        strikethrough: style.strike.then_some(StrikethroughStyle {
            thickness: px(1.0),
            color: Some(rgb(color::TEXT_MUTED).into()),
        }),
        fade_out: None,
    }
}

const fn task_color(state: TaskState) -> u32 {
    match state {
        TaskState::Checked => color::GREEN,
        TaskState::Active => color::YELLOW,
        TaskState::Pending => color::TEXT_DIM,
        TaskState::Cancelled => color::TEXT_MUTED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersection_clips_to_visible_segment() {
        assert_eq!(intersection(&(2..8), &(5..10)), Some(5..8));
        assert_eq!(intersection(&(0..2), &(2..4)), None);
    }
}
