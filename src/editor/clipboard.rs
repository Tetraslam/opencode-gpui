use gpui::{ClipboardEntry, Context, EntityInputHandler, Window};

use super::{Newline, Paste, PastedImage, TextEditor};

impl TextEditor {
    pub(super) fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = cx.read_from_clipboard() else {
            return;
        };
        for entry in item.entries() {
            if let ClipboardEntry::Image(image) = entry {
                cx.emit(PastedImage {
                    image: image.clone(),
                });
            }
        }
        if let Some(text) = item.text() {
            self.replace_text_in_range(
                None,
                &text.replace("\r\n", "\n").replace('\r', "\n"),
                window,
                cx,
            );
        }
    }

    pub(super) fn newline(&mut self, _: &Newline, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text_in_range(None, "\n", window, cx);
    }
}
