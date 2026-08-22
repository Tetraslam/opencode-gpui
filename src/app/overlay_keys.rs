use gpui::{Context, Window};

use super::{
    Workspace,
    command_palette::Overlay,
    navigation::{SelectNextOverlayItem, SelectPreviousOverlayItem},
};

impl Workspace {
    pub(super) fn reset_picker_scroll(&self) {
        self.picker_scroll.scroll_to_top_of_item(0);
    }

    pub(super) fn select_previous_overlay_item(
        &mut self,
        _: &SelectPreviousOverlayItem,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_overlay_selection(-1, cx);
    }

    pub(super) fn select_next_overlay_item(
        &mut self,
        _: &SelectNextOverlayItem,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_overlay_selection(1, cx);
    }

    pub(super) fn move_overlay_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.overlay == Overlay::None {
            if self.move_composer_selection(delta, cx) {
                return;
            }
            if let Some(editor) = self.active_tab().map(|tab| tab.editor.clone()) {
                editor.update(cx, |editor, cx| {
                    editor.move_vertical(delta, cx);
                });
            }
            return;
        }
        if self.overlay == Overlay::Status {
            if self.status_dialog.move_selection(delta) {
                self.picker_scroll
                    .scroll_to_item(self.status_dialog.selected);
                cx.notify();
            }
            return;
        }
        let count = match self.overlay {
            Overlay::Directory => self.directory_suggestions.len(),
            Overlay::Command => self.command_suggestions.len(),
            Overlay::Selection(_) => self.selection_suggestions.len(),
            Overlay::Timeline => self.timeline_suggestions.len(),
            Overlay::MessageActions => 3,
            Overlay::Status => unreachable!("status selection is handled above"),
            Overlay::Debug => 0,
            Overlay::None => return,
        };
        if count == 0 {
            return;
        }
        self.overlay_selection = wrapped_index(self.overlay_selection, delta, count);
        if self.overlay != Overlay::MessageActions {
            self.picker_scroll.scroll_to_item(self.overlay_selection);
        }
        if self.overlay == Overlay::Timeline {
            self.preview_timeline_selection();
        }
        cx.notify();
    }
}

fn wrapped_index(selected: usize, delta: isize, count: usize) -> usize {
    usize::try_from((selected.cast_signed() + delta).rem_euclid(count.cast_signed()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::wrapped_index;

    #[test]
    fn row_indices_wrap_without_chrome_offsets() {
        assert_eq!(wrapped_index(0, -1, 4), 3);
        assert_eq!(wrapped_index(3, 1, 4), 0);
        assert_eq!(wrapped_index(1, 5, 4), 2);
    }
}
