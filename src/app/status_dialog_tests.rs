use std::collections::HashMap;

use gpui::TestAppContext;
use opencode_gpui::api::{McpStatus, StatusConfig, StatusSnapshot};

use super::super::{
    composer_slashes::local_slash,
    debug_dialog::{DebugContext, build_debug_entries, debug_text},
    status_dialog::{
        McpOperation, StatusDialogState, StatusTarget, mcp_operation, mcp_status_label,
        plugin_display,
    },
    workspace_command::Command,
};
use super::{TimelineState, workspace};

#[gpui::test]
fn slash_and_palette_share_status_and_debug_identity(cx: &mut TestAppContext) {
    let workspace = workspace(cx, Vec::new(), TimelineState::Empty);
    cx.read(|cx| {
        let commands = workspace.read(cx).filtered_commands("");
        assert!(commands.contains(&Command::Status));
        assert!(commands.contains(&Command::Debug));
    });
    assert_eq!(local_slash("status"), Some(Command::Status));
    assert_eq!(local_slash("debug"), Some(Command::Debug));
}

#[test]
fn plugin_entries_have_readable_package_and_file_labels() {
    let package = plugin_display(&serde_json::json!("@scope/plugin@1.2.3"));
    assert_eq!(package.name, "@scope/plugin");
    assert_eq!(package.version.as_deref(), Some("1.2.3"));

    let bare = plugin_display(&serde_json::json!("opencode-gpt-imagegen"));
    assert_eq!(bare.name, "opencode-gpt-imagegen");
    assert_eq!(bare.version, None);

    let configured =
        plugin_display(&serde_json::json!(["opencode-goal-plugin", {"maxTurns": 1000}]));
    assert_eq!(configured.name, "opencode-goal-plugin");
    assert_eq!(configured.version, None);

    let file = plugin_display(&serde_json::json!([
        "file:///work/my-plugin/index.ts",
        {"enabled": true}
    ]));
    assert_eq!(file.name, "my-plugin");
    assert_eq!(file.path.as_deref(), Some("/work/my-plugin/index.ts"));

    let unknown = plugin_display(&serde_json::json!({"future": true}));
    assert_eq!(unknown.name, "unrecognized plugin");
}

#[test]
fn every_mcp_status_has_a_useful_rendering_label() {
    let cases = [
        (McpStatus::Connected, "Connected"),
        (McpStatus::Disabled, "Disabled in configuration"),
        (
            McpStatus::Failed {
                error: "boom".into(),
            },
            "boom",
        ),
        (
            McpStatus::NeedsAuth,
            "Needs authentication (run: opencode mcp auth github)",
        ),
        (
            McpStatus::NeedsClientRegistration {
                error: "register first".into(),
            },
            "register first",
        ),
        (
            McpStatus::Unknown {
                status: "warming".into(),
                detail: Some("retrying".into()),
            },
            "warming: retrying",
        ),
    ];
    for (status, expected) in cases {
        assert_eq!(mcp_status_label("github", &status), expected);
    }
}

#[test]
fn only_connected_mcp_servers_toggle_toward_disconnect() {
    let statuses = [
        (McpStatus::Connected, McpOperation::Disconnect),
        (McpStatus::Disabled, McpOperation::Connect),
        (
            McpStatus::Failed {
                error: "failed".into(),
            },
            McpOperation::Connect,
        ),
        (McpStatus::NeedsAuth, McpOperation::Connect),
        (
            McpStatus::NeedsClientRegistration {
                error: "register".into(),
            },
            McpOperation::Connect,
        ),
        (
            McpStatus::Unknown {
                status: "warming".into(),
                detail: None,
            },
            McpOperation::Connect,
        ),
    ];
    for (status, expected) in statuses {
        assert_eq!(mcp_operation(&status), expected);
    }
}

#[test]
fn snapshot_application_sorts_names_and_preserves_selection() {
    let target = StatusTarget {
        directory: "/work".into(),
        session_id: None,
    };
    let mut state = StatusDialogState::default();
    let generation = state.begin(target.clone());
    assert!(state.apply(
        &target,
        generation,
        Ok(snapshot_with(&["zeta", "alpha", "middle"]))
    ));
    assert_eq!(state.mcp_names, ["alpha", "middle", "zeta"]);
    assert!(state.move_selection(-1));
    assert_eq!(state.selected, 2);
    assert!(state.move_selection(1));
    assert_eq!(state.selected, 0);
    state.selected = 1;
    let generation = state.begin(target.clone());
    assert!(state.apply(
        &target,
        generation,
        Ok(snapshot_with(&["middle", "beta", "alpha"]))
    ));
    assert_eq!(state.mcp_names, ["alpha", "beta", "middle"]);
    assert_eq!(state.mcp_names[state.selected], "middle");
}

#[test]
fn pending_operation_blocks_duplicate_activation() {
    let mut state = StatusDialogState::default();
    let generation = state
        .start_operation("github".into(), McpOperation::Connect)
        .unwrap();
    assert!(
        state
            .start_operation("other".into(), McpOperation::Disconnect)
            .is_none()
    );
    state.reset_for_open();
    assert!(!state.operation_is_current(generation, "github"));
}

#[test]
fn stale_status_generations_and_targets_are_ignored() {
    let first = StatusTarget {
        directory: "/one".into(),
        session_id: Some("session-one".into()),
    };
    let second = StatusTarget {
        directory: "/two".into(),
        session_id: Some("session-two".into()),
    };
    let mut state = StatusDialogState::default();
    let old_generation = state.begin(first.clone());
    let generation = state.begin(second.clone());
    assert!(!state.apply(&first, old_generation, Ok(snapshot())));
    assert!(state.snapshot.is_none());
    assert!(state.apply(&second, generation, Ok(snapshot())));
    assert!(state.snapshot.is_some());
}

#[test]
fn debug_text_is_stable_label_value_output() {
    let entries = build_debug_entries(
        DebugContext {
            session_id: Some("ses_1".into()),
            model: Some("openai/gpt-test".into()),
            variant: Some("high".into()),
            server_url: "http://127.0.0.1:4096".into(),
        },
        0,
    );
    let text = debug_text(&entries);
    assert!(text.contains("Date: 1970-01-01T00:00:00Z"));
    assert!(text.contains("Session ID: ses_1"));
    assert!(text.contains("Model: openai/gpt-test"));
    assert!(text.contains("Variant: high"));
    assert!(text.contains("Server URL: http://127.0.0.1:4096"));
    assert!(text.lines().all(|line| line.contains(": ")));
}

fn snapshot() -> StatusSnapshot {
    StatusSnapshot {
        mcp: HashMap::new(),
        lsp: Vec::new(),
        formatters: Vec::new(),
        config: StatusConfig {
            plugins: Vec::new(),
        },
    }
}

fn snapshot_with(names: &[&str]) -> StatusSnapshot {
    let mut snapshot = snapshot();
    snapshot.mcp = names
        .iter()
        .map(|name| ((*name).to_owned(), McpStatus::Connected))
        .collect();
    snapshot
}
