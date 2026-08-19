use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::{
    PartSelection, image_cache::CachedImage, inspector::PreparedPart,
    markdown_cache::CachedMarkdown,
};

pub(super) struct RenderState<'a> {
    pub(super) expanded_parts: &'a HashSet<PartSelection>,
    pub(super) selected_part: Option<&'a PartSelection>,
    pub(super) detail_cache: &'a HashMap<PartSelection, Arc<PreparedPart>>,
    pub(super) markdown_cache: &'a HashMap<PartSelection, CachedMarkdown>,
    pub(super) image_cache: &'a HashMap<PartSelection, CachedImage>,
    pub(super) directory: &'a str,
}
