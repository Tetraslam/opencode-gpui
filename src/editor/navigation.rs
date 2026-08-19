use gpui::{Context, Window};
use unicode_segmentation::UnicodeSegmentation;

use super::{
    DocumentEnd, DocumentHome, End, Home, Left, Right, SelectAll, SelectEnd, SelectHome,
    SelectLeft, SelectRight, TextEditor,
};

impl TextEditor {
    pub fn move_vertical(&mut self, delta: isize, cx: &mut Context<Self>) -> bool {
        let Some(layout) = &self.last_layout else {
            return false;
        };
        let position = layout.position_for_offset(self.cursor_offset());
        let distance = layout.line_height * delta.unsigned_abs();
        let target_y = if delta < 0 {
            position.y.max(distance) - distance
        } else {
            position.y + distance
        };
        let scroll_y = layout.line_height * layout.scroll_row;
        let offset = layout.closest_index(gpui::point(position.x, target_y - scroll_y));
        self.move_to(offset, cx);
        true
    }

    pub(super) fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        let offset = if self.selected_range.is_empty() {
            self.previous_boundary(self.cursor_offset())
        } else {
            self.selected_range.start
        };
        self.move_to(offset, cx);
    }

    pub(super) fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        let offset = if self.selected_range.is_empty() {
            self.next_boundary(self.selected_range.end)
        } else {
            self.selected_range.end
        };
        self.move_to(offset, cx);
    }

    pub(super) fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    pub(super) fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    pub(super) fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx);
    }

    pub(super) fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        let cursor = self.cursor_offset();
        let offset = self.content[..cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        self.move_to(offset, cx);
    }

    pub(super) fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        let cursor = self.cursor_offset();
        let offset = self.content[cursor..]
            .find('\n')
            .map_or(self.content.len(), |index| cursor + index);
        self.move_to(offset, cx);
    }

    pub(super) fn document_home(
        &mut self,
        _: &DocumentHome,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_to(0, cx);
    }

    pub(super) fn document_end(&mut self, _: &DocumentEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    pub(super) fn select_home(&mut self, _: &SelectHome, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(0, cx);
    }

    pub(super) fn select_end(&mut self, _: &SelectEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.content.len(), cx);
    }

    pub(super) fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(index, _)| (index < offset).then_some(index))
            .unwrap_or(0)
    }

    pub(super) fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(index, _)| (index > offset).then_some(index))
            .unwrap_or(self.content.len())
    }
}
