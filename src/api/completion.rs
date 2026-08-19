use std::sync::Arc;

use super::{Client, Error, PromptFile, SlashCommand};

impl Client {
    /// Finds files in the scoped project using the server's ranked search.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the request.
    pub async fn find_files(&self, query: &str, limit: usize) -> Result<Vec<String>, Error> {
        let inner = Arc::clone(&self.inner);
        let query = query.to_owned();
        self.inner
            .runtime
            .run(async move { inner.find_files(&query, limit.clamp(1, 200)).await })
            .await
    }

    /// Lists slash commands available in the scoped project.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the request.
    pub async fn commands(&self) -> Result<Vec<SlashCommand>, Error> {
        let inner = Arc::clone(&self.inner);
        self.inner
            .runtime
            .run(async move { inner.commands().await })
            .await
    }

    /// Executes a project slash command in a session.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the request.
    pub async fn command(
        &self,
        session_id: &str,
        command: &str,
        arguments: &str,
        files: Vec<PromptFile>,
    ) -> Result<(), Error> {
        let inner = Arc::clone(&self.inner);
        let session_id = session_id.to_owned();
        let command = command.to_owned();
        let arguments = arguments.to_owned();
        self.inner
            .runtime
            .run(async move {
                inner
                    .command(&session_id, &command, &arguments, files)
                    .await
            })
            .await
    }
}
