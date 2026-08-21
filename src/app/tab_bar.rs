use std::sync::Arc;

use gpui::{
    Context, Entity, EventEmitter, Render, SharedString, Subscription, div, prelude::*, px, rgb,
};
use opencode_gpui::{
    editor::TextEditor,
    theme::{MONO_FONT, color, size as ui_size},
};

use super::Workspace;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TabBarTab {
    pub(super) label: SharedString,
    pub(super) busy: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum TabBarEvent {
    Activate(usize),
    Close(usize),
    OpenPicker,
}

pub(super) struct TabBar {
    tabs: Arc<Vec<TabBarTab>>,
    active: usize,
    reconnecting: bool,
}

impl TabBar {
    pub(super) fn new() -> Self {
        Self {
            tabs: Arc::new(Vec::new()),
            active: 0,
            reconnecting: false,
        }
    }

    pub(super) fn set_active(&mut self, active: usize, cx: &mut Context<Self>) {
        if self.active != active {
            self.active = active;
            cx.notify();
        }
    }

    #[cfg(test)]
    pub(super) const fn active(&self) -> usize {
        self.active
    }

    fn reconcile(
        &mut self,
        tabs: Vec<TabBarTab>,
        active: usize,
        reconnecting: bool,
        cx: &mut Context<Self>,
    ) {
        if self.tabs.as_ref() == &tabs && self.active == active && self.reconnecting == reconnecting
        {
            return;
        }
        self.tabs = Arc::new(tabs);
        self.active = active;
        self.reconnecting = reconnecting;
        cx.notify();
    }
}

impl EventEmitter<TabBarEvent> for TabBar {}

pub(super) fn create(cx: &mut Context<Workspace>) -> (Entity<TabBar>, Subscription) {
    let tab_bar = cx.new(|_| TabBar::new());
    let subscription = cx.subscribe(
        &tab_bar,
        |workspace, _, event: &TabBarEvent, cx| match *event {
            TabBarEvent::Activate(index) => workspace.switch_directory_immediately(index, cx),
            TabBarEvent::Close(index) => workspace.close_directory(index, cx),
            TabBarEvent::OpenPicker => {
                workspace.overlay = super::command_palette::Overlay::Directory;
                workspace.overlay_selection = 0;
                workspace.directory_editor.update(cx, TextEditor::clear);
                workspace.refresh_directory_suggestions(String::new(), cx);
                workspace.focus_overlay_on_render = true;
                cx.notify();
            }
        },
    );
    (tab_bar, subscription)
}

impl Render for TabBar {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.active;
        let tabs = self.tabs.iter().cloned().enumerate().map(|(index, tab)| {
            div()
                .id(SharedString::from(format!("directory-tab-{index}")))
                .h_full()
                .max_w(px(190.0))
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .cursor_pointer()
                .border_r_1()
                .border_color(rgb(color::BORDER))
                .when(index == active, |tab| tab.bg(rgb(color::SELECTED)))
                .hover(|tab| tab.bg(rgb(color::HOVER)))
                .on_click(cx.listener(move |bar, _, _, cx| {
                    bar.set_active(index, cx);
                    cx.emit(TabBarEvent::Activate(index));
                }))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .text_color(rgb(if index == active {
                            color::TEXT_BRIGHT
                        } else {
                            color::TEXT_DIM
                        }))
                        .child(tab.label),
                )
                .child(close_button(index, cx))
                .when(tab.busy, |tab| {
                    tab.child(div().size(px(5.0)).rounded_full().bg(rgb(color::GREEN)))
                })
        });
        div()
            .h(px(ui_size::TITLEBAR))
            .flex_none()
            .flex()
            .items_center()
            .bg(rgb(color::SURFACE))
            .border_b_1()
            .border_color(rgb(color::BORDER))
            .font_family(MONO_FONT)
            .text_xs()
            .child(
                div()
                    .w(px(ui_size::ACTIVITY_RAIL))
                    .h_full()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(color::ACCENT))
                    .child("oc"),
            )
            .child(div().min_w_0().h_full().flex_1().flex().children(tabs))
            .child(
                div()
                    .id("open-directory")
                    .h_full()
                    .w(px(34.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .text_color(rgb(color::TEXT_DIM))
                    .hover(|button| button.bg(rgb(color::HOVER)).text_color(rgb(color::TEXT)))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(TabBarEvent::OpenPicker)))
                    .child("+"),
            )
            .children(self.reconnecting.then(|| {
                div()
                    .px_3()
                    .flex_none()
                    .text_center()
                    .text_color(rgb(color::YELLOW))
                    .child("reconnecting")
            }))
    }
}

fn close_button(index: usize, cx: &mut Context<TabBar>) -> gpui::AnyElement {
    div()
        .id(SharedString::from(format!("close-directory-{index}")))
        .size(px(20.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .text_color(rgb(color::TEXT_MUTED))
        .hover(|button| button.bg(rgb(color::HOVER)).text_color(rgb(color::TEXT)))
        .on_click(cx.listener(move |_, _, _, cx| {
            cx.stop_propagation();
            cx.emit(TabBarEvent::Close(index));
        }))
        .child("x")
        .into_any_element()
}

impl Workspace {
    pub(super) fn sync_tab_bar(&self, cx: &mut Context<Self>) {
        let busy = self.busy_directories();
        let tabs = self
            .tabs
            .iter()
            .map(|tab| TabBarTab {
                label: super::directory_path::directory_name(&tab.directory)
                    .to_owned()
                    .into(),
                busy: busy.contains(tab.directory.as_str()),
            })
            .collect();
        let reconnecting = self.active_directory().is_some() && !self.active_directory_is_live();
        self.tab_bar.update(cx, |bar, cx| {
            bar.reconcile(tabs, self.active_tab, reconnecting, cx);
        });
    }
}
