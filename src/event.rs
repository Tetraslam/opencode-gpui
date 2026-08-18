use serde::Deserialize;
use serde_json::Value;

use crate::model::{Message, Part, Session};

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Payload {
    #[serde(rename = "type")]
    pub kind: String,
    pub properties: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    ServerConnected,
    SessionCreated(Session),
    SessionUpdated(Session),
    SessionDeleted(Session),
    SessionStatus {
        session_id: String,
        status: SessionStatus,
    },
    SessionIdle {
        session_id: String,
    },
    MessageUpdated(Message),
    MessageRemoved {
        session_id: String,
        message_id: String,
    },
    MessagePartUpdated {
        part: Part,
        delta: Option<String>,
    },
    MessagePartRemoved {
        session_id: String,
        message_id: String,
        part_id: String,
    },
    Unknown(Payload),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SessionStatus {
    Idle,
    Busy,
    Retry {
        attempt: u64,
        message: String,
        next: u64,
    },
}

#[derive(Deserialize)]
struct Info<T> {
    info: T,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusProperties {
    #[serde(rename = "sessionID")]
    session_id: String,
    status: SessionStatus,
}

#[derive(Deserialize)]
struct SessionProperties {
    #[serde(rename = "sessionID")]
    session_id: String,
}

#[derive(Deserialize)]
struct MessageProperties {
    #[serde(rename = "sessionID")]
    session_id: String,
    #[serde(rename = "messageID")]
    message_id: String,
}

#[derive(Deserialize)]
struct PartUpdatedProperties {
    part: Part,
    delta: Option<String>,
}

#[derive(Deserialize)]
struct PartRemovedProperties {
    #[serde(rename = "sessionID")]
    session: String,
    #[serde(rename = "messageID")]
    message: String,
    #[serde(rename = "partID")]
    part: String,
}

impl Payload {
    #[must_use]
    pub fn into_event(self) -> Event {
        let parsed = match self.kind.as_str() {
            "server.connected" => return Event::ServerConnected,
            "session.created" => parse_info(&self.properties, Event::SessionCreated),
            "session.updated" => parse_info(&self.properties, Event::SessionUpdated),
            "session.deleted" => parse_info(&self.properties, Event::SessionDeleted),
            "session.status" => serde_json::from_value::<StatusProperties>(self.properties.clone())
                .map(|properties| Event::SessionStatus {
                    session_id: properties.session_id,
                    status: properties.status,
                }),
            "session.idle" => serde_json::from_value::<SessionProperties>(self.properties.clone())
                .map(|properties| Event::SessionIdle {
                    session_id: properties.session_id,
                }),
            "message.updated" => parse_info(&self.properties, Event::MessageUpdated),
            "message.removed" => serde_json::from_value::<MessageProperties>(
                self.properties.clone(),
            )
            .map(|properties| Event::MessageRemoved {
                session_id: properties.session_id,
                message_id: properties.message_id,
            }),
            "message.part.updated" => serde_json::from_value::<PartUpdatedProperties>(
                self.properties.clone(),
            )
            .map(|properties| Event::MessagePartUpdated {
                part: properties.part,
                delta: properties.delta,
            }),
            "message.part.removed" => serde_json::from_value::<PartRemovedProperties>(
                self.properties.clone(),
            )
            .map(|properties| Event::MessagePartRemoved {
                session_id: properties.session,
                message_id: properties.message,
                part_id: properties.part,
            }),
            _ => return Event::Unknown(self),
        };
        parsed.unwrap_or(Event::Unknown(self))
    }
}

fn parse_info<T: for<'de> Deserialize<'de>>(
    value: &Value,
    event: impl FnOnce(T) -> Event,
) -> Result<Event, serde_json::Error> {
    serde_json::from_value::<Info<T>>(value.clone()).map(|properties| event(properties.info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_session_status() {
        let payload: Payload = serde_json::from_str(
            r#"{"type":"session.status","properties":{"sessionID":"ses_1","status":{"type":"busy"}}}"#,
        )
        .unwrap();

        assert_eq!(
            payload.into_event(),
            Event::SessionStatus {
                session_id: "ses_1".into(),
                status: SessionStatus::Busy
            }
        );
    }

    #[test]
    fn preserves_unknown_and_malformed_events() {
        let unknown: Payload =
            serde_json::from_str(r#"{"type":"future.event","properties":{"answer":42}}"#).unwrap();
        let malformed: Payload =
            serde_json::from_str(r#"{"type":"session.status","properties":{"answer":42}}"#)
                .unwrap();

        assert!(matches!(unknown.into_event(), Event::Unknown(_)));
        assert!(matches!(malformed.into_event(), Event::Unknown(_)));
    }
}
