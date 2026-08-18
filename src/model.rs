use std::cmp::Reverse;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Health {
    pub healthy: bool,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Session {
    pub id: String,
    #[serde(rename = "projectID")]
    pub project_id: String,
    pub directory: String,
    #[serde(rename = "parentID")]
    pub parent_id: Option<String>,
    pub title: String,
    pub version: String,
    pub time: SessionTime,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct SessionTime {
    pub created: u64,
    pub updated: u64,
    pub compacting: Option<u64>,
}

pub fn sort_sessions(sessions: &mut [Session]) {
    sessions.sort_unstable_by_key(|session| Reverse(session.time.updated));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, updated: u64) -> Session {
        Session {
            id: id.into(),
            project_id: "project".into(),
            directory: "/tmp/project".into(),
            parent_id: None,
            title: id.into(),
            version: "1.0.0".into(),
            time: SessionTime {
                created: updated,
                updated,
                compacting: None,
            },
        }
    }

    #[test]
    fn sorts_sessions_by_most_recent_update() {
        let mut sessions = vec![session("old", 1), session("new", 3), session("middle", 2)];

        sort_sessions(&mut sessions);

        assert_eq!(
            sessions
                .iter()
                .map(|session| session.id.as_str())
                .collect::<Vec<_>>(),
            ["new", "middle", "old"]
        );
    }
}
