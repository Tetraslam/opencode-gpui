use std::sync::Arc;

use gpui::{ClickEvent, Context, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    model::Part,
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{
    PartSelection, Workspace, inspector,
    part_format::{kind_color, markers, one_line_summary, part_label, part_marker},
    timeline_state::RenderState,
};

impl Workspace {
    pub(super) fn render_event_part(
        part: &Part,
        selection: &PartSelection,
        expanded: bool,
        state: &RenderState<'_>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let click_selection = selection.clone();
        let click_part = part.clone();
        let (marker, marker_color) = part_marker(part);
        div()
            .id(SharedString::from(part.id.clone()))
            .overflow_hidden()
            .when(expanded, |row| row.bg(rgb(color::SURFACE)))
            .child(
                div()
                    .id(SharedString::from(format!("head-{}", part.id)))
                    .h(px(ui_size::MESSAGE_HEADER))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .cursor_pointer()
                    .hover(|element| element.bg(rgb(color::HOVER)))
                    .on_click(cx.listener(move |workspace, _: &ClickEvent, window, cx| {
                        workspace.toggle_part(
                            click_selection.clone(),
                            click_part.clone(),
                            window,
                            cx,
                        );
                    }))
                    .font_family(MONO_FONT)
                    .children(markers(
                        if expanded { "v" } else { marker },
                        "·",
                        marker_color,
                        kind_color(&part.kind),
                    ))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .gap_2()
                            .text_xs()
                            .child(
                                div()
                                    .flex_none()
                                    .text_color(rgb(kind_color(&part.kind)))
                                    .child(part_label(part)),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .truncate()
                                    .text_color(rgb(color::TEXT_DIM))
                                    .child(one_line_summary(part, state.directory)),
                            ),
                    ),
            )
            .when(expanded, |element| {
                element.child(inspector::render_part_detail(
                    state.detail_cache.get(selection).map(Arc::as_ref),
                    false,
                ))
            })
            .into_any_element()
    }
}
