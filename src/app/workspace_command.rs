use gpui::{AppContext, Context};
use opencode_gpui::{editor::TextEditor, event::SessionStatus};

use super::{TimelineState, Workspace, command_palette::Overlay};

#[derive(Clone, Copy)]
pub(crate) enum Command {
    OpenDirectory,
    NewSession,
    ToggleSessions,
    NextSession,
    PreviousSession,
    AbortSession,
    LoadOlder,
    CloseInspector,
    NextWorkspace,
    PreviousWorkspace,
    CloseWorkspace,
    FocusComposer,
    SelectAgent,
    SelectModel,
    SelectVariant,
    ToggleDiffExpansion,
}

impl Command {
    const ALL: [Self; 16] = [
        Self::OpenDirectory,
        Self::NewSession,
        Self::ToggleSessions,
        Self::NextSession,
        Self::PreviousSession,
        Self::AbortSession,
        Self::LoadOlder,
        Self::CloseInspector,
        Self::NextWorkspace,
        Self::PreviousWorkspace,
        Self::CloseWorkspace,
        Self::FocusComposer,
        Self::SelectAgent,
        Self::SelectModel,
        Self::SelectVariant,
        Self::ToggleDiffExpansion,
    ];

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::OpenDirectory => "open directory",
            Self::NewSession => "new session",
            Self::ToggleSessions => "toggle session history",
            Self::NextSession => "next session",
            Self::PreviousSession => "previous session",
            Self::AbortSession => "abort active response",
            Self::LoadOlder => "load older messages",
            Self::CloseInspector => "close inspector",
            Self::NextWorkspace => "next workspace",
            Self::PreviousWorkspace => "previous workspace",
            Self::CloseWorkspace => "close workspace",
            Self::FocusComposer => "focus composer",
            Self::SelectAgent => "select agent",
            Self::SelectModel => "select model",
            Self::SelectVariant => "select model variant",
            Self::ToggleDiffExpansion => "toggle automatic diff expansion",
        }
    }

    pub(super) const fn category(self) -> &'static str {
        match self {
            Self::OpenDirectory
            | Self::NextWorkspace
            | Self::PreviousWorkspace
            | Self::CloseWorkspace => "workspace",
            Self::FocusComposer | Self::SelectAgent | Self::SelectModel | Self::SelectVariant => {
                "prompt"
            }
            Self::CloseInspector | Self::ToggleDiffExpansion => "view",
            Self::NewSession
            | Self::ToggleSessions
            | Self::NextSession
            | Self::PreviousSession
            | Self::AbortSession
            | Self::LoadOlder => "session",
        }
    }

    pub(super) const fn hint(self) -> &'static str {
        match self {
            Self::OpenDirectory => "ctrl+t",
            Self::NewSession => "ctrl+n",
            Self::ToggleSessions => "ctrl+b",
            Self::NextSession => "alt+down",
            Self::PreviousSession => "alt+up",
            Self::NextWorkspace => "ctrl+tab",
            Self::PreviousWorkspace => "ctrl+shift+tab",
            Self::CloseWorkspace => "ctrl+w",
            Self::AbortSession
            | Self::LoadOlder
            | Self::CloseInspector
            | Self::FocusComposer
            | Self::SelectAgent
            | Self::SelectModel
            | Self::SelectVariant
            | Self::ToggleDiffExpansion => "",
        }
    }

    fn available(self, workspace: &Workspace) -> bool {
        let Some(tab) = workspace.active_tab() else {
            return matches!(self, Self::OpenDirectory);
        };
        match self {
            Self::NextWorkspace | Self::PreviousWorkspace => workspace.tabs.len() > 1,
            Self::NextSession | Self::PreviousSession => {
                workspace.directory_session_count(&tab.directory) > 1
            }
            Self::AbortSession => tab.timeline.session_id().is_some_and(|id| {
                workspace.statuses.get(id).is_some_and(|status| {
                    matches!(status, SessionStatus::Busy | SessionStatus::Retry { .. })
                })
            }),
            Self::LoadOlder => {
                matches!(tab.timeline, TimelineState::Ready { .. })
                    && !tab.history_exhausted
                    && !tab.history_loading
            }
            Self::CloseInspector => tab.selected_part.is_some(),
            Self::SelectVariant => match &tab.catalog {
                super::composer_catalog::CatalogState::Ready(catalog) => {
                    !catalog.variants(tab.selection.model.as_ref()).is_empty()
                }
                super::composer_catalog::CatalogState::Loading
                | super::composer_catalog::CatalogState::Failed(_) => false,
            },
            Self::OpenDirectory
            | Self::NewSession
            | Self::ToggleSessions
            | Self::CloseWorkspace
            | Self::FocusComposer
            | Self::SelectAgent
            | Self::SelectModel
            | Self::ToggleDiffExpansion => true,
        }
    }
}

impl Workspace {
    pub(super) fn refresh_command_suggestions(&mut self, query: &str) {
        self.overlay_selection = 0;
        self.command_suggestions = std::sync::Arc::new(self.filtered_commands(query));
    }

    pub(super) fn filtered_commands(&self, query: &str) -> Vec<Command> {
        let query = query.trim().to_lowercase();
        Command::ALL
            .into_iter()
            .filter(|command| command.available(self))
            .filter(|command| {
                query.is_empty()
                    || command.label().contains(&query)
                    || command.category().contains(&query)
            })
            .collect()
    }

    pub(super) fn execute_command(&mut self, command: Command, cx: &mut Context<Self>) {
        self.overlay = Overlay::None;
        self.command_editor.update(cx, TextEditor::clear);
        match command {
            Command::OpenDirectory => {
                self.overlay = Overlay::Directory;
                self.focus_overlay_on_render = true;
            }
            Command::NewSession => self.create_active_session(cx),
            Command::ToggleSessions => self.sessions_open = !self.sessions_open,
            Command::NextSession => self.move_session(1, cx),
            Command::PreviousSession => self.move_session(-1, cx),
            Command::AbortSession => {
                if let Some(id) = self
                    .active_tab()
                    .and_then(|tab| tab.timeline.session_id())
                    .map(str::to_owned)
                {
                    self.abort_session(id, cx);
                }
            }
            Command::LoadOlder => self.load_older_messages(cx),
            Command::CloseInspector => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.selected_part = None;
                }
            }
            Command::NextWorkspace => {
                self.switch_directory((self.active_tab + 1) % self.tabs.len(), cx);
            }
            Command::PreviousWorkspace => {
                let index = self
                    .active_tab
                    .checked_sub(1)
                    .unwrap_or(self.tabs.len() - 1);
                self.switch_directory(index, cx);
            }
            Command::CloseWorkspace => self.close_directory(self.active_tab, cx),
            Command::FocusComposer => self.focus_editor_on_render = true,
            Command::SelectAgent => {
                self.open_selection(super::selection_overlay::SelectionKind::Agent, cx);
            }
            Command::SelectModel => {
                self.open_selection(super::selection_overlay::SelectionKind::Model, cx);
            }
            Command::SelectVariant => {
                self.open_selection(super::selection_overlay::SelectionKind::Variant, cx);
            }
            Command::ToggleDiffExpansion => {
                self.settings.expand_diffs = !self.settings.expand_diffs;
                let enabled = self.settings.expand_diffs;
                let settings = self.settings.clone();
                let save = cx.background_spawn(async move { super::settings::save(&settings) });
                cx.spawn(async move |workspace, cx| {
                    if let Err(error) = save.await {
                        let _ = workspace.update(cx, |workspace, cx| {
                            if let Some(tab) = workspace.active_tab_mut() {
                                tab.prompt_error =
                                    Some(format!("could not save diff preference: {error}").into());
                            }
                            cx.notify();
                        });
                    }
                })
                .detach();
                if enabled {
                    let directories = self
                        .tabs
                        .iter()
                        .map(|tab| tab.directory.clone())
                        .collect::<Vec<_>>();
                    for directory in directories {
                        self.prepare_default_diffs(&directory, cx);
                    }
                }
            }
        }
        if self.overlay == Overlay::None {
            self.focus_editor_on_render = true;
        }
        cx.notify();
    }
}
