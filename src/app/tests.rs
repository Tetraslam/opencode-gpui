use std::{
    collections::{HashMap, HashSet},
    env,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use gpui::{AppContext, Entity, Task, TestAppContext};
use opencode_gpui::{
    api::Client,
    editor::{Changed, Submit, TextEditor},
    event::{Event, SessionStatus},
    model::{Message, Part, Session},
    theme::size as ui_size,
};

use super::{ServerState, TimelineState, Workspace, command_palette::Overlay, tabs::DirectoryTab};

static TEST_ID: AtomicU64 = AtomicU64::new(1);

#[path = "draft_tests.rs"]
mod draft_tests;
#[path = "overlay_tests.rs"]
mod overlay_tests;
#[path = "performance_tests.rs"]
mod performance_tests;
#[path = "selection_tests.rs"]
mod selection_tests;
#[path = "session_selection_tests.rs"]
mod session_selection_tests;
#[path = "shell_tests.rs"]
mod shell_tests;
#[path = "stream_tests.rs"]
mod stream_tests;
#[path = "test_helpers.rs"]
mod test_helpers;
use test_helpers::{session, session_in};

fn workspace(
    cx: &mut TestAppContext,
    sessions: Vec<Session>,
    timeline: TimelineState,
) -> Entity<Workspace> {
    cx.new(|cx: &mut gpui::Context<Workspace>| {
        let client = Client::new("http://127.0.0.1:1", None, None, None).unwrap();
        let directory_editor = cx.new(|cx| TextEditor::new("directory", cx).preserve_on_submit());
        let directory_subscription = cx.subscribe(
            &directory_editor,
            |workspace, editor, event: &Submit, cx| {
                workspace.submit_directory_picker(&event.text, cx);
                editor.update(cx, |editor, cx| editor.restore_text("", cx));
            },
        );
        let directory_change =
            cx.subscribe(&directory_editor, |workspace, editor, _: &Changed, cx| {
                workspace.refresh_directory_suggestions(editor.read(cx).text().to_owned(), cx);
            });
        let command_editor = cx.new(|cx| TextEditor::new("command", cx).preserve_on_submit());
        let command_submit = cx.subscribe(&command_editor, |workspace, _, event: &Submit, cx| {
            workspace.submit_active_overlay(&event.text, cx);
        });
        let command_change = cx.subscribe(&command_editor, |workspace, editor, _: &Changed, cx| {
            let query = editor.read(cx).text().to_owned();
            if matches!(workspace.overlay, Overlay::Selection(_)) {
                workspace.refresh_selection_suggestions(&query, cx);
            } else {
                workspace.refresh_command_suggestions(&query);
            }
            cx.notify();
        });
        let (tab_bar, tab_bar_subscription) = super::tab_bar::create(cx);
        let editor = cx.new(|cx| TextEditor::new("prompt", cx));
        let subscription = cx.subscribe(&editor, |workspace, _, event: &Submit, cx| {
            workspace.submit_prompt_in("/workspace", event.text.clone(), cx);
        });
        let mut tab = DirectoryTab::new(
            "/workspace".into(),
            client.scoped("/workspace".into()),
            editor,
            vec![subscription],
            Task::ready(()),
        );
        tab.timeline = timeline;
        Workspace {
            focus_handle: cx.focus_handle(),
            client: Some(client),
            server: "test".into(),
            server_state: ServerState::Ready {
                sessions: Arc::new(sessions),
            },
            statuses: Arc::new(HashMap::new()),
            pending_parts: HashMap::new(),
            pending_deltas: HashMap::new(),
            tabs: vec![tab],
            tab_bar,
            _tab_bar_subscription: tab_bar_subscription,
            active_tab: 0,
            directory_switch: None,
            initial_directory: None,
            pending_workspace_layout: None,
            layout_path: env::temp_dir().join(format!(
                "opencode-gpui-test-workspace-{}.json",
                TEST_ID.fetch_add(1, Ordering::Relaxed)
            )),
            layout_save: None,
            overlay: Overlay::None,
            overlay_selection: 0,
            picker_scroll: gpui::ScrollHandle::new(),
            composer_completion_scroll: gpui::ScrollHandle::new(),
            drafts: HashMap::new(),
            draft_save: None,
            draft_path: env::temp_dir().join(format!(
                "opencode-gpui-test-drafts-{}.json",
                std::process::id()
            )),
            directory_history: HashMap::new(),
            directory_history_save: None,
            settings: super::settings::Settings::default(),
            sessions_open: false,
            session_pane_width: gpui::px(ui_size::SESSION_PANE),
            inspector_width: gpui::px(ui_size::INSPECTOR),
            pane_resize: super::pane_resize::PaneResize::Idle,
            focus_editor_on_render: false,
            focus_overlay_on_render: false,
            directory_editor,
            directory_error: None,
            _directory_subscription: directory_subscription,
            _directory_change: directory_change,
            directory_suggestions: Arc::new(Vec::new()),
            directory_suggestion_query: String::new(),
            command_suggestions: Arc::new(Vec::new()),
            selection_suggestions: Arc::new(Vec::new()),
            selection_query: String::new(),
            selection_search: None,
            directory_completion: None,
            command_editor,
            _command_submit: command_submit,
            _command_change: command_change,
            connected_directories: HashSet::new(),
            _load: Task::ready(()),
        }
    })
}

#[gpui::test]
fn reduces_session_updates_without_losing_sort_order(cx: &mut TestAppContext) {
    let workspace = workspace(
        cx,
        vec![session("a", 1), session("b", 2)],
        TimelineState::Empty,
    );
    workspace.update(cx, |workspace, _| {
        workspace.apply_events(
            vec![
                Event::SessionUpdated(session("a", 3)),
                Event::SessionStatus {
                    session_id: "a".into(),
                    status: SessionStatus::Busy,
                },
            ],
            None,
        );
    });
    cx.read(|cx| {
        let workspace = workspace.read(cx);
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
    });
}

#[gpui::test]
fn reduces_streamed_message_parts_in_place(cx: &mut TestAppContext) {
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
    let workspace = workspace(
        cx,
        Vec::new(),
        TimelineState::Ready {
            session_id: "ses_1".into(),
            title: "session".into(),
            messages: Vec::new(),
        },
    );
    workspace.update(cx, |workspace, _| {
        workspace.apply_events(
            vec![
                Event::MessageUpdated(message),
                Event::MessagePartUpdated {
                    part: first,
                    delta: Some("hel".into()),
                },
                Event::MessagePartUpdated {
                    part: complete,
                    delta: Some("lo".into()),
                },
            ],
            None,
        );
    });
    cx.read(|cx| {
        let workspace = workspace.read(cx);
        let TimelineState::Ready { messages, .. } = &workspace.tabs[0].timeline else {
            panic!("timeline should remain loaded");
        };
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].parts.len(), 1);
        assert_eq!(messages[0].parts[0].text(), Some("hello"));
    });
}

#[gpui::test]
fn directory_tabs_scope_sessions_without_losing_global_discovery(cx: &mut TestAppContext) {
    let workspace = workspace(
        cx,
        vec![
            session_in("a-old", "/work/a", 1),
            session_in("b", "/work/b", 3),
            session_in("a-new", "/work/a", 2),
        ],
        TimelineState::Empty,
    );
    workspace.update(cx, |workspace, cx| {
        workspace.tabs[0].directory = "/work/a".into();
        assert_eq!(workspace.directory_session_count("/work/a"), 2);
        assert_eq!(workspace.directory_session_count("/work/b"), 1);
        assert_eq!(workspace.known_directories(), ["/work/b", "/work/a"]);
        workspace.open_directory("/work/b".into(), cx);
        assert_eq!(workspace.tabs.len(), 2);
        assert_eq!(workspace.active_directory(), Some("/work/b"));
        assert_eq!(workspace.tabs[1].timeline.session_id(), Some("b"));
    });
}

#[gpui::test]
fn streamed_deltas_append_when_events_do_not_include_full_text(cx: &mut TestAppContext) {
    let message: Message = serde_json::from_str(
        r#"{"id":"msg_1","sessionID":"ses_1","role":"user","time":{"created":1},"agent":"build","model":{"providerID":"openai","modelID":"test"}}"#,
    )
    .unwrap();
    let first: Part = serde_json::from_str(
        r#"{"id":"part_1","sessionID":"ses_1","messageID":"msg_1","type":"text","text":"hel"}"#,
    )
    .unwrap();
    let workspace = workspace(
        cx,
        Vec::new(),
        TimelineState::Ready {
            session_id: "ses_1".into(),
            title: "session".into(),
            messages: Vec::new(),
        },
    );
    workspace.update(cx, |workspace, _| {
        workspace.apply_events(
            vec![
                Event::MessageUpdated(message),
                Event::MessagePartUpdated {
                    part: first,
                    delta: Some("hel".into()),
                },
                Event::MessagePartDelta {
                    session_id: "ses_1".into(),
                    message_id: "msg_1".into(),
                    part_id: "part_1".into(),
                    field: "text".into(),
                    delta: "lo".into(),
                },
            ],
            None,
        );
        let TimelineState::Ready { messages, .. } = &workspace.tabs[0].timeline else {
            panic!("timeline should remain loaded");
        };
        assert_eq!(messages[0].parts[0].text(), Some("hello"));
    });
}
