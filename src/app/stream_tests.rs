use super::*;

#[gpui::test]
fn streamed_deltas_append_when_events_do_not_include_full_text(cx: &mut TestAppContext) {
    let message: Message = serde_json::from_str(
        r#"{"id":"msg_1","sessionID":"ses_1","role":"user","time":{"created":1},"agent":"build","model":{"providerID":"openai","modelID":"test"}}"#,
    )
    .unwrap();
    let first: Part = serde_json::from_str(
        r#"{"id":"part_1","sessionID":"ses_1","messageID":"msg_1","type":"text","text":"hel"}"#,
    )
    .unwrap();
    let workspace = workspace(
        cx,
        Vec::new(),
        TimelineState::Ready {
            session_id: "ses_1".into(),
            title: "session".into(),
            messages: Vec::new(),
        },
    );
    workspace.update(cx, |workspace, _| {
        workspace.apply_events(
            vec![
                Event::MessageUpdated(message),
                Event::MessagePartUpdated {
                    part: first,
                    delta: Some("hel".into()),
                },
                Event::MessagePartDelta {
                    session_id: "ses_1".into(),
                    message_id: "msg_1".into(),
                    part_id: "part_1".into(),
                    field: "text".into(),
                    delta: "lo".into(),
                },
            ],
            None,
        );
        let TimelineState::Ready { messages, .. } = &workspace.tabs[0].timeline else {
            panic!("timeline should remain loaded");
        };
        assert_eq!(messages[0].parts[0].text(), Some("hello"));
    });
}

#[gpui::test]
fn preserves_deltas_that_arrive_before_their_part_and_message(cx: &mut TestAppContext) {
    let message: Message = serde_json::from_str(
        r#"{"id":"msg_1","sessionID":"ses_1","role":"assistant","time":{"created":1},"parentID":"msg_0","modelID":"test","providerID":"openai","mode":"build","agent":"build","path":{"cwd":"/workspace","root":"/workspace"},"cost":0,"tokens":{"input":0,"output":0,"reasoning":0,"cache":{"read":0,"write":0}}}"#,
    )
    .unwrap();
    let part: Part = serde_json::from_str(
        r#"{"id":"part_1","sessionID":"ses_1","messageID":"msg_1","type":"text","text":""}"#,
    )
    .unwrap();
    let workspace = workspace(
        cx,
        Vec::new(),
        TimelineState::Ready {
            session_id: "ses_1".into(),
            title: "session".into(),
            messages: Vec::new(),
        },
    );
    workspace.update(cx, |workspace, _| {
        workspace.apply_events(
            vec![
                Event::MessagePartDelta {
                    session_id: "ses_1".into(),
                    message_id: "msg_1".into(),
                    part_id: "part_1".into(),
                    field: "text".into(),
                    delta: "hello".into(),
                },
                Event::MessagePartUpdated { part, delta: None },
                Event::MessageUpdated(message),
            ],
            None,
        );
        let TimelineState::Ready { messages, .. } = &workspace.tabs[0].timeline else {
            panic!("timeline should remain loaded");
        };
        assert_eq!(messages[0].parts[0].text(), Some("hello"));
    });
}

#[gpui::test]
fn unselected_cached_session_keeps_events_isolated(cx: &mut TestAppContext) {
    let selected = message_record("selected", "selected", 1);
    let workspace = workspace(
        cx,
        Vec::new(),
        TimelineState::Ready {
            session_id: "selected".into(),
            title: "selected".into(),
            messages: vec![selected.clone()],
        },
    );
    workspace.update(cx, |workspace, _| {
        workspace.timeline_cache.replace("cached", &[]);
        let record = message_record("streamed", "cached", 2);
        let part = record.parts[0].clone();
        workspace.apply_events(
            vec![
                Event::MessagePartDelta {
                    session_id: "cached".into(),
                    message_id: "streamed".into(),
                    part_id: part.id.clone(),
                    field: "text".into(),
                    delta: " live".into(),
                },
                Event::MessagePartUpdated { part, delta: None },
                Event::MessageUpdated(record.info),
            ],
            None,
        );

        let TimelineState::Ready { messages, .. } = &workspace.tabs[0].timeline else {
            panic!("selected timeline should remain ready");
        };
        assert_eq!(messages, &[selected]);
        let cached = workspace.timeline_cache.get("cached").unwrap();
        assert_eq!(cached[0].parts[0].text(), Some("message 2 live"));

        workspace.apply_events(
            vec![Event::MessagePartRemoved {
                session_id: "cached".into(),
                message_id: "streamed".into(),
                part_id: "part-streamed".into(),
            }],
            None,
        );
        assert!(
            workspace.timeline_cache.get("cached").unwrap()[0]
                .parts
                .is_empty()
        );
        workspace.apply_events(
            vec![Event::MessageRemoved {
                session_id: "cached".into(),
                message_id: "streamed".into(),
            }],
            None,
        );
        assert!(workspace.timeline_cache.get("cached").unwrap().is_empty());
    });
}

#[gpui::test]
fn session_cache_keeps_only_latest_message_page(cx: &mut TestAppContext) {
    let workspace = workspace(cx, Vec::new(), TimelineState::Empty);
    workspace.update(cx, |workspace, _| {
        workspace.timeline_cache.replace("cached", &[]);
        for index in 0..(super::super::MESSAGE_PAGE + 4) {
            workspace.apply_events(
                vec![Event::MessageUpdated(
                    message_record(&format!("message-{index}"), "cached", index as u64).info,
                )],
                None,
            );
        }
        let cached = workspace.timeline_cache.get("cached").unwrap();
        assert_eq!(cached.len(), super::super::MESSAGE_PAGE);
        assert_eq!(cached[0].info.id(), "message-4");
        assert_eq!(
            cached[super::super::MESSAGE_PAGE - 1].info.id(),
            "message-19"
        );
    });
}
