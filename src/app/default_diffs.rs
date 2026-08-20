use gpui::Context;

use super::{PartSelection, TimelineState, Workspace};

impl Workspace {
    pub(super) fn prepare_default_diffs(&mut self, directory: &str, cx: &mut Context<Self>) {
        if !self.settings.expand_diffs {
            return;
        }
        let Some(tab) = self.tabs.iter().find(|tab| tab.directory == directory) else {
            return;
        };
        let TimelineState::Ready { messages, .. } = &tab.timeline else {
            return;
        };
        let parts = messages
            .iter()
            .flat_map(|message| &message.parts)
            .filter(|part| super::part_format::produces_diff(part))
            .map(|part| {
                (
                    PartSelection {
                        message_id: part.message_id.clone(),
                        part_id: part.id.clone(),
                    },
                    part.clone(),
                )
            })
            .collect::<Vec<_>>();
        for (selection, part) in parts {
            self.prepare_part_detail(directory, selection, part, cx);
        }
    }
}
