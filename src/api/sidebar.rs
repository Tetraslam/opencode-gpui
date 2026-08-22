use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Deserializer};

use super::{Client, ClientInner, Error};

#[derive(Clone, Debug, PartialEq)]
pub struct SidebarSnapshot {
    pub mcp: HashMap<String, McpStatus>,
    pub lsp: Vec<LspStatus>,
    pub todos: Vec<Todo>,
    pub files: Vec<FileDiff>,
    pub context_limits: HashMap<String, u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum McpStatus {
    Connected,
    Disabled,
    Failed {
        error: String,
    },
    NeedsAuth,
    NeedsClientRegistration {
        error: String,
    },
    Unknown {
        status: String,
        detail: Option<String>,
    },
}

impl<'de> Deserialize<'de> for McpStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let status = value
            .get("status")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| serde::de::Error::missing_field("status"))?;
        let error = value
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        Ok(match status {
            "connected" => Self::Connected,
            "disabled" => Self::Disabled,
            "failed" => Self::Failed { error },
            "needs_auth" => Self::NeedsAuth,
            "needs_client_registration" => Self::NeedsClientRegistration { error },
            other => Self::Unknown {
                status: other.to_owned(),
                detail: (!error.is_empty()).then_some(error),
            },
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct LspStatus {
    pub id: String,
    pub name: String,
    pub root: String,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Todo {
    #[serde(default)]
    pub id: String,
    pub content: String,
    pub status: String,
    pub priority: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct FileDiff {
    #[serde(alias = "path")]
    pub file: String,
    #[serde(default)]
    pub before: String,
    #[serde(default)]
    pub after: String,
    pub additions: u64,
    pub deletions: u64,
}

#[derive(Deserialize)]
struct ProviderCatalog {
    #[serde(alias = "all")]
    providers: Vec<Provider>,
}

#[derive(Deserialize)]
struct Provider {
    id: String,
    models: HashMap<String, ProviderModel>,
}

#[derive(Deserialize)]
struct ProviderModel {
    limit: ProviderLimit,
}

#[derive(Deserialize)]
struct ProviderLimit {
    context: u64,
}

impl Client {
    /// Loads server-owned state displayed in the persistent session sidebar.
    ///
    /// # Errors
    ///
    /// Returns an error when a required endpoint rejects the request.
    pub async fn sidebar_snapshot(&self, session_id: &str) -> Result<SidebarSnapshot, Error> {
        let inner = Arc::clone(&self.inner);
        let session_id = session_id.to_owned();
        self.inner
            .runtime
            .run(async move { inner.sidebar_snapshot(&session_id).await })
            .await
    }
}

impl ClientInner {
    async fn sidebar_snapshot(&self, session_id: &str) -> Result<SidebarSnapshot, Error> {
        let mcp = Self::send_json(self.request_url(self.scoped_url(&["mcp"])?));
        let lsp = Self::send_json(self.request_url(self.scoped_url(&["lsp"])?));
        let todos =
            Self::send_json(self.request_url(self.scoped_url(&["session", session_id, "todo"])?));
        let files =
            Self::send_json(self.request_url(self.scoped_url(&["session", session_id, "diff"])?));
        let providers = Self::send_json::<ProviderCatalog>(
            self.request_url(self.scoped_url(&["config", "providers"])?),
        );
        let (mcp, lsp, todos, files, providers) =
            tokio::try_join!(mcp, lsp, todos, files, providers)?;
        let context_limits = providers
            .providers
            .into_iter()
            .flat_map(|provider| {
                provider.models.into_iter().map(move |(model, data)| {
                    (format!("{}/{model}", provider.id), data.limit.context)
                })
            })
            .collect();
        Ok(SidebarSnapshot {
            mcp,
            lsp,
            todos,
            files,
            context_limits,
        })
    }
}
