use std::sync::Arc;

use base64::{Engine, engine::general_purpose::STANDARD};
use gpui::{AppContext, Context, Image, ImageFormat};
use opencode_gpui::api::PromptFile;

use super::Workspace;

const MAX_IMAGES: usize = 8;
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 20 * 1024 * 1024;

#[derive(Clone, Debug)]
pub(super) struct PromptImage {
    pub(super) id: String,
    pub(super) filename: String,
    pub(super) image: Arc<Image>,
    pub(super) data_url: Option<Arc<str>>,
}

impl PromptImage {
    pub(super) fn mime(&self) -> &'static str {
        self.image.format.mime_type()
    }

    pub(super) fn as_prompt_file(&self) -> Option<PromptFile> {
        Some(PromptFile {
            mime: self.mime().into(),
            filename: self.filename.clone(),
            url: self.data_url.as_ref()?.to_string(),
        })
    }
}

impl Workspace {
    pub(super) fn attach_clipboard_image(
        &mut self,
        directory: &str,
        image: Image,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.directory == directory) else {
            return;
        };
        if image.bytes.is_empty() {
            tab.prompt_error = Some("clipboard image is empty".into());
        } else if image.bytes.len() > MAX_IMAGE_BYTES {
            tab.prompt_error = Some("clipboard image exceeds 10 mb".into());
        } else if tab.attached_images.len() >= MAX_IMAGES {
            tab.prompt_error = Some("a prompt can include up to 8 images".into());
        } else if tab
            .attached_images
            .iter()
            .map(|image| image.image.bytes.len())
            .sum::<usize>()
            .saturating_add(image.bytes.len())
            > MAX_TOTAL_BYTES
        {
            tab.prompt_error = Some("prompt images exceed 20 mb".into());
        } else {
            let number = tab.attached_images.len() + 1;
            let filename = format!("clipboard-{number}.{}", extension(image.format));
            let id = format!("{}-{}", image.id(), super::draft_persistence::next_nonce());
            let image = Arc::new(image);
            tab.attached_images.push(PromptImage {
                id: id.clone(),
                filename,
                image: image.clone(),
                data_url: None,
            });
            tab.prompt_error = None;
            self.capture_draft(directory, false, cx);
            let mime = image.format.mime_type();
            let prepare = cx.background_spawn(async move {
                Arc::<str>::from(format!(
                    "data:{mime};base64,{}",
                    STANDARD.encode(&image.bytes)
                ))
            });
            let task_directory = directory.to_owned();
            cx.spawn(async move |workspace, cx| {
                let data_url = prepare.await;
                let _ = workspace.update(cx, |workspace, cx| {
                    let prepared = workspace
                        .tabs
                        .iter_mut()
                        .find(|tab| tab.directory == task_directory)
                        .and_then(|tab| {
                            tab.attached_images.iter_mut().find(|image| image.id == id)
                        });
                    if let Some(image) = prepared {
                        image.data_url = Some(data_url);
                        workspace.capture_draft(&task_directory, false, cx);
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        cx.notify();
    }

    pub(super) fn remove_prompt_image(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab_mut() {
            tab.attached_images.retain(|image| image.id != id);
        }
        self.capture_active_draft(false, cx);
        cx.notify();
    }
}

pub(super) fn decode_image(mime: &str, data: &str) -> Option<Arc<Image>> {
    let format = ImageFormat::from_mime_type(mime)?;
    let bytes = STANDARD.decode(data).ok()?;
    Some(Arc::new(Image::from_bytes(format, bytes)))
}

pub(super) fn decode_data_url(url: &str) -> Option<Arc<Image>> {
    let (mime, data) = url.strip_prefix("data:")?.split_once(";base64,")?;
    decode_image(mime, data)
}

fn extension(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Webp => "webp",
        ImageFormat::Gif => "gif",
        ImageFormat::Svg => "svg",
        ImageFormat::Bmp => "bmp",
        ImageFormat::Tiff => "tiff",
    }
}
