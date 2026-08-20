use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use gpui::{AppContext, Context};
use serde::{Deserialize, Serialize};

use super::Workspace;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct WorkspaceLayout {
    pub(super) directories: Vec<String>,
    pub(super) active_directory: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct LayoutFile {
    version: u8,
    layout: WorkspaceLayout,
}

impl Workspace {
    pub(super) fn persist_workspace_layout(&mut self, cx: &mut Context<Self>) {
        let layout = WorkspaceLayout {
            directories: self.tabs.iter().map(|tab| tab.directory.clone()).collect(),
            active_directory: self.active_directory().map(str::to_owned),
        };
        let path = self.layout_path.clone();
        let timer = cx
            .background_executor()
            .timer(std::time::Duration::from_millis(120));
        self.layout_save = Some(cx.background_spawn(async move {
            timer.await;
            if let Err(error) = write_to(&path, &layout) {
                eprintln!("workspace layout persistence failed: {error}");
            }
        }));
    }
}

pub(super) fn load() -> Option<WorkspaceLayout> {
    load_from(&path())
}

fn load_from(path: &Path) -> Option<WorkspaceLayout> {
    let content = fs::read_to_string(path).ok()?;
    let file = serde_json::from_str::<LayoutFile>(&content).ok()?;
    (file.version == 1).then_some(file.layout)
}

fn write_to(path: &Path, layout: &WorkspaceLayout) -> std::io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent)?;
    let content = serde_json::to_vec_pretty(&LayoutFile {
        version: 1,
        layout: layout.clone(),
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
    fs::rename(temporary, path)?;
    fs::File::open(parent)?.sync_all()
}

pub(super) fn path() -> PathBuf {
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
        .join("opencode-gpui/workspace.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomically_round_trips_workspace_layout() {
        let root = env::temp_dir().join(format!(
            "opencode-gpui-layout-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let path = root.join("workspace.json");
        let layout = WorkspaceLayout {
            directories: vec!["/work/one".into(), "/work/two".into()],
            active_directory: Some("/work/one".into()),
        };
        write_to(&path, &layout).expect("write layout");
        assert_eq!(load_from(&path), Some(layout));
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
