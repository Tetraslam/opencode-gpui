use gpui::{Context, SharedString};

use super::{MESSAGE_PAGE, ServerState, TimelineState, Workspace};

impl Workspace {
    pub(super) fn select_session(
        &mut self,
        session_id: String,
        title: SharedString,
        cx: &mut Context<Self>,
    ) {
        self.capture_active_draft(true, cx);
        self.dismiss_transients();
        let Some(directory) = self.active_directory().map(str::to_owned) else {
            return;
        };
        self.select_session_in(&directory, session_id, title, cx);
    }

    pub(super) fn select_session_in(
        &mut self,
        directory: &str,
        session_id: String,
        title: SharedString,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        if tab.timeline.session_id() == Some(session_id.as_str()) {
            return;
        }
        tab.selected_part = None;
        tab.expanded_parts.clear();
        tab.collapsed_parts.clear();
        tab.detail_cache.clear();
        tab.preparing_parts.clear();
        tab.detail_tasks.clear();
        tab.markdown = super::markdown_cache::MarkdownCache::default();
        tab.markdown_renders = super::markdown_render_cache::MarkdownRenderCache::default();
        tab.images = super::image_cache::ImageCache::default();
        tab.message_limit = MESSAGE_PAGE;
        tab.history_loading = false;
        tab.history_exhausted = false;
        tab.follow_tail = true;
        tab.timeline_scroll.scroll_to_bottom();
        tab.timeline = TimelineState::Loading {
            session_id: session_id.clone(),
            title: title.clone(),
        };
        let client = tab.client.clone();
        let task_directory = directory.to_owned();
        self.restore_draft(directory, &session_id, cx);
        self.load_sidebar(directory, &session_id, cx);

        let requested_id = session_id;
        let task = cx.spawn(async move |workspace, cx| {
            let result = client
                .messages(&requested_id, MESSAGE_PAGE)
                .await
                .map_err(|error| error.to_string());
            let _ = workspace.update(cx, |workspace, cx| {
                let Some(tab) = workspace
                    .tabs
                    .iter_mut()
                    .find(|tab| tab.directory == task_directory)
                else {
                    return;
                };
                if tab.timeline.session_id() != Some(requested_id.as_str()) {
                    return;
                }
                tab.timeline = match result {
                    Ok(messages) => {
                        tab.history_exhausted = messages.len() < MESSAGE_PAGE;
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
                if let super::composer_catalog::CatalogState::Ready(catalog) = &tab.catalog {
                    tab.selection.initialize(catalog, &tab.timeline);
                }
                workspace.refresh_markdown(&task_directory, cx);
                workspace.refresh_image_cache(&task_directory, cx);
                workspace.prepare_default_diffs(&task_directory, cx);
                cx.notify();
            });
        });
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) {
            tab.timeline_load = Some(task);
        }
        cx.notify();
    }

    pub(super) fn select_default_session(&mut self, cx: &mut Context<Self>) {
        let Some(directory) = self.active_directory().map(str::to_owned) else {
            return;
        };
        self.select_default_session_in(&directory, cx);
    }

    pub(super) fn select_default_session_in(&mut self, directory: &str, cx: &mut Context<Self>) {
        if self
            .tabs
            .iter()
            .find(|tab| tab.directory == directory)
            .is_some_and(|tab| !matches!(tab.timeline, TimelineState::Empty))
        {
            return;
        }
        let default = match &self.server_state {
            ServerState::Ready { sessions, .. } => sessions
                .iter()
                .find(|session| session.parent_id.is_none() && session.directory == directory)
                .map(|session| {
                    (
                        session.id.clone(),
                        super::format::display_title(session).into(),
                    )
                }),
            ServerState::Loading | ServerState::Failed(_) => None,
        };
        if let Some((session_id, title)) = default {
            self.select_session_in(directory, session_id, title, cx);
        }
    }
}
