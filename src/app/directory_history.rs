use std::{
    collections::HashMap,
    env, fs,
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use gpui::{AppContext, Context};
use opencode_gpui::model::Session;
use serde::{Deserialize, Serialize};

use super::Workspace;

#[derive(Serialize, Deserialize)]
struct HistoryFile {
    version: u8,
    directories: HashMap<String, u64>,
}

impl Workspace {
    pub(super) fn record_directory_open(&mut self, directory: &str, cx: &mut Context<Self>) {
        self.directory_history
            .insert(directory.to_owned(), now_millis());
        self.persist_directory_history(cx);
    }

    pub(super) fn merge_server_directory_history(
        &mut self,
        sessions: &[Session],
        cx: &mut Context<Self>,
    ) {
        for session in sessions {
            self.directory_history
                .entry(session.directory.clone())
                .and_modify(|updated| *updated = (*updated).max(session.time.updated))
                .or_insert(session.time.updated);
        }
        self.persist_directory_history(cx);
    }

    fn persist_directory_history(&mut self, cx: &mut Context<Self>) {
        let mut directories = self
            .directory_history
            .iter()
            .map(|(path, updated)| (path.clone(), *updated))
            .collect::<Vec<_>>();
        directories.sort_unstable_by_key(|(_, updated)| std::cmp::Reverse(*updated));
        directories.truncate(1_000);
        let directories = directories.into_iter().collect();
        self.directory_history_save = Some(cx.background_spawn(async move {
            if let Err(error) = write_history(directories) {
                eprintln!("directory history persistence failed: {error}");
            }
        }));
    }
}

pub(super) fn load_directory_history() -> HashMap<String, u64> {
    let Ok(content) = fs::read_to_string(history_path()) else {
        return HashMap::new();
    };
    serde_json::from_str::<HistoryFile>(&content).map_or_else(
        |_| HashMap::new(),
        |history| {
            if history.version == 1 {
                history.directories
            } else {
                HashMap::new()
            }
        },
    )
}

fn write_history(directories: HashMap<String, u64>) -> std::io::Result<()> {
    let path = history_path();
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent)?;
    let content = serde_json::to_vec_pretty(&HistoryFile {
        version: 1,
        directories,
    })?;
    let temporary = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let mut options = fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(&content)?;
    file.sync_all()?;
    fs::rename(temporary, &path)?;
    fs::File::open(parent)?.sync_all()
}

fn history_path() -> PathBuf {
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
        .join("opencode-gpui/directory-history.json")
}

fn now_millis() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}
