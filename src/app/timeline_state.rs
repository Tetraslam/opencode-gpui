use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::{
    PartSelection, TimelineState, image_cache::CachedImage, inspector::PreparedPart,
    markdown_cache::CachedMarkdown, markdown_render_cache::MarkdownRenderCache,
};

impl TimelineState {
    pub(super) fn session_id(&self) -> Option<&str> {
        match self {
            Self::Empty => None,
            Self::Loading { session_id, .. }
            | Self::Ready { session_id, .. }
            | Self::Failed { session_id, .. } => Some(session_id),
        }
    }

    pub(super) fn title(&self) -> Option<gpui::SharedString> {
        match self {
            Self::Empty => None,
            Self::Loading { title, .. }
            | Self::Ready { title, .. }
            | Self::Failed { title, .. } => Some(title.clone()),
        }
    }
}

pub(super) struct RenderState<'a> {
    pub(super) expanded_parts: &'a HashSet<PartSelection>,
    pub(super) collapsed_parts: &'a HashSet<PartSelection>,
    pub(super) expand_diffs: bool,
    pub(super) selected_part: Option<&'a PartSelection>,
    pub(super) detail_cache: &'a HashMap<PartSelection, Arc<PreparedPart>>,
    pub(super) markdown_cache: &'a HashMap<PartSelection, CachedMarkdown>,
    pub(super) markdown_renders: &'a MarkdownRenderCache,
    pub(super) image_cache: &'a HashMap<PartSelection, CachedImage>,
    pub(super) directory: &'a str,
}
