use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use gpui::{AppContext, Context, Image, ImageFormat};
use opencode_gpui::{
    markdown::RenderRequest,
    rich_render::{self, RenderedSvg},
};

use super::Workspace;

pub(super) struct RenderAsset {
    pub(super) image: Arc<Image>,
    pub(super) width: f32,
    pub(super) height: f32,
}

enum CachedRender {
    Ready(RenderAsset),
    Failed,
}

#[derive(Default)]
pub(super) struct MarkdownRenderCache {
    entries: HashMap<RenderRequest, CachedRender>,
    pending: HashSet<RenderRequest>,
}

impl MarkdownRenderCache {
    pub(super) fn asset(&self, request: &RenderRequest) -> Option<&RenderAsset> {
        match self.entries.get(request) {
            Some(CachedRender::Ready(asset)) => Some(asset),
            Some(CachedRender::Failed) | None => None,
        }
    }
}

impl Workspace {
    pub(super) fn refresh_markdown_renders(&mut self, directory: &str, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        let requested = tab
            .markdown
            .documents
            .values()
            .flat_map(|cached| cached.document.render_requests())
            .collect::<HashSet<_>>();
        tab.markdown_renders
            .entries
            .retain(|request, _| requested.contains(request));
        tab.markdown_renders
            .pending
            .retain(|request| requested.contains(request));
        let changed = requested
            .into_iter()
            .filter(|request| {
                !tab.markdown_renders.entries.contains_key(request)
                    && tab.markdown_renders.pending.insert(request.clone())
            })
            .collect::<Vec<_>>();

        for request in changed {
            let render_request = request.clone();
            let render =
                cx.background_spawn(
                    async move { rich_render::render(&render_request).map(to_asset) },
                );
            let task_directory = directory.to_owned();
            cx.spawn(async move |workspace, cx| {
                let result = render.await;
                let _ = workspace.update(cx, |workspace, cx| {
                    let Some(tab) = workspace
                        .tabs
                        .iter_mut()
                        .find(|tab| tab.directory == task_directory)
                    else {
                        return;
                    };
                    if !tab.markdown_renders.pending.remove(&request) {
                        return;
                    }
                    let cached = result.map_or(CachedRender::Failed, CachedRender::Ready);
                    tab.markdown_renders.entries.insert(request, cached);
                    cx.notify();
                });
            })
            .detach();
        }
    }
}

fn to_asset(svg: RenderedSvg) -> RenderAsset {
    RenderAsset {
        image: Arc::new(Image::from_bytes(ImageFormat::Svg, svg.bytes)),
        width: svg.width,
        height: svg.height,
    }
}
