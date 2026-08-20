use super::*;

#[gpui::test]
fn empty_shell_submission_with_an_attachment_reports_the_rejection(cx: &mut TestAppContext) {
    let workspace = workspace(
        cx,
        vec![session("one", 1)],
        TimelineState::Ready {
            session_id: "one".into(),
            title: "one".into(),
            messages: Vec::new(),
        },
    );
    workspace.update(cx, |workspace, cx| {
        workspace.tabs[0].prompt_mode = super::super::prompt_mode::PromptMode::Shell;
        workspace.tabs[0]
            .attached_files
            .insert("src/main.rs".into());

        workspace.submit_shell_in("/workspace", String::new(), cx);

        assert_eq!(
            workspace.tabs[0]
                .prompt_error
                .as_ref()
                .map(std::convert::AsRef::as_ref),
            Some("shell mode does not accept attachments")
        );
    });
}
