use std::{cmp::Reverse, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct MessageRecord {
    pub info: Message,
    pub parts: Vec<Part>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
}

impl Message {
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::User(message) => &message.id,
            Self::Assistant(message) => &message.id,
        }
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        match self {
            Self::User(message) => &message.session_id,
            Self::Assistant(message) => &message.session_id,
        }
    }

    #[must_use]
    pub const fn role(&self) -> &'static str {
        match self {
            Self::User(_) => "you",
            Self::Assistant(_) => "assistant",
        }
    }

    #[must_use]
    pub fn detail(&self) -> String {
        match self {
            Self::User(message) => format!(
                "{} · {}/{}",
                message.agent, message.model.provider_id, message.model.model_id
            ),
            Self::Assistant(message) => format!(
                "{} · {}/{} · ${:.4}",
                message.mode, message.provider_id, message.model_id, message.cost
            ),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserMessage {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub time: MessageTime,
    pub agent: String,
    pub model: ModelRef,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessage {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub time: MessageTime,
    #[serde(rename = "parentID")]
    pub parent_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    pub mode: String,
    #[serde(default)]
    pub cost: f64,
    #[serde(default)]
    pub tokens: TokenUsage,
    pub finish: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
pub struct TokenUsage {
    pub total: Option<u64>,
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    #[serde(default)]
    pub cache: CacheUsage,
}

impl TokenUsage {
    #[must_use]
    pub fn used(self) -> u64 {
        self.total.unwrap_or(
            self.input + self.output + self.reasoning + self.cache.read + self.cache.write,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct CacheUsage {
    pub read: u64,
    pub write: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct MessageTime {
    pub created: u64,
    pub completed: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRef {
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
}

/// A message part that preserves fields unknown to this client.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(rename = "messageID")]
    pub message_id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub data: Arc<Map<String, Value>>,
}

impl Part {
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.data.get("text").and_then(Value::as_str)
    }

    #[must_use]
    pub fn summary(&self) -> Option<String> {
        match self.kind.as_str() {
            "text" | "reasoning" => self.text().map(ToOwned::to_owned),
            "tool" => {
                let tool = self.data.get("tool")?.as_str()?;
                let status = self
                    .data
                    .get("state")?
                    .get("status")?
                    .as_str()
                    .unwrap_or("unknown");
                let title = self.data.get("state")?.get("title").and_then(Value::as_str);
                Some(title.map_or_else(
                    || format!("{tool} [{status}]"),
                    |title| format!("{tool}: {title} [{status}]"),
                ))
            }
            "file" => self
                .data
                .get("filename")
                .and_then(Value::as_str)
                .or_else(|| self.data.get("url").and_then(Value::as_str))
                .map(|value| format!("attachment: {value}")),
            "subtask" => self
                .data
                .get("description")
                .and_then(Value::as_str)
                .map(|value| format!("subtask: {value}")),
            "agent" => self
                .data
                .get("name")
                .and_then(Value::as_str)
                .map(|value| format!("agent: {value}")),
            "retry" => self
                .data
                .get("attempt")
                .and_then(Value::as_u64)
                .map(|attempt| format!("retry {attempt}")),
            "patch" => self
                .data
                .get("files")
                .and_then(Value::as_array)
                .map(|files| format!("patch: {} files", files.len())),
            "step-start" | "step-finish" | "snapshot" | "compaction" => None,
            other => Some(format!("unsupported part: {other}")),
        }
    }
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

    #[test]
    fn preserves_unknown_part_payloads() {
        let part: Part = serde_json::from_str(
            r#"{"id":"part_1","sessionID":"ses_1","messageID":"msg_1","type":"future-part","answer":42}"#,
        )
        .unwrap();

        assert_eq!(part.kind, "future-part");
        assert_eq!(part.data["answer"], 42);
        assert_eq!(
            part.summary().as_deref(),
            Some("unsupported part: future-part")
        );
    }

    #[test]
    fn summarizes_tool_lifecycle_without_modeling_tool_payloads() {
        let part: Part = serde_json::from_str(
            r#"{"id":"part_1","sessionID":"ses_1","messageID":"msg_1","type":"tool","tool":"bash","state":{"status":"completed","title":"cargo test","output":"ok","input":{},"metadata":{},"time":{"start":1,"end":2}}}"#,
        )
        .unwrap();

        assert_eq!(
            part.summary().as_deref(),
            Some("bash: cargo test [completed]")
        );
    }
}
