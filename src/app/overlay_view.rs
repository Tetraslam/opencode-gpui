use gpui::Context;

use super::Workspace;

impl Workspace {
    pub(super) fn render_overlays(&self, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        [
            self.render_directory_picker(cx),
            self.render_command_palette(cx),
            self.render_selection_overlay(cx),
            self.render_timeline_overlay(cx),
            self.render_message_actions(cx),
            self.render_status_dialog(cx),
            self.render_debug_dialog(cx),
            self.render_composer_completion(cx),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}
