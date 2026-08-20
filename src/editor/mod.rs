mod clipboard;
mod completion;
mod element;
mod history;
mod input_handler;
mod keys;
mod layout;
mod navigation;
mod word;

#[cfg(test)]
mod tests;

pub use keys::init;

use std::ops::Range;

use crate::theme::{MONO_FONT, color};
use gpui::{
    App, ClipboardItem, Context, CursorStyle, EntityInputHandler, EventEmitter, FocusHandle,
    Focusable, Image, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    SharedString, Window, actions, div, prelude::*, px, rgb,
};

use element::TextElement;

const MAX_VISIBLE_LINES: usize = 8;
const VERTICAL_PADDING: Pixels = px(3.0);

actions!(
    opencode_editor,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        WordLeft,
        WordRight,
        SelectWordLeft,
        SelectWordRight,
        DeleteWordBackward,
        DeleteWordForward,
        SelectHome,
        SelectEnd,
        Undo,
        Redo,
        SelectAll,
        Home,
        End,
        DocumentHome,
        DocumentEnd,
        Paste,
        Cut,
        Copy,
        Newline,
        SubmitAction,
    ]
);

#[derive(Clone, Debug)]
pub struct Submit {
    pub text: String,
}

#[derive(Clone, Copy, Debug)]
pub struct Changed;

#[derive(Clone, Debug)]
pub struct PastedImage {
    pub image: Image,
}

pub struct TextEditor {
    pub(super) focus_handle: FocusHandle,
    pub(super) content: SharedString,
    pub(super) placeholder: SharedString,
    pub(super) selected_range: Range<usize>,
    pub(super) selection_reversed: bool,
    pub(super) marked_range: Option<Range<usize>>,
    pub(super) last_layout: Option<layout::EditorLayout>,
    pub(super) last_bounds: Option<gpui::Bounds<Pixels>>,
    pub(super) visible_lines: usize,
    pub(super) is_selecting: bool,
    clear_on_submit: bool,
    undo_stack: Vec<EditorSnapshot>,
    redo_stack: Vec<EditorSnapshot>,
}

#[derive(Clone)]
pub(super) struct EditorSnapshot {
    content: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
}

impl EventEmitter<Submit> for TextEditor {}
impl EventEmitter<Changed> for TextEditor {}
impl EventEmitter<PastedImage> for TextEditor {}

impl TextEditor {
    #[must_use]
    pub fn new(placeholder: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            placeholder: placeholder.into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            visible_lines: 1,
            is_selecting: false,
            clear_on_submit: true,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.content
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.content = "".into();
        self.selected_range = 0..0;
        self.marked_range = None;
        cx.emit(Changed);
        cx.notify();
    }

    fn submit(&mut self, _: &SubmitAction, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.content.to_string();
        if self.clear_on_submit {
            self.clear(cx);
        }
        cx.emit(Submit { text });
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        self.copy(&Copy, window, cx);
        if !self.selected_range.is_empty() {
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.is_selecting = true;
        let offset = self.index_for_mouse_position(event.position);
        if event.modifiers.shift {
            self.select_to(offset, cx);
        } else {
            self.move_to(offset, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    pub(super) fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        let (Some(bounds), Some(layout)) = (&self.last_bounds, &self.last_layout) else {
            return 0;
        };
        if position.y < bounds.top() {
            0
        } else if position.y > bounds.bottom() {
            self.content.len()
        } else {
            layout.closest_index(position - bounds.origin)
        }
    }
}

impl gpui::Render for TextEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("TextEditor")
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::word_left))
            .on_action(cx.listener(Self::word_right))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::delete_word_backward))
            .on_action(cx.listener(Self::delete_word_forward))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::document_home))
            .on_action(cx.listener(Self::document_end))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::submit))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .min_h(px(34.0))
            .w_full()
            .px_2()
            .flex()
            .items_center()
            .bg(rgb(color::BASE))
            .font_family(MONO_FONT)
            .text_size(px(13.0))
            .text_color(rgb(color::TEXT))
            .child(TextElement { input: cx.entity() })
    }
}

impl Focusable for TextEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
