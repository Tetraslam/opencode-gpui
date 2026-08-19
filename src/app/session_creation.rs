use gpui::Context;
use opencode_gpui::{api::CreateSession, event::Event};

use super::Workspace;

impl Workspace {
    pub(super) fn create_active_session(&mut self, cx: &mut Context<Self>) {
        self.capture_active_draft(true, cx);
        self.dismiss_transients();
        let Some((directory, client)) = self
            .active_tab()
            .map(|tab| (tab.directory.clone(), tab.client.clone()))
        else {
            return;
        };
        cx.spawn(async move |workspace, cx| {
            let result = client.create_session(CreateSession::default()).await;
            let _ = workspace.update(cx, |workspace, cx| match result {
                Ok(session) => {
                    workspace.apply_events(vec![Event::SessionCreated(session.clone())], None);
                    if workspace.active_directory() == Some(directory.as_str()) {
                        let title = super::format::display_title(&session).into();
                        workspace.select_session(session.id, title, cx);
                        workspace.focus_editor_on_render = true;
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
}
