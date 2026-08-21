use std::sync::Arc;

use reqwest::Method;
use serde::Serialize;

use crate::model::ModelRef;

use super::{Client, ClientInner, Error};

#[derive(Serialize)]
struct MessageActionBody<'a> {
    #[serde(rename = "messageID")]
    message_id: &'a str,
}

impl Client {
    /// Runs a shell command in a session using the selected agent identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the command.
    pub async fn shell(
        &self,
        session_id: &str,
        command: &str,
        agent: &str,
        model: Option<ModelRef>,
    ) -> Result<crate::model::MessageRecord, Error> {
        let inner = Arc::clone(&self.inner);
        let session_id = session_id.to_owned();
        let command = command.to_owned();
        let agent = agent.to_owned();
        self.inner
            .runtime
            .run(async move { inner.shell(&session_id, &command, &agent, model).await })
            .await
    }

    /// Reverts a session to the selected user message, including file changes.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the revert.
    pub async fn revert(
        &self,
        session_id: &str,
        message_id: &str,
    ) -> Result<crate::model::Session, Error> {
        let inner = Arc::clone(&self.inner);
        let session_id = session_id.to_owned();
        let message_id = message_id.to_owned();
        self.inner
            .runtime
            .run(async move {
                inner
                    .message_action(&session_id, "revert", &message_id)
                    .await
            })
            .await
    }

    /// Forks a session immediately before the selected user message.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the fork.
    pub async fn fork(
        &self,
        session_id: &str,
        message_id: &str,
    ) -> Result<crate::model::Session, Error> {
        let inner = Arc::clone(&self.inner);
        let session_id = session_id.to_owned();
        let message_id = message_id.to_owned();
        self.inner
            .runtime
            .run(async move { inner.message_action(&session_id, "fork", &message_id).await })
            .await
    }
}

impl ClientInner {
    async fn message_action(
        &self,
        session_id: &str,
        action: &str,
        message_id: &str,
    ) -> Result<crate::model::Session, Error> {
        let url = self.scoped_url(&["session", session_id, action])?;
        Self::send_json(
            self.request_url_method(Method::POST, url)
                .json(&MessageActionBody { message_id }),
        )
        .await
    }
}
