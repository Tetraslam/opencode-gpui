use std::sync::Arc;

use gpui::Context;
use opencode_gpui::api::SidebarSnapshot;

use super::Workspace;

pub(super) enum SidebarState {
    Empty,
    Loading,
    Ready(Arc<SidebarSnapshot>),
    Failed(String),
}

impl Workspace {
    pub(super) fn load_sidebar(
        &mut self,
        directory: &str,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        self.request_sidebar(directory, session_id, false, cx);
    }

    fn request_sidebar(
        &mut self,
        directory: &str,
        session_id: &str,
        preserve_ready: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        tab.sidebar_generation = tab.sidebar_generation.wrapping_add(1);
        let generation = tab.sidebar_generation;
        let mut visible_changed = false;
        if !preserve_ready || !matches!(tab.sidebar, SidebarState::Ready(_)) {
            tab.sidebar = SidebarState::Loading;
            tab.sidebar_error = None;
            visible_changed = true;
        }
        let client = tab.client.clone();
        let task_directory = directory.to_owned();
        let requested_id = session_id.to_owned();
        let task = cx.spawn(async move |workspace, cx| {
            let result = client
                .sidebar_snapshot(&requested_id)
                .await
                .map(Arc::new)
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
                if tab.sidebar_generation != generation {
                    return;
                }
                match result {
                    Ok(snapshot) => {
                        let changed = !matches!(
                            &tab.sidebar,
                            SidebarState::Ready(current) if current.as_ref() == snapshot.as_ref()
                        ) || tab.sidebar_error.is_some();
                        tab.sidebar = SidebarState::Ready(snapshot);
                        tab.sidebar_error = None;
                        if changed {
                            cx.notify();
                        }
                    }
                    Err(error) if !preserve_ready => {
                        tab.sidebar = SidebarState::Failed(error);
                        cx.notify();
                    }
                    Err(error) => {
                        tab.sidebar_error = Some(format!("context refresh failed: {error}").into());
                        cx.notify();
                    }
                }
            });
        });
        tab.sidebar_load = Some(task);
        if visible_changed {
            cx.notify();
        }
    }

    pub(super) fn refresh_sidebar_for_directory(
        &mut self,
        directory: &str,
        cx: &mut Context<Self>,
    ) {
        let session_id = self
            .tabs
            .iter()
            .find(|tab| tab.directory == directory)
            .and_then(|tab| tab.timeline.session_id())
            .map(str::to_owned);
        if let Some(session_id) = session_id {
            self.request_sidebar(directory, &session_id, true, cx);
        }
    }
}
