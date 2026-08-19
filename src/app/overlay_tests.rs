use super::*;

#[gpui::test]
fn overlay_selection_wraps_for_keyboard_navigation(cx: &mut TestAppContext) {
    let workspace = workspace(
        cx,
        vec![session_in("a", "/work/a", 1), session_in("b", "/work/b", 2)],
        TimelineState::Empty,
    );
    workspace.update(cx, |workspace, cx| {
        workspace.overlay = Overlay::Directory;
        workspace.move_overlay_selection(-1, cx);
        assert_eq!(
            workspace.overlay_selection,
            workspace.known_directories().len() - 1
        );
        workspace.move_overlay_selection(1, cx);
        assert_eq!(workspace.overlay_selection, 0);

        workspace.overlay = Overlay::Command;
        workspace.move_overlay_selection(-1, cx);
        assert_eq!(
            workspace.overlay_selection,
            workspace.filtered_commands("").len() - 1
        );
    });
}

#[gpui::test]
fn empty_query_activates_the_keyboard_selected_command(cx: &mut TestAppContext) {
    let workspace = workspace(cx, Vec::new(), TimelineState::Empty);
    workspace.update(cx, |workspace, cx| {
        workspace.overlay = Overlay::Command;
        workspace.overlay_selection = 2;
        workspace.execute_command_palette("", cx);
        assert!(workspace.sessions_open);
        assert_eq!(workspace.overlay, Overlay::None);
    });
}
