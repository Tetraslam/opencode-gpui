use super::{composer_completion::CompletionItem, workspace_command::Command};

struct SlashDefinition {
    name: &'static str,
    description: &'static str,
    aliases: &'static [&'static str],
    command: Command,
}

const DEFINITIONS: [SlashDefinition; 12] = [
    SlashDefinition {
        name: "sessions",
        description: "switch session",
        aliases: &["resume", "continue"],
        command: Command::ToggleSessions,
    },
    SlashDefinition {
        name: "new",
        description: "new session",
        aliases: &["clear"],
        command: Command::NewSession,
    },
    SlashDefinition {
        name: "agents",
        description: "select agent",
        aliases: &[],
        command: Command::SelectAgent,
    },
    SlashDefinition {
        name: "models",
        description: "select model",
        aliases: &["mo"],
        command: Command::SelectModel,
    },
    SlashDefinition {
        name: "variants",
        description: "select model variant",
        aliases: &[],
        command: Command::SelectVariant,
    },
    SlashDefinition {
        name: "workspaces",
        description: "open directory",
        aliases: &[],
        command: Command::OpenDirectory,
    },
    SlashDefinition {
        name: "timeline",
        description: "browse message timeline",
        aliases: &[],
        command: Command::Timeline,
    },
    SlashDefinition {
        name: "help",
        description: "show command palette",
        aliases: &[],
        command: Command::ShowCommandPalette,
    },
    SlashDefinition {
        name: "status",
        description: "view status",
        aliases: &[],
        command: Command::Status,
    },
    SlashDefinition {
        name: "debug",
        description: "view debug info",
        aliases: &[],
        command: Command::Debug,
    },
    SlashDefinition {
        name: "exit",
        description: "exit the app",
        aliases: &["quit", "q"],
        command: Command::ExitApp,
    },
    SlashDefinition {
        name: "move",
        description: "move to another project directory",
        aliases: &[],
        command: Command::OpenDirectory,
    },
];

pub(super) fn local_slashes(query: &str) -> Vec<CompletionItem> {
    DEFINITIONS
        .iter()
        .filter(|definition| {
            definition.name.contains(query)
                || definition.description.contains(query)
                || definition.aliases.iter().any(|alias| alias.contains(query))
        })
        .map(|definition| CompletionItem::Local {
            name: definition.name,
            description: definition.description,
            action: definition.command,
        })
        .collect()
}

pub(super) fn local_slash(name: &str) -> Option<Command> {
    DEFINITIONS
        .iter()
        .find(|definition| definition.name == name || definition.aliases.contains(&name))
        .map(|definition| definition.command)
}
