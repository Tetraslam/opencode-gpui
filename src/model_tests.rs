use super::*;

fn session(id: &str, updated: u64) -> Session {
    Session {
        id: id.into(),
        project_id: "project".into(),
        directory: "/tmp/project".into(),
        parent_id: None,
        title: id.into(),
        version: "1.0.0".into(),
        time: SessionTime {
            created: updated,
            updated,
            compacting: None,
        },
        revert: None,
    }
}

#[test]
fn sorts_sessions_by_most_recent_update() {
    let mut sessions = vec![session("old", 1), session("new", 3), session("middle", 2)];
    sort_sessions(&mut sessions);
    assert_eq!(
        sessions
            .iter()
            .map(|session| session.id.as_str())
            .collect::<Vec<_>>(),
        ["new", "middle", "old"]
    );
}

#[test]
fn preserves_unknown_part_payloads() {
    let part: Part = serde_json::from_str(
        r#"{"id":"part_1","sessionID":"ses_1","messageID":"msg_1","type":"future-part","answer":42}"#,
    )
    .unwrap();
    assert_eq!(part.kind, "future-part");
    assert_eq!(part.data["answer"], 42);
    assert_eq!(
        part.summary().as_deref(),
        Some("unsupported part: future-part")
    );
}

#[test]
fn summarizes_tool_lifecycle_without_modeling_tool_payloads() {
    let part: Part = serde_json::from_str(
        r#"{"id":"part_1","sessionID":"ses_1","messageID":"msg_1","type":"tool","tool":"bash","state":{"status":"completed","title":"cargo test","output":"ok","input":{},"metadata":{},"time":{"start":1,"end":2}}}"#,
    )
    .unwrap();
    assert_eq!(
        part.summary().as_deref(),
        Some("bash: cargo test [completed]")
    );
}
