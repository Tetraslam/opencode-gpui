use std::sync::Arc;

use opencode_gpui::model::{
    Message, MessageRecord, MessageTime, ModelRef, Part, Session, SessionTime, UserMessage,
};

static TEST_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub(super) fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "opencode-gpui-test-{name}-{}",
        TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ))
}

pub(super) fn session(id: &str, updated: u64) -> Session {
    session_in(id, "/workspace", updated)
}

pub(super) fn session_in(id: &str, directory: &str, updated: u64) -> Session {
    Session {
        id: id.into(),
        project_id: "project".into(),
        directory: directory.into(),
        parent_id: None,
        title: id.into(),
        version: "1.18.16".into(),
        time: SessionTime {
            created: 1,
            updated,
            compacting: None,
        },
        revert: None,
    }
}

pub(super) fn message_record(id: &str, session_id: &str, created: u64) -> MessageRecord {
    let mut data = serde_json::Map::new();
    data.insert(
        "text".into(),
        serde_json::Value::String(format!("message {created}")),
    );
    MessageRecord {
        info: Message::User(UserMessage {
            id: id.into(),
            session_id: session_id.into(),
            time: MessageTime {
                created,
                completed: Some(created + 1),
            },
            agent: "build".into(),
            model: ModelRef {
                provider_id: "openai".into(),
                model_id: "test".into(),
            },
        }),
        parts: vec![Part {
            id: format!("part-{id}"),
            session_id: session_id.into(),
            message_id: id.into(),
            kind: "text".into(),
            data: Arc::new(data),
        }],
    }
}
