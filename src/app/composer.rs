use gpui::{Context, Focusable, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    event::SessionStatus,
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{TimelineState, Workspace};

impl Workspace {
    pub(super) fn render_composer(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(tab) = self.active_tab() else {
            return div().into_any_element();
        };
        let session_id = tab.timeline.session_id().map(str::to_owned);
        let busy = session_id
            .as_ref()
            .and_then(|id| self.statuses.get(id))
            .is_some_and(|status| {
                matches!(status, SessionStatus::Busy | SessionStatus::Retry { .. })
            });
        let error = tab.prompt_error.clone();
        let mode = tab.prompt_mode;
        let editor = tab.editor.clone();
        let images = tab.attached_images.clone();
        let status = session_id
            .as_ref()
            .and_then(|id| self.statuses.get(id))
            .cloned();
        div()
            .flex_none()
            .p_3()
            .bg(rgb(color::BASE))
            .child(
                div()
                    .w_full()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(if error.is_some() {
                        color::RED
                    } else {
                        color::BORDER
                    }))
                    .bg(rgb(color::SURFACE))
                    .children((!images.is_empty()).then(|| Self::render_prompt_images(&images, cx)))
                    .child(
                        div()
                            .min_h(px(ui_size::COMPOSER_PROMPT))
                            .px_2()
                            .flex()
                            .items_start()
                            .gap_2()
                            .font_family(MONO_FONT)
                            .text_sm()
                            .child(activity_cell(status.as_ref(), error.is_some()))
                            .child(
                                div()
                                    .h(px(ui_size::COMPOSER_PROMPT))
                                    .flex()
                                    .items_center()
                                    .text_color(rgb(color::ACCENT))
                                    .child(if mode == super::prompt_mode::PromptMode::Shell {
                                        "$"
                                    } else {
                                        ">"
                                    }),
                            )
                            .child(div().min_w_0().flex_1().child(editor)),
                    )
                    .children(error.clone().map(prompt_error))
                    .child(
                        div()
                            .h(px(ui_size::MESSAGE_HEADER))
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_t_1()
                            .border_color(rgb(color::BORDER))
                            .font_family(MONO_FONT)
                            .text_xs()
                            .text_color(rgb(color::TEXT_DIM))
                            .child(if mode == super::prompt_mode::PromptMode::Shell {
                                "shell mode  |  esc exit".into()
                            } else {
                                self.last_prompt_context()
                            })
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
            .child("enter  send")
            .on_click(cx.listener(|workspace, _, window, cx| {
                let Some((directory, editor)) = workspace
                    .active_tab()
                    .map(|tab| (tab.directory.clone(), tab.editor.clone()))
                else {
                    return;
                };
                let has_images = workspace
                    .active_tab()
                    .is_some_and(|tab| !tab.attached_images.is_empty());
                workspace.capture_active_draft(true, cx);
                let text = editor.update(cx, |editor, cx| {
                    let text = editor.text().trim().to_owned();
                    editor.restore_text("", cx);
                    text
                });
                if let Some(tab) = workspace.active_tab_mut() {
                    tab.composer_completion = None;
                }
                editor.read(cx).focus_handle(cx).focus(window);
                if !text.is_empty() || has_images {
                    workspace.submit_composer_in(&directory, text, cx);
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
            .child("esc  abort")
            .on_click(cx.listener(move |workspace, _, _, cx| {
                workspace.abort_session(session_id.clone(), cx);
            }))
            .into_any_element()
    }

    pub(super) fn abort_session(&mut self, session_id: String, cx: &mut Context<Self>) {
        let Some((directory, client)) = self
            .active_tab()
            .map(|tab| (tab.directory.clone(), tab.client.clone()))
        else {
            return;
        };
        cx.spawn(async move |workspace, cx| {
            let result = client.abort_session(&session_id).await;
            let _ = workspace.update(cx, |workspace, cx| {
                if let Err(error) = result {
                    if let Some(tab) = workspace
                        .tabs
                        .iter_mut()
                        .find(|tab| tab.directory == directory)
                    {
                        tab.prompt_error = Some(error.to_string().into());
                    }
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn last_prompt_context(&self) -> SharedString {
        let Some(tab) = self.active_tab() else {
            return "open a directory to compose".into();
        };
        let TimelineState::Ready { messages, .. } = &tab.timeline else {
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

fn activity_cell(status: Option<&SessionStatus>, failed: bool) -> gpui::AnyElement {
    let (marker, label_color) = if failed {
        ("!", color::RED)
    } else {
        match status {
            Some(SessionStatus::Busy) => ("●", color::GREEN),
            Some(SessionStatus::Retry { .. }) => ("↻", color::YELLOW),
            Some(SessionStatus::Idle) | None => ("·", color::TEXT_MUTED),
        }
    };
    div()
        .w(px(ui_size::KIND_COL))
        .h(px(ui_size::COMPOSER_PROMPT))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .font_family(MONO_FONT)
        .text_color(rgb(label_color))
        .child(marker)
        .into_any_element()
}

fn prompt_error(error: SharedString) -> gpui::AnyElement {
    div()
        .px_3()
        .py_2()
        .border_t_1()
        .border_color(rgb(color::RED))
        .bg(rgb(color::DIFF_REMOVED_BG))
        .font_family(MONO_FONT)
        .text_xs()
        .text_color(rgb(color::RED))
        .child(error)
        .into_any_element()
}
