use std::{collections::HashSet, sync::Arc};

use gpui::{AppContext, Context};

use opencode_gpui::{
    api::CatalogResponse,
    model::{Message, ModelRef},
};

use super::{TimelineState, Workspace};

#[derive(Clone, Debug)]
pub(super) struct AgentOption {
    pub(super) name: String,
    pub(super) description: String,
}

#[derive(Clone, Debug)]
pub(super) struct ModelOption {
    pub(super) reference: ModelRef,
    pub(super) name: String,
    pub(super) provider_name: String,
    pub(super) variants: Vec<String>,
}

#[derive(Clone, Debug)]
pub(super) struct ComposerCatalog {
    pub(super) agents: Vec<AgentOption>,
    pub(super) models: Vec<ModelOption>,
    pub(super) default_model: Option<ModelRef>,
}

#[derive(Clone, Debug)]
pub(super) enum CatalogState {
    Loading,
    Ready(Arc<ComposerCatalog>),
    Failed(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct ComposerSelection {
    pub(super) agent: Option<String>,
    pub(super) model: Option<ModelRef>,
    pub(super) variant: Option<String>,
    pub(super) explicit: bool,
}

impl ComposerCatalog {
    pub(super) fn from_response(response: CatalogResponse) -> Self {
        let mut agents = response
            .agents
            .into_iter()
            .filter(|agent| agent.hidden != Some(true) && agent.mode != "subagent")
            .map(|agent| AgentOption {
                name: agent.name,
                description: agent.description.unwrap_or_default(),
            })
            .collect::<Vec<_>>();
        agents.sort_unstable_by(|left, right| left.name.cmp(&right.name));

        let connected = response
            .providers
            .connected
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let mut models = response
            .providers
            .all
            .into_iter()
            .filter(|provider| connected.contains(provider.id.as_str()))
            .flat_map(|provider| {
                provider.models.into_values().filter_map(move |model| {
                    (model.status.as_deref() != Some("deprecated")).then(|| ModelOption {
                        reference: ModelRef {
                            provider_id: model.provider_id,
                            model_id: model.id,
                        },
                        name: model.name,
                        provider_name: provider.name.clone(),
                        variants: model.variants.into_keys().collect(),
                    })
                })
            })
            .collect::<Vec<_>>();
        models.sort_unstable_by(|left, right| {
            left.provider_name
                .cmp(&right.provider_name)
                .then_with(|| left.name.cmp(&right.name))
        });
        let default_model = response.providers.connected.iter().find_map(|provider_id| {
            let model_id = response.providers.default.get(provider_id)?;
            let reference = ModelRef {
                provider_id: provider_id.clone(),
                model_id: model_id.clone(),
            };
            models
                .iter()
                .any(|model| model.reference == reference)
                .then_some(reference)
        });
        Self {
            agents,
            models,
            default_model,
        }
    }

    pub(super) fn variants(&self, selected: Option<&ModelRef>) -> &[String] {
        selected
            .and_then(|selected| {
                self.models
                    .iter()
                    .find(|model| &model.reference == selected)
            })
            .map_or(&[], |model| model.variants.as_slice())
    }
}

impl ComposerSelection {
    pub(super) fn initialize(&mut self, catalog: &ComposerCatalog, timeline: &TimelineState) {
        let previous = last_identity(timeline);
        if !self.explicit
            || self
                .agent
                .as_ref()
                .is_none_or(|selected| !catalog.agents.iter().any(|agent| &agent.name == selected))
        {
            self.agent = previous
                .as_ref()
                .map(|(agent, _)| agent)
                .filter(|agent| catalog.agents.iter().any(|item| &item.name == *agent))
                .cloned()
                .or_else(|| catalog.agents.first().map(|agent| agent.name.clone()));
        }
        if !self.explicit
            || self.model.as_ref().is_none_or(|selected| {
                !catalog
                    .models
                    .iter()
                    .any(|model| &model.reference == selected)
            })
        {
            self.model = previous
                .map(|(_, model)| model)
                .filter(|selected| {
                    catalog
                        .models
                        .iter()
                        .any(|model| &model.reference == selected)
                })
                .or_else(|| catalog.default_model.clone())
                .or_else(|| catalog.models.first().map(|model| model.reference.clone()));
            self.variant = None;
        }
        if self
            .variant
            .as_ref()
            .is_some_and(|variant| !catalog.variants(self.model.as_ref()).contains(variant))
        {
            self.variant = None;
        }
    }

    pub(super) fn prompt_identity(&self) -> Option<(String, ModelRef, Option<String>)> {
        Some((
            self.agent.clone()?,
            self.model.clone()?,
            self.variant.clone(),
        ))
    }
}

fn last_identity(timeline: &TimelineState) -> Option<(String, ModelRef)> {
    let TimelineState::Ready { messages, .. } = timeline else {
        return None;
    };
    messages
        .iter()
        .rev()
        .find_map(|message| match &message.info {
            Message::User(message) => Some((message.agent.clone(), message.model.clone())),
            Message::Assistant(_) => None,
        })
}

impl Workspace {
    pub(super) fn load_composer_catalog(&mut self, directory: &str, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        if tab.catalog_load.is_some() || matches!(tab.catalog, CatalogState::Ready(_)) {
            return;
        }
        tab.catalog = CatalogState::Loading;
        let client = tab.client.clone();
        let request = cx.background_spawn(async move {
            client
                .catalog()
                .await
                .map(ComposerCatalog::from_response)
                .map_err(|error| error.to_string())
        });
        let directory = directory.to_owned();
        tab.catalog_load = Some(cx.spawn(async move |workspace, cx| {
            let result = request.await;
            let _ = workspace.update(cx, |workspace, cx| {
                let Some(tab) = workspace
                    .tabs
                    .iter_mut()
                    .find(|tab| tab.directory == directory)
                else {
                    return;
                };
                tab.catalog_load = None;
                match result {
                    Ok(catalog) => {
                        tab.selection.initialize(&catalog, &tab.timeline);
                        tab.catalog = CatalogState::Ready(Arc::new(catalog));
                    }
                    Err(error) => tab.catalog = CatalogState::Failed(error),
                }
                if matches!(
                    workspace.overlay,
                    super::command_palette::Overlay::Selection(_)
                ) {
                    let query = workspace.selection_query.clone();
                    workspace.refresh_selection_suggestions(&query, cx);
                }
                cx.notify();
            });
        }));
    }
}
