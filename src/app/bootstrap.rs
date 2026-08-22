use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use gpui::{AppContext, Context};
use opencode_gpui::{
    api::Bootstrap,
    editor::{Changed, Submit, TextEditor},
};

use super::{ServerState, Workspace, command_palette::Overlay};

impl Workspace {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        let directory_editor =
            cx.new(|cx| TextEditor::new("/path/to/project", cx).preserve_on_submit());
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
        let command_editor =
            cx.new(|cx| TextEditor::new("type a command", cx).preserve_on_submit());
        let command_submit = cx.subscribe(&command_editor, |workspace, _, event: &Submit, cx| {
            workspace.submit_active_overlay(&event.text, cx);
        });
        let command_change = cx.subscribe(&command_editor, |workspace, editor, _: &Changed, cx| {
            let query = editor.read(cx).text().to_owned();
            workspace.refresh_active_overlay(&query, cx);
            cx.notify();
        });
        let (tab_bar, tab_bar_subscription) = super::tab_bar::create(cx);
        let (server, client, load) = prepare_connection(cx);
        Self {
            focus_handle: cx.focus_handle(),
            client,
            server: server.into(),
            server_state: ServerState::Loading,
            server_process: None,
            statuses: Arc::new(HashMap::new()),
            interrupt_session: None,
            interrupt_reset: None,
            interrupt_generation: 0,
            pending_parts: HashMap::new(),
            pending_deltas: HashMap::new(),
            timeline_cache: super::timeline_cache::TimelineCache::default(),
            trace_entrances: HashSet::new(),
            tabs: Vec::new(),
            tab_bar,
            _tab_bar_subscription: tab_bar_subscription,
            active_tab: 0,
            directory_switch: None,
            initial_directory: std::env::var("OPENCODE_DIRECTORY").ok(),
            pending_workspace_layout: super::workspace_layout::load(),
            layout_path: super::workspace_layout::path(),
            layout_save: None,
            overlay: Overlay::None,
            overlay_selection: 0,
            picker_scroll: gpui::ScrollHandle::new(),
            composer_completion_scroll: gpui::ScrollHandle::new(),
            drafts: super::draft_persistence::load_drafts(),
            draft_save: None,
            draft_path: super::draft_persistence::draft_path(),
            directory_history: super::directory_history::load_directory_history(),
            directory_history_save: None,
            settings: super::settings::load(),
            sessions_open: false,
            session_pane_width: super::pane_resize::load_session_pane_width(),
            inspector_width: super::pane_resize::load_inspector_width(),
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
            timeline_history: Arc::new(Vec::new()),
            timeline_history_session: None,
            timeline_suggestions: Arc::new(Vec::new()),
            timeline_query: String::new(),
            timeline_message: None,
            selection_suggestions: Arc::new(Vec::new()),
            selection_query: String::new(),
            selection_search: None,
            directory_completion: None,
            command_editor,
            _command_submit: command_submit,
            _command_change: command_change,
            connected_directories: HashSet::new(),
            status_dialog: super::status_dialog::StatusDialogState::default(),
            debug_dialog: super::debug_dialog::DebugDialogState::default(),
            _load: load,
        }
    }

    pub(super) fn apply_bootstrap(
        &mut self,
        result: Result<Bootstrap, String>,
        cx: &mut Context<Self>,
    ) {
        if let Ok(bootstrap) = &result {
            self.merge_server_directory_history(&bootstrap.sessions, cx);
        }
        self.server_state = match result {
            Ok(bootstrap) => ServerState::Ready {
                sessions: Arc::new(bootstrap.sessions),
            },
            Err(error) => ServerState::Failed(error.into()),
        };
    }
}

fn prepare_connection(
    cx: &mut Context<Workspace>,
) -> (String, Option<opencode_gpui::api::Client>, gpui::Task<()>) {
    let connection = super::server_startup::prepare(cx);
    let server = connection.url;
    let client = connection.client.as_ref().ok().cloned();
    let load_client = client.clone();
    let setup_error = connection.client.err().map(|error| error.to_string());
    let load = cx.spawn(async move |workspace, cx| {
        let result =
            super::server_startup::connect(load_client, setup_error, connection.server_start).await;
        let _ = workspace.update(cx, |workspace, cx| {
            workspace.server_process = result.server;
            workspace.apply_bootstrap(result.bootstrap, cx);
            workspace.ensure_initial_tab(cx);
            cx.notify();
        });
    });
    (server, client, load)
}
