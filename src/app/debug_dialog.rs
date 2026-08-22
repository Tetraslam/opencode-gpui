use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{ClipboardItem, Context, Window};

use super::{Workspace, command_palette::Overlay, navigation::CopyDebugInfo};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DebugEntry {
    pub(super) label: &'static str,
    pub(super) value: String,
}

#[derive(Default)]
pub(crate) struct DebugDialogState {
    pub(super) entries: Vec<DebugEntry>,
    pub(super) copied: bool,
}

#[derive(Default)]
pub(super) struct DebugContext {
    pub(super) session_id: Option<String>,
    pub(super) model: Option<String>,
    pub(super) variant: Option<String>,
    pub(super) server_url: String,
}

pub(super) fn build_debug_entries(context: DebugContext, epoch_seconds: u64) -> Vec<DebugEntry> {
    vec![
        entry("Version", app_version()),
        entry("Date", iso_utc(epoch_seconds)),
        entry("OS", std::env::consts::OS),
        entry("Platform", std::env::consts::FAMILY),
        entry("Architecture", std::env::consts::ARCH),
        entry("Desktop/runtime", desktop_runtime()),
        entry(
            "Session ID",
            context.session_id.unwrap_or_else(|| "n/a".into()),
        ),
        entry("Model", context.model.unwrap_or_else(|| "n/a".into())),
        entry("Variant", context.variant.unwrap_or_else(|| "n/a".into())),
        entry("Server URL", context.server_url),
    ]
}

pub(super) fn debug_text(entries: &[DebugEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}: {}", entry.label, entry.value))
        .collect::<Vec<_>>()
        .join("\n")
}

fn entry(label: &'static str, value: impl Into<String>) -> DebugEntry {
    DebugEntry {
        label,
        value: value.into(),
    }
}

fn app_version() -> String {
    let channel = option_env!("OPENCODE_GPUI_CHANNEL").unwrap_or(if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    });
    format!("{} ({channel})", env!("CARGO_PKG_VERSION"))
}

fn desktop_runtime() -> String {
    let desktop = if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        "Wayland"
    } else if std::env::var_os("DISPLAY").is_some() {
        "X11"
    } else {
        "unknown desktop"
    };
    format!("GPUI desktop ({desktop})")
}

fn iso_utc(epoch_seconds: u64) -> String {
    let days = i64::try_from(epoch_seconds / 86_400).unwrap_or(i64::MAX);
    let seconds = epoch_seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        seconds / 3_600,
        (seconds % 3_600) / 60,
        seconds % 60
    )
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

impl Workspace {
    pub(super) fn open_debug_dialog(&mut self, _: &mut Context<Self>) {
        let context = self.active_tab().map_or_else(
            || DebugContext {
                server_url: self.server.to_string(),
                ..DebugContext::default()
            },
            |tab| DebugContext {
                session_id: tab.timeline.session_id().map(str::to_owned),
                model: tab
                    .selection
                    .model
                    .as_ref()
                    .map(|model| format!("{}/{}", model.provider_id, model.model_id)),
                variant: tab.selection.variant.clone(),
                server_url: self.server.to_string(),
            },
        );
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        self.debug_dialog.entries = build_debug_entries(context, now);
        self.debug_dialog.copied = false;
        self.clear_interrupt();
        self.overlay = Overlay::Debug;
        self.focus_overlay_on_render = true;
    }

    pub(super) fn copy_debug_info(
        &mut self,
        _: &CopyDebugInfo,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.overlay != Overlay::Debug {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(debug_text(
            &self.debug_dialog.entries,
        )));
        if !self.debug_dialog.copied {
            self.debug_dialog.copied = true;
            cx.notify();
        }
    }

    pub(super) fn copy_debug_info_click(&mut self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(debug_text(
            &self.debug_dialog.entries,
        )));
        if !self.debug_dialog.copied {
            self.debug_dialog.copied = true;
            cx.notify();
        }
    }
}
