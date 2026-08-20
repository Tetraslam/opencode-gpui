mod activation;
mod bootstrap;
mod chrome;
mod command_palette;
mod composer;
mod composer_completion;
mod composer_completion_view;
mod composer_images;
mod composer_slashes;
mod composer_submit;
mod default_diffs;
mod diff_view;
mod directory_completion;
mod directory_history;
mod directory_path;
mod directory_picker;
mod draft_persistence;
mod draft_store;
mod event_row;
mod events;
mod file_row;
mod format;
mod history;
mod image_attachment;
mod image_cache;
mod image_row;
mod inspector;
mod markdown_cache;
mod markdown_code_view;
mod markdown_inline_view;
mod markdown_render_cache;
mod markdown_tasks;
mod markdown_view;
mod navigation;
mod navigation_keys;
mod optimistic;
mod overlay_keys;
mod pane_resize;
mod part_format;
mod part_interaction;
mod part_merge;
mod reducer;
mod session_creation;
mod session_navigation;
mod session_pane;
mod settings;
mod sidebar_state;
mod sidebar_view;
mod tabs;
mod text_row;
mod timeline;
mod timeline_scroll;
mod timeline_state;
mod tool_row;
mod workspace_command;

#[cfg(test)]
mod tests;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use gpui::{
    App, AppContext, Application, Bounds, Context, Entity, FocusHandle, Focusable, SharedString,
    Subscription, Task, TitlebarOptions, WindowBounds, WindowOptions, px, size,
};
use opencode_gpui::{
    api::Client,
    editor::{self, TextEditor},
    event::SessionStatus,
    model::{MessageRecord, Session},
};

pub(super) const MESSAGE_PAGE: usize = 16;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct PartSelection {
    pub(super) message_id: String,
    pub(super) part_id: String,
}

pub(super) struct PendingDelta {
    pub(super) part_id: String,
    pub(super) field: String,
    pub(super) delta: String,
}

pub(super) enum ServerState {
    Loading,
    Ready { sessions: Arc<Vec<Session>> },
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

pub struct Workspace {
    pub(super) focus_handle: FocusHandle,
    pub(super) client: Option<Client>,
    pub(super) server: SharedString,
    pub(super) server_state: ServerState,
    pub(super) statuses: Arc<HashMap<String, SessionStatus>>,
    pub(super) pending_parts: HashMap<String, Vec<opencode_gpui::model::Part>>,
    pub(super) pending_deltas: HashMap<String, Vec<PendingDelta>>,
    pub(super) tabs: Vec<tabs::DirectoryTab>,
    pub(super) active_tab: usize,
    pub(super) initial_directory: Option<String>,
    pub(super) overlay: command_palette::Overlay,
    pub(super) overlay_selection: usize,
    pub(super) picker_scroll: gpui::ScrollHandle,
    pub(super) composer_completion_scroll: gpui::ScrollHandle,
    drafts: HashMap<draft_store::DraftKey, draft_store::SessionDraft>,
    draft_save: Option<Task<()>>,
    draft_path: std::path::PathBuf,
    directory_history: HashMap<String, u64>,
    directory_history_save: Option<Task<()>>,
    pub(super) settings: settings::Settings,
    pub(super) sessions_open: bool,
    pub(super) session_pane_width: gpui::Pixels,
    pub(super) inspector_width: gpui::Pixels,
    pub(super) pane_resize: pane_resize::PaneResize,
    pub(super) focus_editor_on_render: bool,
    pub(super) focus_overlay_on_render: bool,
    pub(super) directory_editor: Entity<TextEditor>,
    pub(super) directory_error: Option<SharedString>,
    pub(super) _directory_subscription: Subscription,
    pub(super) _directory_change: Subscription,
    pub(super) directory_suggestions: Arc<Vec<String>>,
    pub(super) directory_completion: Option<Task<()>>,
    pub(super) command_editor: Entity<TextEditor>,
    pub(super) _command_submit: Subscription,
    pub(super) _command_change: Subscription,
    pub(super) connected_directories: HashSet<String>,
    pub(super) _load: Task<()>,
}

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
        if self
            .active_tab()
            .is_some_and(|tab| tab.timeline.session_id() == Some(session_id.as_str()))
        {
            return;
        }
        let tab = self.active_tab_mut().expect("active directory has a tab");
        tab.selected_part = None;
        tab.expanded_parts.clear();
        tab.collapsed_parts.clear();
        tab.detail_cache.clear();
        tab.preparing_parts.clear();
        tab.detail_tasks.clear();
        tab.markdown = markdown_cache::MarkdownCache::default();
        tab.markdown_renders = markdown_render_cache::MarkdownRenderCache::default();
        tab.images = image_cache::ImageCache::default();
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
        let task_directory = directory.clone();
        self.restore_draft(&directory, &session_id, cx);
        self.load_sidebar(&directory, &session_id, cx);

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

    fn select_default_session(&mut self, cx: &mut Context<Self>) {
        let Some(directory) = self.active_directory().map(str::to_owned) else {
            return;
        };
        if self
            .active_tab()
            .is_some_and(|tab| !matches!(tab.timeline, TimelineState::Empty))
        {
            return;
        }
        let default = match &self.server_state {
            ServerState::Ready { sessions, .. } => sessions
                .iter()
                .find(|session| session.parent_id.is_none() && session.directory == directory)
                .map(|session| (session.id.clone(), format::display_title(session).into())),
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
        navigation_keys::init(cx);
        let bounds = Bounds::centered(None, size(px(1_440.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("opencode".into()),
                    ..Default::default()
                }),
                app_id: Some("ai.opencode.gpui".into()),
                ..Default::default()
            },
            |window, cx| {
                window.set_window_title("opencode");
                let workspace = cx.new(Workspace::new);
                workspace.read(cx).focus_handle(cx).focus(window);
                workspace
            },
        )
        .expect("failed to open the OpenCode window");
        cx.activate(true);
    });
}
