use super::Workspace;
use gpui::Context;

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
        if !tab.attached_images.is_empty() || !tab.attached_files.is_empty() {
            tab.prompt_error = Some("shell mode does not accept attachments".into());
            tab.editor
                .update(cx, |editor, cx| editor.restore_text(command, cx));
            cx.notify();
            return;
        }
        if command.trim().is_empty() {
            return;
        }
        let Some(session_id) = tab.timeline.session_id().map(str::to_owned) else {
            tab.prompt_error = Some("no active session".into());
            tab.editor
                .update(cx, |editor, cx| editor.restore_text(command, cx));
            cx.notify();
            return;
        };
        let (agent, model) = tab.selection.prompt_identity().map_or_else(
            || {
                super::composer_submit::previous_identity(&tab.timeline).map_or_else(
                    || ("build".into(), None),
                    |(agent, model)| (agent, Some(model)),
                )
            },
            |(agent, model, _)| (agent, Some(model)),
        );
        let client = tab.client.clone();
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
