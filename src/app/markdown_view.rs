use gpui::{
    FontStyle, FontWeight, HighlightStyle, StrikethroughStyle, StyledText, UnderlineStyle, div,
    prelude::*, px, rgb,
};
use opencode_gpui::{
    markdown::{Block, Document, Inline},
    theme::{MONO_FONT, color},
};

pub(super) fn render_document(document: &Document) -> gpui::AnyElement {
    div()
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .gap_2()
        .children(document.blocks.iter().map(render_block))
        .into_any_element()
}

fn render_block(block: &Block) -> gpui::AnyElement {
    match block {
        Block::Heading { level, content } => div()
            .mt_1()
            .text_color(rgb(color::TEXT_BRIGHT))
            .font_weight(FontWeight::SEMIBOLD)
            .when(*level <= 1, gpui::Styled::text_lg)
            .when(*level == 2, gpui::Styled::text_base)
            .when(*level >= 3, gpui::Styled::text_sm)
            .child(render_inline(content))
            .into_any_element(),
        Block::Paragraph { content, quoted } => div()
            .min_w_0()
            .whitespace_normal()
            .text_sm()
            .line_height(px(20.0))
            .text_color(rgb(color::TEXT))
            .when(*quoted, |element| {
                element
                    .pl_3()
                    .border_l_2()
                    .border_color(rgb(color::ACCENT))
                    .text_color(rgb(color::TEXT_DIM))
            })
            .child(render_inline(content))
            .into_any_element(),
        Block::Code { language, content } => render_code(language, content, false),
        Block::Diagram { language, content } => render_code(language, content, true),
        Block::List { start, items } => render_list(*start, items),
        Block::Table { header, rows } => render_table(header, rows),
        Block::Rule => div()
            .my_1()
            .h(px(1.0))
            .w_full()
            .bg(rgb(color::BORDER))
            .into_any_element(),
    }
}

fn render_inline(content: &Inline) -> StyledText {
    let highlights = content.spans.iter().map(|span| {
        let style = &span.style;
        let highlight = HighlightStyle {
            color: style.link.as_ref().map(|_| rgb(color::BLUE).into()),
            font_weight: style.bold.then_some(FontWeight::SEMIBOLD),
            font_style: style.italic.then_some(FontStyle::Italic),
            background_color: style.code.then_some(rgb(color::ELEVATED).into()),
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
        };
        (span.range.clone(), highlight)
    });
    StyledText::new(content.text.clone()).with_highlights(highlights)
}

fn render_code(language: &str, content: &str, diagram: bool) -> gpui::AnyElement {
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

fn render_list(start: Option<u64>, items: &[Inline]) -> gpui::AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(items.iter().enumerate().map(|(index, item)| {
            let marker = start.map_or_else(
                || "•".to_owned(),
                |start| format!("{}.", start + index as u64),
            );
            div()
                .flex()
                .gap_2()
                .text_sm()
                .line_height(px(20.0))
                .text_color(rgb(color::TEXT))
                .child(
                    div()
                        .w(px(24.0))
                        .flex_none()
                        .text_right()
                        .text_color(rgb(color::TEXT_MUTED))
                        .child(marker),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .whitespace_normal()
                        .child(render_inline(item)),
                )
        }))
        .into_any_element()
}

fn render_table(header: &[Inline], rows: &[Vec<Inline>]) -> gpui::AnyElement {
    div()
        .overflow_hidden()
        .rounded_sm()
        .border_1()
        .border_color(rgb(color::BORDER))
        .child(render_table_row(header, true))
        .children(rows.iter().map(|row| render_table_row(row, false)))
        .into_any_element()
}

fn render_table_row(cells: &[Inline], header: bool) -> gpui::AnyElement {
    div()
        .flex()
        .border_b_1()
        .border_color(rgb(color::BORDER_SUBTLE))
        .when(header, |row| {
            row.bg(rgb(color::SURFACE))
                .font_weight(FontWeight::SEMIBOLD)
        })
        .children(cells.iter().map(|cell| {
            div()
                .min_w_0()
                .flex_1()
                .px_2()
                .py_1()
                .border_r_1()
                .border_color(rgb(color::BORDER_SUBTLE))
                .whitespace_normal()
                .text_xs()
                .text_color(rgb(color::TEXT))
                .child(render_inline(cell))
        }))
        .into_any_element()
}
