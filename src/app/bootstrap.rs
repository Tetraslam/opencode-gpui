use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
};

use gpui::{AppContext, Context};
use opencode_gpui::{
    api::{Bootstrap, Client},
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
            workspace.execute_command_palette(&event.text, cx);
        });
        let command_change = cx.subscribe(&command_editor, |workspace, editor, _: &Changed, cx| {
            workspace.refresh_command_suggestions(editor.read(cx).text());
            cx.notify();
        });
        let server =
            env::var("OPENCODE_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:4096".into());
        let client_result = Client::new(
            &server,
            None,
            env::var("OPENCODE_SERVER_USERNAME").ok(),
            env::var("OPENCODE_SERVER_PASSWORD").ok(),
        );
        let client = client_result.as_ref().ok().cloned();
        let load_client = client.clone();
        let setup_error = client_result.err().map(|error| error.to_string());
        let load = cx.spawn(async move |workspace, cx| {
            let result = match load_client {
                Some(client) => client.bootstrap().await.map_err(|error| error.to_string()),
                None => Err(setup_error.unwrap_or_else(|| "client setup failed".into())),
            };
            let _ = workspace.update(cx, |workspace, cx| {
                workspace.apply_bootstrap(result, cx);
                workspace.ensure_initial_tab(cx);
                cx.notify();
            });
        });
        Self {
            focus_handle: cx.focus_handle(),
            client,
            server: server.into(),
            server_state: ServerState::Loading,
            statuses: Arc::new(HashMap::new()),
            pending_parts: HashMap::new(),
            pending_deltas: HashMap::new(),
            tabs: Vec::new(),
            active_tab: 0,
            initial_directory: env::var("OPENCODE_DIRECTORY").ok(),
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
            directory_completion: None,
            command_editor,
            _command_submit: command_submit,
            _command_change: command_change,
            connected_directories: HashSet::new(),
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
