use gpui::{
    Context, CursorStyle, MouseButton, MouseDownEvent, Pixels, ScrollHandle, ScrollWheelEvent,
    Window, div, point, prelude::*, px, rgb,
};
use opencode_gpui::theme::color;

use super::Workspace;

impl Workspace {
    pub(super) fn handle_timeline_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        handle: &ScrollHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.overlay != super::command_palette::Overlay::None {
            return;
        }
        let delta = event.delta.pixel_delta(window.line_height()).y;
        let Some(tab) = self.active_tab_mut() else {
            return;
        };
        if delta > px(0.0) {
            tab.follow_tail = false;
        } else if delta < px(0.0) {
            let predicted = handle.offset().y + delta;
            tab.follow_tail = predicted <= -handle.max_offset().height + px(2.0);
        }
        cx.notify();
    }
}

pub(super) fn render_scrollbar(
    handle: &ScrollHandle,
    cx: &mut Context<Workspace>,
) -> gpui::AnyElement {
    let metrics = scrollbar_metrics(handle);
    let click_handle = handle.clone();
    div()
        .id("timeline-scrollbar")
        .relative()
        .h_full()
        .w(px(27.0))
        .flex_none()
        .border_l_1()
        .border_color(rgb(color::BORDER_SUBTLE))
        .bg(rgb(color::BASE))
        .cursor(CursorStyle::PointingHand)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |workspace, event: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                workspace.start_timeline_scroll_drag(event, &click_handle, cx);
            }),
        )
        .child(
            div()
                .absolute()
                .top(px(4.0))
                .right(px(9.0))
                .h(metrics.track)
                .w(px(9.0))
                .rounded_full()
                .bg(rgb(color::BORDER))
                .child(
                    div()
                        .absolute()
                        .top(metrics.top)
                        .h(metrics.thumb)
                        .w_full()
                        .rounded_full()
                        .bg(rgb(color::TEXT_MUTED)),
                ),
        )
        .into_any_element()
}

struct ScrollbarMetrics {
    track: Pixels,
    thumb: Pixels,
    top: Pixels,
}

fn scrollbar_metrics(handle: &ScrollHandle) -> ScrollbarMetrics {
    let viewport = handle.bounds().size.height;
    let max_offset = handle.max_offset().height;
    let track = (viewport - px(8.0)).max(px(1.0));
    let content = viewport + max_offset;
    let thumb = if content > px(0.0) {
        (track * (viewport / content)).clamp(px(28.0), track)
    } else {
        track
    };
    let progress = if max_offset > px(0.0) {
        (-handle.offset().y / max_offset).clamp(0.0, 1.0)
    } else {
        0.0
    };
    ScrollbarMetrics {
        track,
        thumb,
        top: (track - thumb) * progress,
    }
}

impl Workspace {
    fn start_timeline_scroll_drag(
        &mut self,
        event: &MouseDownEvent,
        handle: &ScrollHandle,
        cx: &mut Context<Self>,
    ) {
        let metrics = scrollbar_metrics(handle);
        let pointer = event.position.y - handle.bounds().top() - px(4.0);
        let grab = if pointer >= metrics.top && pointer <= metrics.top + metrics.thumb {
            pointer - metrics.top
        } else {
            metrics.thumb / 2.0
        };
        self.pane_resize = super::pane_resize::PaneResize::Timeline { grab };
        scroll_to_pointer(handle, event.position.y, grab);
        if let Some(tab) = self.active_tab_mut() {
            tab.follow_tail = is_at_bottom(handle);
        }
        cx.notify();
    }
}

pub(super) fn scroll_to_pointer(handle: &ScrollHandle, pointer_y: Pixels, grab: Pixels) {
    let metrics = scrollbar_metrics(handle);
    let available = metrics.track - metrics.thumb;
    let progress = if available > px(0.0) {
        ((pointer_y - handle.bounds().top() - px(4.0) - grab) / available).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut offset = handle.offset();
    offset.y = -handle.max_offset().height * progress;
    handle.set_offset(point(offset.x, offset.y));
}

pub(super) fn is_at_bottom(handle: &ScrollHandle) -> bool {
    handle.offset().y <= -handle.max_offset().height + px(2.0)
}
