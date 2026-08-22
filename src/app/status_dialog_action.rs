use gpui::{AppContext, Context, Window};
use opencode_gpui::api::Client;

use super::{
    Workspace,
    command_palette::Overlay,
    navigation::ToggleStatusMcp,
    status_dialog::{McpOperation, StatusTarget, mcp_operation},
};

impl Workspace {
    pub(super) fn open_status_dialog(&mut self, cx: &mut Context<Self>) {
        let Some((client, target)) = self.status_client_target() else {
            return;
        };
        self.status_dialog.reset_for_open();
        self.clear_interrupt();
        self.reset_picker_scroll();
        self.overlay = Overlay::Status;
        self.focus_overlay_on_render = true;
        self.start_status_refresh(client, target, cx);
    }

    fn start_status_refresh(
        &mut self,
        client: Client,
        target: StatusTarget,
        cx: &mut Context<Self>,
    ) -> u64 {
        let generation = self.status_dialog.begin(target.clone());
        let request = cx.background_spawn(async move {
            client
                .status_snapshot()
                .await
                .map_err(|error| error.to_string())
        });
        cx.spawn(async move |workspace, cx| {
            let result = request.await;
            let _ = workspace.update(cx, |workspace, cx| {
                if workspace.overlay != Overlay::Status
                    || workspace.status_target() != Some(target.clone())
                {
                    return;
                }
                if workspace.status_dialog.apply(&target, generation, result) {
                    cx.notify();
                }
            });
        })
        .detach();
        generation
    }

    pub(super) fn toggle_selected_status_mcp(
        &mut self,
        _: &ToggleStatusMcp,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_status_mcp(cx);
    }

    pub(super) fn select_and_toggle_status_mcp(&mut self, index: usize, cx: &mut Context<Self>) {
        self.status_dialog.select(index);
        self.toggle_status_mcp(cx);
    }

    fn toggle_status_mcp(&mut self, cx: &mut Context<Self>) {
        if self.overlay != Overlay::Status || self.status_dialog.pending.is_some() {
            return;
        }
        let Some(name) = self
            .status_dialog
            .mcp_names
            .get(self.status_dialog.selected)
            .cloned()
        else {
            return;
        };
        let Some(status) = self
            .status_dialog
            .snapshot
            .as_deref()
            .and_then(|snapshot| snapshot.mcp.get(&name))
        else {
            return;
        };
        let operation = mcp_operation(status);
        let Some((client, target)) = self.status_client_target() else {
            return;
        };
        let Some(generation) = self.status_dialog.start_operation(name.clone(), operation) else {
            return;
        };
        cx.notify();
        let action_name = name.clone();
        let request = cx.background_spawn(async move {
            match operation {
                McpOperation::Connect => client.connect_mcp(&action_name).await,
                McpOperation::Disconnect => client.disconnect_mcp(&action_name).await,
            }
            .map_err(|error| error.to_string())
        });
        cx.spawn(async move |workspace, cx| {
            let result = request.await;
            let _ = workspace.update(cx, |workspace, cx| {
                if workspace.overlay != Overlay::Status
                    || workspace.status_target() != Some(target.clone())
                    || !workspace
                        .status_dialog
                        .operation_is_current(generation, &name)
                {
                    return;
                }
                match result {
                    Ok(true) => {
                        let Some((client, current)) = workspace.status_client_target() else {
                            return;
                        };
                        let refresh = workspace.start_status_refresh(client, current, cx);
                        workspace
                            .status_dialog
                            .set_operation_refresh(generation, refresh);
                    }
                    Ok(false) => {
                        let verb = operation.verb();
                        workspace.status_dialog.fail_operation(
                            generation,
                            format!("Could not {verb} {name}: OpenCode returned false. Retry the toggle."),
                        );
                    }
                    Err(error) => {
                        let verb = operation.verb();
                        workspace.status_dialog.fail_operation(
                            generation,
                            format!("Could not {verb} {name}: {error}. Retry the toggle."),
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn status_client_target(&self) -> Option<(Client, StatusTarget)> {
        let tab = self.active_tab()?;
        Some((
            tab.client.clone(),
            StatusTarget {
                directory: tab.directory.clone(),
                session_id: tab.timeline.session_id().map(str::to_owned),
            },
        ))
    }

    pub(super) fn status_target(&self) -> Option<StatusTarget> {
        self.status_client_target().map(|(_, target)| target)
    }
}

impl McpOperation {
    const fn verb(self) -> &'static str {
        match self {
            Self::Connect => "connect",
            Self::Disconnect => "disconnect",
        }
    }
}
