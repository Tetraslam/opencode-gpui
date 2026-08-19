use gpui::Context;

use super::{MESSAGE_PAGE, TimelineState, Workspace};

impl Workspace {
    pub(super) fn load_older_messages(&mut self, cx: &mut Context<Self>) {
        if self.history_loading || self.history_exhausted {
            return;
        }
        let Some(client) = self.client.clone() else {
            return;
        };
        let TimelineState::Ready { session_id, .. } = &self.timeline else {
            return;
        };
        let session_id = session_id.clone();
        let next_limit = self.message_limit.saturating_add(MESSAGE_PAGE);
        self.history_loading = true;
        self.history_load = Some(cx.spawn(async move |workspace, cx| {
            let result = client.messages(&session_id, next_limit).await;
            let _ = workspace.update(cx, |workspace, cx| {
                workspace.history_loading = false;
                if workspace.timeline.session_id() != Some(session_id.as_str()) {
                    return;
                }
                if let Ok(messages) = result {
                    let previous_count = match &workspace.timeline {
                        TimelineState::Ready { messages, .. } => messages.len(),
                        _ => 0,
                    };
                    workspace.history_exhausted =
                        messages.len() < next_limit || messages.len() == previous_count;
                    workspace.message_limit = next_limit;
                    if let TimelineState::Ready {
                        messages: current, ..
                    } = &mut workspace.timeline
                    {
                        *current = messages;
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
}
