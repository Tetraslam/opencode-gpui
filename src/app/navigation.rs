use gpui::{
    ClickEvent, Context, Focusable, SharedString, Window, actions, div, prelude::*, px, rgb,
};
use opencode_gpui::{
    editor::TextEditor,
    event::SessionStatus,
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{Workspace, command_palette::Overlay, directory_path::directory_name};

actions!(
    workspace_navigation,
    [
        ToggleDirectoryPicker,
        ToggleCommandPalette,
        DismissOverlay,
        ToggleSessions,
        NextDirectory,
        PreviousDirectory,
        CloseDirectory,
        NewSession,
        PreviousSession,
        NextSession,
        SelectPreviousOverlayItem,
        SelectNextOverlayItem
    ]
);

impl Workspace {
    pub(super) fn toggle_directory_picker(
        &mut self,
        _: &ToggleDirectoryPicker,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.overlay = if self.overlay == Overlay::Directory {
            Overlay::None
        } else {
            Overlay::Directory
        };
        if self.overlay == Overlay::Directory {
            self.overlay_selection = 0;
            self.directory_editor.update(cx, TextEditor::clear);
            self.directory_editor
                .read(cx)
                .focus_handle(cx)
                .focus(window);
        } else {
            self.focus_active_editor(window, cx);
        }
        cx.notify();
    }

    pub(super) fn submit_directory_picker(&mut self, query: &str, cx: &mut Context<Self>) {
        if let Some(directory) = self
            .directory_candidates(query)
            .get(self.overlay_selection)
            .cloned()
        {
            self.create_directory_session(&directory, cx);
        } else if !query.trim().is_empty() {
            self.create_directory_session(query, cx);
        }
    }

    pub(super) fn toggle_sessions(
        &mut self,
        _: &ToggleSessions,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sessions_open = !self.sessions_open;
        cx.notify();
    }

    pub(super) fn next_directory(
        &mut self,
        _: &NextDirectory,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.tabs.is_empty() {
            self.switch_directory((self.active_tab + 1) % self.tabs.len(), cx);
            self.focus_active_editor(window, cx);
        }
    }

    pub(super) fn previous_directory(
        &mut self,
        _: &PreviousDirectory,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.tabs.is_empty() {
            let index = self
                .active_tab
                .checked_sub(1)
                .unwrap_or(self.tabs.len() - 1);
            self.switch_directory(index, cx);
            self.focus_active_editor(window, cx);
        }
    }

    pub(super) fn close_directory_action(
        &mut self,
        _: &CloseDirectory,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_directory(self.active_tab, cx);
    }

    pub(super) fn close_directory(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }
        self.capture_active_draft(true, cx);
        self.dismiss_transients();
        let directory = self.tabs.remove(index).directory;
        self.connected_directories.remove(&directory);
        if index < self.active_tab || self.active_tab == self.tabs.len() {
            self.active_tab = self.active_tab.saturating_sub(1);
        }
        if self.tabs.is_empty() {
            self.overlay = Overlay::Directory;
            self.focus_overlay_on_render = true;
        } else {
            self.overlay = Overlay::None;
            self.focus_editor_on_render = true;
        }
        cx.notify();
    }

    pub(super) fn switch_directory(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() || index == self.active_tab {
            self.dismiss_transients();
            cx.notify();
            return;
        }
        self.capture_active_draft(true, cx);
        self.dismiss_transients();
        self.active_tab = index;
        cx.notify();
    }

    pub(super) fn focus_active_editor(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = self.active_tab().map(|tab| tab.editor.clone()) {
            editor.read(cx).focus_handle(cx).focus(window);
        }
    }

    pub(super) fn render_titlebar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let active = self.active_tab;
        let tabs = self.tabs.iter().enumerate().map(|(index, tab)| {
            let directory = tab.directory.clone();
            let busy = self.statuses.iter().any(|(session_id, status)| {
                matches!(status, SessionStatus::Busy | SessionStatus::Retry { .. })
                    && self.session_directory(session_id) == Some(directory.as_str())
            });
            div()
                .id(SharedString::from(format!("directory-tab-{index}")))
                .h_full()
                .max_w(px(190.0))
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .cursor_pointer()
                .border_r_1()
                .border_color(rgb(color::BORDER))
                .when(index == active, |tab| tab.bg(rgb(color::SELECTED)))
                .hover(|tab| tab.bg(rgb(color::HOVER)))
                .on_click(cx.listener(move |workspace, _: &ClickEvent, window, cx| {
                    workspace.switch_directory(index, cx);
                    if let Some(editor) = workspace.active_tab().map(|tab| tab.editor.clone()) {
                        editor.read(cx).focus_handle(cx).focus(window);
                    }
                }))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .text_color(rgb(if index == active {
                            color::TEXT_BRIGHT
                        } else {
                            color::TEXT_DIM
                        }))
                        .child(SharedString::from(directory_name(&directory).to_owned())),
                )
                .child(tab_close_button(index, cx))
                .when(busy, |tab| {
                    tab.child(div().size(px(5.0)).rounded_full().bg(rgb(color::GREEN)))
                })
        });
        let reconnecting = self.active_directory().is_some() && !self.active_directory_is_live();

        div()
            .h(px(ui_size::TITLEBAR))
            .flex_none()
            .flex()
            .items_center()
            .bg(rgb(color::SURFACE))
            .border_b_1()
            .border_color(rgb(color::BORDER))
            .font_family(MONO_FONT)
            .text_xs()
            .child(
                div()
                    .w(px(ui_size::ACTIVITY_RAIL))
                    .h_full()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(color::ACCENT))
                    .child("oc"),
            )
            .child(div().min_w_0().h_full().flex_1().flex().children(tabs))
            .child(
                div()
                    .id("open-directory")
                    .h_full()
                    .w(px(34.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .text_color(rgb(color::TEXT_DIM))
                    .hover(|button| button.bg(rgb(color::HOVER)).text_color(rgb(color::TEXT)))
                    .on_click(cx.listener(|workspace, _, window, cx| {
                        workspace.toggle_directory_picker(&ToggleDirectoryPicker, window, cx);
                    }))
                    .child("+"),
            )
            .children(reconnecting.then(|| {
                div()
                    .px_3()
                    .flex_none()
                    .text_center()
                    .text_color(rgb(color::YELLOW))
                    .child("reconnecting")
            }))
            .into_any_element()
    }

    pub(super) fn session_directory(&self, session_id: &str) -> Option<&str> {
        let super::ServerState::Ready { sessions, .. } = &self.server_state else {
            return None;
        };
        sessions
            .iter()
            .find(|session| session.id == session_id)
            .map(|session| session.directory.as_str())
    }

    fn active_directory_is_live(&self) -> bool {
        self.active_directory()
            .is_some_and(|directory| self.connected_directories.contains(directory))
    }
}

fn tab_close_button(index: usize, cx: &mut Context<Workspace>) -> gpui::AnyElement {
    div()
        .id(SharedString::from(format!("close-directory-{index}")))
        .size(px(20.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .text_color(rgb(color::TEXT_MUTED))
        .hover(|button| button.bg(rgb(color::HOVER)).text_color(rgb(color::TEXT)))
        .on_click(cx.listener(move |workspace, _, _, cx| {
            cx.stop_propagation();
            workspace.close_directory(index, cx);
        }))
        .child("x")
        .into_any_element()
}
