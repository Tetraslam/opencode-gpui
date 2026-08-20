use gpui::Context;

use super::{MESSAGE_PAGE, TimelineState, Workspace};

impl Workspace {
    pub(super) fn load_older_messages(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab_mut() else {
            return;
        };
        if tab.history_loading || tab.history_exhausted {
            return;
        }
        let TimelineState::Ready { session_id, .. } = &tab.timeline else {
            return;
        };
        let client = tab.client.clone();
        let directory = tab.directory.clone();
        let task_directory = directory.clone();
        let session_id = session_id.clone();
        let next_limit = tab.message_limit.saturating_add(MESSAGE_PAGE);
        tab.history_loading = true;
        tab.follow_tail = false;
        let task = cx.spawn(async move |workspace, cx| {
            let result = client.messages(&session_id, next_limit).await;
            let _ = workspace.update(cx, |workspace, cx| {
                let Some(tab) = workspace
                    .tabs
                    .iter_mut()
                    .find(|tab| tab.directory == task_directory)
                else {
                    return;
                };
                tab.history_loading = false;
                if tab.timeline.session_id() != Some(session_id.as_str()) {
                    return;
                }
                if let Ok(messages) = result {
                    let previous_count = match &tab.timeline {
                        TimelineState::Ready { messages, .. } => messages.len(),
                        _ => 0,
                    };
                    tab.history_exhausted =
                        messages.len() < next_limit || messages.len() == previous_count;
                    tab.message_limit = next_limit;
                    if let TimelineState::Ready {
                        messages: current, ..
                    } = &mut tab.timeline
                    {
                        *current = messages;
                    }
                }
                workspace.refresh_markdown(&task_directory, cx);
                workspace.refresh_image_cache(&task_directory, cx);
                workspace.prepare_default_diffs(&task_directory, cx);
                cx.notify();
            });
        });
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) {
            tab.history_load = Some(task);
        }
        cx.notify();
    }
}
