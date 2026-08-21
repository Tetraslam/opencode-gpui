use gpui::{Context, Focusable, Window, actions};
use opencode_gpui::editor::TextEditor;

use super::{Workspace, command_palette::Overlay};

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
        CompleteDirectory,
        SelectPreviousOverlayItem,
        SelectNextOverlayItem,
        SubmitMessageAction
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
            self.refresh_directory_suggestions(String::new(), cx);
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
        if query != self.directory_suggestion_query {
            if !query.trim().is_empty() {
                self.create_directory_session(query, cx);
            }
            return;
        }
        if let Some(directory) = self
            .directory_suggestions
            .get(self.overlay_selection)
            .cloned()
        {
            self.create_directory_session(&directory, cx);
        } else if !query.trim().is_empty() {
            self.create_directory_session(query, cx);
        }
    }

    pub(super) fn complete_directory(
        &mut self,
        _: &CompleteDirectory,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.complete_directory_picker(cx);
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
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.tabs.is_empty() {
            self.switch_directory((self.active_tab + 1) % self.tabs.len(), cx);
        }
    }

    pub(super) fn previous_directory(
        &mut self,
        _: &PreviousDirectory,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.tabs.is_empty() {
            let index = self
                .active_tab
                .checked_sub(1)
                .unwrap_or(self.tabs.len() - 1);
            self.switch_directory(index, cx);
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
        self.persist_workspace_layout(cx);
        cx.notify();
    }

    pub(super) fn focus_active_editor(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = self.active_tab().map(|tab| tab.editor.clone()) {
            editor.read(cx).focus_handle(cx).focus(window);
        }
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

    pub(super) fn active_directory_is_live(&self) -> bool {
        self.active_directory()
            .is_some_and(|directory| self.connected_directories.contains(directory))
    }
}
