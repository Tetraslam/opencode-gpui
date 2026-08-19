use gpui::{Context, EntityInputHandler, Window};
use unicode_segmentation::UnicodeSegmentation;

use super::{
    DeleteWordBackward, DeleteWordForward, SelectWordLeft, SelectWordRight, TextEditor, WordLeft,
    WordRight,
};

impl TextEditor {
    pub(super) fn word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        let offset = if self.selected_range.is_empty() {
            self.previous_word_boundary(self.cursor_offset())
        } else {
            self.selected_range.start
        };
        self.move_to(offset, cx);
    }

    pub(super) fn word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        let offset = if self.selected_range.is_empty() {
            self.next_word_boundary(self.cursor_offset())
        } else {
            self.selected_range.end
        };
        self.move_to(offset, cx);
    }

    pub(super) fn select_word_left(
        &mut self,
        _: &SelectWordLeft,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(self.previous_word_boundary(self.cursor_offset()), cx);
    }

    pub(super) fn select_word_right(
        &mut self,
        _: &SelectWordRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(self.next_word_boundary(self.cursor_offset()), cx);
    }

    pub(super) fn delete_word_backward(
        &mut self,
        _: &DeleteWordBackward,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_word_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    pub(super) fn delete_word_forward(
        &mut self,
        _: &DeleteWordForward,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_word_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    pub(super) fn previous_word_boundary(&self, offset: usize) -> usize {
        let graphemes = self.content[..offset]
            .grapheme_indices(true)
            .collect::<Vec<_>>();
        let mut index = graphemes.len();
        while index > 0 && graphemes[index - 1].1.chars().all(char::is_whitespace) {
            index -= 1;
        }
        let Some((_, sample)) = graphemes.get(index.saturating_sub(1)) else {
            return 0;
        };
        let word = is_word(sample);
        while index > 0 && is_word(graphemes[index - 1].1) == word {
            index -= 1;
        }
        graphemes.get(index).map_or(0, |(offset, _)| *offset)
    }

    pub(super) fn next_word_boundary(&self, offset: usize) -> usize {
        let graphemes = self.content[offset..]
            .grapheme_indices(true)
            .collect::<Vec<_>>();
        let mut index = 0;
        if graphemes
            .first()
            .is_some_and(|(_, value)| value.chars().all(char::is_whitespace))
        {
            while index < graphemes.len() && graphemes[index].1.chars().all(char::is_whitespace) {
                index += 1;
            }
            return graphemes
                .get(index)
                .map_or(self.content.len(), |(next, _)| offset + next);
        }
        let Some((_, sample)) = graphemes.first() else {
            return self.content.len();
        };
        let word = is_word(sample);
        while index < graphemes.len() && is_word(graphemes[index].1) == word {
            index += 1;
        }
        while index < graphemes.len() && graphemes[index].1.chars().all(char::is_whitespace) {
            index += 1;
        }
        graphemes
            .get(index)
            .map_or(self.content.len(), |(next, _)| offset + next)
    }
}

fn is_word(grapheme: &str) -> bool {
    grapheme
        .chars()
        .all(|character| character.is_alphanumeric() || character == '_')
}
