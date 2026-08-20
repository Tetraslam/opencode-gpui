use std::{collections::HashSet, time::Duration};

use super::{Workspace, command_palette::Overlay, image_attachment::PromptImage};
use gpui::{AppContext, Context, MouseDownEvent, Window};

pub(super) type DraftKey = (String, String);

#[derive(Clone, Debug, Default)]
pub(super) struct SessionDraft {
    pub(super) text: String,
    pub(super) attached_files: HashSet<String>,
    pub(super) attached_images: Vec<PromptImage>,
    pub(super) prompt_mode: super::prompt_mode::PromptMode,
    pub(super) updated_at: u64,
}

impl Workspace {
    pub(super) fn composer_changed(&mut self, directory: &str, cx: &mut Context<Self>) {
        if self.update_prompt_mode(directory, cx) {
            self.capture_draft(directory, false, cx);
            return;
        }
        if self
            .tabs
            .iter_mut()
            .find(|tab| tab.directory == directory)
            .is_some_and(|tab| {
                if tab.prompt_mode == super::prompt_mode::PromptMode::Shell {
                    tab.composer_completion = None;
                    true
                } else {
                    false
                }
            })
        {
            self.capture_draft(directory, false, cx);
            return;
        }
        self.refresh_composer_completion(directory, cx);
        self.capture_draft(directory, false, cx);
    }

    pub(super) fn capture_active_draft(&mut self, flush: bool, cx: &mut Context<Self>) {
        if let Some(directory) = self.active_directory().map(str::to_owned) {
            self.capture_draft(&directory, flush, cx);
        }
    }

    pub(super) fn capture_draft(&mut self, directory: &str, flush: bool, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.iter().find(|tab| tab.directory == directory) else {
            return;
        };
        let Some(session_id) = tab.timeline.session_id().map(str::to_owned) else {
            return;
        };
        let text = tab.editor.read(cx).text().to_owned();
        let attached_files = tab
            .attached_files
            .iter()
            .filter(|path| text.contains(&format!("@{path}")))
            .cloned()
            .collect::<HashSet<_>>();
        let attached_images = tab.attached_images.clone();
        let key = (directory.to_owned(), session_id);
        if text.is_empty() && attached_files.is_empty() && attached_images.is_empty() {
            self.drafts.remove(&key);
        } else {
            self.drafts.insert(
                key,
                SessionDraft {
                    text,
                    attached_files,
                    attached_images,
                    prompt_mode: tab.prompt_mode,
                    updated_at: super::draft_persistence::now_millis(),
                },
            );
        }
        self.persist_drafts(flush, cx);
    }

    pub(super) fn restore_draft(
        &mut self,
        directory: &str,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        let draft = self
            .drafts
            .get(&(directory.to_owned(), session_id.to_owned()))
            .cloned()
            .unwrap_or_default();
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) {
            if tab.timeline.session_id() != Some(session_id) {
                return;
            }
            tab.attached_files = draft.attached_files;
            tab.attached_images = draft.attached_images;
            tab.prompt_mode = draft.prompt_mode;
            tab.editor
                .update(cx, |editor, cx| editor.restore_text(draft.text, cx));
        }
    }

    pub(super) fn clear_draft(
        &mut self,
        directory: &str,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        self.drafts
            .remove(&(directory.to_owned(), session_id.to_owned()));
        self.persist_drafts(true, cx);
    }

    pub(super) fn dismiss_transients(&mut self) {
        self.overlay = Overlay::None;
        self.directory_error = None;
        if let Some(tab) = self.active_tab_mut() {
            tab.composer_completion = None;
        }
    }

    pub(super) fn dismiss_on_click(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let completion_open = self
            .active_tab()
            .is_some_and(|tab| tab.composer_completion.is_some());
        if self.overlay != Overlay::None || completion_open {
            self.dismiss_transients();
            cx.notify();
        }
    }

    fn persist_drafts(&mut self, flush: bool, cx: &mut Context<Self>) {
        let drafts = self.drafts.clone();
        let path = self.draft_path.clone();
        let timer = cx.background_executor().timer(if flush {
            Duration::ZERO
        } else {
            Duration::from_millis(180)
        });
        self.draft_save = Some(cx.background_spawn(async move {
            timer.await;
            if let Err(error) = super::draft_persistence::write_drafts_to(&path, drafts) {
                eprintln!("draft persistence failed: {error}");
            }
        }));
    }
}
