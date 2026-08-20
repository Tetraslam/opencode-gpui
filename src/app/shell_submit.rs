use gpui::Context;
use opencode_gpui::model::{Message, ModelRef};

use super::{TimelineState, Workspace};

impl Workspace {
    pub(super) fn submit_shell_in(
        &mut self,
        directory: &str,
        command: String,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        if command.trim().is_empty() {
            return;
        }
        if !tab.attached_images.is_empty() || !tab.attached_files.is_empty() {
            tab.prompt_error = Some("shell mode does not accept attachments".into());
            tab.editor
                .update(cx, |editor, cx| editor.restore_text(command, cx));
            cx.notify();
            return;
        }
        let Some(session_id) = tab.timeline.session_id().map(str::to_owned) else {
            tab.prompt_error = Some("no active session".into());
            tab.editor
                .update(cx, |editor, cx| editor.restore_text(command, cx));
            cx.notify();
            return;
        };
        let client = tab.client.clone();
        let (agent, model) = shell_identity(&tab.timeline);
        let directory = directory.to_owned();
        tab.prompt_error = None;
        cx.spawn(async move |workspace, cx| {
            let result = client.shell(&session_id, &command, &agent, model).await;
            let _ = workspace.update(cx, |workspace, cx| match result {
                Ok(message) => {
                    workspace.apply_message_record(message, Some(&directory));
                    workspace.clear_draft(&directory, &session_id, cx);
                    cx.notify();
                }
                Err(error) => {
                    if let Some(tab) = workspace
                        .tabs
                        .iter_mut()
                        .find(|tab| tab.directory == directory)
                    {
                        tab.prompt_error = Some(error.to_string().into());
                        tab.editor
                            .update(cx, |editor, cx| editor.restore_text(command, cx));
                    }
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

fn shell_identity(timeline: &TimelineState) -> (String, Option<ModelRef>) {
    let TimelineState::Ready { messages, .. } = timeline else {
        return ("build".into(), None);
    };
    messages
        .iter()
        .rev()
        .find_map(|message| match &message.info {
            Message::User(message) => Some((message.agent.clone(), Some(message.model.clone()))),
            Message::Assistant(_) => None,
        })
        .unwrap_or_else(|| ("build".into(), None))
}
