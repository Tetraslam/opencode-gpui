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
        self.reset_picker_scroll();
        self.directory_suggestion_query.clone_from(&query);
        self.directory_suggestions = Arc::new(Vec::new());
        let roots = completion_roots(self.active_directory());
        let search = query.clone();
        let completion = cx.background_spawn(async move { complete_directories(&search, &roots) });
        self.directory_completion = Some(cx.spawn(async move |workspace, cx| {
            let suggestions = completion.await;
            let _ = workspace.update(cx, |workspace, cx| {
                if workspace.directory_editor.read(cx).text() == query {
                    workspace.directory_suggestions =
                        Arc::new(workspace.merge_directory_candidates(&query, suggestions));
                    workspace.reset_picker_scroll();
                    cx.notify();
                }
            });
        }));
    }

    fn merge_directory_candidates(&self, query: &str, suggestions: Vec<String>) -> Vec<String> {
        let query = query.trim();
        let needle = query.to_lowercase();
        let mut seen = HashSet::new();
        self.known_directories()
            .into_iter()
            .filter(|directory| needle.is_empty() || directory.to_lowercase().contains(&needle))
            .chain(suggestions)
            .filter(|directory| seen.insert(directory.clone()))
            .take(60)
            .collect()
    }

    pub(super) fn complete_directory_picker(&mut self, cx: &mut Context<Self>) {
        if self.overlay != super::command_palette::Overlay::Directory {
            return;
        }
        let query = self.directory_editor.read(cx).text().to_owned();
        if query == self.directory_suggestion_query
            && let Some(completion) = path_completion(&query, &self.directory_suggestions)
        {
            self.apply_directory_completion(&completion, cx);
            return;
        }
        let roots = completion_roots(self.active_directory());
        let search = query.clone();
        let completion = cx.background_spawn(async move { complete_directories(&search, &roots) });
        self.directory_completion = Some(cx.spawn(async move |workspace, cx| {
            let suggestions = completion.await;
            let _ = workspace.update(cx, |workspace, cx| {
                if workspace.overlay != super::command_palette::Overlay::Directory
                    || workspace.directory_editor.read(cx).text() != query
                {
                    return;
                }
                let suggestions = workspace.merge_directory_candidates(&query, suggestions);
                if let Some(completion) = path_completion(&query, &suggestions) {
                    workspace.apply_directory_completion(&completion, cx);
                }
            });
        }));
    }

    fn apply_directory_completion(&mut self, completion: &str, cx: &mut Context<Self>) {
        let length = self.directory_editor.read(cx).text().len();
        self.directory_editor.update(cx, |editor, cx| {
            editor.replace_range(0..length, completion, cx);
        });
    }
}

fn path_completion(query: &str, candidates: &[String]) -> Option<String> {
    let query = query.trim();
    let explicit = query.starts_with(['/', '~', '.']) || query.contains('/');
    let mut matching = candidates.iter().filter_map(|candidate| {
        let value = if explicit {
            candidate.as_str()
        } else {
            Path::new(candidate).file_name()?.to_str()?
        };
        value.starts_with(query).then_some(value)
    });
    let first = matching.next()?;
    let Some(second) = matching.next() else {
        return Some(format!("{}/", first.trim_end_matches('/')));
    };
    let mut prefix = common_prefix(first, second);
    for candidate in matching {
        prefix = common_prefix(&prefix, candidate);
    }
    (prefix.len() > query.len()).then_some(prefix)
}

fn common_prefix(left: &str, right: &str) -> String {
    for ((index, left_char), right_char) in left.char_indices().zip(right.chars()) {
        if left_char != right_char {
            return left[..index].to_owned();
        }
    }
    left[..left.len().min(right.len())].to_owned()
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

    #[test]
    fn tab_completion_extends_common_prefix_then_finishes_unique_directory() {
        assert_eq!(
            path_completion("/work/al", &["/work/alpha".into(), "/work/alpine".into()]),
            Some("/work/alp".into())
        );
        assert_eq!(
            path_completion("/work/alph", &["/work/alpha".into()]),
            Some("/work/alpha/".into())
        );
    }

    #[test]
    fn bare_tab_completion_preserves_shell_style_input() {
        assert_eq!(
            path_completion("al", &["/work/alpha".into(), "/tmp/alpine".into()]),
            Some("alp".into())
        );
        assert_eq!(path_completion("alp", &[]), None);
    }
}
