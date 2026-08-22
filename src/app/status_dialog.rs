use std::sync::Arc;

use opencode_gpui::api::{McpStatus, StatusSnapshot};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StatusTarget {
    pub(super) directory: String,
    pub(super) session_id: Option<String>,
}

#[derive(Default)]
pub(crate) struct StatusDialogState {
    target: Option<StatusTarget>,
    generation: u64,
    pub(super) loading: bool,
    pub(super) snapshot: Option<Arc<StatusSnapshot>>,
    pub(super) error: Option<String>,
    pub(super) mcp_names: Vec<String>,
    pub(super) selected: usize,
    pub(super) pending: Option<PendingMcp>,
    pub(super) action_error: Option<String>,
    operation_generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum McpOperation {
    Connect,
    Disconnect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PendingMcp {
    pub(super) name: String,
    pub(super) operation: McpOperation,
    generation: u64,
    refresh_generation: Option<u64>,
}

impl StatusDialogState {
    pub(super) fn reset_for_open(&mut self) {
        self.operation_generation = self.operation_generation.wrapping_add(1);
        self.pending = None;
        self.action_error = None;
    }

    pub(super) fn begin(&mut self, target: StatusTarget) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        if self.target.as_ref() != Some(&target) {
            self.snapshot = None;
            self.mcp_names.clear();
            self.selected = 0;
        }
        self.error = None;
        self.target = Some(target);
        self.loading = true;
        self.generation
    }

    pub(super) fn apply(
        &mut self,
        target: &StatusTarget,
        generation: u64,
        result: Result<StatusSnapshot, String>,
    ) -> bool {
        if self.generation != generation || self.target.as_ref() != Some(target) {
            return false;
        }
        self.loading = false;
        let operation_refresh = self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.refresh_generation == Some(generation));
        match result {
            Ok(snapshot) => {
                let changed = self.snapshot.as_deref() != Some(&snapshot)
                    || self.error.is_some()
                    || operation_refresh;
                self.reconcile_mcp_names(&snapshot);
                self.snapshot = Some(Arc::new(snapshot));
                self.error = None;
                if operation_refresh {
                    self.pending = None;
                    self.action_error = None;
                }
                changed
            }
            Err(error) => {
                let changed = self.error.as_deref() != Some(&error) || operation_refresh;
                self.error = Some(error);
                if operation_refresh {
                    self.pending = None;
                }
                changed
            }
        }
    }

    fn reconcile_mcp_names(&mut self, snapshot: &StatusSnapshot) {
        let selected = self.mcp_names.get(self.selected).cloned();
        self.mcp_names = snapshot.mcp.keys().cloned().collect();
        self.mcp_names.sort_unstable();
        self.selected = selected
            .and_then(|name| self.mcp_names.binary_search(&name).ok())
            .unwrap_or_else(|| self.selected.min(self.mcp_names.len().saturating_sub(1)));
    }

    pub(super) fn move_selection(&mut self, delta: isize) -> bool {
        let count = self.mcp_names.len();
        if count == 0 {
            return false;
        }
        self.selected =
            usize::try_from((self.selected.cast_signed() + delta).rem_euclid(count.cast_signed()))
                .unwrap_or_default();
        true
    }

    pub(super) fn select(&mut self, index: usize) {
        if index < self.mcp_names.len() {
            self.selected = index;
        }
    }

    pub(super) fn start_operation(&mut self, name: String, operation: McpOperation) -> Option<u64> {
        if self.pending.is_some() {
            return None;
        }
        self.operation_generation = self.operation_generation.wrapping_add(1);
        let generation = self.operation_generation;
        self.pending = Some(PendingMcp {
            name,
            operation,
            generation,
            refresh_generation: None,
        });
        self.action_error = None;
        Some(generation)
    }

    pub(super) fn operation_is_current(&self, generation: u64, name: &str) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| pending.generation == generation && pending.name == name)
    }

    pub(super) fn set_operation_refresh(&mut self, generation: u64, refresh: u64) {
        if let Some(pending) = self.pending.as_mut()
            && pending.generation == generation
        {
            pending.refresh_generation = Some(refresh);
        }
    }

    pub(super) fn fail_operation(&mut self, generation: u64, error: String) -> bool {
        if self
            .pending
            .as_ref()
            .is_none_or(|pending| pending.generation != generation)
        {
            return false;
        }
        self.pending = None;
        self.action_error = Some(error);
        true
    }
}

pub(super) const fn mcp_operation(status: &McpStatus) -> McpOperation {
    match status {
        McpStatus::Connected => McpOperation::Disconnect,
        McpStatus::Disabled
        | McpStatus::Failed { .. }
        | McpStatus::NeedsAuth
        | McpStatus::NeedsClientRegistration { .. }
        | McpStatus::Unknown { .. } => McpOperation::Connect,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PluginDisplay {
    pub(super) name: String,
    pub(super) version: Option<String>,
    pub(super) path: Option<String>,
}

pub(super) fn plugin_display(value: &serde_json::Value) -> PluginDisplay {
    let candidate = match value {
        serde_json::Value::String(value) => return parse_plugin_string(value),
        serde_json::Value::Array(items) => items.first().and_then(serde_json::Value::as_str),
        serde_json::Value::Object(fields) => {
            if let Some(path) = ["path", "url"]
                .iter()
                .find_map(|key| fields.get(*key).and_then(serde_json::Value::as_str))
            {
                let mut plugin = parse_plugin_string(path);
                if let Some(name) = fields.get("name").and_then(serde_json::Value::as_str) {
                    name.clone_into(&mut plugin.name);
                }
                plugin.version = fields
                    .get("version")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned);
                return plugin;
            }
            if let Some(name) = fields.get("name").and_then(serde_json::Value::as_str) {
                return PluginDisplay {
                    name: name.to_owned(),
                    version: fields
                        .get("version")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                    path: None,
                };
            }
            None
        }
        _ => None,
    };
    candidate.map_or_else(
        || PluginDisplay {
            name: "unrecognized plugin".into(),
            version: None,
            path: Some(value.to_string()),
        },
        parse_plugin_string,
    )
}

fn parse_plugin_string(value: &str) -> PluginDisplay {
    if let Ok(url) = url::Url::parse(value)
        && url.scheme() == "file"
    {
        let path = url
            .to_file_path()
            .unwrap_or_else(|()| std::path::PathBuf::from(url.path()));
        let name = plugin_path_name(&path);
        return PluginDisplay {
            name,
            version: None,
            path: Some(path.to_string_lossy().into_owned()),
        };
    }
    let path = std::path::Path::new(value);
    if path.is_absolute() || value.starts_with("./") || value.starts_with("../") {
        return PluginDisplay {
            name: plugin_path_name(path),
            version: None,
            path: Some(value.to_owned()),
        };
    }
    let version_index = value.rfind('@').filter(|index| *index > 0);
    let (name, version) = version_index.map_or((value, None), |index| {
        (&value[..index], Some(value[index + 1..].to_owned()))
    });
    PluginDisplay {
        name: name.to_owned(),
        version,
        path: None,
    }
}

fn plugin_path_name(path: &std::path::Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("plugin");
    if stem == "index" {
        path.parent()
            .and_then(std::path::Path::file_name)
            .and_then(|value| value.to_str())
            .unwrap_or(stem)
            .to_owned()
    } else {
        stem.to_owned()
    }
}

pub(super) fn mcp_status_label(server: &str, status: &McpStatus) -> String {
    match status {
        McpStatus::Connected => "Connected".into(),
        McpStatus::Disabled => "Disabled in configuration".into(),
        McpStatus::Failed { error } if error.is_empty() => "Failed (check server logs)".into(),
        McpStatus::NeedsAuth => format!("Needs authentication (run: opencode mcp auth {server})"),
        McpStatus::NeedsClientRegistration { error } if error.is_empty() => {
            "Needs client registration (check MCP configuration)".into()
        }
        McpStatus::Failed { error } | McpStatus::NeedsClientRegistration { error } => error.clone(),
        McpStatus::Unknown { status, detail } => detail
            .as_ref()
            .map_or_else(|| status.clone(), |detail| format!("{status}: {detail}")),
    }
}
