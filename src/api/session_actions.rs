use std::sync::Arc;

use crate::model::ModelRef;

use super::{Client, Error};

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
}
