mod catalog;
mod completion;
mod request;
mod runtime;
mod session_actions;
mod sidebar;
mod status;
mod stream;

#[cfg(test)]
mod mutation_tests;
#[cfg(test)]
mod status_tests;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use reqwest::{Client as HttpClient, Method, StatusCode};
use thiserror::Error;
use tokio::sync::oneshot;
use url::Url;

use crate::{
    event::Event,
    model::{Health, MessageRecord, Session, sort_sessions},
};

use runtime::Runtime;

#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

struct ClientInner {
    base_url: Url,
    directory: Option<String>,
    username: Option<String>,
    password: Option<String>,
    http: HttpClient,
    runtime: Runtime,
}

#[derive(Debug)]
pub struct Bootstrap {
    pub health: Health,
    pub sessions: Vec<Session>,
}

pub struct EventSubscription {
    receiver: tokio::sync::mpsc::Receiver<Result<Event, String>>,
    cancel: Option<oneshot::Sender<()>>,
}

#[derive(Clone, Debug, Default)]
pub struct CreateSession {
    pub parent_id: Option<String>,
    pub title: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Prompt {
    pub message_id: String,
    pub text_part_id: String,
    pub text: String,
    pub model: Option<crate::model::ModelRef>,
    pub agent: Option<String>,
    pub variant: Option<String>,
    pub files: Vec<PromptFile>,
}

#[derive(Clone, Debug)]
pub struct PromptFile {
    pub mime: String,
    pub filename: String,
    pub url: String,
}

pub use catalog::{
    AgentCatalogEntry, CatalogResponse, ModelCatalogEntry, ProviderCatalog, ProviderCatalogEntry,
};
pub use sidebar::{FileDiff, LspStatus, McpStatus, SidebarSnapshot, Todo};
pub use status::{FormatterStatus, StatusConfig, StatusSnapshot};

#[derive(Clone, Debug, serde::Deserialize)]
pub struct SlashCommand {
    pub name: String,
    pub description: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid OpenCode server URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("OpenCode server URL cannot be used as an HTTP base URL")]
    InvalidBaseUrl,
    #[error("could not start the network runtime")]
    RuntimeStart,
    #[error("the network runtime stopped unexpectedly")]
    RuntimeStopped,
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("OpenCode returned HTTP {status}: {message}")]
    Http { status: StatusCode, message: String },
}

impl Client {
    /// Creates a client for an `OpenCode` server and optional directory scope.
    ///
    /// # Errors
    ///
    /// Returns an error when setup fails.
    pub fn new(
        base_url: &str,
        directory: Option<String>,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<Self, Error> {
        let base_url = Url::parse(base_url)?;
        let http = HttpClient::builder()
            .connect_timeout(std::time::Duration::from_secs(3))
            .build()?;
        Ok(Self {
            inner: Arc::new(ClientInner {
                base_url,
                directory,
                username,
                password,
                http,
                runtime: Runtime::new()?,
            }),
        })
    }

    /// Returns a client sharing this connection pool and runtime, scoped to one directory.
    #[must_use]
    pub fn scoped(&self, directory: String) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                base_url: self.inner.base_url.clone(),
                directory: Some(directory),
                username: self.inner.username.clone(),
                password: self.inner.password.clone(),
                http: self.inner.http.clone(),
                runtime: self.inner.runtime.clone(),
            }),
        }
    }

    /// Fetches health and session metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when either request fails.
    pub async fn bootstrap(&self) -> Result<Bootstrap, Error> {
        let inner = Arc::clone(&self.inner);
        self.inner
            .runtime
            .run(async move {
                let health = inner.get_json::<Health>("global/health").await?;
                let mut sessions = inner.get_sessions().await?;
                sort_sessions(&mut sessions);
                Ok(Bootstrap { health, sessions })
            })
            .await
    }

    /// Fetches a bounded message window.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails.
    pub async fn messages(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<MessageRecord>, Error> {
        let inner = Arc::clone(&self.inner);
        let session_id = session_id.to_owned();
        self.inner
            .runtime
            .run(async move { inner.get_messages(&session_id, limit.clamp(1, 1_000)).await })
            .await
    }

    /// Opens a cancellable event subscription.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the subscription.
    pub async fn subscribe_events(&self) -> Result<EventSubscription, Error> {
        let inner = Arc::clone(&self.inner);
        self.inner
            .runtime
            .run(async move { inner.subscribe_events().await })
            .await
    }

    /// Creates a session.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the request.
    pub async fn create_session(&self, options: CreateSession) -> Result<Session, Error> {
        let inner = Arc::clone(&self.inner);
        self.inner
            .runtime
            .run(async move { inner.create_session(options).await })
            .await
    }

    /// Renames a session.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the request.
    pub async fn rename_session(&self, session_id: &str, title: &str) -> Result<Session, Error> {
        let inner = Arc::clone(&self.inner);
        let session_id = session_id.to_owned();
        let title = title.to_owned();
        self.inner
            .runtime
            .run(async move { inner.rename_session(&session_id, &title).await })
            .await
    }

    /// Deletes a session and its server-owned data.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the request.
    pub async fn delete_session(&self, session_id: &str) -> Result<bool, Error> {
        let inner = Arc::clone(&self.inner);
        let session_id = session_id.to_owned();
        self.inner
            .runtime
            .run(async move {
                inner
                    .session_boolean(Method::DELETE, &session_id, None)
                    .await
            })
            .await
    }

    /// Aborts work in a session.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the request.
    pub async fn abort_session(&self, session_id: &str) -> Result<bool, Error> {
        let inner = Arc::clone(&self.inner);
        let session_id = session_id.to_owned();
        self.inner
            .runtime
            .run(async move {
                inner
                    .session_boolean(Method::POST, &session_id, Some("abort"))
                    .await
            })
            .await
    }

    /// Fetches direct child sessions.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails.
    pub async fn children(&self, session_id: &str) -> Result<Vec<Session>, Error> {
        let inner = Arc::clone(&self.inner);
        let session_id = session_id.to_owned();
        self.inner
            .runtime
            .run(async move { inner.children(&session_id).await })
            .await
    }

    /// Submits text asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the prompt.
    pub async fn prompt(&self, session_id: &str, prompt: Prompt) -> Result<(), Error> {
        let inner = Arc::clone(&self.inner);
        let session_id = session_id.to_owned();
        self.inner
            .runtime
            .run(async move { inner.prompt(&session_id, prompt).await })
            .await
    }
}
