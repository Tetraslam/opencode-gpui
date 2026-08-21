use gpui::Context;

use super::{
    Workspace,
    command_palette::Overlay,
    composer_completion::{CompletionItem, LocalSlash},
};

pub(super) fn local_slashes(query: &str) -> Vec<CompletionItem> {
    const ITEMS: [(&str, &str, &[&str], LocalSlash); 9] = [
        (
            "sessions",
            "switch session",
            &["resume", "continue"],
            LocalSlash::Sessions,
        ),
        ("new", "new session", &["clear"], LocalSlash::New),
        ("agents", "select agent", &[], LocalSlash::Agents),
        ("models", "select model", &["mo"], LocalSlash::Models),
        (
            "variants",
            "select model variant",
            &[],
            LocalSlash::Variants,
        ),
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

pub(super) fn local_slash(name: &str) -> Option<LocalSlash> {
    match name {
        "sessions" | "resume" | "continue" => Some(LocalSlash::Sessions),
        "new" | "clear" => Some(LocalSlash::New),
        "workspaces" | "move" => Some(LocalSlash::Workspaces),
        "agents" => Some(LocalSlash::Agents),
        "models" | "mo" => Some(LocalSlash::Models),
        "variants" => Some(LocalSlash::Variants),
        "help" => Some(LocalSlash::Help),
        "exit" | "quit" | "q" => Some(LocalSlash::Exit),
        _ => None,
    }
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
            LocalSlash::Agents => {
                self.open_selection(super::selection_overlay::SelectionKind::Agent, cx);
            }
            LocalSlash::Models => {
                self.open_selection(super::selection_overlay::SelectionKind::Model, cx);
            }
            LocalSlash::Variants => {
                self.open_selection(super::selection_overlay::SelectionKind::Variant, cx);
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
