use gpui::{Context, Window};

use super::{
    Workspace,
    command_palette::Overlay,
    navigation::{SelectNextOverlayItem, SelectPreviousOverlayItem},
};

impl Workspace {
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
        let count = match self.overlay {
            Overlay::Directory => self.directory_suggestions.len(),
            Overlay::Command => self.command_suggestions.len(),
            Overlay::Selection(_) => self.selection_suggestions.len(),
            Overlay::Timeline => self.timeline_suggestions.len(),
            Overlay::MessageActions => 3,
            Overlay::None => return,
        };
        if count == 0 {
            return;
        }
        self.overlay_selection = usize::try_from(
            (self.overlay_selection.cast_signed() + delta).rem_euclid(count.cast_signed()),
        )
        .unwrap_or_default();
        let item = self.overlay_selection
            + match self.overlay {
                Overlay::Command | Overlay::Timeline => 1,
                Overlay::Directory
                | Overlay::Selection(_)
                | Overlay::MessageActions
                | Overlay::None => 0,
            };
        self.picker_scroll.scroll_to_item(item);
        if self.overlay == Overlay::Timeline {
            self.preview_timeline_selection();
        }
        cx.notify();
    }
}
