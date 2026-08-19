use gpui::{Context, Focusable, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    api::Prompt,
    event::SessionStatus,
    theme::{MONO_FONT, color},
};

use super::{TimelineState, Workspace};

impl Workspace {
    pub(super) fn submit_prompt(&mut self, text: String, cx: &mut Context<Self>) {
        let (Some(client), Some(session_id)) = (
            self.client.clone(),
            self.timeline.session_id().map(str::to_owned),
        ) else {
            self.prompt_error = Some("No active session".into());
            cx.notify();
            return;
        };
        self.prompt_error = None;
        cx.spawn(async move |workspace, cx| {
            let result = client
                .prompt(
                    &session_id,
                    Prompt {
                        text,
                        model: None,
                        agent: None,
                    },
                )
                .await;
            let _ = workspace.update(cx, |workspace, cx| {
                if let Err(error) = result {
                    workspace.prompt_error = Some(error.to_string().into());
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(super) fn render_composer(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let session_id = self.timeline.session_id().map(str::to_owned);
        let busy = session_id
            .as_ref()
            .and_then(|id| self.statuses.get(id))
            .is_some_and(|status| {
                matches!(status, SessionStatus::Busy | SessionStatus::Retry { .. })
            });
        let error = self.prompt_error.clone();

        div()
            .flex_none()
            .px_3()
            .pb_2()
            .bg(rgb(color::BASE))
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(if error.is_some() {
                        color::RED
                    } else {
                        color::BORDER
                    }))
                    .bg(rgb(color::SURFACE))
                    .child(
                        div()
                            .h(px(36.0))
                            .px_2()
                            .flex()
                            .items_center()
                            .gap_2()
                            .font_family(MONO_FONT)
                            .text_sm()
                            .child(div().text_color(rgb(color::ACCENT)).child(">"))
                            .child(div().min_w_0().flex_1().child(self.editor.clone())),
                    )
                    .child(
                        div()
                            .h(px(25.0))
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_t_1()
                            .border_color(rgb(color::BORDER))
                            .font_family(MONO_FONT)
                            .text_xs()
                            .text_color(rgb(color::TEXT_DIM))
                            .child(error.unwrap_or_else(|| self.last_prompt_context()))
                            .child(if busy {
                                Self::abort_button(session_id.expect("busy session has an id"), cx)
                            } else {
                                Self::send_button(cx)
                            }),
                    ),
            )
            .into_any_element()
    }

    fn send_button(cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .id("send-prompt")
            .cursor_pointer()
            .text_color(rgb(color::ACCENT))
            .hover(|button| button.text_color(rgb(color::TEXT_BRIGHT)))
            .child("ENTER  SEND")
            .on_click(cx.listener(|workspace, _, window, cx| {
                let editor = workspace.editor.clone();
                let text = editor.update(cx, |editor, cx| {
                    let text = editor.text().trim().to_owned();
                    editor.clear(cx);
                    text
                });
                editor.read(cx).focus_handle(cx).focus(window);
                if !text.is_empty() {
                    workspace.submit_prompt(text, cx);
                }
            }))
            .into_any_element()
    }

    fn abort_button(session_id: String, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .id("abort-prompt")
            .cursor_pointer()
            .text_color(rgb(color::RED))
            .hover(|button| button.text_color(rgb(color::TEXT_BRIGHT)))
            .child("ESC  ABORT")
            .on_click(cx.listener(move |workspace, _, _, cx| {
                workspace.abort_session(session_id.clone(), cx);
            }))
            .into_any_element()
    }

    fn abort_session(&mut self, session_id: String, cx: &mut Context<Self>) {
        let Some(client) = self.client.clone() else {
            return;
        };
        cx.spawn(async move |workspace, cx| {
            let result = client.abort_session(&session_id).await;
            let _ = workspace.update(cx, |workspace, cx| {
                if let Err(error) = result {
                    workspace.prompt_error = Some(error.to_string().into());
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn last_prompt_context(&self) -> SharedString {
        let TimelineState::Ready { messages, .. } = &self.timeline else {
            return "select a session to compose".into();
        };
        messages
            .iter()
            .rev()
            .find_map(|message| match &message.info {
                opencode_gpui::model::Message::User(message) => Some(format!(
                    "{}  |  {}/{}",
                    message.agent, message.model.provider_id, message.model.model_id
                )),
                opencode_gpui::model::Message::Assistant(_) => None,
            })
            .unwrap_or_else(|| "server default agent and model".into())
            .into()
    }
}
