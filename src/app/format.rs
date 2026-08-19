use std::time::SystemTime;

use opencode_gpui::model::Session;

pub(super) fn display_title(session: &Session) -> String {
    if session.title.trim().is_empty() {
        "untitled session".into()
    } else if let Some(timestamp) = session.title.strip_prefix("New session - ") {
        compact_default_title(timestamp).unwrap_or_else(|| "new session".into())
    } else {
        session.title.clone()
    }
}

fn compact_default_title(timestamp: &str) -> Option<String> {
    let month = match timestamp.get(5..7)? {
        "01" => "jan",
        "02" => "feb",
        "03" => "mar",
        "04" => "apr",
        "05" => "may",
        "06" => "jun",
        "07" => "jul",
        "08" => "aug",
        "09" => "sep",
        "10" => "oct",
        "11" => "nov",
        "12" => "dec",
        _ => return None,
    };
    Some(format!(
        "new session · {month} {} {}",
        timestamp.get(8..10)?,
        timestamp.get(11..16)?
    ))
}

pub(super) fn relative_time(timestamp_ms: u64) -> String {
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        });
    let seconds = now_ms.saturating_sub(timestamp_ms) / 1_000;
    match seconds {
        0..=59 => "now".into(),
        60..=3_599 => format!("{}m", seconds / 60),
        3_600..=86_399 => format!("{}h", seconds / 3_600),
        86_400..=2_591_999 => format!("{}d", seconds / 86_400),
        _ => format!("{}mo", seconds / 2_592_000),
    }
}
