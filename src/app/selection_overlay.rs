use std::sync::Arc;

use gpui::{Context, MouseButton, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    editor::TextEditor,
    model::ModelRef,
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{
    Workspace, command_palette::Overlay, composer_catalog::CatalogState,
    selection_filter::filter_items,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SelectionKind {
    Agent,
    Model,
    Variant,
}

#[derive(Clone, Debug)]
pub(crate) enum SelectionItem {
    Agent {
        name: String,
        description: String,
    },
    Model {
        reference: ModelRef,
        name: String,
        provider: String,
    },
    Variant(String),
}

impl SelectionItem {
    fn title(&self) -> SharedString {
        match self {
            Self::Variant(name) if name.is_empty() => "default".into(),
            Self::Agent { name, .. } | Self::Model { name, .. } | Self::Variant(name) => {
                name.clone().into()
            }
        }
    }

    fn detail(&self) -> SharedString {
        match self {
            Self::Agent { description, .. } => description.clone().into(),
            Self::Model {
                reference,
                provider,
                ..
            } => format!(
                "{provider}  |  {}/{}",
                reference.provider_id, reference.model_id
            )
            .into(),
            Self::Variant(name) if name.is_empty() => "let OpenCode choose the variant".into(),
            Self::Variant(name) if name == "none" => "reasoning disabled".into(),
            Self::Variant(_) => "selected model variant".into(),
        }
    }
}

impl Workspace {
    pub(super) fn refresh_active_overlay(&mut self, query: &str, cx: &mut Context<Self>) {
        match self.overlay {
            Overlay::Selection(_) => self.refresh_selection_suggestions(query, cx),
            Overlay::Timeline => {
                self.refresh_timeline_suggestions(query);
                self.preview_timeline_selection();
            }
            Overlay::Command => self.refresh_command_suggestions(query),
            Overlay::Directory
            | Overlay::MessageActions
            | Overlay::Status
            | Overlay::Debug
            | Overlay::None => {}
        }
    }

    pub(super) fn open_selection(&mut self, kind: SelectionKind, cx: &mut Context<Self>) {
        if let Some(directory) = self.active_directory().map(str::to_owned) {
            self.load_composer_catalog(&directory, cx);
        }
        self.overlay = Overlay::Selection(kind);
        self.clear_interrupt();
        self.overlay_selection = 0;
        self.reset_picker_scroll();
        self.command_editor.update(cx, TextEditor::clear);
        self.refresh_selection_suggestions("", cx);
        self.focus_overlay_on_render = true;
        cx.notify();
    }

    pub(super) fn refresh_selection_suggestions(&mut self, query: &str, cx: &mut Context<Self>) {
        let Overlay::Selection(kind) = self.overlay else {
            return;
        };
        self.overlay_selection = 0;
        query.clone_into(&mut self.selection_query);
        self.selection_suggestions = Arc::new(Vec::new());
        let Some(catalog) = self.active_tab().and_then(|tab| match &tab.catalog {
            CatalogState::Ready(catalog) => Some(Arc::clone(catalog)),
            CatalogState::Loading | CatalogState::Failed(_) => None,
        }) else {
            cx.notify();
            return;
        };
        let model = self
            .active_tab()
            .and_then(|tab| tab.selection.model.clone());
        let query = query.to_lowercase();
        let request = cx
            .background_spawn(async move { filter_items(&catalog, kind, &query, model.as_ref()) });
        let expected_query = self.selection_query.clone();
        self.selection_search = Some(cx.spawn(async move |workspace, cx| {
            let items = request.await;
            let _ = workspace.update(cx, |workspace, cx| {
                if workspace.overlay == Overlay::Selection(kind)
                    && workspace.selection_query == expected_query
                {
                    workspace.selection_suggestions = Arc::new(items);
                    workspace.overlay_selection = 0;
                    workspace.reset_picker_scroll();
                    cx.notify();
                }
            });
        }));
    }

    pub(super) fn submit_active_overlay(&mut self, query: &str, cx: &mut Context<Self>) {
        match self.overlay {
            Overlay::Command => self.execute_command_palette(query, cx),
            Overlay::Selection(_) => self.accept_selection(self.overlay_selection, cx),
            Overlay::Timeline => self.open_message_actions(cx),
            Overlay::MessageActions => self.execute_message_action(cx),
            Overlay::Directory | Overlay::Status | Overlay::Debug | Overlay::None => {}
        }
    }

    pub(super) fn accept_selection(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(item) = self.selection_suggestions.get(index).cloned() else {
            return;
        };
        let Some(tab) = self.active_tab_mut() else {
            return;
        };
        match item {
            SelectionItem::Agent { name, .. } => tab.selection.agent = Some(name),
            SelectionItem::Model { reference, .. } => {
                tab.selection.model = Some(reference);
                tab.selection.variant = None;
            }
            SelectionItem::Variant(variant) => {
                tab.selection.variant = (!variant.is_empty()).then_some(variant);
            }
        }
        tab.selection.explicit = true;
        self.overlay = Overlay::None;
        self.selection_suggestions = Arc::new(Vec::new());
        self.command_editor.update(cx, TextEditor::clear);
        self.focus_editor_on_render = true;
        cx.notify();
    }

    pub(super) fn render_selection_overlay(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let Overlay::Selection(kind) = self.overlay else {
            return None;
        };
        let status = self.selection_status(kind);
        Some(
            div()
                .absolute()
                .top(px(ui_size::TITLEBAR + 16.0))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .id("selection-overlay")
                        .w(px(560.0))
                        .max_h(px(500.0))
                        .flex()
                        .flex_col()
                        .bg(rgb(color::ELEVATED))
                        .border_1()
                        .border_color(rgb(color::BORDER))
                        .shadow_lg()
                        .font_family(MONO_FONT)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_, _, _, cx| cx.stop_propagation()),
                        )
                        .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                        .child(
                            div()
                                .px_3()
                                .pt_3()
                                .pb_2()
                                .text_sm()
                                .child(selection_title(kind)),
                        )
                        .child(
                            div()
                                .p_2()
                                .border_b_1()
                                .border_color(rgb(color::BORDER))
                                .child(self.command_editor.clone()),
                        )
                        .children(status.map(|status| {
                            div()
                                .p_3()
                                .border_b_1()
                                .border_color(rgb(color::BORDER))
                                .text_xs()
                                .text_color(rgb(color::TEXT_DIM))
                                .child(status)
                        }))
                        .child(
                            div()
                                .id("selection-results")
                                .min_h_0()
                                .flex_1()
                                .overflow_y_scroll()
                                .track_scroll(&self.picker_scroll)
                                .children(self.selection_suggestions.iter().enumerate().map(
                                    |(index, item)| {
                                        selection_row(item, index, self.overlay_selection, cx)
                                    },
                                )),
                        ),
                )
                .into_any_element(),
        )
    }

    fn selection_status(&self, kind: SelectionKind) -> Option<SharedString> {
        let tab = self.active_tab()?;
        match &tab.catalog {
            CatalogState::Loading => Some("loading catalog…".into()),
            CatalogState::Failed(error) => Some(format!("catalog unavailable: {error}").into()),
            CatalogState::Ready(_) if self.selection_suggestions.is_empty() => Some(
                if kind == SelectionKind::Variant {
                    "selected model has no variants"
                } else {
                    "no matches"
                }
                .into(),
            ),
            CatalogState::Ready(_) => None,
        }
    }
}

const fn selection_title(kind: SelectionKind) -> &'static str {
    match kind {
        SelectionKind::Agent => "Select agent",
        SelectionKind::Model => "Select model",
        SelectionKind::Variant => "Select variant",
    }
}

fn selection_row(
    item: &SelectionItem,
    index: usize,
    selected: usize,
    cx: &mut Context<Workspace>,
) -> gpui::AnyElement {
    let title = item.title();
    let detail = item.detail();
    div()
        .id(SharedString::from(format!("selection-{index}")))
        .min_h(px(42.0))
        .px_3()
        .py_1()
        .flex()
        .flex_col()
        .justify_center()
        .cursor_pointer()
        .border_b_1()
        .border_color(rgb(color::BORDER_SUBTLE))
        .when(index == selected, |row| row.bg(rgb(color::SELECTED)))
        .hover(|row| row.bg(rgb(color::HOVER)))
        .on_click(cx.listener(move |workspace, _, _, cx| workspace.accept_selection(index, cx)))
        .child(div().text_sm().text_color(rgb(color::TEXT)).child(title))
        .child(
            div()
                .text_xs()
                .text_color(rgb(color::TEXT_DIM))
                .child(detail),
        )
        .into_any_element()
}
