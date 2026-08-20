use gpui::{SharedString, div, prelude::*, px, rgb};
use opencode_gpui::theme::{MONO_FONT, color};

#[derive(Clone, Copy)]
pub(super) enum DiffKind {
    File,
    Hunk,
    Added,
    Removed,
    Context,
}

pub(super) struct DiffLine {
    old: Option<u64>,
    new: Option<u64>,
    kind: DiffKind,
    text: SharedString,
}

pub(super) fn parse_diff(diff: &str) -> Vec<DiffLine> {
    let mut old = 0;
    let mut new = 0;
    let total = diff.lines().count();
    let mut lines = diff
        .lines()
        .take(800)
        .map(|line| {
            if line.starts_with("@@") {
                if let Some((old_start, new_start)) = hunk_starts(line) {
                    old = old_start;
                    new = new_start;
                }
                return make_line(None, None, DiffKind::Hunk, line);
            }
            if line.starts_with("diff ")
                || line.starts_with("Index:")
                || line.starts_with("===")
                || line.starts_with("---")
                || line.starts_with("+++")
            {
                return make_line(None, None, DiffKind::File, line);
            }
            if line.starts_with('+') {
                let current = new;
                new += 1;
                return make_line(None, Some(current), DiffKind::Added, line);
            }
            if line.starts_with('-') {
                let current = old;
                old += 1;
                return make_line(Some(current), None, DiffKind::Removed, line);
            }
            let current_old = old;
            let current_new = new;
            old += 1;
            new += 1;
            make_line(
                Some(current_old),
                Some(current_new),
                DiffKind::Context,
                line,
            )
        })
        .collect::<Vec<_>>();
    if total > lines.len() {
        lines.push(make_line(
            None,
            None,
            DiffKind::Hunk,
            &format!("... {} more lines", total - lines.len()),
        ));
    }
    lines
}

fn make_line(old: Option<u64>, new: Option<u64>, kind: DiffKind, text: &str) -> DiffLine {
    DiffLine {
        old,
        new,
        kind,
        text: text.to_owned().into(),
    }
}

fn hunk_starts(line: &str) -> Option<(u64, u64)> {
    let mut ranges = line
        .split_whitespace()
        .filter(|part| part.starts_with('-') || part.starts_with('+'));
    Some((range_start(ranges.next()?)?, range_start(ranges.next()?)?))
}

fn range_start(range: &str) -> Option<u64> {
    range
        .trim_start_matches(['-', '+'])
        .split(',')
        .next()?
        .parse()
        .ok()
}

pub(super) fn render_diff(lines: &[DiffLine]) -> gpui::AnyElement {
    div()
        .mt_2()
        .overflow_hidden()
        .rounded_sm()
        .border_1()
        .border_color(rgb(color::BORDER))
        .font_family(MONO_FONT)
        .text_xs()
        .children(lines.iter().map(render_line))
        .into_any_element()
}

fn render_line(line: &DiffLine) -> gpui::AnyElement {
    let (background, foreground) = match line.kind {
        DiffKind::Added => (color::DIFF_ADDED_BG, color::GREEN),
        DiffKind::Removed => (color::DIFF_REMOVED_BG, color::RED),
        DiffKind::Hunk => (color::ELEVATED, color::CYAN),
        DiffKind::File => (color::SURFACE, color::TEXT_BRIGHT),
        DiffKind::Context => (color::DIFF_CONTEXT_BG, color::TEXT_DIM),
    };
    div()
        .min_h(px(18.0))
        .flex()
        .bg(rgb(background))
        .text_color(rgb(foreground))
        .child(line_number(line.old))
        .child(line_number(line.new))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .px_2()
                .whitespace_normal()
                .child(line.text.clone()),
        )
        .into_any_element()
}

fn line_number(number: Option<u64>) -> gpui::AnyElement {
    div()
        .w(px(38.0))
        .flex_none()
        .pr_2()
        .text_right()
        .border_r_1()
        .border_color(rgb(color::BORDER_SUBTLE))
        .text_color(rgb(color::TEXT))
        .child(number.map_or_else(String::new, |number| number.to_string()))
        .into_any_element()
}
