use std::{ops::Range, sync::Arc};

use gpui::{AppContext, Context, SharedString};

use super::{Workspace, composer_slashes::local_slashes};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CompletionMode {
    File,
    Command,
}

#[derive(Clone, Debug)]
pub(super) enum CompletionItem {
    File(String),
    Local {
        name: &'static str,
        description: &'static str,
        action: LocalSlash,
    },
    Command {
        name: String,
        description: SharedString,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LocalSlash {
    Sessions,
    New,
    Workspaces,
    Agents,
    Models,
    Variants,
    Help,
    Exit,
}

#[derive(Clone, Debug)]
pub(super) struct ComposerCompletion {
    pub(super) mode: CompletionMode,
    pub(super) range: Range<usize>,
    pub(super) query: String,
    pub(super) items: Arc<Vec<CompletionItem>>,
    pub(super) selected: usize,
    pub(super) loading: bool,
}

impl Workspace {
    pub(super) fn refresh_composer_completion(&mut self, directory: &str, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        let editor = tab.editor.read(cx);
        let Some((mode, range, query)) = completion_trigger(editor.text(), editor.cursor_offset())
        else {
            tab.composer_completion = None;
            cx.notify();
            return;
        };
        let client = tab.client.clone();
        let initial_items = match mode {
            CompletionMode::File => Vec::new(),
            CompletionMode::Command => local_slashes(&query.to_lowercase()),
        };
        tab.composer_completion = Some(ComposerCompletion {
            mode,
            range: range.clone(),
            query: query.clone(),
            items: Arc::new(initial_items),
            selected: 0,
            loading: true,
        });
        let completion_query = query.clone();
        let request = cx.background_spawn(async move {
            match mode {
                CompletionMode::File => client
                    .find_files(&query, 20)
                    .await
                    .map(|files| files.into_iter().map(CompletionItem::File).collect()),
                CompletionMode::Command => client.commands().await.map(|commands| {
                    let needle = query.to_lowercase();
                    local_slashes(&needle)
                        .into_iter()
                        .chain(
                            commands
                                .into_iter()
                                .filter(|command| {
                                    command.name.to_lowercase().contains(&needle)
                                        || command.description.as_deref().is_some_and(|value| {
                                            value.to_lowercase().contains(&needle)
                                        })
                                })
                                .map(|command| CompletionItem::Command {
                                    name: command.name,
                                    description: command.description.unwrap_or_default().into(),
                                }),
                        )
                        .take(20)
                        .collect()
                }),
            }
        });
        let completion_directory = directory.to_owned();
        tab.completion_task = Some(cx.spawn(async move |workspace, cx| {
            let items = request.await.unwrap_or_default();
            let _ = workspace.update(cx, |workspace, cx| {
                let Some(tab) = workspace
                    .tabs
                    .iter_mut()
                    .find(|tab| tab.directory == completion_directory)
                else {
                    return;
                };
                let Some(completion) = &mut tab.composer_completion else {
                    return;
                };
                if completion.mode == mode
                    && completion.range == range
                    && completion.query == completion_query
                {
                    completion.items = Arc::new(items);
                    completion.selected = 0;
                    completion.loading = false;
                    cx.notify();
                }
            });
        }));
    }

    pub(super) fn move_composer_selection(&mut self, delta: isize, cx: &mut Context<Self>) -> bool {
        let selected = {
            let Some(completion) = self
                .active_tab_mut()
                .and_then(|tab| tab.composer_completion.as_mut())
            else {
                return false;
            };
            let count = completion.items.len();
            if count == 0 {
                return true;
            }
            completion.selected = usize::try_from(
                (completion.selected.cast_signed() + delta).rem_euclid(count.cast_signed()),
            )
            .unwrap_or_default();
            completion.selected
        };
        self.composer_completion_scroll.scroll_to_item(selected);
        cx.notify();
        true
    }

    pub(super) fn accept_composer_completion(
        &mut self,
        directory: &str,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return false;
        };
        let Some(completion) = tab.composer_completion.clone() else {
            return false;
        };
        let Some(item) = completion.items.get(completion.selected) else {
            return false;
        };
        let replacement = match item {
            CompletionItem::File(path) => {
                tab.attached_files.insert(path.clone());
                format!("@{path} ")
            }
            CompletionItem::Local { action, .. } => {
                let action = *action;
                tab.composer_completion = None;
                tab.attached_files.clear();
                tab.attached_images.clear();
                let editor = tab.editor.clone();
                editor.update(cx, |editor, cx| editor.restore_text("", cx));
                self.capture_active_draft(true, cx);
                self.execute_local_slash(action, cx);
                return true;
            }
            CompletionItem::Command { name, .. } => format!("/{name} "),
        };
        tab.editor.update(cx, |editor, cx| {
            editor.replace_range(completion.range, &replacement, cx);
        });
        tab.composer_completion = None;
        cx.notify();
        true
    }
}

fn completion_trigger(text: &str, cursor: usize) -> Option<(CompletionMode, Range<usize>, String)> {
    let prefix = text.get(..cursor)?;
    if let Some(query) = prefix.strip_prefix('/')
        && !query.chars().any(char::is_whitespace)
    {
        return Some((CompletionMode::Command, 0..cursor, query.to_owned()));
    }
    let start = prefix.rfind('@')?;
    if start > 0 && !prefix[..start].ends_with(char::is_whitespace) {
        return None;
    }
    let query = &prefix[start + 1..];
    (!query.chars().any(char::is_whitespace))
        .then(|| (CompletionMode::File, start..cursor, query.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_leading_commands() {
        assert_eq!(
            completion_trigger("/rev", 4),
            Some((CompletionMode::Command, 0..4, "rev".into()))
        );
        assert_eq!(completion_trigger("ask /rev", 8), None);
    }

    #[test]
    fn recognizes_mentions_at_word_boundaries() {
        assert_eq!(
            completion_trigger("read @src/app", 13),
            Some((CompletionMode::File, 5..13, "src/app".into()))
        );
        assert_eq!(completion_trigger("mail@example", 12), None);
        assert_eq!(
            completion_trigger("first line\n@src", 15),
            Some((CompletionMode::File, 11..15, "src".into()))
        );
        assert_eq!(completion_trigger("/review\nnotes", 13), None);
    }
}
