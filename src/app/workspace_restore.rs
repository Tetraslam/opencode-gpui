use std::collections::HashSet;

use gpui::{AppContext, Context};

use super::{ServerState, Workspace, command_palette::Overlay};

impl Workspace {
    pub(super) fn restore_initial_workspace(&mut self, cx: &mut Context<Self>) {
        if !self.tabs.is_empty() {
            return;
        }
        if let Some(directory) = self.initial_directory.take() {
            self.open_directory(directory, cx);
            return;
        }
        let Some(layout) = self.pending_workspace_layout.take() else {
            self.open_bootstrap_directory(cx);
            return;
        };
        if layout.directories.is_empty() {
            self.overlay = Overlay::Directory;
            self.focus_overlay_on_render = true;
            cx.notify();
            return;
        }
        let fallback = self.bootstrap_directory();
        let validation = cx.background_spawn(async move {
            let mut seen = HashSet::new();
            let mut active = None;
            let directories = layout
                .directories
                .into_iter()
                .filter_map(|directory| {
                    let canonical = std::fs::canonicalize(&directory).ok()?;
                    canonical.is_dir().then_some((directory, canonical))
                })
                .filter_map(|(original, canonical)| {
                    let canonical = canonical.to_string_lossy().into_owned();
                    if layout.active_directory.as_deref() == Some(original.as_str()) {
                        active = Some(canonical.clone());
                    }
                    seen.insert(canonical.clone()).then_some(canonical)
                })
                .collect::<Vec<_>>();
            (directories, active)
        });
        cx.spawn(async move |workspace, cx| {
            let (directories, active) = validation.await;
            let _ = workspace.update(cx, |workspace, cx| {
                if directories.is_empty() {
                    if let Some(directory) = fallback {
                        workspace.open_directory(directory, cx);
                    }
                    return;
                }
                for directory in directories {
                    workspace.open_directory(directory, cx);
                }
                if let Some(index) = active.and_then(|active| {
                    workspace
                        .tabs
                        .iter()
                        .position(|tab| tab.directory == active)
                }) {
                    workspace.switch_directory_immediately(index, cx);
                }
            });
        })
        .detach();
    }

    fn open_bootstrap_directory(&mut self, cx: &mut Context<Self>) {
        if let Some(directory) = self.bootstrap_directory() {
            self.open_directory(directory, cx);
        }
    }

    fn bootstrap_directory(&self) -> Option<String> {
        let ServerState::Ready { sessions, .. } = &self.server_state else {
            return None;
        };
        sessions
            .iter()
            .find(|session| session.parent_id.is_none())
            .map(|session| session.directory.clone())
    }
}
