use std::sync::Arc;

use opencode_gpui::model::Part;

pub(super) fn merge_part(parts: &mut Vec<Part>, mut incoming: Part, delta: Option<&str>) {
    let Some(current) = parts.iter_mut().find(|part| part.id == incoming.id) else {
        reconcile_file(parts, &incoming);
        if incoming.text().is_none_or(str::is_empty)
            && let Some(delta) = delta
        {
            Arc::make_mut(&mut incoming.data)
                .insert("text".into(), serde_json::Value::String(delta.into()));
        }
        parts.push(incoming);
        return;
    };
    let current_text = current.text().unwrap_or_default();
    let incoming_text = incoming.text().unwrap_or_default();
    if let Some(delta) = delta
        && !incoming_text.starts_with(current_text)
    {
        let mut text = current_text.to_owned();
        text.push_str(delta);
        Arc::make_mut(&mut incoming.data).insert("text".into(), serde_json::Value::String(text));
    }
    *current = incoming;
}

fn reconcile_file(parts: &mut Vec<Part>, incoming: &Part) {
    if incoming.kind != "file" {
        return;
    }
    let Some(url) = incoming.data.get("url").and_then(serde_json::Value::as_str) else {
        return;
    };
    parts.retain(|part| {
        part.kind != "file" || part.data.get("url").and_then(serde_json::Value::as_str) != Some(url)
    });
}

pub(super) fn append_part_field(part: &mut Part, field: &str, delta: &str) {
    let data = Arc::make_mut(&mut part.data);
    let mut value = data
        .get(field)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    value.push_str(delta);
    data.insert(field.to_owned(), serde_json::Value::String(value));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authoritative_file_replaces_optimistic_part_with_the_same_url() {
        let file = |id: &str| Part {
            id: id.into(),
            session_id: "ses".into(),
            message_id: "msg".into(),
            kind: "file".into(),
            data: Arc::new(serde_json::Map::from_iter([(
                "url".into(),
                serde_json::Value::String("data:image/png;base64,AQID".into()),
            )])),
        };
        let mut parts = vec![file("optimistic")];
        merge_part(&mut parts, file("server"), None);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].id, "server");
    }
}
