use std::{collections::HashMap, sync::Arc};

use reqwest::Method;
use serde::{Deserialize, Deserializer};

use super::{Client, ClientInner, Error, LspStatus, McpStatus};

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct FormatterStatus {
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct StatusConfig {
    #[serde(
        default,
        rename = "plugin",
        deserialize_with = "deserialize_optional_array"
    )]
    pub plugins: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StatusSnapshot {
    pub mcp: HashMap<String, McpStatus>,
    pub lsp: Vec<LspStatus>,
    pub formatters: Vec<FormatterStatus>,
    pub config: StatusConfig,
}

impl Client {
    /// Loads the directory-scoped state shown in the status dialog.
    ///
    /// # Errors
    ///
    /// Returns an error when any status endpoint rejects the request.
    pub async fn status_snapshot(&self) -> Result<StatusSnapshot, Error> {
        let inner = Arc::clone(&self.inner);
        self.inner
            .runtime
            .run(async move { inner.status_snapshot().await })
            .await
    }

    /// Connects one directory-scoped MCP server.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the request.
    pub async fn connect_mcp(&self, name: &str) -> Result<bool, Error> {
        self.set_mcp_connection(name, "connect").await
    }

    /// Disconnects one directory-scoped MCP server.
    ///
    /// # Errors
    ///
    /// Returns an error when the server rejects the request.
    pub async fn disconnect_mcp(&self, name: &str) -> Result<bool, Error> {
        self.set_mcp_connection(name, "disconnect").await
    }

    async fn set_mcp_connection(&self, name: &str, action: &str) -> Result<bool, Error> {
        let inner = Arc::clone(&self.inner);
        let name = name.to_owned();
        let action = action.to_owned();
        self.inner
            .runtime
            .run(async move {
                let url = inner.scoped_url(&["mcp", &name, &action])?;
                ClientInner::send_json(inner.request_url_method(Method::POST, url)).await
            })
            .await
    }
}

impl ClientInner {
    async fn status_snapshot(&self) -> Result<StatusSnapshot, Error> {
        let mcp = Self::send_json(self.request_url(self.scoped_url(&["mcp"])?));
        let lsp = Self::send_json(self.request_url(self.scoped_url(&["lsp"])?));
        let formatters = Self::send_json(self.request_url(self.scoped_url(&["formatter"])?));
        let config = Self::send_json(self.request_url(self.scoped_url(&["config"])?));
        let (mcp, lsp, formatters, config) = tokio::try_join!(mcp, lsp, formatters, config)?;
        Ok(StatusSnapshot {
            mcp,
            lsp,
            formatters,
            config,
        })
    }
}

fn deserialize_optional_array<'de, D>(deserializer: D) -> Result<Vec<serde_json::Value>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<Vec<serde_json::Value>>::deserialize(deserializer)?.unwrap_or_default())
}
