use std::path::{Component, Path, PathBuf};

use crate::plugins::{diagnostics::PluginContractError, manifest::PluginManifest};

/// File extension used by packaged FluxEngine runtime plugins.
pub const PACKAGED_PLUGIN_EXTENSION: &str = "fluxplugin";

/// Root manifest filename required by plugin packages and expanded folders.
pub const MANIFEST_FILE_NAME: &str = "manifest.toml";

/// Plugin source descriptor used by future discovery layers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginSource {
    Archive(PackagedPluginSource),
    Directory(ExpandedPluginSource),
}

/// One packaged `.fluxplugin` archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackagedPluginSource {
    archive_path: PathBuf,
}

impl PackagedPluginSource {
    /// Creates a packaged plugin source wrapper.
    pub fn new(archive_path: PathBuf) -> Self {
        Self { archive_path }
    }

    /// Returns the archive path on disk.
    pub fn archive_path(&self) -> &Path {
        &self.archive_path
    }

    /// Returns a short source name for diagnostics.
    pub fn display_name(&self) -> String {
        self.archive_path
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| self.archive_path.display().to_string())
    }
}

/// One expanded dev plugin directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpandedPluginSource {
    root_dir: PathBuf,
}

impl ExpandedPluginSource {
    /// Creates an expanded plugin source wrapper.
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    /// Returns the plugin root directory on disk.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }
}

/// Resolved runtime paths for one plugin root and manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginResolvedPaths {
    pub root_dir: PathBuf,
    pub dll_path: PathBuf,
    pub configs_dir: PathBuf,
    pub assets_dir: PathBuf,
}

/// Returns `true` when a path points to a packaged plugin archive.
pub fn is_packaged_plugin_path(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case(PACKAGED_PLUGIN_EXTENSION))
            .unwrap_or(false)
}

/// Validates one manifest-relative path used inside the plugin contract.
pub fn validate_relative_plugin_path(value: &str) -> Result<PathBuf, PluginContractError> {
    validate_relative_path_impl(value, PluginContractError::Manifest)
}

/// Validates one packaged archive entry path before extraction.
pub fn validate_archive_entry_path(value: &str) -> Result<PathBuf, PluginContractError> {
    validate_relative_path_impl(value, PluginContractError::Archive)
}

/// Resolves manifest paths against one plugin root and checks the DLL location.
pub fn resolve_plugin_layout(
    root_dir: &Path,
    manifest: &PluginManifest,
) -> Result<PluginResolvedPaths, PluginContractError> {
    if !root_dir.exists() {
        return Err(PluginContractError::Io(format!(
            "plugin root '{}' does not exist",
            root_dir.display()
        )));
    }

    let dll_path = root_dir.join(&manifest.dll);
    if !dll_path.is_file() {
        return Err(PluginContractError::Dll(format!(
            "missing DLL '{}'",
            manifest.dll.display()
        )));
    }

    Ok(PluginResolvedPaths {
        root_dir: root_dir.to_path_buf(),
        dll_path,
        configs_dir: root_dir.join(&manifest.configs),
        assets_dir: root_dir.join(&manifest.assets),
    })
}

fn validate_relative_path_impl(
    value: &str,
    error_builder: fn(String) -> PluginContractError,
) -> Result<PathBuf, PluginContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(error_builder("plugin path must not be empty".to_string()));
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(error_builder(format!(
            "plugin path '{}' must be relative",
            trimmed
        )));
    }

    let mut normalized = PathBuf::new();
    let mut has_normal_component = false;
    for component in path.components() {
        match component {
            Component::Prefix(_) => {
                return Err(error_builder(format!(
                    "plugin path '{}' must not contain a Windows drive prefix",
                    trimmed
                )));
            }
            Component::RootDir => {
                return Err(error_builder(format!(
                    "plugin path '{}' must not start at a filesystem root",
                    trimmed
                )));
            }
            Component::ParentDir => {
                return Err(error_builder(format!(
                    "plugin path '{}' must not contain '..'",
                    trimmed
                )));
            }
            Component::CurDir => {
                return Err(error_builder(format!(
                    "plugin path '{}' must not contain '.' path segments",
                    trimmed
                )));
            }
            Component::Normal(part) => {
                has_normal_component = true;
                normalized.push(part);
            }
        }
    }

    if !has_normal_component {
        return Err(error_builder(format!(
            "plugin path '{}' must contain at least one path segment",
            trimmed
        )));
    }

    Ok(normalized)
}
