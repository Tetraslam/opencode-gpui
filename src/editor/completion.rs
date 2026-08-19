use std::ops::Range;

use gpui::{Context, SharedString};

use super::{Changed, TextEditor};

impl TextEditor {
    #[must_use]
    pub fn preserve_on_submit(mut self) -> Self {
        self.clear_on_submit = false;
        self
    }

    pub fn replace_range(&mut self, range: Range<usize>, text: &str, cx: &mut Context<Self>) {
        self.record_undo();
        self.content = SharedString::from(
            self.content[..range.start].to_owned() + text + &self.content[range.end..],
        );
        let cursor = range.start + text.len();
        self.selected_range = cursor..cursor;
        self.marked_range = None;
        cx.emit(Changed);
        cx.notify();
    }

    pub fn restore_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.content = text.into();
        self.selected_range = self.content.len()..self.content.len();
        self.selection_reversed = false;
        self.marked_range = None;
        cx.notify();
    }

    #[must_use]
    pub fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }
}
