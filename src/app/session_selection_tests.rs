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

#[gpui::test]
fn cached_target_is_ready_synchronously(cx: &mut TestAppContext) {
    let workspace = workspace(cx, Vec::new(), TimelineState::Empty);
    workspace.update(cx, |workspace, cx| {
        let cached = vec![message_record("cached", "target", 1)];
        workspace.timeline_cache.replace("target", &cached);
        workspace.interrupt_session = Some("other".into());

        workspace.select_session_in("/workspace", "target".into(), "cached".into(), cx);

        let TimelineState::Ready {
            session_id,
            messages,
            ..
        } = &workspace.tabs[0].timeline
        else {
            panic!("cached selection must not enter loading");
        };
        assert_eq!(session_id, "target");
        assert_eq!(messages, &cached);
        assert!(workspace.interrupt_session.is_none());
        assert!(workspace.tabs[0].timeline_load.is_some());
        assert!(workspace.trace_entrances.is_empty());
    });
}

#[gpui::test]
fn disabled_trace_motion_does_not_record_entrances(cx: &mut TestAppContext) {
    let workspace = workspace(cx, Vec::new(), TimelineState::Empty);
    workspace.update(cx, |workspace, cx| {
        workspace.settings.animate_trace_entries = false;
        let record = message_record("message", "session", 1);
        workspace.mark_part_entrances(&record.parts, cx);
        assert!(workspace.trace_entrances.is_empty());
    });
}

#[gpui::test]
fn cached_busy_and_idle_sessions_use_the_same_ready_path(cx: &mut TestAppContext) {
    let workspace = workspace(cx, Vec::new(), TimelineState::Empty);
    workspace.update(cx, |workspace, cx| {
        for session_id in ["busy", "idle"] {
            workspace
                .timeline_cache
                .replace(session_id, &[message_record(session_id, session_id, 1)]);
        }
        Arc::make_mut(&mut workspace.statuses).insert("busy".into(), SessionStatus::Busy);
        Arc::make_mut(&mut workspace.statuses).insert("idle".into(), SessionStatus::Idle);

        workspace.select_session_in("/workspace", "busy".into(), "busy".into(), cx);
        assert!(matches!(
            workspace.tabs[0].timeline,
            TimelineState::Ready { .. }
        ));
        workspace.select_session_in("/workspace", "idle".into(), "idle".into(), cx);
        assert!(matches!(
            workspace.tabs[0].timeline,
            TimelineState::Ready { .. }
        ));
    });
}

#[gpui::test]
fn optimistic_message_survives_an_immediate_switch(cx: &mut TestAppContext) {
    let workspace = workspace(
        cx,
        Vec::new(),
        TimelineState::Ready {
            session_id: "active".into(),
            title: "active".into(),
            messages: Vec::new(),
        },
    );
    workspace.update(cx, |workspace, cx| {
        workspace
            .timeline_cache
            .replace("other", &[message_record("other", "other", 1)]);
        workspace.submit_prompt_in("/workspace", "keep me".into(), cx);
        workspace.select_session_in("/workspace", "other".into(), "other".into(), cx);
        workspace.select_session_in("/workspace", "active".into(), "active".into(), cx);

        let TimelineState::Ready { messages, .. } = &workspace.tabs[0].timeline else {
            panic!("optimistic session should select from cache");
        };
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].parts[0].text(), Some("keep me"));
    });
}
