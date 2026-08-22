use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Settings {
    #[serde(default = "enabled")]
    pub(super) expand_diffs: bool,
    #[serde(default = "enabled")]
    pub(super) animate_trace_entries: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            expand_diffs: true,
            animate_trace_entries: true,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct SettingsFile {
    version: u8,
    settings: Settings,
}

const fn enabled() -> bool {
    true
}

pub(super) fn load() -> Settings {
    let path = path();
    let Ok(contents) = fs::read_to_string(&path) else {
        return Settings::default();
    };
    serde_json::from_str::<SettingsFile>(&contents).map_or_else(
        |_| Settings::default(),
        |file| {
            if file.version == 1 {
                file.settings
            } else {
                Settings::default()
            }
        },
    )
}

pub(super) fn save(settings: &Settings) -> io::Result<()> {
    let path = path();
    let Some(parent) = path.parent() else {
        return Err(io::Error::other("settings path has no parent"));
    };
    fs::create_dir_all(parent)?;
    let bytes = serde_json::to_vec_pretty(&SettingsFile {
        version: 1,
        settings: settings.clone(),
    })?;
    atomic_write(&path, &bytes)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    fs::File::open(path.parent().expect("settings path has a parent"))?.sync_all()
}

fn path() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME").map_or_else(
        || {
            env::var_os("HOME").map_or_else(
                || PathBuf::from(".opencode-gpui"),
                |home| PathBuf::from(home).join(".config/opencode-gpui/settings.json"),
            )
        },
        |config| PathBuf::from(config).join("opencode-gpui/settings.json"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_keep_diff_expansion_enabled() {
        let file: SettingsFile = serde_json::from_str(r#"{"version":1,"settings":{}}"#).unwrap();
        assert!(file.settings.expand_diffs);
        assert!(file.settings.animate_trace_entries);
    }
}
