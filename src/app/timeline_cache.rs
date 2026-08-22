use std::collections::HashMap;

use opencode_gpui::model::{Message, MessageRecord, Part};

use super::{MESSAGE_PAGE, PendingDelta, part_merge::append_part_field, part_merge::merge_part};

#[derive(Default)]
pub(crate) struct TimelineCache {
    entries: HashMap<String, Vec<MessageRecord>>,
}

impl TimelineCache {
    pub(super) fn get(&self, session_id: &str) -> Option<&[MessageRecord]> {
        self.entries.get(session_id).map(Vec::as_slice)
    }

    pub(super) fn contains(&self, session_id: &str) -> bool {
        self.entries.contains_key(session_id)
    }

    pub(super) fn replace(&mut self, session_id: &str, messages: &[MessageRecord]) {
        let start = messages.len().saturating_sub(MESSAGE_PAGE);
        self.entries
            .insert(session_id.to_owned(), messages[start..].to_vec());
    }

    pub(super) fn push_optimistic(&mut self, session_id: &str, message: MessageRecord) {
        let messages = self.entries.entry(session_id.to_owned()).or_default();
        messages.push(message);
        trim(messages);
    }

    pub(super) fn update_message(&mut self, info: &Message, pending: &[Part]) -> bool {
        let Some(messages) = self.entries.get_mut(info.session_id()) else {
            return false;
        };
        if let Some(message) = messages.iter_mut().find(|item| item.info.id() == info.id()) {
            message.info = info.clone();
            for part in pending {
                merge_part(&mut message.parts, part.clone(), None);
            }
        } else {
            messages.push(MessageRecord {
                info: info.clone(),
                parts: pending.to_vec(),
            });
            trim(messages);
        }
        true
    }

    pub(super) fn remove_message(&mut self, session_id: &str, message_id: &str) {
        if let Some(messages) = self.entries.get_mut(session_id) {
            messages.retain(|message| message.info.id() != message_id);
        }
    }

    pub(super) fn update_part(
        &mut self,
        part: &Part,
        delta: Option<&str>,
        pending: &[PendingDelta],
    ) -> (bool, bool) {
        let Some(messages) = self.entries.get_mut(&part.session_id) else {
            return (false, false);
        };
        let Some(message) = messages
            .iter_mut()
            .find(|message| message.info.id() == part.message_id)
        else {
            return (true, false);
        };
        merge_part(&mut message.parts, part.clone(), delta);
        if let Some(current) = message.parts.iter_mut().find(|item| item.id == part.id) {
            for item in pending.iter().filter(|item| item.part_id == part.id) {
                append_part_field(current, &item.field, &item.delta);
            }
        }
        (true, true)
    }

    pub(super) fn update_delta(
        &mut self,
        session_id: &str,
        message_id: &str,
        part_id: &str,
        field: &str,
        delta: &str,
    ) -> (bool, bool) {
        let Some(messages) = self.entries.get_mut(session_id) else {
            return (false, false);
        };
        let Some(part) = messages
            .iter_mut()
            .find(|message| message.info.id() == message_id)
            .and_then(|message| message.parts.iter_mut().find(|part| part.id == part_id))
        else {
            return (true, false);
        };
        append_part_field(part, field, delta);
        (true, true)
    }

    pub(super) fn remove_part(&mut self, session_id: &str, message_id: &str, part_id: &str) {
        if let Some(message) = self.entries.get_mut(session_id).and_then(|messages| {
            messages
                .iter_mut()
                .find(|message| message.info.id() == message_id)
        }) {
            message.parts.retain(|part| part.id != part_id);
        }
    }

    pub(super) fn remove_session(&mut self, session_id: &str) {
        self.entries.remove(session_id);
    }
}

fn trim(messages: &mut Vec<MessageRecord>) {
    let excess = messages.len().saturating_sub(MESSAGE_PAGE);
    if excess > 0 {
        messages.drain(..excess);
    }
}
