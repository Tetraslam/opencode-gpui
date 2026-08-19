use gpui::Context;

use super::{
    Workspace,
    command_palette::Overlay,
    composer_completion::{CompletionItem, LocalSlash},
};

pub(super) fn local_slashes(query: &str) -> Vec<CompletionItem> {
    const ITEMS: [(&str, &str, &[&str], LocalSlash); 6] = [
        (
            "sessions",
            "switch session",
            &["resume", "continue"],
            LocalSlash::Sessions,
        ),
        ("new", "new session", &["clear"], LocalSlash::New),
        ("workspaces", "open directory", &[], LocalSlash::Workspaces),
        (
            "move",
            "move to another project directory",
            &[],
            LocalSlash::Workspaces,
        ),
        ("help", "show command palette", &[], LocalSlash::Help),
        ("exit", "exit the app", &["quit", "q"], LocalSlash::Exit),
    ];
    ITEMS
        .into_iter()
        .filter(|(name, description, aliases, _)| {
            name.contains(query)
                || description.contains(query)
                || aliases.iter().any(|alias| alias.contains(query))
        })
        .map(|(name, description, _, action)| CompletionItem::Local {
            name,
            description,
            action,
        })
        .collect()
}

impl Workspace {
    pub(super) fn execute_local_slash(&mut self, action: LocalSlash, cx: &mut Context<Self>) {
        match action {
            LocalSlash::Sessions => self.sessions_open = !self.sessions_open,
            LocalSlash::New => self.create_active_session(cx),
            LocalSlash::Workspaces => {
                self.overlay = Overlay::Directory;
                self.focus_overlay_on_render = true;
            }
            LocalSlash::Help => {
                self.overlay = Overlay::Command;
                self.focus_overlay_on_render = true;
            }
            LocalSlash::Exit => cx.quit(),
        }
        cx.notify();
    }
}
