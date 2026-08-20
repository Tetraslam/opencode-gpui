use gpui::{AppContext, TestAppContext, point, px};

use super::TextEditor;

#[gpui::test]
fn placeholder_click_never_indexes_empty_content(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| TextEditor::new("placeholder", cx));
    editor.update(cx, |editor, _| {
        assert_eq!(
            editor.index_for_mouse_position(point(px(80.0), px(10.0))),
            0
        );
    });
}

#[gpui::test]
fn cursor_navigation_uses_grapheme_boundaries(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| TextEditor::new("placeholder", cx));
    editor.update(cx, |editor, _| {
        editor.content = "a👨‍👩‍👧b".into();
        assert_eq!(editor.next_boundary(1), 19);
        assert_eq!(editor.previous_boundary(19), 1);
    });
}

#[gpui::test]
fn word_navigation_skips_whitespace_and_preserves_unicode(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| TextEditor::new("placeholder", cx));
    editor.update(cx, |editor, _| {
        editor.content = "alpha  βeta.gamma".into();
        assert_eq!(editor.next_word_boundary(0), 7);
        assert_eq!(editor.previous_word_boundary(12), 7);
        assert_eq!(editor.next_word_boundary(11), 12);
    });
}

#[gpui::test]
fn explicit_newlines_grow_layout_without_waiting_for_paint(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| TextEditor::new("placeholder", cx));
    editor.update(cx, |editor, _| {
        editor.content = "one\ntwo\nthree".into();
        assert_eq!(editor.explicit_lines(), 3);
    });
}
