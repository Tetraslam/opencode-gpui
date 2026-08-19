use gpui::{Context, Window};

use super::{Changed, EditorSnapshot, Redo, TextEditor, Undo};

const HISTORY_LIMIT: usize = 100;

impl TextEditor {
    pub(super) fn record_undo(&mut self) {
        if self.undo_stack.len() == HISTORY_LIMIT {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.snapshot());
        self.redo_stack.clear();
    }

    fn snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            content: self.content.clone(),
            selected_range: self.selected_range.clone(),
            selection_reversed: self.selection_reversed,
        }
    }

    fn restore_snapshot(&mut self, snapshot: EditorSnapshot, cx: &mut Context<Self>) {
        self.content = snapshot.content;
        self.selected_range = snapshot.selected_range;
        self.selection_reversed = snapshot.selection_reversed;
        self.marked_range = None;
        cx.emit(Changed);
        cx.notify();
    }

    pub(super) fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = self.undo_stack.pop() else {
            return;
        };
        self.redo_stack.push(self.snapshot());
        self.restore_snapshot(snapshot, cx);
    }

    pub(super) fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = self.redo_stack.pop() else {
            return;
        };
        self.undo_stack.push(self.snapshot());
        self.restore_snapshot(snapshot, cx);
    }
}
