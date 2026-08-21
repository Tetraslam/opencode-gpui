use std::collections::{BTreeMap, HashMap};

use opencode_gpui::api::{
    AgentCatalogEntry, CatalogResponse, ModelCatalogEntry, ProviderCatalog, ProviderCatalogEntry,
};
use opencode_gpui::model::ModelRef;

use super::*;
use crate::app::composer_catalog::{ComposerCatalog, ComposerSelection};
use crate::app::composer_completion::LocalSlash;
use crate::app::composer_slashes;
use crate::app::selection_overlay::{SelectionItem, SelectionKind};

#[test]
fn catalog_filters_non_composer_entries_and_exposes_model_variants() {
    let catalog = ComposerCatalog::from_response(catalog_response());
    assert_eq!(
        catalog
            .agents
            .iter()
            .map(|agent| agent.name.as_str())
            .collect::<Vec<_>>(),
        ["build"]
    );
    assert_eq!(catalog.models.len(), 1);
    assert_eq!(catalog.models[0].reference.provider_id, "connected");
    assert_eq!(catalog.models[0].variants, ["high"]);
}

#[test]
fn selection_initializes_from_real_catalog_and_tracks_variant() {
    let catalog = ComposerCatalog::from_response(catalog_response());
    let mut selection = ComposerSelection::default();
    selection.initialize(&catalog, &TimelineState::Empty);
    selection.variant = Some("high".into());
    let (agent, model, variant) = selection.prompt_identity().unwrap();
    assert_eq!(agent, "build");
    assert_eq!(model.provider_id, "connected");
    assert_eq!(model.model_id, "active");
    assert_eq!(variant.as_deref(), Some("high"));
}

#[test]
fn variant_picker_distinguishes_server_default_from_reasoning_off() {
    let mut response = catalog_response();
    response.providers.all[0]
        .models
        .get_mut("active")
        .unwrap()
        .variants = BTreeMap::from([
        ("none".into(), serde_json::json!({})),
        ("high".into(), serde_json::json!({})),
    ]);
    let catalog = ComposerCatalog::from_response(response);
    let model = catalog.models[0].reference.clone();
    let items = crate::app::selection_filter::filter_items(
        &catalog,
        SelectionKind::Variant,
        "",
        Some(&model),
    );
    assert!(matches!(items.first(), Some(SelectionItem::Variant(value)) if value.is_empty()));
    assert!(
        items
            .iter()
            .any(|item| matches!(item, SelectionItem::Variant(value) if value == "none"))
    );
}

#[gpui::test]
fn selecting_default_variant_omits_it_from_prompts(cx: &mut TestAppContext) {
    let workspace = workspace(cx, Vec::new(), TimelineState::Empty);
    workspace.update(cx, |workspace, cx| {
        workspace.tabs[0].selection.variant = Some("none".into());
        workspace.overlay = Overlay::Selection(SelectionKind::Variant);
        workspace.selection_suggestions = Arc::new(vec![SelectionItem::Variant(String::new())]);
        workspace.accept_selection(0, cx);
        assert_eq!(workspace.tabs[0].selection.variant, None);
        assert!(workspace.tabs[0].selection.explicit);
    });
}

#[gpui::test]
fn keyboard_selection_changes_model_and_clears_stale_variant(cx: &mut TestAppContext) {
    let workspace = workspace(cx, Vec::new(), TimelineState::Empty);
    workspace.update(cx, |workspace, cx| {
        workspace.tabs[0].selection.variant = Some("old".into());
        workspace.overlay = Overlay::Selection(SelectionKind::Model);
        workspace.selection_suggestions = Arc::new(vec![model_item("first"), model_item("second")]);
        workspace.move_overlay_selection(-1, cx);
        assert_eq!(workspace.overlay_selection, 1);
        workspace.accept_selection(1, cx);
        assert_eq!(
            workspace.tabs[0].selection.model.as_ref().unwrap().model_id,
            "second"
        );
        assert_eq!(workspace.tabs[0].selection.variant, None);
        assert_eq!(workspace.overlay, Overlay::None);
    });
}

#[test]
fn local_model_alias_resolves_to_the_real_selector() {
    assert_eq!(
        composer_slashes::local_slash("mo"),
        Some(LocalSlash::Models)
    );
}

fn model_item(id: &str) -> SelectionItem {
    SelectionItem::Model {
        reference: ModelRef {
            provider_id: "provider".into(),
            model_id: id.into(),
        },
        name: id.into(),
        provider: "Provider".into(),
    }
}

fn catalog_response() -> CatalogResponse {
    CatalogResponse {
        agents: vec![
            agent("build", "primary", false),
            agent("hidden", "primary", true),
            agent("explore", "subagent", false),
        ],
        providers: ProviderCatalog {
            all: vec![
                provider(
                    "connected",
                    vec![
                        model("connected", "active", "active", true),
                        model("connected", "old", "deprecated", false),
                    ],
                ),
                provider("offline", vec![model("offline", "unused", "active", false)]),
            ],
            default: HashMap::from([("connected".into(), "active".into())]),
            connected: vec!["connected".into()],
        },
    }
}

fn agent(name: &str, mode: &str, hidden: bool) -> AgentCatalogEntry {
    AgentCatalogEntry {
        name: name.into(),
        description: Some(format!("{name} agent")),
        mode: mode.into(),
        hidden: Some(hidden),
    }
}

fn provider(id: &str, models: Vec<ModelCatalogEntry>) -> ProviderCatalogEntry {
    ProviderCatalogEntry {
        id: id.into(),
        name: format!("{id} provider"),
        models: models
            .into_iter()
            .map(|model| (model.id.clone(), model))
            .collect(),
    }
}

fn model(provider_id: &str, id: &str, status: &str, with_variant: bool) -> ModelCatalogEntry {
    ModelCatalogEntry {
        id: id.into(),
        provider_id: provider_id.into(),
        name: format!("{id} model"),
        status: Some(status.into()),
        variants: if with_variant {
            BTreeMap::from([("high".into(), serde_json::json!({}))])
        } else {
            BTreeMap::new()
        },
    }
}
