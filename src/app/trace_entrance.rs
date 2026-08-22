use std::time::Duration;

use gpui::{Context, Timer};
use opencode_gpui::{event::Event, model::Part};

use super::{PartSelection, TimelineState, Workspace};

impl Workspace {
    pub(super) fn mark_event_entrances(
        &mut self,
        events: &[Event],
        directory: &str,
        cx: &mut Context<Self>,
    ) {
        if !self.settings.animate_trace_entries {
            return;
        }
        let Some(tab) = self.tabs.iter().find(|tab| tab.directory == directory) else {
            return;
        };
        let TimelineState::Ready {
            session_id,
            messages,
            ..
        } = &tab.timeline
        else {
            return;
        };
        let selections = events
            .iter()
            .filter_map(|event| match event {
                Event::MessagePartUpdated { part, .. }
                    if part.session_id == *session_id
                        && visible(part)
                        && !messages.iter().any(|message| {
                            message.info.id() == part.message_id
                                && message.parts.iter().any(|current| current.id == part.id)
                        }) =>
                {
                    Some(PartSelection {
                        message_id: part.message_id.clone(),
                        part_id: part.id.clone(),
                    })
                }
                _ => None,
            })
            .collect();
        self.mark_selections(selections, cx);
    }

    pub(super) fn mark_part_entrances(&mut self, parts: &[Part], cx: &mut Context<Self>) {
        if !self.settings.animate_trace_entries {
            return;
        }
        let selections = parts
            .iter()
            .map(|part| PartSelection {
                message_id: part.message_id.clone(),
                part_id: part.id.clone(),
            })
            .collect();
        self.mark_selections(selections, cx);
    }

    fn mark_selections(&mut self, selections: Vec<PartSelection>, cx: &mut Context<Self>) {
        if !self.settings.animate_trace_entries {
            return;
        }
        let selections = selections
            .into_iter()
            .filter(|selection| self.trace_entrances.insert(selection.clone()))
            .collect::<Vec<_>>();
        if selections.is_empty() {
            return;
        }
        cx.spawn(async move |workspace, cx| {
            Timer::after(Duration::from_millis(140)).await;
            let _ = workspace.update(cx, |workspace, _| {
                for selection in selections {
                    workspace.trace_entrances.remove(&selection);
                }
            });
        })
        .detach();
    }
}

fn visible(part: &Part) -> bool {
    !(matches!(
        part.kind.as_str(),
        "step-start" | "step-finish" | "snapshot" | "compaction" | "patch"
    ) || part.kind == "text"
        && part
            .data
            .get("synthetic")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false))
}
