use std::collections::HashSet;

use opencode_gpui::event::SessionStatus;

use super::{ServerState, Workspace};

impl Workspace {
    pub(super) fn busy_directories(&self) -> HashSet<&str> {
        let ServerState::Ready { sessions, .. } = &self.server_state else {
            return HashSet::new();
        };
        sessions
            .iter()
            .filter(|session| {
                self.statuses.get(&session.id).is_some_and(|status| {
                    matches!(status, SessionStatus::Busy | SessionStatus::Retry { .. })
                })
            })
            .map(|session| session.directory.as_str())
            .collect()
    }
}
