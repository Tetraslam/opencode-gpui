use gpui::{FontWeight, div, prelude::*, px, rgb};
use opencode_gpui::{
    markdown::{Block, Document, Inline},
    theme::color,
};

use super::markdown_render_cache::MarkdownRenderCache;

pub(super) fn render_document(
    document: &Document,
    renders: &MarkdownRenderCache,
) -> gpui::AnyElement {
    div()
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .gap_2()
        .children(
            document
                .blocks
                .iter()
                .map(|block| render_block(block, renders)),
        )
        .into_any_element()
}

fn render_block(block: &Block, renders: &MarkdownRenderCache) -> gpui::AnyElement {
    match block {
        Block::Heading { level, content } => div()
            .mt_1()
            .text_color(rgb(color::TEXT_BRIGHT))
            .font_weight(FontWeight::SEMIBOLD)
            .when(*level <= 1, gpui::Styled::text_lg)
            .when(*level == 2, gpui::Styled::text_base)
            .when(*level >= 3, gpui::Styled::text_sm)
            .child(super::markdown_inline_view::render_inline(content, renders))
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
            .child(super::markdown_inline_view::render_inline(content, renders))
            .into_any_element(),
        Block::Code { language, content } => {
            super::markdown_code_view::render_code(language, content, false)
        }
        Block::Diagram { language, content } => {
            super::markdown_code_view::render_diagram(language, content, renders)
        }
        Block::Math { content } => super::markdown_code_view::render_math(content, renders),
        Block::List { start, items } => render_list(*start, items, renders),
        Block::Table { header, rows } => render_table(header, rows, renders),
        Block::Rule => div()
            .my_1()
            .h(px(1.0))
            .w_full()
            .bg(rgb(color::BORDER))
            .into_any_element(),
    }
}

fn render_list(
    start: Option<u64>,
    items: &[Inline],
    renders: &MarkdownRenderCache,
) -> gpui::AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(items.iter().enumerate().map(|(index, item)| {
            let task = super::markdown_tasks::item(item);
            let marker = task.as_ref().map_or_else(
                || {
                    start.map_or_else(
                        || "•".to_owned(),
                        |start| format!("{}.", start + index as u64),
                    )
                },
                |(state, _)| super::markdown_tasks::marker(*state).to_owned(),
            );
            let content = task.as_ref().map_or(item, |(_, content)| content);
            div()
                .flex()
                .gap_2()
                .text_sm()
                .line_height(px(20.0))
                .text_color(rgb(color::TEXT))
                .child(super::markdown_tasks::checkbox(
                    task.as_ref().map(|(state, _)| *state),
                    marker,
                ))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .whitespace_normal()
                        .child(super::markdown_inline_view::render_inline(content, renders)),
                )
        }))
        .into_any_element()
}

fn render_table(
    header: &[Inline],
    rows: &[Vec<Inline>],
    renders: &MarkdownRenderCache,
) -> gpui::AnyElement {
    div()
        .overflow_hidden()
        .rounded_sm()
        .border_1()
        .border_color(rgb(color::BORDER))
        .child(render_table_row(header, true, false, renders))
        .children(
            rows.iter()
                .enumerate()
                .map(|(index, row)| render_table_row(row, false, index % 2 == 1, renders)),
        )
        .into_any_element()
}

fn render_table_row(
    cells: &[Inline],
    header: bool,
    alternate: bool,
    renders: &MarkdownRenderCache,
) -> gpui::AnyElement {
    div()
        .flex()
        .border_b_1()
        .border_color(rgb(color::BORDER_SUBTLE))
        .when(header, |row| {
            row.bg(rgb(color::SURFACE))
                .font_weight(FontWeight::SEMIBOLD)
        })
        .when(alternate, |row| row.bg(rgb(color::ELEVATED)))
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
                .child(super::markdown_inline_view::render_inline(cell, renders))
        }))
        .into_any_element()
}
