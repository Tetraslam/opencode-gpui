use super::*;

#[gpui::test]
fn inactive_workspace_timeline_can_preload_without_switching(cx: &mut TestAppContext) {
    let workspace = workspace(cx, Vec::new(), TimelineState::Empty);
    workspace.update(cx, |workspace, cx| {
        workspace.tabs[0].directory = "/work/a".into();
        workspace.open_directory("/work/b".into(), cx);
        workspace.active_tab = 0;
        workspace.tabs[1].timeline = TimelineState::Empty;

        workspace.select_session_in("/work/b", "session-b".into(), "background".into(), cx);

        assert_eq!(workspace.active_tab, 0);
        assert_eq!(workspace.tabs[0].directory, "/work/a");
        assert_eq!(workspace.tabs[1].timeline.session_id(), Some("session-b"));
    });
}
