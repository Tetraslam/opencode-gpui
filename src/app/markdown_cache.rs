use std::{collections::HashMap, sync::Arc};

use gpui::{AppContext, Context};
use opencode_gpui::markdown::{self, Document};

use super::{PartSelection, TimelineState, Workspace};

pub(super) struct CachedMarkdown {
    pub(super) source: String,
    pub(super) document: Arc<Document>,
}

#[derive(Default)]
pub(super) struct MarkdownCache {
    pub(super) documents: HashMap<PartSelection, CachedMarkdown>,
    pending: HashMap<PartSelection, String>,
}

impl Workspace {
    pub(super) fn refresh_markdown(&mut self, directory: &str, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        let TimelineState::Ready { messages, .. } = &tab.timeline else {
            return;
        };
        let mut changed = Vec::new();
        for message in messages {
            for part in &message.parts {
                let Some(text) = (part.kind == "text").then(|| part.text()).flatten() else {
                    continue;
                };
                let selection = PartSelection {
                    message_id: part.message_id.clone(),
                    part_id: part.id.clone(),
                };
                let current = tab
                    .markdown
                    .documents
                    .get(&selection)
                    .is_some_and(|cached| cached.source == text);
                if current {
                    continue;
                }
                let source = text.to_owned();
                if let Some(pending) = tab.markdown.pending.get_mut(&selection) {
                    *pending = source;
                    continue;
                }
                tab.markdown
                    .pending
                    .insert(selection.clone(), source.clone());
                changed.push((selection, source));
            }
        }

        for (selection, source) in changed {
            let parsed_source = source.clone();
            let parse =
                cx.background_spawn(async move { Arc::new(markdown::parse(&parsed_source)) });
            let task_directory = directory.to_owned();
            cx.spawn(async move |workspace, cx| {
                let document = parse.await;
                let _ = workspace.update(cx, |workspace, cx| {
                    let stale = {
                        let Some(tab) = workspace
                            .tabs
                            .iter_mut()
                            .find(|tab| tab.directory == task_directory)
                        else {
                            return;
                        };
                        let current = tab.markdown.pending.get(&selection) == Some(&source);
                        tab.markdown.pending.remove(&selection);
                        if current {
                            tab.markdown
                                .documents
                                .insert(selection, CachedMarkdown { source, document });
                        }
                        !current
                    };
                    if stale {
                        workspace.refresh_markdown(&task_directory, cx);
                    }
                    cx.notify();
                });
            })
            .detach();
        }
    }
}
