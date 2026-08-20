use gpui::Context;

use super::Workspace;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub(super) enum PromptMode {
    #[default]
    Normal,
    Shell,
}

impl Workspace {
    pub(super) fn update_prompt_mode(&mut self, directory: &str, cx: &mut Context<Self>) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return false;
        };
        if tab.prompt_mode != PromptMode::Normal {
            return false;
        }
        let text = tab.editor.read(cx).text().to_owned();
        let Some(command) = text.strip_prefix('!').map(str::to_owned) else {
            return false;
        };
        tab.prompt_mode = PromptMode::Shell;
        tab.composer_completion = None;
        tab.editor
            .update(cx, |editor, cx| editor.restore_text(command, cx));
        cx.notify();
        true
    }
}
