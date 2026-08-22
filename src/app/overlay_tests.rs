use super::*;

#[gpui::test]
fn idle_status_events_clear_only_the_matching_interrupt_arm(cx: &mut TestAppContext) {
    let workspace = workspace(cx, Vec::new(), TimelineState::Empty);
    workspace.update(cx, |workspace, _| {
        workspace.interrupt_session = Some("active".into());
        workspace.apply_events(
            vec![Event::SessionStatus {
                session_id: "other".into(),
                status: SessionStatus::Idle,
            }],
            None,
        );
        assert_eq!(workspace.interrupt_session.as_deref(), Some("active"));

        workspace.apply_events(
            vec![Event::SessionIdle {
                session_id: "active".into(),
            }],
            None,
        );
        assert!(workspace.interrupt_session.is_none());
    });
}

#[gpui::test]
fn overlay_selection_wraps_for_keyboard_navigation(cx: &mut TestAppContext) {
    let workspace = workspace(
        cx,
        vec![session_in("a", "/work/a", 1), session_in("b", "/work/b", 2)],
        TimelineState::Empty,
    );
    workspace.update(cx, |workspace, cx| {
        workspace.overlay = Overlay::Directory;
        workspace.directory_suggestions = Arc::new(workspace.known_directories());
        workspace.move_overlay_selection(-1, cx);
        assert_eq!(
            workspace.overlay_selection,
            workspace.known_directories().len() - 1
        );
        workspace.move_overlay_selection(1, cx);
        assert_eq!(workspace.overlay_selection, 0);

        workspace.overlay = Overlay::Command;
        workspace.refresh_command_suggestions("");
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
        workspace.refresh_command_suggestions("");
        workspace.overlay_selection = 2;
        workspace.execute_command_palette("", cx);
        assert!(workspace.sessions_open);
        assert_eq!(workspace.overlay, Overlay::None);
    });
}

#[gpui::test]
fn slash_and_palette_share_timeline_identity(cx: &mut TestAppContext) {
    let workspace = workspace(
        cx,
        Vec::new(),
        TimelineState::Ready {
            session_id: "session".into(),
            title: "title".into(),
            messages: Vec::new(),
        },
    );
    workspace.update(cx, |workspace, _| {
        let slash = super::super::composer_slashes::local_slash("timeline").unwrap();
        assert_eq!(slash, super::super::workspace_command::Command::Timeline);
        assert!(workspace.filtered_commands("").contains(&slash));
        assert!(
            !workspace
                .filtered_commands("")
                .contains(&super::super::workspace_command::Command::ShowCommandPalette)
        );
        let completion = super::super::composer_slashes::local_slashes("timeline");
        assert!(matches!(
            completion.as_slice(),
            [super::super::composer_completion::CompletionItem::Local { action, .. }]
                if *action == slash
        ));
    });
}
