use super::*;

#[gpui::test]
fn drafts_are_scoped_to_sessions(cx: &mut TestAppContext) {
    let workspace = workspace(
        cx,
        vec![session("one", 2), session("two", 1)],
        TimelineState::Ready {
            session_id: "one".into(),
            title: "one".into(),
            messages: Vec::new(),
        },
    );
    workspace.update(cx, |workspace, cx| {
        workspace.tabs[0]
            .editor
            .update(cx, |editor, cx| editor.restore_text("hello", cx));
        workspace.composer_changed("/workspace", cx);

        workspace.select_session("two".into(), "two".into(), cx);
        assert_eq!(workspace.tabs[0].editor.read(cx).text(), "");
        assert_eq!(workspace.overlay, Overlay::None);

        workspace.tabs[0]
            .editor
            .update(cx, |editor, cx| editor.restore_text("world", cx));
        workspace.composer_changed("/workspace", cx);
        workspace.select_session("one".into(), "one".into(), cx);

        assert_eq!(workspace.tabs[0].editor.read(cx).text(), "hello");
    });
}
