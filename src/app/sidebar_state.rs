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
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        tab.sidebar = SidebarState::Loading;
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
                tab.sidebar = match result {
                    Ok(snapshot) => SidebarState::Ready(snapshot),
                    Err(error) => SidebarState::Failed(error),
                };
                cx.notify();
            });
        });
        tab.sidebar_load = Some(task);
        cx.notify();
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
            self.load_sidebar(directory, &session_id, cx);
        }
    }
}
