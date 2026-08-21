use std::sync::Arc;

use gpui::{Context, MouseButton, SharedString, div, prelude::*, px, rgb};
use opencode_gpui::{
    editor::TextEditor,
    model::ModelRef,
    theme::{MONO_FONT, color, size as ui_size},
};

use super::{
    Workspace,
    command_palette::Overlay,
    composer_catalog::{CatalogState, ComposerCatalog},
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
            Self::Variant(_) => "selected model variant".into(),
        }
    }
}

impl Workspace {
    pub(super) fn open_selection(&mut self, kind: SelectionKind, cx: &mut Context<Self>) {
        if let Some(directory) = self.active_directory().map(str::to_owned) {
            self.load_composer_catalog(&directory, cx);
        }
        self.overlay = Overlay::Selection(kind);
        self.overlay_selection = 0;
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
                    cx.notify();
                }
            });
        }));
    }

    pub(super) fn submit_active_overlay(&mut self, query: &str, cx: &mut Context<Self>) {
        match self.overlay {
            Overlay::Command => self.execute_command_palette(query, cx),
            Overlay::Selection(_) => self.accept_selection(self.overlay_selection, cx),
            Overlay::Directory | Overlay::None => {}
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
            SelectionItem::Variant(variant) => tab.selection.variant = Some(variant),
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
                        .overflow_scroll()
                        .track_scroll(&self.picker_scroll)
                        .bg(rgb(color::ELEVATED))
                        .border_1()
                        .border_color(rgb(color::BORDER))
                        .shadow_lg()
                        .font_family(MONO_FONT)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_, _, _, cx| cx.stop_propagation()),
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
                                .text_xs()
                                .text_color(rgb(color::TEXT_DIM))
                                .child(status)
                        }))
                        .children(self.selection_suggestions.iter().enumerate().map(
                            |(index, item)| selection_row(item, index, self.overlay_selection, cx),
                        )),
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

fn filter_items(
    catalog: &ComposerCatalog,
    kind: SelectionKind,
    query: &str,
    model: Option<&ModelRef>,
) -> Vec<SelectionItem> {
    match kind {
        SelectionKind::Agent => catalog
            .agents
            .iter()
            .filter(|agent| matches_query(&[&agent.name, &agent.description], query))
            .map(|agent| SelectionItem::Agent {
                name: agent.name.clone(),
                description: agent.description.clone(),
            })
            .take(100)
            .collect(),
        SelectionKind::Model => catalog
            .models
            .iter()
            .filter(|item| {
                matches_query(
                    &[
                        &item.name,
                        &item.provider_name,
                        &item.reference.provider_id,
                        &item.reference.model_id,
                    ],
                    query,
                )
            })
            .map(|item| SelectionItem::Model {
                reference: item.reference.clone(),
                name: item.name.clone(),
                provider: item.provider_name.clone(),
            })
            .take(100)
            .collect(),
        SelectionKind::Variant => catalog
            .variants(model)
            .iter()
            .filter(|variant| variant.to_lowercase().contains(query))
            .cloned()
            .map(SelectionItem::Variant)
            .collect(),
    }
}

fn matches_query(values: &[&str], query: &str) -> bool {
    query.is_empty()
        || values
            .iter()
            .any(|value| value.to_lowercase().contains(query))
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
