use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use gpui::Task;
use opencode_gpui::{
    event::{Event, SessionStatus},
    model::{Message, Part, Session, SessionTime},
};

use super::{ServerState, TimelineState, Workspace};

fn session(id: &str, updated: u64) -> Session {
    Session {
        id: id.into(),
        project_id: "project".into(),
        directory: "/workspace".into(),
        parent_id: None,
        title: id.into(),
        version: "1.18.16".into(),
        time: SessionTime {
            created: 1,
            updated,
            compacting: None,
        },
    }
}

fn workspace(sessions: Vec<Session>, timeline: TimelineState) -> Workspace {
    Workspace {
        client: None,
        server: "test".into(),
        server_state: ServerState::Ready {
            version: "1.18.16".into(),
            sessions: Arc::new(sessions),
        },
        timeline,
        statuses: Arc::new(HashMap::new()),
        expanded_parts: HashSet::new(),
        selected_part: None,
        detail_cache: HashMap::new(),
        preparing_parts: HashSet::new(),
        detail_tasks: Vec::new(),
        message_limit: super::MESSAGE_PAGE,
        history_loading: false,
        history_exhausted: false,
        live: true,
        _load: Task::ready(()),
        _events: Task::ready(()),
        timeline_load: None,
        history_load: None,
    }
}

#[test]
fn reduces_session_updates_without_losing_sort_order() {
    let mut workspace = workspace(vec![session("a", 1), session("b", 2)], TimelineState::Empty);
    workspace.apply_events(vec![
        Event::SessionUpdated(session("a", 3)),
        Event::SessionStatus {
            session_id: "a".into(),
            status: SessionStatus::Busy,
        },
    ]);

    let ServerState::Ready { sessions, .. } = &workspace.server_state else {
        panic!("server should remain ready");
    };
    assert_eq!(
        sessions
            .iter()
            .map(|session| session.id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert_eq!(workspace.statuses["a"], SessionStatus::Busy);
}

#[test]
fn reduces_streamed_message_parts_in_place() {
    let message: Message = serde_json::from_str(
        r#"{"id":"msg_1","sessionID":"ses_1","role":"user","time":{"created":1},"agent":"build","model":{"providerID":"openai","modelID":"test"}}"#,
    )
    .unwrap();
    let first: Part = serde_json::from_str(
        r#"{"id":"part_1","sessionID":"ses_1","messageID":"msg_1","type":"text","text":"hel"}"#,
    )
    .unwrap();
    let complete: Part = serde_json::from_str(
        r#"{"id":"part_1","sessionID":"ses_1","messageID":"msg_1","type":"text","text":"hello"}"#,
    )
    .unwrap();
    let mut workspace = workspace(
        Vec::new(),
        TimelineState::Ready {
            session_id: "ses_1".into(),
            title: "session".into(),
            messages: Vec::new(),
        },
    );
    workspace.apply_events(vec![
        Event::MessageUpdated(message),
        Event::MessagePartUpdated {
            part: first,
            delta: Some("hel".into()),
        },
        Event::MessagePartUpdated {
            part: complete,
            delta: Some("lo".into()),
        },
    ]);

    let TimelineState::Ready { messages, .. } = &workspace.timeline else {
        panic!("timeline should remain loaded");
    };
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].parts.len(), 1);
    assert_eq!(messages[0].parts[0].text(), Some("hello"));
}
