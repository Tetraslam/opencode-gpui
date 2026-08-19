use super::*;

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
