use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use gpui::{AppContext, Context, Entity, Pixels, ScrollHandle, SharedString, Subscription, Task};
use opencode_gpui::{
    api::{Client, CreateSession},
    editor::TextEditor,
    model::{Session, sort_sessions},
};

use super::{
    MESSAGE_PAGE, PartSelection, ServerState, TimelineState, Workspace,
    directory_path::normalize_directory, inspector,
};

pub(crate) struct DirectoryTab {
    pub(super) directory: String,
    pub(super) client: Client,
    pub(super) timeline: TimelineState,
    pub(super) expanded_parts: HashSet<PartSelection>,
    pub(super) collapsed_parts: HashSet<PartSelection>,
    pub(super) selected_part: Option<PartSelection>,
    pub(super) detail_cache: HashMap<PartSelection, Arc<inspector::PreparedPart>>,
    pub(super) preparing_parts: HashSet<PartSelection>,
    pub(super) detail_tasks: Vec<Task<()>>,
    pub(super) markdown: super::markdown_cache::MarkdownCache,
    pub(super) markdown_renders: super::markdown_render_cache::MarkdownRenderCache,
    pub(super) images: super::image_cache::ImageCache,
    pub(super) sidebar: super::sidebar_state::SidebarState,
    pub(super) sidebar_load: Option<Task<()>>,
    pub(super) message_limit: usize,
    pub(super) history_loading: bool,
    pub(super) history_exhausted: bool,
    pub(super) timeline_load: Option<Task<()>>,
    pub(super) history_load: Option<Task<()>>,
    pub(super) timeline_scroll: ScrollHandle,
    pub(super) follow_tail: bool,
    pub(super) pending_detail_anchor: Option<Pixels>,
    pub(super) editor: Entity<TextEditor>,
    pub(super) prompt_error: Option<SharedString>,
    pub(super) prompt_mode: super::prompt_mode::PromptMode,
    pub(super) composer_completion: Option<super::composer_completion::ComposerCompletion>,
    pub(super) completion_task: Option<Task<()>>,
    pub(super) attached_files: HashSet<String>,
    pub(super) attached_images: Vec<super::image_attachment::PromptImage>,
    pub(super) _editor_subscriptions: Vec<Subscription>,
    pub(super) _events: Task<()>,
}

impl DirectoryTab {
    pub(crate) fn new(
        directory: String,
        client: Client,
        editor: Entity<TextEditor>,
        subscriptions: Vec<Subscription>,
        events: Task<()>,
    ) -> Self {
        Self {
            directory,
            client,
            timeline: TimelineState::Empty,
            expanded_parts: HashSet::new(),
            collapsed_parts: HashSet::new(),
            selected_part: None,
            detail_cache: HashMap::new(),
            preparing_parts: HashSet::new(),
            detail_tasks: Vec::new(),
            markdown: super::markdown_cache::MarkdownCache::default(),
            markdown_renders: super::markdown_render_cache::MarkdownRenderCache::default(),
            images: super::image_cache::ImageCache::default(),
            sidebar: super::sidebar_state::SidebarState::Empty,
            sidebar_load: None,
            message_limit: MESSAGE_PAGE,
            history_loading: false,
            history_exhausted: false,
            timeline_load: None,
            history_load: None,
            timeline_scroll: ScrollHandle::new(),
            follow_tail: true,
            pending_detail_anchor: None,
            editor,
            prompt_error: None,
            prompt_mode: super::prompt_mode::PromptMode::Normal,
            composer_completion: None,
            completion_task: None,
            attached_files: HashSet::new(),
            attached_images: Vec::new(),
            _editor_subscriptions: subscriptions,
            _events: events,
        }
    }
}

impl Workspace {
    pub(super) fn active_tab(&self) -> Option<&DirectoryTab> {
        self.tabs.get(self.active_tab)
    }

    pub(super) fn active_tab_mut(&mut self) -> Option<&mut DirectoryTab> {
        self.tabs.get_mut(self.active_tab)
    }

    pub(super) fn known_directories(&self) -> Vec<String> {
        let mut recency = self.directory_history.clone();
        for tab in &self.tabs {
            recency.entry(tab.directory.clone()).or_default();
        }
        if let ServerState::Ready { sessions, .. } = &self.server_state {
            for session in sessions.iter() {
                recency
                    .entry(session.directory.clone())
                    .and_modify(|updated| *updated = (*updated).max(session.time.updated))
                    .or_insert(session.time.updated);
            }
        }
        let mut directories = recency.into_iter().collect::<Vec<_>>();
        directories.sort_unstable_by_key(|(_, updated)| std::cmp::Reverse(*updated));
        directories
            .into_iter()
            .map(|(directory, _)| directory)
            .collect()
    }

    pub(super) fn ensure_initial_tab(&mut self, cx: &mut Context<Self>) {
        self.restore_initial_workspace(cx);
    }

    pub(super) fn open_directory(&mut self, directory: String, cx: &mut Context<Self>) {
        let should_hydrate = self.activate_directory(directory.clone(), cx);
        if should_hydrate {
            self.hydrate_directory(directory, false, cx);
        }
    }

    pub(super) fn directory_session_count(&self, directory: &str) -> usize {
        match &self.server_state {
            ServerState::Ready { sessions, .. } => sessions
                .iter()
                .filter(|session| session.parent_id.is_none() && session.directory == directory)
                .count(),
            ServerState::Loading | ServerState::Failed(_) => 0,
        }
    }

    pub(super) fn create_directory_session(&mut self, path: &str, cx: &mut Context<Self>) {
        let directory = normalize_directory(path);
        if directory.is_empty() {
            return;
        }
        self.directory_error = None;
        let validation = cx.background_spawn(async move {
            let path = std::fs::canonicalize(&directory)
                .map_err(|error| format!("cannot open {directory}: {error}"))?;
            if !path.is_dir() {
                return Err(format!("{} is not a directory", path.display()));
            }
            Ok(path.to_string_lossy().into_owned())
        });
        cx.spawn(async move |workspace, cx| {
            let result = validation.await;
            let _ = workspace.update(cx, |workspace, cx| match result {
                Ok(directory) => {
                    workspace.activate_directory(directory.clone(), cx);
                    workspace.hydrate_directory(directory, true, cx);
                }
                Err(error) => {
                    workspace.directory_error = Some(error.into());
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn hydrate_directory(
        &mut self,
        directory: String,
        create_if_empty: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(client) = self
            .tabs
            .iter()
            .find(|tab| tab.directory == directory)
            .map(|tab| tab.client.clone())
        else {
            return;
        };
        cx.spawn(async move |workspace, cx| {
            let result = async {
                let bootstrap = client.bootstrap().await?;
                let existing = bootstrap
                    .sessions
                    .iter()
                    .find(|session| session.parent_id.is_none())
                    .cloned();
                let selected = if existing.is_none() && create_if_empty {
                    Some(client.create_session(CreateSession::default()).await?)
                } else {
                    existing
                };
                Ok::<_, opencode_gpui::api::Error>((bootstrap.sessions, selected))
            }
            .await;
            let _ = workspace.update(cx, |workspace, cx| match result {
                Ok((mut sessions, selected)) => {
                    if let Some(session) = &selected
                        && !sessions.iter().any(|current| current.id == session.id)
                    {
                        sessions.push(session.clone());
                    }
                    workspace.merge_directory_sessions(&directory, sessions);
                    if workspace.active_directory() == Some(directory.as_str())
                        && let Some(session) = selected
                    {
                        let title = super::format::display_title(&session).into();
                        workspace.select_session(session.id, title, cx);
                    }
                    cx.notify();
                }
                Err(error) => {
                    if let Some(tab) = workspace
                        .tabs
                        .iter_mut()
                        .find(|tab| tab.directory == directory)
                    {
                        tab.prompt_error = Some(error.to_string().into());
                    }
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(super) fn merge_directory_sessions(&mut self, directory: &str, incoming: Vec<Session>) {
        let ServerState::Ready { sessions, .. } = &mut self.server_state else {
            return;
        };
        let sessions = Arc::make_mut(sessions);
        sessions.retain(|session| session.directory != directory);
        sessions.extend(incoming);
        sort_sessions(sessions);
    }
}
