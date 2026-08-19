use std::sync::Arc;

use gpui::{
    ClickEvent, Context, CursorStyle, MouseButton, Pixels, SharedString, div, prelude::*, px, rgb,
    uniform_list,
};
use opencode_gpui::{
    event::SessionStatus,
    model::Session,
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{ServerState, Workspace, chrome::centered_message, format};

impl Workspace {
    fn render_session(
        session: &Session,
        selected: bool,
        status: Option<&SessionStatus>,
        title_width: Pixels,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let title: SharedString = format::display_title(session).into();
        let age: SharedString = format::relative_time(session.time.updated).into();
        let status_color = match status {
            Some(SessionStatus::Busy) => color::GREEN,
            Some(SessionStatus::Retry { .. }) => color::YELLOW,
            Some(SessionStatus::Idle) | None => color::BORDER,
        };
        let session_id = session.id.clone();
        let selected_title = title.clone();

        div()
            .id(SharedString::from(session.id.clone()))
            .w_full()
            .h(px(ui_size::SESSION_ROW))
            .overflow_hidden()
            .bg(rgb(color::SURFACE))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(rgb(color::BORDER_SUBTLE))
            .cursor_pointer()
            .when(selected, |row| row.bg(rgb(color::SELECTED)))
            .hover(|row| row.bg(rgb(color::HOVER)))
            .on_click(
                cx.listener(move |workspace, _event: &ClickEvent, _window, cx| {
                    workspace.select_session(session_id.clone(), selected_title.clone(), cx);
                }),
            )
            .child(div().h_full().w(px(2.0)).flex_none().bg(rgb(status_color)))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .items_center()
                    .pl(px(10.0))
                    .pr_3()
                    .gap_2()
                    .when(session.parent_id.is_some(), |row| {
                        row.child(
                            div()
                                .flex_none()
                                .font_family(MONO_FONT)
                                .text_xs()
                                .text_color(rgb(color::TEXT_MUTED))
                                .child("+"),
                        )
                    })
                    .child(
                        div()
                            .w(title_width)
                            .flex_none()
                            .truncate()
                            .text_sm()
                            .text_color(rgb(if selected {
                                color::TEXT_BRIGHT
                            } else {
                                color::TEXT
                            }))
                            .child(title),
                    )
                    .child(
                        div()
                            .w(px(ui_size::AGE_COL))
                            .flex_none()
                            .text_right()
                            .font_family(MONO_FONT)
                            .text_xs()
                            .text_color(rgb(color::TEXT_DIM))
                            .child(age),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn render_sidebar(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let directory = self.active_directory().map(str::to_owned);
        let selected = self
            .active_tab()
            .and_then(|tab| tab.timeline.session_id())
            .map(ToOwned::to_owned);
        let statuses = Arc::clone(&self.statuses);
        let pane_width = self.session_pane_width;
        let title_width = (pane_width - px(72.0)).max(px(80.0));
        let count = directory
            .as_deref()
            .map_or(0, |directory| self.directory_session_count(directory));
        let content = match &self.server_state {
            ServerState::Loading => centered_message("connecting to opencode"),
            ServerState::Failed(error) => div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .px_5()
                .child(
                    div()
                        .text_color(rgb(color::TEXT))
                        .child("server unavailable"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(color::TEXT_DIM))
                        .child(error.clone()),
                )
                .into_any_element(),
            ServerState::Ready { sessions, .. } => {
                let sessions = Arc::clone(sessions);
                let directory = directory.unwrap_or_default();
                let roots = Arc::new(
                    sessions
                        .iter()
                        .enumerate()
                        .filter_map(|(index, session)| {
                            (session.parent_id.is_none() && session.directory == directory)
                                .then_some(index)
                        })
                        .collect::<Vec<_>>(),
                );
                uniform_list(
                    "sessions",
                    roots.len(),
                    cx.processor(
                        move |_workspace, range: std::ops::Range<usize>, _window, cx| {
                            range
                                .map(|index| {
                                    let session = &sessions[roots[index]];
                                    Self::render_session(
                                        session,
                                        selected.as_deref() == Some(session.id.as_str()),
                                        statuses.get(&session.id),
                                        title_width,
                                        cx,
                                    )
                                })
                                .collect()
                        },
                    ),
                )
                .h_full()
                .into_any_element()
            }
        };

        div()
            .w(pane_width)
            .h_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            .flex_none()
            .border_r_1()
            .border_color(rgb(color::BORDER))
            .child(pane_header("sessions", format!("{count:>4}")))
            .child(div().min_h_0().flex_1().child(content))
            .into_any_element()
    }

    pub(super) fn render_sidebar_resize_handle(cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .id("session-pane-resize")
            .w(px(5.0))
            .h_full()
            .flex_none()
            .cursor(CursorStyle::ResizeLeftRight)
            .hover(|handle| handle.bg(rgb(color::BORDER)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|workspace, _, _, cx| {
                    workspace.pane_resize = super::pane_resize::PaneResize::Sessions;
                    cx.notify();
                }),
            )
            .into_any_element()
    }
}

fn pane_header(label: &'static str, value: String) -> gpui::AnyElement {
    div()
        .h(px(ui_size::PANE_HEADER))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .border_b_1()
        .border_color(rgb(color::BORDER))
        .bg(rgb(color::SURFACE))
        .font_family(MONO_FONT)
        .text_xs()
        .text_color(rgb(color::TEXT_DIM))
        .child(label)
        .child(value)
        .into_any_element()
}
