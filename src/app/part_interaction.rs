use gpui::{Context, Window};
use opencode_gpui::model::Part;

use super::{PartSelection, Workspace};

impl Workspace {
    pub(super) fn restore_pending_detail_anchor(&mut self, window: &mut Window) {
        let Some(tab) = self.active_tab_mut() else {
            return;
        };
        let Some(old_max) = tab.pending_detail_anchor.take() else {
            return;
        };
        let handle = tab.timeline_scroll.clone();
        window.on_next_frame(move |_, _| {
            let mut offset = handle.offset();
            offset.y -= handle.max_offset().height - old_max;
            handle.set_offset(offset);
        });
    }

    pub(super) fn toggle_part(
        &mut self,
        selection: PartSelection,
        part: Part,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let default_expanded =
            self.settings.expand_diffs && super::part_format::produces_diff(&part);
        self.select_part(selection.clone(), part, cx);
        let Some(tab) = self.active_tab_mut() else {
            return;
        };
        let handle = tab.timeline_scroll.clone();
        let old_max = handle.max_offset().height;
        let preserve_lower_edge = !tab.follow_tail;
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
        if preserve_lower_edge {
            window.on_next_frame(move |_, _| {
                let mut offset = handle.offset();
                offset.y -= handle.max_offset().height - old_max;
                handle.set_offset(offset);
            });
        } else {
            handle.scroll_to_bottom();
        }
        cx.notify();
    }
}
