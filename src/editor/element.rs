use gpui::{
    App, Bounds, ContentMask, Element, ElementId, ElementInputHandler, Entity, GlobalElementId,
    IntoElement, LayoutId, PaintQuad, Pixels, Style, TextAlign, TextRun, UnderlineStyle, Window,
    fill, point, px, relative, rgba, size,
};

use crate::theme::color;

use super::{MAX_VISIBLE_LINES, TextEditor, VERTICAL_PADDING, layout::EditorLayout};

pub(super) struct TextElement {
    pub(super) input: Entity<TextEditor>,
}

pub(super) struct PrepaintState {
    layout: EditorLayout,
    cursor: Option<PaintQuad>,
    selection: Vec<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        let input = self.input.read(cx);
        let lines = input
            .visible_lines
            .max(input.explicit_lines())
            .clamp(1, MAX_VISIBLE_LINES);
        style.size.height = (window.line_height() * lines + VERTICAL_PADDING * 2).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> PrepaintState {
        let input = self.input.read(cx);
        let content = input.content.clone();
        let selected_range = input.selected_range.clone();
        let cursor_offset = input.cursor_offset();
        let style = window.text_style();
        let (display_text, text_color) = if content.is_empty() {
            (input.placeholder.clone(), gpui::rgb(color::TEXT_DIM).into())
        } else {
            (content, style.color)
        };
        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = marked_runs(input.marked_range.as_ref(), display_text.len(), run);
        let font_size = style.font_size.to_pixels(window.rem_size());
        let lines = window
            .text_system()
            .shape_text(
                display_text,
                font_size,
                &runs,
                Some(bounds.size.width),
                None,
            )
            .expect("editor text must shape")
            .into_vec();
        let layout = EditorLayout::new(lines, window.line_height(), cursor_offset);
        let cursor_position = layout.position_for_offset(cursor_offset);
        let scroll_y = layout.line_height * layout.scroll_row;
        let cursor = selected_range.is_empty().then(|| {
            let origin = point(
                bounds.left() + cursor_position.x,
                bounds.top() + VERTICAL_PADDING + cursor_position.y - scroll_y,
            );
            fill(
                Bounds::new(origin, size(px(1.0), layout.line_height)),
                gpui::rgb(color::ACCENT),
            )
        });
        let selection = selection_quads(&layout, &selected_range, bounds);
        PrepaintState {
            layout,
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut (),
        state: &mut PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for selection in state.selection.drain(..) {
                window.paint_quad(selection);
            }
            let mut row = 0;
            for line in &state.layout.lines {
                let y = bounds.top() + state.layout.line_height * row + VERTICAL_PADDING
                    - state.layout.line_height * state.layout.scroll_row;
                line.paint(
                    point(bounds.left(), y),
                    state.layout.line_height,
                    TextAlign::Left,
                    Some(bounds),
                    window,
                    cx,
                )
                .expect("editor text layout must paint");
                row += line.wrap_boundaries().len() + 1;
            }
            if focus.is_focused(window)
                && let Some(cursor) = state.cursor.take()
            {
                window.paint_quad(cursor);
            }
        });
        let visible_lines = state.layout.visual_lines().clamp(1, MAX_VISIBLE_LINES);
        self.input.update(cx, |input, cx| {
            input.last_layout = Some(state.layout.clone());
            input.last_bounds = Some(Bounds::new(
                point(bounds.left(), bounds.top() + VERTICAL_PADDING),
                size(bounds.size.width, bounds.size.height - VERTICAL_PADDING * 2),
            ));
            if input.visible_lines != visible_lines {
                input.visible_lines = visible_lines;
                cx.notify();
            }
        });
    }
}

fn marked_runs(marked: Option<&std::ops::Range<usize>>, len: usize, run: TextRun) -> Vec<TextRun> {
    let Some(marked) = marked else {
        return vec![run];
    };
    [
        TextRun {
            len: marked.start,
            ..run.clone()
        },
        TextRun {
            len: marked.end - marked.start,
            underline: Some(UnderlineStyle {
                color: Some(run.color),
                thickness: px(1.0),
                wavy: false,
            }),
            ..run.clone()
        },
        TextRun {
            len: len - marked.end,
            ..run
        },
    ]
    .into_iter()
    .filter(|run| run.len > 0)
    .collect()
}

fn selection_quads(
    layout: &EditorLayout,
    range: &std::ops::Range<usize>,
    bounds: Bounds<Pixels>,
) -> Vec<PaintQuad> {
    if range.is_empty() {
        return Vec::new();
    }
    let start = layout.position_for_offset(range.start);
    let end = layout.position_for_offset(range.end);
    let first_row = layout.row_for_y(start.y);
    let last_row = layout.row_for_y(end.y);
    let scroll_y = layout.line_height * layout.scroll_row;
    (first_row..=last_row)
        .map(|row| {
            let left = if row == first_row {
                start.x
            } else {
                Pixels::ZERO
            };
            let right = if row == last_row {
                end.x
            } else {
                bounds.size.width
            };
            let origin = point(
                bounds.left() + left,
                bounds.top() + VERTICAL_PADDING + layout.line_height * row - scroll_y,
            );
            fill(
                Bounds::new(
                    origin,
                    size((right - left).max(px(1.0)), layout.line_height),
                ),
                rgba(0x5294_e240),
            )
        })
        .collect()
}
