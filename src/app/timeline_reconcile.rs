use gpui::Context;

use super::{TimelineState, Workspace};

impl Workspace {
    pub(super) fn reconcile_idle_timeline(
        &mut self,
        directory: &str,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Some((client, limit)) = self
            .tabs
            .iter()
            .find(|tab| tab.directory == directory)
            .filter(|tab| {
                tab.timeline.session_id() == Some(session_id)
                    || self.timeline_cache.contains(session_id)
            })
            .map(|tab| {
                let limit = if tab.timeline.session_id() == Some(session_id) {
                    tab.message_limit
                } else {
                    super::MESSAGE_PAGE
                };
                (tab.client.clone(), limit)
            })
        else {
            return;
        };
        let directory = directory.to_owned();
        let session_id = session_id.to_owned();
        cx.spawn(async move |workspace, cx| {
            let Ok(messages) = client.messages(&session_id, limit).await else {
                return;
            };
            let _ = workspace.update(cx, |workspace, cx| {
                let selected = workspace.tabs.iter().any(|tab| {
                    tab.directory == directory
                        && tab.timeline.session_id() == Some(session_id.as_str())
                });
                if !workspace.tabs.iter().any(|tab| tab.directory == directory)
                    || (!selected && !workspace.timeline_cache.contains(&session_id))
                {
                    return;
                }
                workspace.timeline_cache.replace(&session_id, &messages);
                let Some(tab) = workspace.tabs.iter_mut().find(|tab| {
                    tab.directory == directory
                        && tab.timeline.session_id() == Some(session_id.as_str())
                }) else {
                    return;
                };
                let TimelineState::Ready {
                    messages: current, ..
                } = &mut tab.timeline
                else {
                    return;
                };
                if *current == messages {
                    return;
                }
                *current = messages;
                workspace.refresh_markdown(&directory, cx);
                workspace.refresh_image_cache(&directory, cx);
                workspace.prepare_default_diffs(&directory, cx);
                cx.notify();
            });
        })
        .detach();
    }
}
