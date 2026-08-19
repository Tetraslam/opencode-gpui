use super::Workspace;

impl Workspace {
    pub(super) fn active_directory(&self) -> Option<&str> {
        self.active_tab().map(|tab| tab.directory.as_str())
    }
}

pub(super) fn directory_name(directory: &str) -> &str {
    directory
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(directory)
}

pub(super) fn normalize_directory(path: &str) -> String {
    let trimmed = path.trim().trim_end_matches('/');
    if let Some(rest) = trimmed.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{home}/{rest}");
    }
    trimmed.to_owned()
}
