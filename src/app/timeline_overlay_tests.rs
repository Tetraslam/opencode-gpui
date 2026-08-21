use gpui::TestAppContext;
use opencode_gpui::model::MessageRecord;

use super::super::{
    timeline_actions::{MessageAction, action_at},
    timeline_overlay::{extract_entries, preferred_entry_index, rendered_message_index},
};
use super::{TimelineState, workspace};

#[test]
fn extracts_real_user_text_newest_first_and_flattens_it() {
    let messages = vec![
        user_message(
            "old",
            1_000,
            &[("generated", true, false), ("old\nprompt", false, false)],
        ),
        assistant_message(),
        user_message("ignored", 2_000, &[("hidden", false, true)]),
        user_message("new", 3_000, &[("new prompt", false, false)]),
    ];

    let entries = extract_entries(&messages, "");

    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.message_id.as_str())
            .collect::<Vec<_>>(),
        ["new", "old"]
    );
    assert_eq!(entries[1].title, "old prompt");
}

#[test]
fn timeline_filter_is_case_insensitive() {
    let messages = vec![
        user_message("one", 1, &[("Fix Parser", false, false)]),
        user_message("two", 2, &[("write tests", false, false)]),
    ];

    let entries = extract_entries(&messages, "parser");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].message_id, "one");
}

#[test]
fn message_action_routing_matches_upstream_order() {
    assert_eq!(action_at(0), Some(MessageAction::Revert));
    assert_eq!(action_at(1), Some(MessageAction::Copy));
    assert_eq!(action_at(2), Some(MessageAction::Fork));
    assert_eq!(action_at(3), None);
}

#[test]
fn full_history_refresh_preserves_selected_message_id() {
    let initial = extract_entries(
        &[user_message("selected", 2, &[("selected", false, false)])],
        "",
    );
    let refreshed = extract_entries(
        &[
            user_message("older", 1, &[("older", false, false)]),
            user_message("selected", 2, &[("selected", false, false)]),
            user_message("newer", 3, &[("newer", false, false)]),
        ],
        "",
    );

    let index = preferred_entry_index(&refreshed, Some(&initial[0].message_id));

    assert_eq!(refreshed[index].message_id, "selected");
}

#[gpui::test]
fn overlay_history_refresh_does_not_expand_rendered_timeline(cx: &mut TestAppContext) {
    let loaded = user_message("selected", 2, &[("selected", false, false)]);
    let workspace = workspace(
        cx,
        Vec::new(),
        TimelineState::Ready {
            session_id: "session".into(),
            title: "title".into(),
            messages: vec![loaded.clone()],
        },
    );
    workspace.update(cx, |workspace, _| {
        workspace.timeline_history = std::sync::Arc::new(vec![loaded]);
        workspace.timeline_query.clear();
        workspace.timeline_message = Some("selected".into());
        workspace.replace_timeline_history(
            vec![
                user_message("older", 1, &[("older", false, false)]),
                user_message("selected", 2, &[("selected", false, false)]),
                user_message("newer", 3, &[("newer", false, false)]),
            ],
            true,
        );

        let tab = workspace.active_tab().unwrap();
        let TimelineState::Ready { messages, .. } = &tab.timeline else {
            panic!()
        };
        assert_eq!(messages.len(), 1);
        assert_eq!(tab.message_limit, super::super::MESSAGE_PAGE);
        assert!(!tab.history_exhausted);
        assert_eq!(workspace.timeline_message.as_deref(), Some("selected"));
    });
}

#[test]
fn preview_mapping_uses_the_bounded_rendered_timeline() {
    let rendered = vec![
        user_message("loaded", 2, &[("loaded", false, false)]),
        assistant_message(),
    ];

    assert_eq!(rendered_message_index(&rendered, "loaded"), Some(0));
    assert_eq!(rendered_message_index(&rendered, "old-unloaded"), None);
}

fn user_message(id: &str, created: u64, texts: &[(&str, bool, bool)]) -> MessageRecord {
    let parts = texts
        .iter()
        .enumerate()
        .map(|(index, (text, synthetic, ignored))| {
            serde_json::json!({
                "id": format!("part-{id}-{index}"), "sessionID": "session", "messageID": id,
                "type": "text", "text": text, "synthetic": synthetic, "ignored": ignored
            })
        })
        .collect::<Vec<_>>();
    serde_json::from_value(serde_json::json!({
        "info": { "id": id, "sessionID": "session", "role": "user", "time": { "created": created },
            "agent": "build", "model": { "providerID": "test", "modelID": "model" } },
        "parts": parts
    }))
    .unwrap()
}

fn assistant_message() -> MessageRecord {
    serde_json::from_value(serde_json::json!({
        "info": { "id": "assistant", "sessionID": "session", "role": "assistant",
            "time": { "created": 1 }, "parentID": "old", "modelID": "model", "providerID": "test",
            "mode": "build", "finish": "stop" },
        "parts": []
    }))
    .unwrap()
}
