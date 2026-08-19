use gpui::{Context, MouseButton, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::theme::{MONO_FONT, color, size as ui_size};

use super::{
    Workspace,
    composer_completion::{CompletionItem, CompletionMode},
};

impl Workspace {
    pub(super) fn render_composer_completion(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let completion = self.active_tab()?.composer_completion.as_ref()?;
        let left = px(12.0 + ui_size::ACTIVITY_RAIL)
            + if self.sessions_open {
                self.session_pane_width + px(5.0)
            } else {
                px(0.0)
            };
        Some({
            let rows = completion.items.iter().enumerate().map(|(index, item)| {
                let (label, description): (SharedString, SharedString) = match item {
                    CompletionItem::File(path) => (format!("@{path}").into(), "file".into()),
                    CompletionItem::Local {
                        name, description, ..
                    } => (format!("/{name}").into(), (*description).into()),
                    CompletionItem::Command { name, description } => {
                        (format!("/{name}").into(), description.clone())
                    }
                };
                let selected = index == completion.selected;
                div()
                    .id(SharedString::from(format!("composer-completion-{index}")))
                    .h(px(34.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_3()
                    .cursor_pointer()
                    .when(selected, |row| row.bg(rgb(color::SELECTED)))
                    .hover(|row| row.bg(rgb(color::HOVER)))
                    .on_click(cx.listener(move |workspace, _, _, cx| {
                        if let Some(completion) = workspace
                            .active_tab_mut()
                            .and_then(|tab| tab.composer_completion.as_mut())
                        {
                            completion.selected = index;
                        }
                        let directory = workspace.active_directory().map(str::to_owned);
                        if let Some(directory) = directory {
                            workspace.accept_composer_completion(&directory, cx);
                        }
                    }))
                    .child(div().flex_none().text_color(rgb(color::TEXT)).child(label))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .truncate()
                            .text_color(rgb(color::TEXT_DIM))
                            .child(description),
                    )
            });
            let state = completion
                .items
                .is_empty()
                .then(|| empty_state(completion.mode, completion.loading));
            div()
                .id("composer-completion")
                .absolute()
                .left(left)
                .bottom(px(96.0))
                .w(px(720.0))
                .max_h(px(340.0))
                .overflow_scroll()
                .track_scroll(&self.composer_completion_scroll)
                .bg(rgb(color::ELEVATED))
                .border_1()
                .border_color(rgb(color::BORDER))
                .shadow_lg()
                .font_family(MONO_FONT)
                .text_xs()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|_, _, _, cx| {
                        cx.stop_propagation();
                    }),
                )
                .children(state)
                .children(rows)
                .into_any_element()
        })
    }
}

fn empty_state(mode: CompletionMode, loading: bool) -> gpui::AnyElement {
    let message = match (mode, loading) {
        (CompletionMode::File, true) => "searching files...",
        (CompletionMode::Command, true) => "loading commands...",
        (CompletionMode::File, false) => "no matching files",
        (CompletionMode::Command, false) => "no matching commands",
    };
    div()
        .h(px(34.0))
        .px_3()
        .flex()
        .items_center()
        .text_color(rgb(color::TEXT_DIM))
        .child(message)
        .into_any_element()
}
