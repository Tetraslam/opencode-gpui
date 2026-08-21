use gpui::{
    ClickEvent, Context, Focusable, MouseButton, SharedString, Window, div, prelude::*, px, rgb,
};
use opencode_gpui::{
    editor::TextEditor,
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{
    Workspace,
    navigation::{DismissOverlay, ToggleCommandPalette},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Overlay {
    #[default]
    None,
    Directory,
    Command,
    Selection(super::selection_overlay::SelectionKind),
    Timeline,
    MessageActions,
}

impl Workspace {
    pub(super) fn toggle_command_palette(
        &mut self,
        _: &ToggleCommandPalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.overlay = if self.overlay == Overlay::Command {
            Overlay::None
        } else {
            Overlay::Command
        };
        if self.overlay == Overlay::Command {
            self.overlay_selection = 0;
            self.command_editor.update(cx, TextEditor::clear);
            self.refresh_command_suggestions("");
            self.command_editor.read(cx).focus_handle(cx).focus(window);
        } else {
            self.focus_active_editor(window, cx);
        }
        cx.notify();
    }

    pub(super) fn dismiss_overlay(
        &mut self,
        _: &DismissOverlay,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.overlay == Overlay::None {
            let busy_session = self.active_tab().and_then(|tab| {
                let session_id = tab.timeline.session_id()?;
                self.statuses
                    .get(session_id)
                    .is_some_and(|status| {
                        matches!(
                            status,
                            opencode_gpui::event::SessionStatus::Busy
                                | opencode_gpui::event::SessionStatus::Retry { .. }
                        )
                    })
                    .then(|| session_id.to_owned())
            });
            if let Some(session_id) = busy_session {
                self.abort_session(session_id, cx);
                return;
            }
            if let Some(tab) = self.active_tab_mut()
                && tab.composer_completion.take().is_some()
            {
                cx.notify();
                return;
            }
            if let Some(tab) = self.active_tab_mut()
                && tab.prompt_mode == super::prompt_mode::PromptMode::Shell
            {
                tab.prompt_mode = super::prompt_mode::PromptMode::Normal;
                cx.notify();
                return;
            }
            if let Some(tab) = self.active_tab_mut()
                && let Some(selection) = tab.selected_part.take()
            {
                tab.expanded_parts.remove(&selection);
                tab.collapsed_parts.insert(selection);
                cx.notify();
            }
            return;
        }
        if self.overlay == Overlay::MessageActions {
            self.overlay = Overlay::Timeline;
            self.overlay_selection = self
                .timeline_suggestions
                .iter()
                .position(|entry| {
                    Some(entry.message_id.as_str()) == self.timeline_message.as_deref()
                })
                .unwrap_or_default();
            self.command_editor.read(cx).focus_handle(cx).focus(window);
            cx.notify();
            return;
        }
        self.overlay = Overlay::None;
        self.focus_active_editor(window, cx);
        cx.notify();
    }

    pub(super) fn execute_command_palette(&mut self, _query: &str, cx: &mut Context<Self>) {
        if let Some(command) = self
            .command_suggestions
            .get(self.overlay_selection)
            .copied()
        {
            self.execute_command(command, cx);
        }
    }

    pub(super) fn render_command_palette(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        (self.overlay == Overlay::Command).then(|| {
            let commands = &self.command_suggestions;
            div()
                .absolute()
                .top(px(ui_size::TITLEBAR + 16.0))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .id("command-palette-list")
                        .w(px(520.0))
                        .max_h(px(460.0))
                        .flex()
                        .flex_col()
                        .bg(rgb(color::ELEVATED))
                        .border_1()
                        .border_color(rgb(color::BORDER))
                        .shadow_lg()
                        .font_family(MONO_FONT)
                        .text_color(rgb(color::TEXT))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_, _, _, cx| {
                                cx.stop_propagation();
                            }),
                        )
                        .child(
                            div()
                                .p_2()
                                .border_b_1()
                                .border_color(rgb(color::BORDER))
                                .child(self.command_editor.clone()),
                        )
                        .child(
                            div()
                                .id("command-palette-results")
                                .min_h_0()
                                .flex_1()
                                .overflow_y_scroll()
                                .track_scroll(&self.picker_scroll)
                                .children(commands.iter().copied().enumerate().map(
                                    |(index, command)| {
                                        command_row(command, index == self.overlay_selection, cx)
                                    },
                                )),
                        ),
                )
                .into_any_element()
        })
    }
}

fn command_row(
    command: super::workspace_command::Command,
    selected: bool,
    cx: &mut Context<Workspace>,
) -> gpui::AnyElement {
    div()
        .id(SharedString::from(format!("command-{}", command.label())))
        .h(px(34.0))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .cursor_pointer()
        .border_b_1()
        .border_color(rgb(color::BORDER_SUBTLE))
        .when(selected, |row| row.bg(rgb(color::SELECTED)))
        .hover(|row| row.bg(rgb(color::HOVER)))
        .on_click(cx.listener(move |workspace, _: &ClickEvent, _, cx| {
            workspace.execute_command(command, cx);
        }))
        .child(
            div()
                .flex()
                .gap_3()
                .child(
                    div()
                        .w(px(72.0))
                        .flex_none()
                        .text_xs()
                        .text_color(rgb(color::TEXT_MUTED))
                        .child(command.category()),
                )
                .child(command.label()),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(color::TEXT_DIM))
                .child(command.hint()),
        )
        .into_any_element()
}
