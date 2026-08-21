use gpui::Context;

use super::Workspace;

impl Workspace {
    pub(super) fn switch_directory(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() || index == self.active_tab {
            return;
        }
        self.dismiss_transients();
        self.active_tab = index;
        self.tab_bar.update(cx, |bar, cx| bar.set_active(index, cx));
        let timer = cx
            .background_executor()
            .timer(std::time::Duration::from_millis(75));
        self.directory_switch = Some(cx.spawn(async move |workspace, cx| {
            timer.await;
            let _ = workspace.update(cx, |workspace, cx| {
                workspace.focus_editor_on_render = true;
                workspace.persist_workspace_layout(cx);
                cx.notify();
            });
        }));
    }

    pub(super) fn switch_directory_immediately(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() || index == self.active_tab {
            return;
        }
        self.directory_switch = None;
        self.dismiss_transients();
        self.active_tab = index;
        self.tab_bar.update(cx, |bar, cx| bar.set_active(index, cx));
        self.focus_editor_on_render = true;
        self.persist_workspace_layout(cx);
        cx.notify();
    }
}
