use opencode_gpui::model::ModelRef;

use super::{
    composer_catalog::ComposerCatalog,
    selection_overlay::{SelectionItem, SelectionKind},
};

pub(super) fn filter_items(
    catalog: &ComposerCatalog,
    kind: SelectionKind,
    query: &str,
    model: Option<&ModelRef>,
) -> Vec<SelectionItem> {
    match kind {
        SelectionKind::Agent => catalog
            .agents
            .iter()
            .filter(|agent| matches_query(&[&agent.name, &agent.description], query))
            .map(|agent| SelectionItem::Agent {
                name: agent.name.clone(),
                description: agent.description.clone(),
            })
            .take(100)
            .collect(),
        SelectionKind::Model => catalog
            .models
            .iter()
            .filter(|item| {
                matches_query(
                    &[
                        &item.name,
                        &item.provider_name,
                        &item.reference.provider_id,
                        &item.reference.model_id,
                    ],
                    query,
                )
            })
            .map(|item| SelectionItem::Model {
                reference: item.reference.clone(),
                name: item.name.clone(),
                provider: item.provider_name.clone(),
            })
            .take(100)
            .collect(),
        SelectionKind::Variant => catalog
            .variants(model)
            .first()
            .map(|_| String::new())
            .into_iter()
            .chain(catalog.variants(model).iter().cloned())
            .filter(|variant| {
                let label = if variant.is_empty() {
                    "default"
                } else {
                    variant
                };
                label.to_lowercase().contains(query)
            })
            .map(SelectionItem::Variant)
            .collect(),
    }
}

fn matches_query(values: &[&str], query: &str) -> bool {
    query.is_empty()
        || values
            .iter()
            .any(|value| value.to_lowercase().contains(query))
}
