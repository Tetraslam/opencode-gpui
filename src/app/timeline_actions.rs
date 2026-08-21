use std::{collections::HashSet, sync::Arc};

use gpui::{AppContext, ClipboardItem, Context};
use opencode_gpui::{
    editor::TextEditor,
    event::Event,
    model::{MessageRecord, Part},
};

use super::{TimelineState, Workspace, command_palette::Overlay, image_attachment::PromptImage};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MessageAction {
    Revert,
    Copy,
    Fork,
}

pub(super) const ACTIONS: [MessageAction; 3] = [
    MessageAction::Revert,
    MessageAction::Copy,
    MessageAction::Fork,
];

impl MessageAction {
    pub(super) const fn title(self) -> &'static str {
        match self {
            Self::Revert => "Revert",
            Self::Copy => "Copy",
            Self::Fork => "Fork",
        }
    }

    pub(super) const fn description(self) -> &'static str {
        match self {
            Self::Revert => "undo messages and file changes",
            Self::Copy => "message text to clipboard",
            Self::Fork => "create a new session",
        }
    }
}

pub(super) fn action_at(index: usize) -> Option<MessageAction> {
    ACTIONS.get(index).copied()
}

#[derive(Default)]
struct PreparedPrompt {
    text: String,
    files: HashSet<String>,
    images: Vec<PromptImage>,
}

impl Workspace {
    pub(super) fn open_message_actions(&mut self, cx: &mut Context<Self>) {
        let Some(entry) = self.timeline_suggestions.get(self.overlay_selection) else {
            return;
        };
        self.timeline_message = Some(entry.message_id.clone());
        self.overlay = Overlay::MessageActions;
        self.overlay_selection = 0;
        self.focus_overlay_on_render = true;
        cx.notify();
    }

    pub(super) fn submit_message_action(
        &mut self,
        _: &super::navigation::SubmitMessageAction,
        _: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        if self.overlay == Overlay::MessageActions {
            self.execute_message_action(cx);
        }
    }

    pub(super) fn execute_message_action(&mut self, cx: &mut Context<Self>) {
        let Some(action) = action_at(self.overlay_selection) else {
            return;
        };
        let Some(message_id) = self.timeline_message.clone() else {
            return;
        };
        match action {
            MessageAction::Copy => self.copy_user_message(&message_id, cx),
            MessageAction::Revert => self.revert_to_message(message_id, cx),
            MessageAction::Fork => self.fork_from_message(message_id, cx),
        }
    }

    fn selected_message(&self, message_id: &str) -> Option<MessageRecord> {
        self.timeline_history
            .iter()
            .find(|message| message.info.id() == message_id)
            .cloned()
    }

    fn copy_user_message(&mut self, message_id: &str, cx: &mut Context<Self>) {
        let Some(message) = self.selected_message(message_id) else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(user_text(&message)));
        self.close_timeline_overlay(cx);
    }

    fn revert_to_message(&mut self, message_id: String, cx: &mut Context<Self>) {
        let Some((directory, session_id, client, message)) = self.action_context(&message_id)
        else {
            return;
        };
        let action_directory = directory.clone();
        let preparation = cx.background_spawn(async move { prepare_prompt(&directory, &message) });
        self.overlay = Overlay::None;
        self.command_editor.update(cx, TextEditor::clear);
        self.focus_editor_on_render = true;
        cx.spawn(async move |workspace, cx| {
            let (result, prompt) =
                futures_util::join!(client.revert(&session_id, &message_id), preparation);
            let _ = workspace.update(cx, |workspace, cx| match result {
                Ok(session) => {
                    workspace.apply_events(vec![Event::SessionUpdated(session)], None);
                    if let Some(tab) = workspace
                        .tabs
                        .iter_mut()
                        .find(|tab| tab.directory == action_directory)
                        && let TimelineState::Ready { messages, .. } = &mut tab.timeline
                        && let Some(index) = messages
                            .iter()
                            .position(|item| item.info.id() == message_id)
                    {
                        messages.truncate(index);
                    }
                    workspace.restore_prepared_prompt(&action_directory, prompt, cx);
                }
                Err(error) => workspace.show_action_error(&action_directory, error.to_string()),
            });
        })
        .detach();
        cx.notify();
    }

    fn fork_from_message(&mut self, message_id: String, cx: &mut Context<Self>) {
        let Some((directory, session_id, client, message)) = self.action_context(&message_id)
        else {
            return;
        };
        let action_directory = directory.clone();
        let preparation = cx.background_spawn(async move { prepare_prompt(&directory, &message) });
        self.overlay = Overlay::None;
        self.command_editor.update(cx, TextEditor::clear);
        self.focus_editor_on_render = true;
        cx.spawn(async move |workspace, cx| {
            let (result, prompt) =
                futures_util::join!(client.fork(&session_id, &message_id), preparation);
            let _ = workspace.update(cx, |workspace, cx| match result {
                Ok(session) => {
                    let title = super::format::display_title(&session).into();
                    let fork_id = session.id.clone();
                    workspace.apply_events(vec![Event::SessionCreated(session)], None);
                    workspace.select_session_in(&action_directory, fork_id, title, cx);
                    workspace.restore_prepared_prompt(&action_directory, prompt, cx);
                }
                Err(error) => workspace.show_action_error(&action_directory, error.to_string()),
            });
        })
        .detach();
        cx.notify();
    }

    fn action_context(
        &self,
        message_id: &str,
    ) -> Option<(String, String, opencode_gpui::api::Client, MessageRecord)> {
        let tab = self.active_tab()?;
        Some((
            tab.directory.clone(),
            tab.timeline.session_id()?.to_owned(),
            tab.client.clone(),
            self.selected_message(message_id)?,
        ))
    }

    fn restore_prepared_prompt(
        &mut self,
        directory: &str,
        prompt: PreparedPrompt,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        tab.editor
            .update(cx, |editor, cx| editor.restore_text(prompt.text, cx));
        tab.attached_files = prompt.files;
        tab.attached_images = prompt.images;
        tab.prompt_error = None;
        self.focus_editor_on_render = true;
        self.capture_draft(directory, false, cx);
        cx.notify();
    }

    fn show_action_error(&mut self, directory: &str, error: String) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) {
            tab.prompt_error = Some(error.into());
        }
    }

    fn close_timeline_overlay(&mut self, cx: &mut Context<Self>) {
        self.overlay = Overlay::None;
        self.command_editor.update(cx, TextEditor::clear);
        self.focus_editor_on_render = true;
        cx.notify();
    }
}

fn user_text(message: &MessageRecord) -> String {
    message
        .parts
        .iter()
        .filter(|part| part.kind == "text" && !synthetic(part))
        .filter_map(Part::text)
        .collect()
}

fn prepare_prompt(directory: &str, message: &MessageRecord) -> PreparedPrompt {
    let mut prompt = PreparedPrompt {
        text: user_text(message),
        ..PreparedPrompt::default()
    };
    for part in message.parts.iter().filter(|part| part.kind == "file") {
        let Some(url) = part.data.get("url").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let mime = part
            .data
            .get("mime")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let filename = part
            .data
            .get("filename")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("attachment");
        if mime.starts_with("image/")
            && let Some(image) = super::image_attachment::decode_data_url(url)
        {
            prompt.images.push(PromptImage {
                id: format!("restored-{}", part.id),
                filename: filename.into(),
                image,
                data_url: Some(Arc::from(url)),
            });
        } else if let Ok(url) = url::Url::parse(url)
            && let Ok(path) = url.to_file_path()
            && let Ok(relative) = path.strip_prefix(directory)
        {
            prompt.files.insert(relative.to_string_lossy().into_owned());
        }
    }
    prompt
}

fn synthetic(part: &Part) -> bool {
    part.data
        .get("synthetic")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}
