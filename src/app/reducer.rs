use std::sync::Arc;

use opencode_gpui::{
    event::{Event, SessionStatus},
    model::{MessageRecord, Part, Session, sort_sessions},
};

use super::{
    PendingDelta, ServerState, TimelineState, Workspace,
    part_merge::{append_part_field, merge_part},
};

impl Workspace {
    pub(super) fn apply_message_record(&mut self, message: MessageRecord, directory: Option<&str>) {
        self.update_message(&message.info, directory);
        for part in message.parts {
            self.update_part(part, None, directory);
        }
    }

    pub(super) fn apply_events(&mut self, events: Vec<Event>, directory: Option<&str>) {
        for event in events {
            match event {
                Event::ServerConnected | Event::Unknown(_) => {}
                Event::SessionCreated(session) | Event::SessionUpdated(session) => {
                    self.upsert_session(session);
                }
                Event::SessionDeleted(session) => self.delete_session(&session),
                Event::SessionStatus { session_id, status } => {
                    if status == SessionStatus::Busy {
                        for tab in &mut self.tabs {
                            if tab.timeline.session_id() == Some(session_id.as_str()) {
                                tab.prompt_error = None;
                            }
                        }
                    }
                    Arc::make_mut(&mut self.statuses).insert(session_id, status);
                }
                Event::SessionIdle { session_id } => {
                    Arc::make_mut(&mut self.statuses).insert(session_id, SessionStatus::Idle);
                }
                Event::MessageUpdated(info) => self.update_message(&info, directory),
                Event::MessageRemoved {
                    session_id,
                    message_id,
                } => {
                    self.remove_message(&session_id, &message_id, directory);
                }
                Event::MessagePartUpdated { part, delta } => {
                    self.update_part(part, delta.as_deref(), directory);
                }
                Event::MessagePartDelta {
                    session_id,
                    message_id,
                    part_id,
                    field,
                    delta,
                } => self.update_part_delta(
                    &session_id,
                    &message_id,
                    &part_id,
                    &field,
                    delta,
                    directory,
                ),
                Event::MessagePartRemoved {
                    session_id,
                    message_id,
                    part_id,
                } => {
                    self.remove_part(&session_id, &message_id, &part_id, directory);
                }
            }
        }
    }

    fn upsert_session(&mut self, session: Session) {
        let ServerState::Ready { sessions, .. } = &mut self.server_state else {
            return;
        };
        let sessions = Arc::make_mut(sessions);
        if let Some(current) = sessions.iter_mut().find(|current| current.id == session.id) {
            *current = session;
        } else {
            sessions.push(session);
        }
        sort_sessions(sessions);
    }

    fn delete_session(&mut self, session: &Session) {
        if let ServerState::Ready { sessions, .. } = &mut self.server_state {
            Arc::make_mut(sessions).retain(|current| current.id != session.id);
        }
        Arc::make_mut(&mut self.statuses).remove(&session.id);
        for tab in &mut self.tabs {
            if tab.timeline.session_id() == Some(session.id.as_str()) {
                tab.timeline = TimelineState::Empty;
                tab.timeline_load = None;
            }
        }
    }

    fn update_message(&mut self, info: &opencode_gpui::model::Message, directory: Option<&str>) {
        let pending = self.pending_parts.remove(info.id()).unwrap_or_default();
        for tab in &mut self.tabs {
            if directory.is_some_and(|directory| tab.directory != directory) {
                continue;
            }
            let TimelineState::Ready {
                session_id,
                messages,
                ..
            } = &mut tab.timeline
            else {
                continue;
            };
            if info.session_id() != session_id {
                continue;
            }
            if let Some(message) = messages.iter_mut().find(|item| item.info.id() == info.id()) {
                message.info = info.clone();
                for part in &pending {
                    merge_part(&mut message.parts, part.clone(), None);
                }
            } else {
                messages.push(MessageRecord {
                    info: info.clone(),
                    parts: pending.clone(),
                });
            }
        }
    }

    fn remove_message(&mut self, session_id: &str, message_id: &str, directory: Option<&str>) {
        for tab in &mut self.tabs {
            if directory.is_some_and(|directory| tab.directory != directory) {
                continue;
            }
            if let TimelineState::Ready {
                session_id: selected,
                messages,
                ..
            } = &mut tab.timeline
                && selected == session_id
            {
                messages.retain(|message| message.info.id() != message_id);
            }
        }
    }

    fn update_part(&mut self, part: Part, delta: Option<&str>, directory: Option<&str>) {
        let message_id = part.message_id.clone();
        let part_id = part.id.clone();
        let mut pending = self.pending_deltas.remove(&message_id).unwrap_or_default();
        let mut matched_timeline = false;
        let mut matched_message = false;
        for tab in &mut self.tabs {
            if directory.is_some_and(|directory| tab.directory != directory) {
                continue;
            }
            let TimelineState::Ready {
                session_id,
                messages,
                ..
            } = &mut tab.timeline
            else {
                continue;
            };
            if part.session_id != *session_id {
                continue;
            }
            matched_timeline = true;
            if let Some(message) = messages
                .iter_mut()
                .find(|item| item.info.id() == part.message_id)
            {
                merge_part(&mut message.parts, part.clone(), delta);
                if let Some(current) = message.parts.iter_mut().find(|item| item.id == part_id) {
                    for item in pending.iter().filter(|item| item.part_id == part_id) {
                        append_part_field(current, &item.field, &item.delta);
                    }
                }
                matched_message = true;
            }
        }
        if matched_timeline && !matched_message {
            let parts = self
                .pending_parts
                .entry(part.message_id.clone())
                .or_default();
            merge_part(parts, part, delta);
            if let Some(current) = parts.iter_mut().find(|item| item.id == part_id) {
                for item in pending.iter().filter(|item| item.part_id == part_id) {
                    append_part_field(current, &item.field, &item.delta);
                }
            }
        }
        if matched_timeline {
            pending.retain(|item| item.part_id != part_id);
        }
        if !pending.is_empty() {
            self.pending_deltas.insert(message_id, pending);
        }
    }

    fn update_part_delta(
        &mut self,
        session_id: &str,
        message_id: &str,
        part_id: &str,
        field: &str,
        delta: String,
        directory: Option<&str>,
    ) {
        let mut applied = false;
        let mut matched_timeline = false;
        for tab in &mut self.tabs {
            if directory.is_some_and(|directory| tab.directory != directory) {
                continue;
            }
            let TimelineState::Ready {
                session_id: selected,
                messages,
                ..
            } = &mut tab.timeline
            else {
                continue;
            };
            if selected != session_id {
                continue;
            }
            matched_timeline = true;
            if let Some(part) = messages
                .iter_mut()
                .find(|message| message.info.id() == message_id)
                .and_then(|message| message.parts.iter_mut().find(|part| part.id == part_id))
            {
                append_part_field(part, field, &delta);
                applied = true;
            }
        }
        if matched_timeline && !applied {
            self.pending_deltas
                .entry(message_id.to_owned())
                .or_default()
                .push(PendingDelta {
                    part_id: part_id.to_owned(),
                    field: field.to_owned(),
                    delta,
                });
        }
    }

    fn remove_part(
        &mut self,
        session_id: &str,
        message_id: &str,
        part_id: &str,
        directory: Option<&str>,
    ) {
        for tab in &mut self.tabs {
            if directory.is_some_and(|directory| tab.directory != directory) {
                continue;
            }
            if let TimelineState::Ready {
                session_id: selected,
                messages,
                ..
            } = &mut tab.timeline
                && selected == session_id
                && let Some(message) = messages
                    .iter_mut()
                    .find(|item| item.info.id() == message_id)
            {
                message.parts.retain(|part| part.id != part_id);
            }
        }
    }
}
