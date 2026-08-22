use std::sync::Arc;

use opencode_gpui::{
    api::PromptFile,
    model::{Message, MessageRecord, MessageTime, ModelRef, Part, UserMessage},
};

use super::{TimelineState, tabs::DirectoryTab};

#[allow(clippy::too_many_arguments)]
pub(super) fn push_optimistic_message(
    tab: &mut DirectoryTab,
    session_id: &str,
    message_id: &str,
    part_id: &str,
    text: &str,
    files: &[PromptFile],
    agent: String,
    model: ModelRef,
    created: u64,
) -> Option<MessageRecord> {
    let TimelineState::Ready { messages, .. } = &mut tab.timeline else {
        return None;
    };
    let mut text_data = serde_json::Map::new();
    text_data.insert("text".into(), serde_json::Value::String(text.into()));
    let mut parts = vec![Part {
        id: part_id.into(),
        session_id: session_id.into(),
        message_id: message_id.into(),
        kind: "text".into(),
        data: Arc::new(text_data),
    }];
    parts.extend(files.iter().enumerate().map(|(index, file)| {
        let mut data = serde_json::Map::new();
        data.insert("mime".into(), serde_json::Value::String(file.mime.clone()));
        data.insert(
            "filename".into(),
            serde_json::Value::String(file.filename.clone()),
        );
        data.insert("url".into(), serde_json::Value::String(file.url.clone()));
        Part {
            id: format!("{part_id}_file_{index}"),
            session_id: session_id.into(),
            message_id: message_id.into(),
            kind: "file".into(),
            data: Arc::new(data),
        }
    }));
    let record = MessageRecord {
        info: Message::User(UserMessage {
            id: message_id.into(),
            session_id: session_id.into(),
            time: MessageTime {
                created,
                completed: None,
            },
            agent,
            model,
        }),
        parts,
    };
    messages.push(record.clone());
    tab.timeline_scroll.scroll_to_bottom();
    tab.follow_tail = true;
    Some(record)
}
