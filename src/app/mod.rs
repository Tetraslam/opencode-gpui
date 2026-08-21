mod activation;
mod bootstrap;
mod chrome;
mod command_palette;
mod composer;
mod composer_catalog;
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
mod local_server;
mod markdown_cache;
mod markdown_code_view;
mod markdown_inline_view;
mod markdown_render_cache;
mod markdown_tasks;
mod markdown_view;
mod message_footer;
mod navigation;
mod navigation_keys;
mod optimistic;
mod overlay_keys;
mod pane_resize;
mod part_format;
mod part_interaction;
mod part_merge;
mod prompt_mode;
mod reducer;
mod selection_overlay;
mod server_startup;
mod session_creation;
mod session_navigation;
mod session_pane;
mod session_selection;
mod settings;
mod shell_submit;
mod sidebar_state;
mod sidebar_view;
mod tab_bar;
mod tabs;
mod text_row;
mod timeline;
mod timeline_scroll;
mod timeline_state;
mod tool_row;
mod workspace_activity;
mod workspace_command;
mod workspace_layout;
mod workspace_restore;
mod workspace_switch;

#[cfg(test)]
mod tests;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use gpui::{
    App, AppContext, Application, Bounds, Entity, FocusHandle, Focusable, SharedString,
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
    server_process: Option<local_server::ManagedServer>,
    pub(super) statuses: Arc<HashMap<String, SessionStatus>>,
    pub(super) pending_parts: HashMap<String, Vec<opencode_gpui::model::Part>>,
    pub(super) pending_deltas: HashMap<String, Vec<PendingDelta>>,
    pub(super) tabs: Vec<tabs::DirectoryTab>,
    tab_bar: Entity<tab_bar::TabBar>,
    pub(super) _tab_bar_subscription: Subscription,
    pub(super) active_tab: usize,
    directory_switch: Option<Task<()>>,
    pub(super) initial_directory: Option<String>,
    pub(super) pending_workspace_layout: Option<workspace_layout::WorkspaceLayout>,
    layout_path: std::path::PathBuf,
    layout_save: Option<Task<()>>,
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
    pub(super) directory_suggestion_query: String,
    pub(super) command_suggestions: Arc<Vec<workspace_command::Command>>,
    pub(super) selection_suggestions: Arc<Vec<selection_overlay::SelectionItem>>,
    pub(super) selection_query: String,
    pub(super) selection_search: Option<Task<()>>,
    pub(super) directory_completion: Option<Task<()>>,
    pub(super) command_editor: Entity<TextEditor>,
    pub(super) _command_submit: Subscription,
    pub(super) _command_change: Subscription,
    pub(super) connected_directories: HashSet<String>,
    pub(super) _load: Task<()>,
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
