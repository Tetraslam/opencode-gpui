use gpui::{App, KeyBinding};

use super::{
    Backspace, Copy, Cut, Delete, DeleteWordBackward, DeleteWordForward, DocumentEnd, DocumentHome,
    End, Home, Left, Newline, Paste, Redo, Right, SelectAll, SelectEnd, SelectHome, SelectLeft,
    SelectRight, SelectWordLeft, SelectWordRight, SubmitAction, Undo, WordLeft, WordRight,
};

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("TextEditor")),
        KeyBinding::new("delete", Delete, Some("TextEditor")),
        KeyBinding::new("left", Left, Some("TextEditor")),
        KeyBinding::new("right", Right, Some("TextEditor")),
        KeyBinding::new("shift-left", SelectLeft, Some("TextEditor")),
        KeyBinding::new("shift-right", SelectRight, Some("TextEditor")),
        KeyBinding::new("ctrl-left", WordLeft, Some("TextEditor")),
        KeyBinding::new("ctrl-right", WordRight, Some("TextEditor")),
        KeyBinding::new("ctrl-shift-left", SelectWordLeft, Some("TextEditor")),
        KeyBinding::new("ctrl-shift-right", SelectWordRight, Some("TextEditor")),
        KeyBinding::new("ctrl-backspace", DeleteWordBackward, Some("TextEditor")),
        KeyBinding::new("ctrl-delete", DeleteWordForward, Some("TextEditor")),
        KeyBinding::new("secondary-a", SelectAll, Some("TextEditor")),
        KeyBinding::new("secondary-v", Paste, Some("TextEditor")),
        KeyBinding::new("secondary-c", Copy, Some("TextEditor")),
        KeyBinding::new("secondary-x", Cut, Some("TextEditor")),
        KeyBinding::new("home", Home, Some("TextEditor")),
        KeyBinding::new("end", End, Some("TextEditor")),
        KeyBinding::new("ctrl-home", DocumentHome, Some("TextEditor")),
        KeyBinding::new("ctrl-end", DocumentEnd, Some("TextEditor")),
        KeyBinding::new("shift-home", SelectHome, Some("TextEditor")),
        KeyBinding::new("shift-end", SelectEnd, Some("TextEditor")),
        KeyBinding::new("ctrl-shift-home", SelectHome, Some("TextEditor")),
        KeyBinding::new("ctrl-shift-end", SelectEnd, Some("TextEditor")),
        KeyBinding::new("secondary-z", Undo, Some("TextEditor")),
        KeyBinding::new("secondary-shift-z", Redo, Some("TextEditor")),
        KeyBinding::new("ctrl-y", Redo, Some("TextEditor")),
        KeyBinding::new("shift-enter", Newline, Some("TextEditor")),
        KeyBinding::new("ctrl-enter", Newline, Some("TextEditor")),
        KeyBinding::new("alt-enter", Newline, Some("TextEditor")),
        KeyBinding::new("ctrl-j", Newline, Some("TextEditor")),
        KeyBinding::new("enter", SubmitAction, Some("TextEditor")),
    ]);
}
