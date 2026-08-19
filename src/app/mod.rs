mod chrome;
mod composer;
mod events;
mod format;
mod history;
mod inspector;
mod part_format;
mod session_pane;
mod timeline;

#[cfg(test)]
mod tests;

use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
};

use gpui::{
    App, AppContext, Application, Bounds, Context, Entity, Focusable, SharedString, Subscription,
    Task, TitlebarOptions, WindowBounds, WindowOptions, px, size,
};
use opencode_gpui::{
    api::{Bootstrap, Client},
    editor::{self, Submit, TextEditor},
    event::SessionStatus,
    model::{MessageRecord, Session},
};

pub(super) const MESSAGE_PAGE: usize = 16;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct PartSelection {
    pub(super) message_id: String,
    pub(super) part_id: String,
}

pub(super) enum ServerState {
    Loading,
    Ready {
        version: SharedString,
        sessions: Arc<Vec<Session>>,
    },
    Failed(SharedString),
}

pub(super) enum TimelineState {
    Empty,
    Loading {
        session_id: String,
        title: SharedString,
    },
    Ready {
        session_id: String,
        title: SharedString,
        messages: Vec<MessageRecord>,
    },
    Failed {
        session_id: String,
        title: SharedString,
        error: SharedString,
    },
}

impl TimelineState {
    pub(super) fn session_id(&self) -> Option<&str> {
        match self {
            Self::Empty => None,
            Self::Loading { session_id, .. }
            | Self::Ready { session_id, .. }
            | Self::Failed { session_id, .. } => Some(session_id),
        }
    }

    pub(super) fn title(&self) -> Option<SharedString> {
        match self {
            Self::Empty => None,
            Self::Loading { title, .. }
            | Self::Ready { title, .. }
            | Self::Failed { title, .. } => Some(title.clone()),
        }
    }
}

pub struct Workspace {
    pub(super) client: Option<Client>,
    pub(super) server: SharedString,
    pub(super) server_state: ServerState,
    pub(super) timeline: TimelineState,
    pub(super) statuses: Arc<HashMap<String, SessionStatus>>,
    pub(super) expanded_parts: HashSet<PartSelection>,
    pub(super) selected_part: Option<PartSelection>,
    pub(super) detail_cache: HashMap<PartSelection, Arc<inspector::PreparedPart>>,
    pub(super) preparing_parts: HashSet<PartSelection>,
    pub(super) detail_tasks: Vec<Task<()>>,
    pub(super) message_limit: usize,
    pub(super) history_loading: bool,
    pub(super) history_exhausted: bool,
    pub(super) live: bool,
    pub(super) _load: Task<()>,
    pub(super) _events: Task<()>,
    pub(super) timeline_load: Option<Task<()>>,
    pub(super) history_load: Option<Task<()>>,
    pub(super) editor: Entity<TextEditor>,
    pub(super) prompt_error: Option<SharedString>,
    pub(super) _editor_subscription: Subscription,
}

impl Workspace {
    fn new(cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| TextEditor::new("Ask anything, @ files, / commands", cx));
        let editor_subscription = cx.subscribe(&editor, |workspace, _, event: &Submit, cx| {
            workspace.submit_prompt(event.text.clone(), cx);
        });
        let server =
            env::var("OPENCODE_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:4096".into());
        let client_result = Client::new(
            &server,
            env::var("OPENCODE_DIRECTORY").ok(),
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
                workspace.apply_bootstrap(result);
                workspace.select_default_session(cx);
                cx.notify();
            });
        });
        let events = Self::spawn_event_loop(client.clone(), cx);

        Self {
            client,
            server: server.into(),
            server_state: ServerState::Loading,
            timeline: TimelineState::Empty,
            statuses: Arc::new(HashMap::new()),
            expanded_parts: HashSet::new(),
            selected_part: None,
            detail_cache: HashMap::new(),
            preparing_parts: HashSet::new(),
            detail_tasks: Vec::new(),
            message_limit: MESSAGE_PAGE,
            history_loading: false,
            history_exhausted: false,
            live: false,
            _load: load,
            _events: events,
            timeline_load: None,
            history_load: None,
            editor,
            prompt_error: None,
            _editor_subscription: editor_subscription,
        }
    }

    pub(super) fn apply_bootstrap(&mut self, result: Result<Bootstrap, String>) {
        self.server_state = match result {
            Ok(bootstrap) => ServerState::Ready {
                version: bootstrap.health.version.into(),
                sessions: Arc::new(bootstrap.sessions),
            },
            Err(error) => ServerState::Failed(error.into()),
        };
    }

    pub(super) fn select_session(
        &mut self,
        session_id: String,
        title: SharedString,
        cx: &mut Context<Self>,
    ) {
        if self.timeline.session_id() == Some(session_id.as_str()) {
            return;
        }
        self.selected_part = None;
        self.expanded_parts.clear();
        self.message_limit = MESSAGE_PAGE;
        self.history_loading = false;
        self.history_exhausted = false;
        self.timeline = TimelineState::Loading {
            session_id: session_id.clone(),
            title: title.clone(),
        };
        let Some(client) = self.client.clone() else {
            self.timeline = TimelineState::Failed {
                session_id,
                title,
                error: "OpenCode client is unavailable".into(),
            };
            cx.notify();
            return;
        };

        let requested_id = session_id.clone();
        self.timeline_load = Some(cx.spawn(async move |workspace, cx| {
            let result = client
                .messages(&requested_id, MESSAGE_PAGE)
                .await
                .map_err(|error| error.to_string());
            let _ = workspace.update(cx, |workspace, cx| {
                if workspace.timeline.session_id() != Some(requested_id.as_str()) {
                    return;
                }
                workspace.timeline = match result {
                    Ok(messages) => {
                        workspace.history_exhausted = messages.len() < MESSAGE_PAGE;
                        TimelineState::Ready {
                            session_id: requested_id,
                            title,
                            messages,
                        }
                    }
                    Err(error) => TimelineState::Failed {
                        session_id: requested_id,
                        title,
                        error: error.into(),
                    },
                };
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn select_default_session(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.timeline, TimelineState::Empty) {
            return;
        }
        let default = match &self.server_state {
            ServerState::Ready { sessions, .. } => sessions
                .iter()
                .find(|session| session.parent_id.is_none())
                .map(|session| (session.id.clone(), session.title.clone().into())),
            ServerState::Loading | ServerState::Failed(_) => None,
        };
        if let Some((session_id, title)) = default {
            self.select_session(session_id, title, cx);
        }
    }
}

pub fn run() {
    Application::new().run(|cx: &mut App| {
        editor::init(cx);
        let bounds = Bounds::centered(None, size(px(1_440.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("OpenCode".into()),
                    ..Default::default()
                }),
                app_id: Some("ai.opencode.gpui".into()),
                ..Default::default()
            },
            |window, cx| {
                window.set_window_title("OpenCode");
                let workspace = cx.new(Workspace::new);
                let editor = workspace.read(cx).editor.clone();
                editor.read(cx).focus_handle(cx).focus(window);
                workspace
            },
        )
        .expect("failed to open the OpenCode window");
        cx.activate(true);
    });
}
