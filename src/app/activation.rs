use gpui::{AppContext, Context};
use opencode_gpui::editor::{Changed, PastedImage, Submit, TextEditor};

use super::{Workspace, tabs::DirectoryTab};

impl Workspace {
    pub(super) fn activate_directory(&mut self, directory: String, cx: &mut Context<Self>) -> bool {
        self.capture_active_draft(true, cx);
        self.dismiss_transients();
        self.record_directory_open(&directory, cx);
        if let Some(index) = self.tabs.iter().position(|tab| tab.directory == directory) {
            self.active_tab = index;
            self.select_default_session(cx);
            self.focus_editor_on_render = true;
            cx.notify();
            return self.tabs[index].timeline.session_id().is_none();
        }
        let Some(client) = self
            .client
            .as_ref()
            .map(|client| client.scoped(directory.clone()))
        else {
            return false;
        };
        let editor = cx.new(|cx| {
            TextEditor::new("ask anything, @ files, / commands", cx).preserve_on_submit()
        });
        let event_directory = directory.clone();
        let submit = cx.subscribe(&editor, move |workspace, editor, event: &Submit, cx| {
            if workspace.accept_composer_completion(&event_directory, cx) {
                return;
            }
            let has_images = workspace
                .tabs
                .iter()
                .find(|tab| tab.directory == event_directory)
                .is_some_and(|tab| !tab.attached_images.is_empty());
            if event.text.trim().is_empty() && !has_images {
                return;
            }
            workspace.capture_draft(&event_directory, true, cx);
            editor.update(cx, |editor, cx| editor.restore_text("", cx));
            workspace.submit_composer_in(&event_directory, event.text.clone(), cx);
        });
        let completion_directory = directory.clone();
        let changed = cx.subscribe(&editor, move |workspace, _, _: &Changed, cx| {
            workspace.composer_changed(&completion_directory, cx);
        });
        let image_directory = directory.clone();
        let pasted_image = cx.subscribe(&editor, move |workspace, _, event: &PastedImage, cx| {
            workspace.attach_clipboard_image(&image_directory, event.image.clone(), cx);
        });
        let events = Self::spawn_event_loop(client.clone(), directory.clone(), cx);
        self.tabs.push(DirectoryTab::new(
            directory,
            client,
            editor,
            vec![submit, changed, pasted_image],
            events,
        ));
        self.active_tab = self.tabs.len() - 1;
        self.focus_editor_on_render = true;
        self.select_default_session(cx);
        cx.notify();
        self.active_tab()
            .is_some_and(|tab| tab.timeline.session_id().is_none())
    }
}
