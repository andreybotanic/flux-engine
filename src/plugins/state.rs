use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use bevy::prelude::Resource;
use serde::Deserialize;

use crate::plugins::{
    id::DEFAULT_PLUGIN_ID_VALUE, source::PluginSourceKind, PluginId, PluginVersion,
};

/// Schema version used by `plugin_state.toml`.
pub const PLUGIN_STATE_SCHEMA_VERSION: u32 = 1;

/// Persistent set of plugins enabled by the user.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct EnabledPluginSet {
    enabled_plugins: BTreeSet<PluginId>,
}

impl EnabledPluginSet {
    /// Returns `true` when the requested plugin is enabled.
    pub fn is_enabled(&self, plugin_id: &PluginId) -> bool {
        self.enabled_plugins.contains(plugin_id)
    }

    /// Enables or disables one plugin in the set.
    pub fn set_enabled(&mut self, plugin_id: &PluginId, enabled: bool) {
        if enabled {
            self.enabled_plugins.insert(plugin_id.clone());
        } else {
            self.enabled_plugins.remove(plugin_id);
        }
        self.enforce_default_plugin();
    }

    /// Re-enables the built-in default plugin if a caller removed it.
    pub fn enforce_default_plugin(&mut self) {
        self.enabled_plugins.insert(PluginId::default_plugin());
    }

    /// Returns the enabled plugin ids in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = &PluginId> {
        self.enabled_plugins.iter()
    }

    /// Loads `plugin_state.toml` from disk or falls back to the default set.
    pub fn load_from_path(path: &Path) -> Result<PluginStateLoadResult, String> {
        if !path.exists() {
            let mut enabled_set = Self::default();
            enabled_set.enforce_default_plugin();
            return Ok(PluginStateLoadResult {
                enabled_set,
                warnings: Vec::new(),
            });
        }

        let contents = fs::read_to_string(path).map_err(|error| {
            format!(
                "failed to read plugin state file '{}': {}",
                path.display(),
                error
            )
        })?;
        Self::from_toml_str(&contents)
    }

    /// Parses `plugin_state.toml` contents into the runtime enabled set.
    pub fn from_toml_str(value: &str) -> Result<PluginStateLoadResult, String> {
        let raw = toml::from_str::<RawPluginStateFile>(value)
            .map_err(|error| format!("failed to parse plugin_state.toml: {}", error))?;
        if raw.schema_version != PLUGIN_STATE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported plugin_state.toml schema_version '{}'; expected '{}'",
                raw.schema_version, PLUGIN_STATE_SCHEMA_VERSION
            ));
        }

        let default_plugin = PluginId::default_plugin();
        let mut enabled_set = Self::default();
        let mut warnings = Vec::new();
        for (raw_id, enabled) in flatten_plugin_state_table("", &raw.plugins)? {
            let plugin_id = PluginId::parse(&raw_id).map_err(|error| {
                format!(
                    "plugin_state.toml contains invalid plugin id '{}': {}",
                    raw_id, error
                )
            })?;
            if !enabled {
                if plugin_id == default_plugin {
                    warnings.push(format!(
                        "plugin_state.toml cannot disable '{}'; keeping it enabled",
                        DEFAULT_PLUGIN_ID_VALUE
                    ));
                }
                continue;
            }
            enabled_set.enabled_plugins.insert(plugin_id);
        }

        enabled_set.enforce_default_plugin();
        Ok(PluginStateLoadResult {
            enabled_set,
            warnings,
        })
    }

    /// Serializes the enabled set into `plugin_state.toml`.
    pub fn to_toml_string(&self) -> Result<String, String> {
        let mut enabled_set = self.clone();
        enabled_set.enforce_default_plugin();
        let mut output = format!(
            "schema_version = {}\n\n[plugins]\n",
            PLUGIN_STATE_SCHEMA_VERSION
        );
        for plugin_id in enabled_set.iter() {
            output.push_str(&format!("\"{}\" = true\n", plugin_id.as_str()));
        }
        Ok(output)
    }

    /// Saves the enabled set to one `plugin_state.toml` file.
    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        let contents = self.to_toml_string()?;
        fs::write(path, contents).map_err(|error| {
            format!(
                "failed to write plugin state file '{}': {}",
                path.display(),
                error
            )
        })
    }
}

/// Parsed result of one `plugin_state.toml` load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginStateLoadResult {
    pub enabled_set: EnabledPluginSet,
    pub warnings: Vec<String>,
}

/// High-level runtime state of one plugin in the registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginRuntimeStatus {
    Enabled,
    Disabled,
    Missing,
    Error,
}

impl PluginRuntimeStatus {
    /// Returns the lowercase label used by status text and logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Missing => "missing",
            Self::Error => "error",
        }
    }
}

/// One resolved plugin entry shown by the runtime registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginRegistryEntry {
    pub plugin_id: Option<PluginId>,
    pub display_name: String,
    pub version: Option<PluginVersion>,
    pub source_kind: Option<PluginSourceKind>,
    pub status: PluginRuntimeStatus,
    pub locked: bool,
    pub content: bool,
    pub source_name: String,
    pub source_path: Option<PathBuf>,
    pub error_message: Option<String>,
}

impl PluginRegistryEntry {
    /// Returns the short display label used in menu diagnostics.
    pub fn label(&self) -> &str {
        self.plugin_id
            .as_ref()
            .map(PluginId::as_str)
            .unwrap_or(&self.source_name)
    }

    /// Builds the compact one-line status fragment for the main menu.
    pub fn to_status_line(&self) -> String {
        let mut parts = vec![self.status.as_str().to_string()];
        if self.locked {
            parts.push("locked".to_string());
        }
        format!("{} [{}]", self.label(), parts.join(", "))
    }

    /// Builds a detailed log line with optional error context.
    pub fn to_log_line(&self) -> String {
        let source_kind = self
            .source_kind
            .map(|value| format!("{:?}", value))
            .unwrap_or_else(|| "Unknown".to_string());
        match self.error_message.as_deref() {
            Some(error_message) => format!(
                "Plugin '{}' is {} (source: {} {}): {}",
                self.label(),
                self.status.as_str(),
                self.source_name,
                source_kind,
                error_message
            ),
            None => format!(
                "Plugin '{}' is {} (source: {} {})",
                self.label(),
                self.status.as_str(),
                self.source_name,
                source_kind
            ),
        }
    }
}

/// Aggregate runtime plugin state inserted into the Bevy app.
#[derive(Resource, Clone, Debug, Default)]
pub struct PluginRegistryState {
    pub entries: Vec<PluginRegistryEntry>,
    pub warnings: Vec<String>,
}

impl PluginRegistryState {
    /// Returns the visible main-menu diagnostics line.
    pub fn main_menu_status_text(&self) -> Option<String> {
        if self.entries.is_empty() && self.warnings.is_empty() {
            return None;
        }

        let entries_text = if self.entries.is_empty() {
            "Plugins: none".to_string()
        } else {
            format!(
                "Plugins: {}",
                self.entries
                    .iter()
                    .map(PluginRegistryEntry::to_status_line)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        match self.warnings.first() {
            Some(warning) => Some(format!("Plugin warning: {} | {}", warning, entries_text)),
            None => Some(entries_text),
        }
    }

    /// Writes warnings and detailed per-plugin states to stderr.
    pub fn log_to_stderr(&self) {
        for warning in &self.warnings {
            eprintln!("Plugin warning: {}", warning);
        }
        for entry in &self.entries {
            eprintln!("{}", entry.to_log_line());
        }
    }
}

#[derive(Deserialize)]
struct RawPluginStateFile {
    schema_version: u32,
    #[serde(default)]
    plugins: toml::Table,
}

fn flatten_plugin_state_table(
    prefix: &str,
    table: &toml::Table,
) -> Result<BTreeMap<String, bool>, String> {
    let mut flattened = BTreeMap::new();
    for (key, value) in table {
        let full_key = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}.{}", prefix, key)
        };
        match value {
            toml::Value::Boolean(enabled) => {
                flattened.insert(full_key, *enabled);
            }
            toml::Value::Table(nested) => {
                flattened.extend(flatten_plugin_state_table(&full_key, nested)?);
            }
            _ => {
                return Err(format!(
                    "plugin_state.toml entry '{}' must be a boolean or nested table",
                    full_key
                ));
            }
        }
    }
    Ok(flattened)
}

#[cfg(test)]
mod tests {
    use crate::plugins::{
        state::{EnabledPluginSet, PluginRuntimeStatus},
        PluginId, DEFAULT_PLUGIN_ID_VALUE,
    };

    #[test]
    fn plugin_state_default_plugin_is_always_enforced() {
        let mut enabled = EnabledPluginSet::default();
        enabled.enforce_default_plugin();
        let default_plugin = PluginId::default_plugin();

        assert!(enabled.is_enabled(&default_plugin));

        enabled.set_enabled(&default_plugin, false);
        assert!(enabled.is_enabled(&default_plugin));
    }

    #[test]
    fn plugin_state_default_plugin_false_entry_becomes_warning() {
        let load_result = EnabledPluginSet::from_toml_str(
            r#"
schema_version = 1

[plugins]
flux.default = false
sample.plugin = true
"#,
        )
        .expect("plugin state should parse");

        let default_plugin = PluginId::default_plugin();
        let sample_plugin = PluginId::parse("sample.plugin").expect("valid plugin id");

        assert!(load_result.enabled_set.is_enabled(&default_plugin));
        assert!(load_result.enabled_set.is_enabled(&sample_plugin));
        assert_eq!(load_result.warnings.len(), 1);
        assert!(load_result.warnings[0].contains(DEFAULT_PLUGIN_ID_VALUE));
    }

    #[test]
    fn plugin_state_round_trip_preserves_enabled_plugins() {
        let default_plugin = PluginId::default_plugin();
        let sample_plugin = PluginId::parse("sample.plugin").expect("valid plugin id");
        let mut enabled = EnabledPluginSet::default();
        enabled.set_enabled(&default_plugin, true);
        enabled.set_enabled(&sample_plugin, true);

        let serialized = enabled.to_toml_string().expect("serialize plugin state");
        let reparsed = EnabledPluginSet::from_toml_str(&serialized)
            .expect("reparse plugin state")
            .enabled_set;

        assert!(reparsed.is_enabled(&default_plugin));
        assert!(reparsed.is_enabled(&sample_plugin));
    }

    #[test]
    fn plugin_state_runtime_status_labels_are_stable() {
        assert_eq!(PluginRuntimeStatus::Enabled.as_str(), "enabled");
        assert_eq!(PluginRuntimeStatus::Disabled.as_str(), "disabled");
        assert_eq!(PluginRuntimeStatus::Missing.as_str(), "missing");
        assert_eq!(PluginRuntimeStatus::Error.as_str(), "error");
    }
}
