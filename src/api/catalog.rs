use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;

use super::{Client, ClientInner, Error};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct AgentCatalogEntry {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub mode: String,
    #[serde(default)]
    pub hidden: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProviderCatalog {
    pub all: Vec<ProviderCatalogEntry>,
    #[serde(default)]
    pub default: HashMap<String, String>,
    #[serde(default)]
    pub connected: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProviderCatalogEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub models: BTreeMap<String, ModelCatalogEntry>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ModelCatalogEntry {
    pub id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    pub name: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub variants: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug)]
pub struct CatalogResponse {
    pub agents: Vec<AgentCatalogEntry>,
    pub providers: ProviderCatalog,
}

impl Client {
    /// Fetches the directory-scoped agent and provider catalogs together.
    ///
    /// # Errors
    ///
    /// Returns an error when either request fails.
    pub async fn catalog(&self) -> Result<CatalogResponse, Error> {
        let inner = std::sync::Arc::clone(&self.inner);
        self.inner
            .runtime
            .run(async move {
                let (agents, providers) =
                    tokio::try_join!(inner.agents_catalog(), inner.providers_catalog())?;
                Ok(CatalogResponse { agents, providers })
            })
            .await
    }
}

impl ClientInner {
    async fn agents_catalog(&self) -> Result<Vec<AgentCatalogEntry>, Error> {
        let url = self.scoped_url(&["agent"])?;
        Self::send_json(self.request_url(url)).await
    }

    async fn providers_catalog(&self) -> Result<ProviderCatalog, Error> {
        let url = self.scoped_url(&["provider"])?;
        Self::send_json(self.request_url(url)).await
    }
}
