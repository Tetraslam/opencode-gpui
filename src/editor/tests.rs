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
