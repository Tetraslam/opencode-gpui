use std::{
    collections::HashMap,
    env, fs,
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use super::{
    draft_store::{DraftKey, SessionDraft},
    image_attachment::{PromptImage, decode_image},
};

static NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Serialize, Deserialize)]
struct DraftFile {
    version: u8,
    drafts: Vec<PersistedDraft>,
}

#[derive(Serialize, Deserialize)]
struct PersistedDraft {
    directory: String,
    session_id: String,
    text: String,
    attached_files: Vec<String>,
    #[serde(default)]
    attached_images: Vec<PersistedImage>,
    #[serde(default)]
    prompt_mode: super::prompt_mode::PromptMode,
    updated_at: u64,
}

#[derive(Serialize, Deserialize)]
struct PersistedImage {
    id: String,
    filename: String,
    mime: String,
    data: String,
}

pub(super) fn load_drafts() -> HashMap<DraftKey, SessionDraft> {
    load_drafts_from(&draft_path())
}

fn load_drafts_from(path: &std::path::Path) -> HashMap<DraftKey, SessionDraft> {
    let Ok(content) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    let Ok(file) = serde_json::from_str::<DraftFile>(&content) else {
        quarantine(path);
        return HashMap::new();
    };
    if file.version != 1 {
        quarantine(path);
        return HashMap::new();
    }
    file.drafts
        .into_iter()
        .map(|draft| {
            let images = draft
                .attached_images
                .into_iter()
                .filter_map(|image| {
                    Some(PromptImage {
                        id: image.id,
                        filename: image.filename,
                        image: decode_image(&image.mime, &image.data)?,
                        data_url: Some(format!("data:{};base64,{}", image.mime, image.data).into()),
                    })
                })
                .collect();
            (
                (draft.directory, draft.session_id),
                SessionDraft {
                    text: draft.text,
                    attached_files: draft.attached_files.into_iter().collect(),
                    attached_images: images,
                    prompt_mode: draft.prompt_mode,
                    updated_at: draft.updated_at,
                },
            )
        })
        .collect()
}

pub(super) fn write_drafts_to(
    path: &std::path::Path,
    drafts: HashMap<DraftKey, SessionDraft>,
) -> std::io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent)?;
    let mut drafts = drafts
        .into_iter()
        .map(|((directory, session_id), draft)| PersistedDraft {
            directory,
            session_id,
            text: draft.text,
            attached_files: draft.attached_files.into_iter().collect(),
            attached_images: draft
                .attached_images
                .into_iter()
                .map(|image| {
                    let mime = image.mime().to_owned();
                    PersistedImage {
                        id: image.id,
                        filename: image.filename,
                        mime,
                        data: STANDARD.encode(&image.image.bytes),
                    }
                })
                .collect(),
            prompt_mode: draft.prompt_mode,
            updated_at: draft.updated_at,
        })
        .collect::<Vec<_>>();
    drafts.sort_unstable_by_key(|draft| std::cmp::Reverse(draft.updated_at));
    drafts.truncate(200);
    let content = serde_json::to_vec_pretty(&DraftFile { version: 1, drafts })?;
    atomic_write(path, &content)
}

fn atomic_write(path: &std::path::Path, content: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().expect("draft path has a parent");
    let temporary = path.with_extension(format!("tmp-{}-{}", std::process::id(), next_nonce()));
    let mut options = fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(content)?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    fs::File::open(parent)?.sync_all()
}

fn quarantine(path: &std::path::Path) {
    let _ = fs::rename(
        path,
        path.with_extension(format!("corrupt-{}", now_millis())),
    );
}

pub(super) fn draft_path() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map_or_else(
            || {
                env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_default()
                    .join(".config")
            },
            PathBuf::from,
        )
        .join("opencode-gpui/drafts.json")
}

pub(super) fn now_millis() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}

pub(super) fn next_nonce() -> u64 {
    NONCE.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Image, ImageFormat};
    use std::collections::HashSet;

    #[test]
    fn atomically_round_trips_session_drafts() {
        let root = env::temp_dir().join(format!("opencode-gpui-drafts-{}", now_millis()));
        let path = root.join("drafts.json");
        let key = ("/work/project".into(), "ses_test".into());
        let drafts = HashMap::from([(
            key.clone(),
            SessionDraft {
                text: "review @src/main.rs".into(),
                attached_files: HashSet::from(["src/main.rs".into()]),
                attached_images: vec![PromptImage {
                    id: "image-1".into(),
                    filename: "clipboard-1.png".into(),
                    image: std::sync::Arc::new(Image::from_bytes(ImageFormat::Png, vec![1, 2, 3])),
                    data_url: Some("data:image/png;base64,AQID".into()),
                }],
                prompt_mode: super::super::prompt_mode::PromptMode::Shell,
                updated_at: 42,
            },
        )]);
        write_drafts_to(&path, drafts).expect("write drafts");
        let loaded = load_drafts_from(&path);
        assert_eq!(loaded[&key].text, "review @src/main.rs");
        assert!(loaded[&key].attached_files.contains("src/main.rs"));
        assert_eq!(loaded[&key].attached_images[0].image.bytes, [1, 2, 3]);
        assert_eq!(
            loaded[&key].prompt_mode,
            super::super::prompt_mode::PromptMode::Shell
        );
        assert_eq!(fs::read_dir(&root).expect("read fixture").count(), 1);
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
