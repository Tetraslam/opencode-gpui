use gpui::{App, KeyBinding};

use super::navigation::{
    CloseDirectory, CompleteDirectory, CopyDebugInfo, DismissOverlay, NewSession, NextDirectory,
    NextSession, PreviousDirectory, PreviousSession, SelectNextOverlayItem,
    SelectPreviousOverlayItem, SubmitMessageAction, ToggleCommandPalette, ToggleDirectoryPicker,
    ToggleSessions, ToggleStatusMcp,
};

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("secondary-t", ToggleDirectoryPicker, None),
        KeyBinding::new("secondary-k", ToggleCommandPalette, None),
        KeyBinding::new("ctrl-p", ToggleCommandPalette, None),
        KeyBinding::new("escape", DismissOverlay, None),
        KeyBinding::new("secondary-b", ToggleSessions, None),
        KeyBinding::new("ctrl-tab", NextDirectory, None),
        KeyBinding::new("ctrl-shift-tab", PreviousDirectory, None),
        KeyBinding::new("secondary-w", CloseDirectory, None),
        KeyBinding::new("ctrl-n", NewSession, None),
        KeyBinding::new("alt-up", PreviousSession, None),
        KeyBinding::new("alt-down", NextSession, None),
        KeyBinding::new("up", SelectPreviousOverlayItem, None),
        KeyBinding::new("down", SelectNextOverlayItem, None),
        KeyBinding::new("tab", CompleteDirectory, None),
        KeyBinding::new("enter", SubmitMessageAction, Some("MessageActions")),
        KeyBinding::new("enter", CopyDebugInfo, Some("DebugDialog")),
        KeyBinding::new("space", ToggleStatusMcp, Some("StatusDialog")),
        KeyBinding::new("enter", ToggleStatusMcp, Some("StatusDialog")),
    ]);
}
