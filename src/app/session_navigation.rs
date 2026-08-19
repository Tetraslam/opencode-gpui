use gpui::{Context, Window};

use super::{
    ServerState, Workspace,
    navigation::{NewSession, NextSession, PreviousSession},
};

impl Workspace {
    pub(super) fn new_session_action(
        &mut self,
        _: &NewSession,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.create_active_session(cx);
    }

    pub(super) fn previous_session(
        &mut self,
        _: &PreviousSession,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_session(-1, cx);
    }

    pub(super) fn next_session(&mut self, _: &NextSession, _: &mut Window, cx: &mut Context<Self>) {
        self.move_session(1, cx);
    }

    pub(super) fn move_session(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some(directory) = self.active_directory() else {
            return;
        };
        let selected = self.active_tab().and_then(|tab| tab.timeline.session_id());
        let ServerState::Ready { sessions, .. } = &self.server_state else {
            return;
        };
        let roots = sessions
            .iter()
            .filter(|session| session.parent_id.is_none() && session.directory == directory)
            .collect::<Vec<_>>();
        if roots.is_empty() {
            return;
        }
        let current = selected
            .and_then(|id| roots.iter().position(|session| session.id == id))
            .unwrap_or_default();
        let next =
            usize::try_from((current.cast_signed() + delta).rem_euclid(roots.len().cast_signed()))
                .unwrap_or_default();
        let id = roots[next].id.clone();
        let title = super::format::display_title(roots[next]).into();
        self.select_session(id, title, cx);
        self.focus_editor_on_render = true;
    }
}
