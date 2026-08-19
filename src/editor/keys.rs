use gpui::{App, KeyBinding};

use super::{
    Backspace, Copy, Cut, Delete, End, Home, Left, Paste, Right, SelectAll, SelectLeft,
    SelectRight, SubmitAction,
};

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("TextEditor")),
        KeyBinding::new("delete", Delete, Some("TextEditor")),
        KeyBinding::new("left", Left, Some("TextEditor")),
        KeyBinding::new("right", Right, Some("TextEditor")),
        KeyBinding::new("shift-left", SelectLeft, Some("TextEditor")),
        KeyBinding::new("shift-right", SelectRight, Some("TextEditor")),
        KeyBinding::new("secondary-a", SelectAll, Some("TextEditor")),
        KeyBinding::new("secondary-v", Paste, Some("TextEditor")),
        KeyBinding::new("secondary-c", Copy, Some("TextEditor")),
        KeyBinding::new("secondary-x", Cut, Some("TextEditor")),
        KeyBinding::new("home", Home, Some("TextEditor")),
        KeyBinding::new("end", End, Some("TextEditor")),
        KeyBinding::new("enter", SubmitAction, Some("TextEditor")),
    ]);
}
