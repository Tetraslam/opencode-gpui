use gpui::{App, FocusHandle, Focusable, MouseButton, Render, Window, div, prelude::*, px, rgb};
use opencode_gpui::{
    event::SessionStatus,
    theme::{MONO_FONT, UI_FONT, color, size as ui_size},
};

use super::{
    ServerState, TimelineState, Workspace, command_palette::Overlay,
    navigation::ToggleCommandPalette,
};

impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Workspace {
    fn render_activity_rail(&self, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let sessions_open = self.sessions_open;
        let sessions = div()
            .id("sessions")
            .size(px(28.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .cursor_pointer()
            .bg(rgb(if sessions_open {
                color::SELECTED
            } else {
                color::SURFACE
            }))
            .text_color(rgb(if sessions_open {
                color::ACCENT
            } else {
                color::TEXT_MUTED
            }))
            .hover(|item| item.bg(rgb(color::HOVER)).text_color(rgb(color::TEXT)))
            .on_click(cx.listener(|workspace, _, _, cx| {
                workspace.sessions_open = !workspace.sessions_open;
                cx.notify();
            }))
            .child("s");
        let commands = div()
            .id("commands")
            .size(px(28.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .cursor_pointer()
            .when(self.overlay == Overlay::Command, |item| {
                item.bg(rgb(color::SELECTED)).text_color(rgb(color::ACCENT))
            })
            .when(self.overlay != Overlay::Command, |item| {
                item.text_color(rgb(color::TEXT_MUTED))
            })
            .hover(|item| item.bg(rgb(color::HOVER)).text_color(rgb(color::TEXT)))
            .on_click(cx.listener(|workspace, _, window, cx| {
                workspace.toggle_command_palette(&ToggleCommandPalette, window, cx);
            }))
            .child("k");
        div()
            .w(px(ui_size::ACTIVITY_RAIL))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_start()
            .py_2()
            .bg(rgb(color::SURFACE))
            .border_r_1()
            .border_color(rgb(color::BORDER))
            .font_family(MONO_FONT)
            .text_xs()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children([sessions, commands]),
            )
            .into_any_element()
    }

    fn render_statusline(&self) -> gpui::AnyElement {
        let active_directory = self.active_directory();
        let (sessions, busy) = match &self.server_state {
            ServerState::Ready { .. } => (
                active_directory.map_or(0, |directory| self.directory_session_count(directory)),
                self.statuses
                    .iter()
                    .filter(|(session_id, status)| {
                        matches!(status, SessionStatus::Busy)
                            && self.session_directory(session_id) == active_directory
                    })
                    .count(),
            ),
            ServerState::Loading | ServerState::Failed(_) => (0, 0),
        };
        div()
            .h(px(ui_size::STATUSLINE))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .bg(rgb(color::SURFACE))
            .border_t_1()
            .border_color(rgb(color::BORDER))
            .font_family(MONO_FONT)
            .text_xs()
            .text_color(rgb(color::TEXT_DIM))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .text_color(rgb(color::TEXT))
                    .child(
                        self.active_directory()
                            .map_or_else(|| self.server.clone(), |path| path.to_owned().into()),
                    ),
            )
            .child(if busy == 0 {
                format!("{sessions} sessions")
            } else {
                format!("{sessions} sessions  ·  {busy} running")
            })
            .into_any_element()
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        self.restore_pending_detail_anchor(window);
        if std::mem::take(&mut self.focus_editor_on_render)
            && let Some(editor) = self.active_tab().map(|tab| tab.editor.clone())
        {
            window.defer(cx, move |window, cx| {
                editor.read(cx).focus_handle(cx).focus(window);
            });
        }
        if std::mem::take(&mut self.focus_overlay_on_render) {
            let editor = match self.overlay {
                super::command_palette::Overlay::Directory => Some(self.directory_editor.clone()),
                super::command_palette::Overlay::Command => Some(self.command_editor.clone()),
                super::command_palette::Overlay::None => None,
            };
            if let Some(editor) = editor {
                window.defer(cx, move |window, cx| {
                    editor.read(cx).focus_handle(cx).focus(window);
                });
            }
        }
        let timeline = self.render_timeline(cx);
        let composer = self.render_composer(cx);
        let sidebar = self.sessions_open.then(|| self.render_sidebar(cx));
        let sidebar_resize = self
            .sessions_open
            .then(|| Workspace::render_sidebar_resize_handle(cx));
        let inspector_open = self
            .active_tab()
            .is_some_and(|tab| tab.timeline.session_id().is_some())
            && window.bounds().size.width >= px(ui_size::INSPECTOR_BREAKPOINT);
        let inspector = inspector_open.then(|| self.render_inspector());
        let inspector_resize = inspector_open.then(|| Self::render_inspector_resize_handle(cx));
        let message_count = match self.active_tab().map(|tab| &tab.timeline) {
            Some(TimelineState::Ready { messages, .. }) => messages.len(),
            None
            | Some(
                TimelineState::Empty | TimelineState::Loading { .. } | TimelineState::Failed { .. },
            ) => 0,
        };
        let conversation_title = self
            .active_tab()
            .and_then(|tab| tab.timeline.title())
            .unwrap_or_else(|| "conversation".into());
        div()
            .size_full()
            .relative()
            .key_context("Workspace")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Workspace::toggle_directory_picker))
            .on_action(cx.listener(Workspace::toggle_command_palette))
            .on_action(cx.listener(Workspace::dismiss_overlay))
            .on_action(cx.listener(Workspace::toggle_sessions))
            .on_action(cx.listener(Workspace::next_directory))
            .on_action(cx.listener(Workspace::previous_directory))
            .on_action(cx.listener(Workspace::close_directory_action))
            .on_action(cx.listener(Workspace::new_session_action))
            .on_action(cx.listener(Workspace::previous_session))
            .on_action(cx.listener(Workspace::next_session))
            .on_action(cx.listener(Workspace::select_previous_overlay_item))
            .on_action(cx.listener(Workspace::select_next_overlay_item))
            .on_mouse_down(MouseButton::Left, cx.listener(Workspace::dismiss_on_click))
            .on_mouse_move(cx.listener(Workspace::handle_pointer_drag))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(Workspace::finish_pointer_drag),
            )
            .flex()
            .flex_col()
            .bg(rgb(color::BASE))
            .font_family(UI_FONT)
            .child(self.render_titlebar(cx))
            .child(
                div()
                    .min_h_0()
                    .flex_1()
                    .flex()
                    .child(self.render_activity_rail(cx))
                    .children(sidebar)
                    .children(sidebar_resize)
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .bg(rgb(color::BASE))
                            .child(pane_header(
                                conversation_title,
                                format!("{message_count:>4} messages"),
                            ))
                            .child(div().min_h_0().flex_1().child(timeline))
                            .child(composer),
                    )
                    .children(inspector_resize)
                    .children(inspector),
            )
            .child(self.render_statusline())
            .children(self.render_directory_picker(cx))
            .children(self.render_command_palette(cx))
            .children(self.render_composer_completion(cx))
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

pub(super) fn pane_header(label: gpui::SharedString, value: String) -> gpui::AnyElement {
    div()
        .h(px(ui_size::PANE_HEADER))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
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
