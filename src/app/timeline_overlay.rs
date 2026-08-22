use std::sync::Arc;

use gpui::Context;
use opencode_gpui::{
    editor::TextEditor,
    model::{Message, MessageRecord},
};

use super::{TimelineState, Workspace, command_palette::Overlay};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TimelineEntry {
    pub(super) message_id: String,
    pub(super) title: String,
    pub(super) created: u64,
}

pub(super) fn extract_entries(messages: &[MessageRecord], query: &str) -> Vec<TimelineEntry> {
    let query = query.trim().to_lowercase();
    let mut entries = messages
        .iter()
        .filter_map(|message| {
            let Message::User(user) = &message.info else {
                return None;
            };
            let text = message
                .parts
                .iter()
                .find(|part| {
                    part.kind == "text"
                        && !part
                            .data
                            .get("synthetic")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false)
                        && !part
                            .data
                            .get("ignored")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false)
                })?
                .text()?;
            let title = text.replace('\n', " ");
            (query.is_empty() || title.to_lowercase().contains(&query)).then(|| TimelineEntry {
                message_id: user.id.clone(),
                title,
                created: user.time.created,
            })
        })
        .collect::<Vec<_>>();
    entries.reverse();
    entries
}

impl Workspace {
    pub(super) fn open_timeline(&mut self, cx: &mut Context<Self>) {
        let Some((directory, session_id, client, initial)) = self.active_tab().and_then(|tab| {
            let TimelineState::Ready {
                session_id,
                messages,
                ..
            } = &tab.timeline
            else {
                return None;
            };
            Some((
                tab.directory.clone(),
                session_id.clone(),
                tab.client.clone(),
                messages.clone(),
            ))
        }) else {
            return;
        };
        self.overlay = Overlay::Timeline;
        self.clear_interrupt();
        self.overlay_selection = 0;
        self.timeline_query.clear();
        self.timeline_history = Arc::new(initial);
        self.timeline_history_session = Some(session_id.clone());
        self.command_editor.update(cx, TextEditor::clear);
        self.refresh_timeline_suggestions("");
        self.preview_timeline_selection();
        self.focus_overlay_on_render = true;
        let requested_id = session_id.clone();
        cx.spawn(async move |workspace, cx| {
            let result = client.messages(&requested_id, 1_000).await;
            let _ = workspace.update(cx, |workspace, cx| {
                let Ok(messages) = result else { return };
                let active_matches = workspace.active_tab().is_some_and(|tab| {
                    tab.directory == directory
                        && tab.timeline.session_id() == Some(session_id.as_str())
                });
                if !active_matches
                    || workspace.timeline_history_session.as_deref() != Some(session_id.as_str())
                {
                    return;
                }
                workspace.replace_timeline_history(messages, true);
                if workspace.overlay == Overlay::Timeline {
                    workspace.preview_timeline_selection();
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn refresh_timeline_suggestions(&mut self, query: &str) {
        query.clone_into(&mut self.timeline_query);
        self.timeline_suggestions = Arc::new(extract_entries(&self.timeline_history, query));
        self.overlay_selection = 0;
        self.reset_picker_scroll();
        self.timeline_message = self
            .timeline_suggestions
            .first()
            .map(|entry| entry.message_id.clone());
    }

    pub(super) fn replace_timeline_history(
        &mut self,
        messages: Vec<MessageRecord>,
        preserve_selection: bool,
    ) {
        let preferred = if preserve_selection {
            self.timeline_message.clone()
        } else {
            None
        };
        self.timeline_history = Arc::new(messages);
        let entries = Arc::new(extract_entries(
            &self.timeline_history,
            &self.timeline_query,
        ));
        let selected = preferred_entry_index(&entries, preferred.as_deref());
        self.timeline_message = entries.get(selected).map(|entry| entry.message_id.clone());
        self.timeline_suggestions = entries;
        if self.overlay == Overlay::Timeline {
            self.overlay_selection = selected;
            if preserve_selection {
                self.picker_scroll.scroll_to_item(selected);
            } else {
                self.reset_picker_scroll();
            }
        }
    }

    pub(super) fn preview_timeline_selection(&mut self) {
        let Some(entry) = self.timeline_suggestions.get(self.overlay_selection) else {
            return;
        };
        let message_id = entry.message_id.clone();
        self.timeline_message = Some(message_id.clone());
        if let Some(tab) = self.active_tab_mut() {
            let TimelineState::Ready { messages, .. } = &tab.timeline else {
                return;
            };
            let Some(message_index) = rendered_message_index(messages, &message_id) else {
                return;
            };
            tab.follow_tail = false;
            let older_offset = usize::from(!tab.history_exhausted);
            tab.timeline_scroll
                .scroll_to_item(message_index + older_offset);
        }
    }
}

pub(super) fn preferred_entry_index(entries: &[TimelineEntry], message_id: Option<&str>) -> usize {
    message_id
        .and_then(|id| entries.iter().position(|entry| entry.message_id == id))
        .unwrap_or_default()
}

pub(super) fn rendered_message_index(
    messages: &[MessageRecord],
    message_id: &str,
) -> Option<usize> {
    messages
        .iter()
        .position(|message| message.info.id() == message_id)
}
