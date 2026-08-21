use opencode_gpui::model::{Session, SessionTime};

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
    }
}
