use opencode_gpui::model::{Session, SessionTime};

static TEST_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub(super) fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "opencode-gpui-test-{name}-{}",
        TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ))
}

pub(super) fn session(id: &str, updated: u64) -> Session {
    session_in(id, "/workspace", updated)
}

pub(super) fn session_in(id: &str, directory: &str, updated: u64) -> Session {
    Session {
        id: id.into(),
        project_id: "project".into(),
        directory: directory.into(),
        parent_id: None,
        title: id.into(),
        version: "1.18.16".into(),
        time: SessionTime {
            created: 1,
            updated,
            compacting: None,
        },
        revert: None,
    }
}
