use std::{collections::HashMap, sync::Arc};

use gpui::{AppContext, Context, Image};

use super::{PartSelection, TimelineState, Workspace, image_attachment::decode_data_url};

pub(super) struct CachedImage {
    pub(super) source: String,
    pub(super) image: Arc<Image>,
}

#[derive(Default)]
pub(super) struct ImageCache {
    pub(super) images: HashMap<PartSelection, CachedImage>,
    pending: HashMap<PartSelection, String>,
}

impl Workspace {
    pub(super) fn refresh_image_cache(&mut self, directory: &str, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        let TimelineState::Ready { messages, .. } = &tab.timeline else {
            return;
        };
        let mut changed = Vec::new();
        for message in messages {
            for part in &message.parts {
                let mime = part.data.get("mime").and_then(serde_json::Value::as_str);
                let source = part.data.get("url").and_then(serde_json::Value::as_str);
                let (Some(mime), Some(source)) = (mime, source) else {
                    continue;
                };
                if part.kind != "file"
                    || !mime.starts_with("image/")
                    || !source.starts_with("data:")
                {
                    continue;
                }
                let selection = PartSelection {
                    message_id: part.message_id.clone(),
                    part_id: part.id.clone(),
                };
                let current = tab
                    .images
                    .images
                    .get(&selection)
                    .is_some_and(|cached| cached.source == source);
                if current {
                    continue;
                }
                let source = source.to_owned();
                if let Some(pending) = tab.images.pending.get_mut(&selection) {
                    *pending = source;
                    continue;
                }
                tab.images.pending.insert(selection.clone(), source.clone());
                changed.push((selection, source));
            }
        }
        for (selection, source) in changed {
            let encoded = source.clone();
            let decode = cx.background_spawn(async move { decode_data_url(&encoded) });
            let task_directory = directory.to_owned();
            cx.spawn(async move |workspace, cx| {
                let image = decode.await;
                let _ = workspace.update(cx, |workspace, cx| {
                    let stale = {
                        let Some(tab) = workspace
                            .tabs
                            .iter_mut()
                            .find(|tab| tab.directory == task_directory)
                        else {
                            return;
                        };
                        let current = tab.images.pending.get(&selection) == Some(&source);
                        tab.images.pending.remove(&selection);
                        if current && let Some(image) = image {
                            tab.images
                                .images
                                .insert(selection, CachedImage { source, image });
                        }
                        !current
                    };
                    if stale {
                        workspace.refresh_image_cache(&task_directory, cx);
                    }
                    cx.notify();
                });
            })
            .detach();
        }
    }
}
