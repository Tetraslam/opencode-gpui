use gpui::Context;
use opencode_gpui::{
    api::{Prompt, PromptFile},
    model::{Message, ModelRef},
};

use super::{TimelineState, Workspace};

impl Workspace {
    pub(super) fn submit_composer_in(
        &mut self,
        directory: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if self
            .tabs
            .iter()
            .find(|tab| tab.directory == directory)
            .is_some_and(|tab| tab.prompt_mode == super::prompt_mode::PromptMode::Shell)
        {
            self.submit_shell_in(directory, text, cx);
            return;
        }
        let trimmed = text.trim();
        if let Some(command) = trimmed.strip_prefix('/') {
            let (name, arguments) = command
                .split_once(char::is_whitespace)
                .map_or((command, ""), |(name, arguments)| (name, arguments.trim()));
            if !name.is_empty() {
                if let Some(action) = super::composer_slashes::local_slash(name) {
                    self.execute_command(action, cx);
                    return;
                }
                self.submit_command_in(directory, name, arguments, cx);
                return;
            }
        }
        self.submit_prompt_in(directory, text, cx);
    }

    pub(super) fn submit_prompt_in(
        &mut self,
        directory: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        if text.trim().is_empty() && tab.attached_images.is_empty() {
            return;
        }
        if tab
            .attached_images
            .iter()
            .any(|image| image.data_url.is_none())
        {
            tab.prompt_error = Some("images are still processing".into());
            tab.editor
                .update(cx, |editor, cx| editor.restore_text(text, cx));
            cx.notify();
            return;
        }
        let Some(session_id) = tab.timeline.session_id().map(str::to_owned) else {
            tab.prompt_error = Some("no active session".into());
            tab.editor
                .update(cx, |editor, cx| editor.restore_text(text, cx));
            cx.notify();
            return;
        };
        let selected = tab.selection.prompt_identity();
        let (optimistic_agent, optimistic_model) = selected.as_ref().map_or_else(
            || fallback_identity(&tab.timeline),
            |(agent, model, _)| (agent.clone(), model.clone()),
        );
        let (agent, model, variant) = selected
            .map_or((None, None, None), |(agent, model, variant)| {
                (Some(agent), Some(model), variant)
            });
        let client = tab.client.clone();
        let created = now_millis();
        let message_id = format!("msg_gpui_{created:x}");
        let text_part_id = format!("prt_gpui_{created:x}");
        let files = prompt_files(tab, directory, &text);
        let optimistic = super::optimistic::push_optimistic_message(
            tab,
            &session_id,
            &message_id,
            &text_part_id,
            &text,
            &files,
            optimistic_agent,
            optimistic_model,
            created,
        );
        tab.prompt_error = None;
        if let Some(message) = optimistic {
            self.mark_part_entrances(&message.parts, cx);
            self.timeline_cache.push_optimistic(&session_id, message);
        }
        let directory = directory.to_owned();
        cx.notify();
        cx.spawn(async move |workspace, cx| {
            let result = client
                .prompt(
                    &session_id,
                    Prompt {
                        message_id: message_id.clone(),
                        text_part_id,
                        text,
                        model,
                        agent,
                        variant,
                        files,
                    },
                )
                .await;
            let _ = workspace.update(cx, |workspace, cx| match result {
                Ok(()) => workspace.clear_draft(&directory, &session_id, cx),
                Err(error) => {
                    if let Some(tab) = workspace
                        .tabs
                        .iter_mut()
                        .find(|tab| tab.directory == directory)
                    {
                        tab.prompt_error = Some(error.to_string().into());
                        if let TimelineState::Ready { messages, .. } = &mut tab.timeline {
                            messages.retain(|message| message.info.id() != message_id);
                        }
                    }
                    workspace
                        .timeline_cache
                        .remove_message(&session_id, &message_id);
                    workspace.restore_draft(&directory, &session_id, cx);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn submit_command_in(
        &mut self,
        directory: &str,
        name: &str,
        arguments: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        let Some(session_id) = tab.timeline.session_id().map(str::to_owned) else {
            tab.prompt_error = Some("no active session".into());
            let command = if arguments.is_empty() {
                format!("/{name}")
            } else {
                format!("/{name} {arguments}")
            };
            tab.editor
                .update(cx, |editor, cx| editor.restore_text(command, cx));
            cx.notify();
            return;
        };
        let client = tab.client.clone();
        let command_text = if arguments.is_empty() {
            format!("/{name}")
        } else {
            format!("/{name} {arguments}")
        };
        if tab
            .attached_images
            .iter()
            .any(|image| image.data_url.is_none())
        {
            tab.prompt_error = Some("images are still processing".into());
            tab.editor
                .update(cx, |editor, cx| editor.restore_text(command_text, cx));
            cx.notify();
            return;
        }
        let files = prompt_files(tab, directory, &command_text);
        let directory = directory.to_owned();
        let name = name.to_owned();
        let arguments = arguments.to_owned();
        tab.prompt_error = None;
        cx.spawn(async move |workspace, cx| {
            let result = client.command(&session_id, &name, &arguments, files).await;
            let _ = workspace.update(cx, |workspace, cx| match result {
                Ok(()) => workspace.clear_draft(&directory, &session_id, cx),
                Err(error) => {
                    if let Some(tab) = workspace
                        .tabs
                        .iter_mut()
                        .find(|tab| tab.directory == directory)
                    {
                        tab.prompt_error = Some(error.to_string().into());
                    }
                    workspace.restore_draft(&directory, &session_id, cx);
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

pub(super) fn fallback_identity(timeline: &TimelineState) -> (String, ModelRef) {
    previous_identity(timeline).unwrap_or_else(|| {
        (
            "build".into(),
            ModelRef {
                provider_id: "server".into(),
                model_id: "default".into(),
            },
        )
    })
}

pub(super) fn previous_identity(timeline: &TimelineState) -> Option<(String, ModelRef)> {
    match timeline {
        TimelineState::Ready { messages, .. } => {
            messages
                .iter()
                .rev()
                .find_map(|message| match &message.info {
                    Message::User(message) => Some((message.agent.clone(), message.model.clone())),
                    Message::Assistant(_) => None,
                })
        }
        TimelineState::Empty | TimelineState::Loading { .. } | TimelineState::Failed { .. } => None,
    }
}

fn prompt_files(
    tab: &mut super::tabs::DirectoryTab,
    directory: &str,
    text: &str,
) -> Vec<PromptFile> {
    let files = tab
        .attached_files
        .iter()
        .filter(|path| text.contains(&format!("@{path}")))
        .filter_map(|path| {
            let absolute = std::path::Path::new(directory).join(path);
            Some(PromptFile {
                mime: "text/plain".into(),
                filename: path.clone(),
                url: url::Url::from_file_path(absolute).ok()?.to_string(),
            })
        })
        .chain(
            tab.attached_images
                .iter()
                .filter_map(super::image_attachment::PromptImage::as_prompt_file),
        )
        .collect();
    tab.attached_files.clear();
    tab.attached_images.clear();
    files
}

fn now_millis() -> u64 {
    u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}
