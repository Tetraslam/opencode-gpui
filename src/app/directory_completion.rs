use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui::{AppContext, Context};

use super::Workspace;

impl Workspace {
    pub(super) fn refresh_directory_suggestions(&mut self, query: String, cx: &mut Context<Self>) {
        self.overlay_selection = 0;
        let roots = completion_roots(self.active_directory());
        let search = query.clone();
        let completion = cx.background_spawn(async move { complete_directories(&search, &roots) });
        self.directory_completion = Some(cx.spawn(async move |workspace, cx| {
            let suggestions = Arc::new(completion.await);
            let _ = workspace.update(cx, |workspace, cx| {
                if workspace.directory_editor.read(cx).text() == query {
                    workspace.directory_suggestions = suggestions;
                    cx.notify();
                }
            });
        }));
    }

    pub(super) fn directory_candidates(&self, query: &str) -> Vec<String> {
        let query = query.trim();
        let needle = query.to_lowercase();
        let mut seen = HashSet::new();
        self.known_directories()
            .into_iter()
            .filter(|directory| needle.is_empty() || directory.to_lowercase().contains(&needle))
            .chain(self.directory_suggestions.iter().cloned())
            .filter(|directory| seen.insert(directory.clone()))
            .take(60)
            .collect()
    }
}

fn completion_roots(active: Option<&str>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(parent) = active.and_then(|directory| Path::new(directory).parent()) {
        roots.push(parent.to_owned());
    }
    if let Some(home) = env::var_os("HOME") {
        roots.push(PathBuf::from(home));
    }
    if let Ok(current) = env::current_dir() {
        roots.push(current);
    }
    roots.dedup();
    roots
}

fn complete_directories(query: &str, roots: &[PathBuf]) -> Vec<String> {
    let query = query.trim();
    let explicit = query.starts_with(['/', '~', '.']) || query.contains('/');
    let candidates = if explicit {
        vec![(PathBuf::from(expand_home(query)), query.ends_with('/'))]
    } else {
        roots
            .iter()
            .map(|root| (root.join(query), query.is_empty()))
            .collect()
    };
    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .flat_map(|(path, browse)| complete_path(&path, browse))
        .filter(|path| seen.insert(path.clone()))
        .take(60)
        .collect()
}

fn complete_path(path: &Path, browse: bool) -> Vec<String> {
    let (parent, prefix) = if browse {
        (path, "")
    } else {
        (
            path.parent().unwrap_or_else(|| Path::new(".")),
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(""),
        )
    };
    let mut matches = Vec::new();
    if path.is_dir() {
        matches.push(display_path(path));
    }
    if let Ok(entries) = fs::read_dir(parent) {
        matches.extend(entries.flatten().filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            if !name.starts_with(prefix) || (!prefix.starts_with('.') && name.starts_with('.')) {
                return None;
            }
            entry
                .file_type()
                .ok()?
                .is_dir()
                .then(|| display_path(&entry.path()))
        }));
    }
    matches.sort_unstable();
    matches.dedup();
    matches
}

fn expand_home(path: &str) -> String {
    if path == "~"
        && let Some(home) = env::var_os("HOME")
    {
        return PathBuf::from(home).to_string_lossy().into_owned();
    }
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = env::var_os("HOME")
    {
        return PathBuf::from(home)
            .join(rest)
            .to_string_lossy()
            .into_owned();
    }
    path.to_owned()
}

fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    let Some(home) = env::var_os("HOME") else {
        return value.into_owned();
    };
    let home = PathBuf::from(home);
    path.strip_prefix(&home).map_or_else(
        |_| value.into_owned(),
        |relative| format!("~/{}", relative.to_string_lossy()),
    )
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn completes_only_matching_visible_directories() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must follow epoch")
            .as_nanos();
        let root = env::temp_dir().join(format!("opencode-gpui-completion-{unique}"));
        fs::create_dir_all(root.join("alpha")).expect("create alpha");
        fs::create_dir_all(root.join("alpine")).expect("create alpine");
        fs::create_dir_all(root.join(".also-hidden")).expect("create hidden directory");
        fs::write(root.join("almanac"), b"file").expect("create file");

        let matches = complete_directories(&format!("{}/al", root.display()), &[]);

        assert_eq!(
            matches,
            [
                root.join("alpha").to_string_lossy(),
                root.join("alpine").to_string_lossy()
            ]
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn expands_home_directory_itself() {
        let Some(home) = env::var_os("HOME") else {
            return;
        };
        assert_eq!(expand_home("~"), PathBuf::from(home).to_string_lossy());
    }

    #[test]
    fn bare_queries_search_the_active_parent() {
        let root = env::temp_dir().join("opencode-gpui-sibling-completion");
        let project = root.join("current");
        fs::create_dir_all(&project).expect("create current project");
        fs::create_dir_all(root.join("mal-sync")).expect("create sibling project");

        let matches = complete_directories("mal", std::slice::from_ref(&root));

        assert_eq!(matches, [root.join("mal-sync").to_string_lossy()]);
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
