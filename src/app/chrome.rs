use gpui::{Render, Window, div, prelude::*, px, rgb};
use opencode_gpui::{
    event::SessionStatus,
    theme::{MONO_FONT, UI_FONT, color, size as ui_size},
};

use super::{ServerState, TimelineState, Workspace};

impl Workspace {
    fn render_titlebar(&self) -> gpui::AnyElement {
        let context = self.timeline.title().unwrap_or_else(|| "sessions".into());
        let (connection, connection_color) = if self.live {
            ("LIVE", color::GREEN)
        } else {
            ("RECONNECT", color::YELLOW)
        };
        div()
            .h(px(ui_size::TITLEBAR))
            .flex_none()
            .px_3()
            .flex()
            .items_center()
            .justify_between()
            .bg(rgb(color::SURFACE))
            .border_b_1()
            .border_color(rgb(color::BORDER))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .font_family(MONO_FONT)
                    .text_xs()
                    .child(div().text_color(rgb(color::TEXT_MUTED)).child("opencode /"))
                    .child(
                        div()
                            .max_w(px(720.0))
                            .truncate()
                            .text_color(rgb(color::TEXT_BRIGHT))
                            .child(context),
                    ),
            )
            .child(
                div()
                    .font_family(MONO_FONT)
                    .text_xs()
                    .text_color(rgb(connection_color))
                    .child(connection),
            )
            .into_any_element()
    }

    fn render_activity_rail() -> gpui::AnyElement {
        div()
            .w(px(ui_size::ACTIVITY_RAIL))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_between()
            .py_2()
            .bg(rgb(color::SURFACE))
            .border_r_1()
            .border_color(rgb(color::BORDER))
            .font_family(MONO_FONT)
            .text_xs()
            .child(div().flex().flex_col().gap_2().children([
                rail_item("S", "sessions", true),
                rail_item("D", "diffs", false),
                rail_item("A", "agents", false),
                rail_item("M", "models", false),
            ]))
            .child(rail_item("?", "help", false))
            .into_any_element()
    }

    fn render_statusline(&self) -> gpui::AnyElement {
        let (sessions, busy, version) = match &self.server_state {
            ServerState::Ready { sessions, version } => (
                sessions.len(),
                self.statuses
                    .values()
                    .filter(|status| matches!(status, SessionStatus::Busy))
                    .count(),
                version.as_ref(),
            ),
            ServerState::Loading | ServerState::Failed(_) => (0, 0, "--"),
        };
        div()
            .h(px(ui_size::STATUSLINE))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .bg(rgb(color::SURFACE))
            .border_t_1()
            .border_color(rgb(color::BORDER))
            .font_family(MONO_FONT)
            .text_xs()
            .text_color(rgb(color::TEXT_DIM))
            .child(format!(
                "NORMAL  |  sessions {sessions}  busy {busy}  |  server {version}  |  history {}",
                self.message_limit
            ))
            .child(
                div()
                    .max_w(px(520.0))
                    .truncate()
                    .text_color(rgb(color::TEXT_MUTED))
                    .child(self.server.clone()),
            )
            .into_any_element()
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let titlebar = self.render_titlebar();
        let rail = Self::render_activity_rail();
        let timeline = self.render_timeline(cx);
        let sidebar = self.render_sidebar(cx);
        let inspector = (self.selected_part.is_some() && window.bounds().size.width >= px(1_240.0))
            .then(|| self.render_inspector());
        let statusline = self.render_statusline();
        let message_count = match &self.timeline {
            TimelineState::Ready { messages, .. } => messages.len(),
            TimelineState::Empty | TimelineState::Loading { .. } | TimelineState::Failed { .. } => {
                0
            }
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(color::BASE))
            .font_family(UI_FONT)
            .child(titlebar)
            .child(
                div()
                    .min_h_0()
                    .flex_1()
                    .flex()
                    .child(rail)
                    .child(sidebar)
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(pane_header(
                                "CONVERSATION",
                                format!("{message_count:>4} MSG"),
                            ))
                            .child(div().min_h_0().flex_1().child(timeline)),
                    )
                    .children(inspector),
            )
            .child(statusline)
    }
}

pub(super) fn centered_message(message: &'static str) -> gpui::AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .font_family(MONO_FONT)
        .text_xs()
        .text_color(rgb(color::TEXT_MUTED))
        .child(message)
        .into_any_element()
}

fn pane_header(label: &'static str, value: String) -> gpui::AnyElement {
    div()
        .h(px(ui_size::PANE_HEADER))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px_2()
        .bg(rgb(color::SURFACE))
        .border_b_1()
        .border_color(rgb(color::BORDER))
        .font_family(MONO_FONT)
        .text_xs()
        .text_color(rgb(color::TEXT_DIM))
        .child(label)
        .child(value)
        .into_any_element()
}

fn rail_item(label: &'static str, name: &'static str, active: bool) -> gpui::AnyElement {
    div()
        .id(name)
        .size(px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .cursor_pointer()
        .bg(rgb(if active {
            color::SELECTED
        } else {
            color::SURFACE
        }))
        .text_color(rgb(if active {
            color::ACCENT
        } else {
            color::TEXT_MUTED
        }))
        .hover(|element| element.bg(rgb(color::HOVER)).text_color(rgb(color::TEXT)))
        .child(label)
        .into_any_element()
}
