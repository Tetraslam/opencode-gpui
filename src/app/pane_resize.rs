use std::{env, fs, path::PathBuf};

use gpui::{AppContext, Context, MouseMoveEvent, MouseUpEvent, Pixels, Window, px};
use opencode_gpui::theme::size as ui_size;

use super::Workspace;

const MIN_WIDTH: f32 = 220.0;
const MAX_WIDTH: f32 = 520.0;
const MIN_INSPECTOR: f32 = 320.0;
const MAX_INSPECTOR: f32 = 720.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum PaneResize {
    #[default]
    Idle,
    Sessions,
    Inspector,
    Timeline {
        grab: Pixels,
    },
}

pub(super) fn load_session_pane_width() -> Pixels {
    load_width(
        "session-pane-width",
        ui_size::SESSION_PANE,
        MIN_WIDTH,
        MAX_WIDTH,
    )
}

pub(super) fn load_inspector_width() -> Pixels {
    load_width(
        "inspector-width",
        ui_size::INSPECTOR,
        MIN_INSPECTOR,
        MAX_INSPECTOR,
    )
}

fn load_width(name: &str, default: f32, min: f32, max: f32) -> Pixels {
    fs::read_to_string(state_path(name))
        .ok()
        .and_then(|value| value.trim().parse::<f32>().ok())
        .map(|value| value.clamp(min, max))
        .map_or_else(|| px(default), px)
}

impl Workspace {
    pub(super) fn handle_pointer_drag(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.pane_resize {
            PaneResize::Idle => return,
            PaneResize::Sessions => {
                self.session_pane_width = (event.position.x - px(ui_size::ACTIVITY_RAIL))
                    .clamp(px(MIN_WIDTH), px(MAX_WIDTH));
            }
            PaneResize::Inspector => {
                self.inspector_width = (window.bounds().size.width - event.position.x)
                    .clamp(px(MIN_INSPECTOR), px(MAX_INSPECTOR));
            }
            PaneResize::Timeline { grab } => {
                if let Some(handle) = self.active_tab().map(|tab| tab.timeline_scroll.clone()) {
                    super::timeline_scroll::scroll_to_pointer(&handle, event.position.y, grab);
                    if let Some(tab) = self.active_tab_mut() {
                        tab.follow_tail = super::timeline_scroll::is_at_bottom(&handle);
                    }
                }
            }
        }
        cx.notify();
    }

    pub(super) fn finish_pointer_drag(
        &mut self,
        _: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let drag = std::mem::replace(&mut self.pane_resize, PaneResize::Idle);
        let persisted = match drag {
            PaneResize::Sessions => Some(("session-pane-width", self.session_pane_width)),
            PaneResize::Inspector => Some(("inspector-width", self.inspector_width)),
            PaneResize::Idle | PaneResize::Timeline { .. } => None,
        };
        if let Some((name, width)) = persisted {
            let width = width / px(1.0);
            cx.background_spawn(async move {
                let path = state_path(name);
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(path, width.to_string());
            })
            .detach();
        }
        cx.notify();
    }
}

fn state_path(name: &str) -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map_or_else(
            || {
                env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_default()
                    .join(".config")
            },
            PathBuf::from,
        )
        .join("opencode-gpui")
        .join(name)
}
