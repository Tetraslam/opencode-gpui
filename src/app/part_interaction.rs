use gpui::{Context, Window};
use opencode_gpui::model::Part;

use super::{PartSelection, Workspace};

impl Workspace {
    pub(super) fn toggle_part(
        &mut self,
        selection: PartSelection,
        part: Part,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let default_expanded =
            self.settings.expand_diffs && super::part_format::produces_diff(&part);
        self.select_part(selection.clone(), part, cx);
        let Some(tab) = self.active_tab_mut() else {
            return;
        };
        let handle = tab.timeline_scroll.clone();
        let follow_tail = tab.follow_tail;
        let expanded = tab.expanded_parts.contains(&selection)
            || (default_expanded && !tab.collapsed_parts.contains(&selection));
        if expanded {
            tab.expanded_parts.remove(&selection);
            if default_expanded {
                tab.collapsed_parts.insert(selection);
            }
        } else {
            tab.collapsed_parts.remove(&selection);
            tab.expanded_parts.insert(selection);
        }
        if follow_tail {
            handle.scroll_to_bottom();
        }
        cx.notify();
    }
}
