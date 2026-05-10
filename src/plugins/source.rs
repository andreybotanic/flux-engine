use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

use crate::plugins::{
    diagnostics::PluginContractError,
    loader::{
        read_expanded_plugin_candidate, read_packaged_plugin_candidate,
        validate_expanded_plugin_candidate, validate_packaged_plugin_candidate,
        ExpandedPluginCandidate, ValidatedExpandedPlugin, ValidatedPackagedPlugin,
    },
    manifest::PluginManifest,
    registration::PluginRuntimeRegistration,
    PluginId, PluginVersion,
};

/// File extension used by packaged FluxEngine runtime plugins.
pub const PACKAGED_PLUGIN_EXTENSION: &str = "fluxplugin";

/// Root manifest filename required by plugin packages and expanded folders.
pub const MANIFEST_FILE_NAME: &str = "manifest.toml";

/// High-level source category used by the registry bootstrap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginSourceKind {
    Builtin,
    Packaged,
    Dev,
}

/// Plugin source descriptor used by discovery and future registry layers.
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

    /// Returns a short source name for diagnostics.
    pub fn display_name(&self) -> String {
        self.root_dir
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| self.root_dir.display().to_string())
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

/// One fully validated physical plugin source.
#[derive(Clone, Debug, PartialEq)]
pub struct DiscoveredPluginSource {
    pub plugin_id: PluginId,
    pub display_name: String,
    pub version: PluginVersion,
    pub source_kind: PluginSourceKind,
    pub source_name: String,
    pub source_path: PathBuf,
    pub source: PluginSource,
    pub manifest: PluginManifest,
    pub registration: PluginRuntimeRegistration,
}

impl DiscoveredPluginSource {
    fn from_packaged(validated: ValidatedPackagedPlugin) -> Self {
        Self {
            plugin_id: validated.manifest.id.clone(),
            display_name: validated.manifest.display_name.clone(),
            version: validated.manifest.version.clone(),
            source_kind: PluginSourceKind::Packaged,
            source_name: validated.source.display_name(),
            source_path: validated.source.archive_path().to_path_buf(),
            source: PluginSource::Archive(validated.source),
            manifest: validated.manifest,
            registration: validated.registration,
        }
    }

    fn from_expanded(validated: ValidatedExpandedPlugin) -> Self {
        Self {
            plugin_id: validated.manifest.id.clone(),
            display_name: validated.manifest.display_name.clone(),
            version: validated.manifest.version.clone(),
            source_kind: PluginSourceKind::Dev,
            source_name: validated.source.display_name(),
            source_path: validated.source.root_dir().to_path_buf(),
            source: PluginSource::Directory(validated.source),
            manifest: validated.manifest,
            registration: validated.registration,
        }
    }
}

/// One plugin source that was discovered but rejected during validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RejectedPluginSource {
    pub plugin_id: Option<PluginId>,
    pub source_kind: PluginSourceKind,
    pub source_name: String,
    pub source_path: PathBuf,
    pub error: PluginContractError,
}

/// Discovery result for packaged and expanded plugin sources.
#[derive(Clone, Debug, Default)]
pub struct PluginSourceDiscovery {
    pub packaged_sources: Vec<DiscoveredPluginSource>,
    pub dev_sources: Vec<DiscoveredPluginSource>,
    pub rejected_sources: Vec<RejectedPluginSource>,
}

/// Scans packaged and expanded plugin roots and returns validated sources.
pub fn discover_plugin_sources(packaged_root: &Path, dev_root: &Path) -> PluginSourceDiscovery {
    let mut discovery = discover_packaged_plugin_sources(packaged_root);
    let dev_discovery = discover_dev_plugin_sources(dev_root);
    discovery.dev_sources = dev_discovery.dev_sources;
    discovery
        .rejected_sources
        .extend(dev_discovery.rejected_sources);
    discovery
}

/// Scans `plugins/*.fluxplugin` and validates every unique packaged plugin source.
pub fn discover_packaged_plugin_sources(packaged_root: &Path) -> PluginSourceDiscovery {
    let mut rejected_sources = Vec::new();
    let mut parsed_candidates = Vec::new();
    for archive_path in collect_packaged_archives(packaged_root, &mut rejected_sources) {
        let source = PackagedPluginSource::new(archive_path.clone());
        match read_packaged_plugin_candidate(&archive_path) {
            Ok(candidate) => parsed_candidates.push(candidate),
            Err(error) => rejected_sources.push(RejectedPluginSource {
                plugin_id: None,
                source_kind: PluginSourceKind::Packaged,
                source_name: source.display_name(),
                source_path: archive_path,
                error,
            }),
        }
    }

    let mut packaged_sources = Vec::new();
    for candidate in reject_duplicate_packaged_candidates(parsed_candidates, &mut rejected_sources)
    {
        match validate_packaged_plugin_candidate(&candidate) {
            Ok(validated) => {
                packaged_sources.push(DiscoveredPluginSource::from_packaged(validated))
            }
            Err(error) => rejected_sources.push(RejectedPluginSource {
                plugin_id: Some(candidate.manifest.id.clone()),
                source_kind: PluginSourceKind::Packaged,
                source_name: candidate.source.display_name(),
                source_path: candidate.source.archive_path().to_path_buf(),
                error,
            }),
        }
    }

    packaged_sources.sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));
    PluginSourceDiscovery {
        packaged_sources,
        dev_sources: Vec::new(),
        rejected_sources,
    }
}

/// Scans `plugins_dev/<plugin_id>/` and validates every unique dev plugin source.
pub fn discover_dev_plugin_sources(dev_root: &Path) -> PluginSourceDiscovery {
    let mut rejected_sources = Vec::new();
    let mut parsed_candidates = Vec::new();
    for root_dir in collect_expanded_plugin_roots(dev_root, &mut rejected_sources) {
        let source = ExpandedPluginSource::new(root_dir.clone());
        match read_expanded_plugin_candidate(&root_dir) {
            Ok(candidate) => parsed_candidates.push(candidate),
            Err(error) => rejected_sources.push(RejectedPluginSource {
                plugin_id: None,
                source_kind: PluginSourceKind::Dev,
                source_name: source.display_name(),
                source_path: root_dir,
                error,
            }),
        }
    }

    let mut dev_sources = Vec::new();
    for candidate in reject_duplicate_dev_candidates(parsed_candidates, &mut rejected_sources) {
        match validate_expanded_plugin_candidate(&candidate) {
            Ok(validated) => dev_sources.push(DiscoveredPluginSource::from_expanded(validated)),
            Err(error) => rejected_sources.push(RejectedPluginSource {
                plugin_id: Some(candidate.manifest.id.clone()),
                source_kind: PluginSourceKind::Dev,
                source_name: candidate.source.display_name(),
                source_path: candidate.source.root_dir().to_path_buf(),
                error,
            }),
        }
    }

    dev_sources.sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));
    PluginSourceDiscovery {
        packaged_sources: Vec::new(),
        dev_sources,
        rejected_sources,
    }
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

fn collect_packaged_archives(
    packaged_root: &Path,
    rejected_sources: &mut Vec<RejectedPluginSource>,
) -> Vec<PathBuf> {
    let entries = match fs::read_dir(packaged_root) {
        Ok(entries) => entries,
        Err(_error) if !packaged_root.exists() => return Vec::new(),
        Err(error) => {
            rejected_sources.push(RejectedPluginSource {
                plugin_id: None,
                source_kind: PluginSourceKind::Packaged,
                source_name: packaged_root.display().to_string(),
                source_path: packaged_root.to_path_buf(),
                error: PluginContractError::Io(format!(
                    "failed to read plugins root '{}': {}",
                    packaged_root.display(),
                    error
                )),
            });
            return Vec::new();
        }
    };

    let mut archives = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) if is_packaged_plugin_path(&entry.path()) => archives.push(entry.path()),
            Ok(_) => {}
            Err(error) => rejected_sources.push(RejectedPluginSource {
                plugin_id: None,
                source_kind: PluginSourceKind::Packaged,
                source_name: packaged_root.display().to_string(),
                source_path: packaged_root.to_path_buf(),
                error: PluginContractError::Io(format!(
                    "failed to read one entry from '{}': {}",
                    packaged_root.display(),
                    error
                )),
            }),
        }
    }

    archives.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .cmp(right.file_name().unwrap_or_default())
    });
    archives
}

fn collect_expanded_plugin_roots(
    dev_root: &Path,
    rejected_sources: &mut Vec<RejectedPluginSource>,
) -> Vec<PathBuf> {
    let entries = match fs::read_dir(dev_root) {
        Ok(entries) => entries,
        Err(_error) if !dev_root.exists() => return Vec::new(),
        Err(error) => {
            rejected_sources.push(RejectedPluginSource {
                plugin_id: None,
                source_kind: PluginSourceKind::Dev,
                source_name: dev_root.display().to_string(),
                source_path: dev_root.to_path_buf(),
                error: PluginContractError::Io(format!(
                    "failed to read plugins dev root '{}': {}",
                    dev_root.display(),
                    error
                )),
            });
            return Vec::new();
        }
    };

    let mut roots = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) if entry.path().is_dir() => roots.push(entry.path()),
            Ok(_) => {}
            Err(error) => rejected_sources.push(RejectedPluginSource {
                plugin_id: None,
                source_kind: PluginSourceKind::Dev,
                source_name: dev_root.display().to_string(),
                source_path: dev_root.to_path_buf(),
                error: PluginContractError::Io(format!(
                    "failed to read one entry from '{}': {}",
                    dev_root.display(),
                    error
                )),
            }),
        }
    }

    roots.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .cmp(right.file_name().unwrap_or_default())
    });
    roots
}

fn reject_duplicate_packaged_candidates(
    candidates: Vec<crate::plugins::loader::PackagedPluginCandidate>,
    rejected_sources: &mut Vec<RejectedPluginSource>,
) -> Vec<crate::plugins::loader::PackagedPluginCandidate> {
    let mut grouped =
        BTreeMap::<PluginId, Vec<crate::plugins::loader::PackagedPluginCandidate>>::new();
    for candidate in candidates {
        grouped
            .entry(candidate.manifest.id.clone())
            .or_default()
            .push(candidate);
    }

    let mut unique_candidates = Vec::new();
    for (plugin_id, group) in grouped {
        if group.len() == 1 {
            unique_candidates.push(group.into_iter().next().expect("single candidate"));
            continue;
        }

        let sources = group
            .iter()
            .map(|candidate| candidate.source.display_name())
            .collect::<Vec<_>>();
        let message = format!(
            "duplicate packaged plugin id '{}' is declared by {}",
            plugin_id,
            sources.join(", ")
        );

        for candidate in group {
            rejected_sources.push(RejectedPluginSource {
                plugin_id: Some(candidate.manifest.id.clone()),
                source_kind: PluginSourceKind::Packaged,
                source_name: candidate.source.display_name(),
                source_path: candidate.source.archive_path().to_path_buf(),
                error: PluginContractError::DuplicateId(message.clone()),
            });
        }
    }

    unique_candidates.sort_by(|left, right| left.manifest.id.cmp(&right.manifest.id));
    unique_candidates
}

fn reject_duplicate_dev_candidates(
    candidates: Vec<ExpandedPluginCandidate>,
    rejected_sources: &mut Vec<RejectedPluginSource>,
) -> Vec<ExpandedPluginCandidate> {
    let mut grouped = BTreeMap::<PluginId, Vec<ExpandedPluginCandidate>>::new();
    for candidate in candidates {
        grouped
            .entry(candidate.manifest.id.clone())
            .or_default()
            .push(candidate);
    }

    let mut unique_candidates = Vec::new();
    for (plugin_id, group) in grouped {
        if group.len() == 1 {
            unique_candidates.push(group.into_iter().next().expect("single candidate"));
            continue;
        }

        let sources = group
            .iter()
            .map(|candidate| candidate.source.display_name())
            .collect::<Vec<_>>();
        let message = format!(
            "duplicate dev plugin id '{}' is declared by {}",
            plugin_id,
            sources.join(", ")
        );

        for candidate in group {
            rejected_sources.push(RejectedPluginSource {
                plugin_id: Some(candidate.manifest.id.clone()),
                source_kind: PluginSourceKind::Dev,
                source_name: candidate.source.display_name(),
                source_path: candidate.source.root_dir().to_path_buf(),
                error: PluginContractError::DuplicateId(message.clone()),
            });
        }
    }

    unique_candidates.sort_by(|left, right| left.manifest.id.cmp(&right.manifest.id));
    unique_candidates
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

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::{Read, Write},
        path::{Path, PathBuf},
        process::Command,
    };

    use zip::{write::SimpleFileOptions, ZipWriter};

    use crate::plugins::source::{
        discover_dev_plugin_sources, discover_packaged_plugin_sources, PluginSourceKind,
    };

    fn make_temp_root(prefix: &str) -> PathBuf {
        let unique = format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn sample_plugin_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("plugins")
            .join("flux_stage1_sample_plugin")
    }

    fn cargo_binary() -> PathBuf {
        if let Some(path) = std::env::var_os("CARGO") {
            return PathBuf::from(path);
        }

        let fallback = PathBuf::from(r"C:\Users\andreybotanic\.cargo\bin\cargo.exe");
        if fallback.exists() {
            return fallback;
        }

        PathBuf::from("cargo")
    }

    fn build_sample_plugin_cdylib() -> PathBuf {
        let crate_root = sample_plugin_root();
        let status = Command::new(cargo_binary())
            .arg("build")
            .arg("--manifest-path")
            .arg(crate_root.join("Cargo.toml"))
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
            .status()
            .expect("spawn cargo build for sample plugin");
        assert!(status.success(), "sample plugin cargo build failed");

        crate_root
            .join("target")
            .join("debug")
            .join("flux_stage1_sample_plugin.dll")
    }

    fn create_working_sample_plugin_archive(archive_path: &Path) {
        let crate_root = sample_plugin_root();
        let dll_path = build_sample_plugin_cdylib();
        let template_root = crate_root.join("package_template");
        let file = File::create(archive_path).expect("create sample plugin archive");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        for relative in [
            "manifest.toml",
            "config/sample.toml",
            "assets/placeholder.txt",
        ] {
            let source_path = template_root.join(relative);
            let mut bytes = Vec::new();
            File::open(&source_path)
                .expect("open template file")
                .read_to_end(&mut bytes)
                .expect("read template file");
            writer
                .start_file(relative.replace('\\', "/"), options)
                .expect("start template file");
            writer.write_all(&bytes).expect("write template file");
        }

        let mut dll_bytes = Vec::new();
        File::open(&dll_path)
            .expect("open sample DLL")
            .read_to_end(&mut dll_bytes)
            .expect("read sample DLL");
        writer
            .start_file("bin/flux_stage1_sample_plugin.dll", options)
            .expect("start DLL entry");
        writer.write_all(&dll_bytes).expect("write DLL entry");
        writer.finish().expect("finish plugin archive");
    }

    fn create_working_sample_plugin_directory(root_dir: &Path) {
        let crate_root = sample_plugin_root();
        let dll_path = build_sample_plugin_cdylib();
        let template_root = crate_root.join("package_template");

        fs::create_dir_all(root_dir.join("config")).expect("create config dir");
        fs::create_dir_all(root_dir.join("assets")).expect("create assets dir");
        fs::create_dir_all(root_dir.join("bin")).expect("create bin dir");
        fs::copy(
            template_root.join("manifest.toml"),
            root_dir.join("manifest.toml"),
        )
        .expect("copy manifest");
        fs::copy(
            template_root.join("config/sample.toml"),
            root_dir.join("config/sample.toml"),
        )
        .expect("copy config");
        fs::copy(
            template_root.join("assets/placeholder.txt"),
            root_dir.join("assets/placeholder.txt"),
        )
        .expect("copy asset");
        fs::copy(dll_path, root_dir.join("bin/flux_stage1_sample_plugin.dll")).expect("copy dll");
    }

    #[test]
    fn plugin_registry_packaged_source_discovery_accepts_valid_archive() {
        let root = make_temp_root("flux_packaged_discovery");
        create_working_sample_plugin_archive(&root.join("sample.fluxplugin"));

        let discovery = discover_packaged_plugin_sources(&root);
        assert_eq!(discovery.packaged_sources.len(), 1);
        assert!(discovery.dev_sources.is_empty());
        assert!(discovery.rejected_sources.is_empty());
        assert_eq!(
            discovery.packaged_sources[0].plugin_id.as_str(),
            "flux.sample_stage1"
        );
        assert_eq!(
            discovery.packaged_sources[0].source_kind,
            PluginSourceKind::Packaged
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_registry_dev_source_discovery_accepts_valid_directory() {
        let root = make_temp_root("flux_dev_discovery");
        let plugin_dir = root.join("flux.sample_stage1");
        create_working_sample_plugin_directory(&plugin_dir);

        let discovery = discover_dev_plugin_sources(&root);
        assert_eq!(discovery.dev_sources.len(), 1);
        assert!(discovery.packaged_sources.is_empty());
        assert!(discovery.rejected_sources.is_empty());
        assert_eq!(
            discovery.dev_sources[0].plugin_id.as_str(),
            "flux.sample_stage1"
        );
        assert_eq!(discovery.dev_sources[0].source_kind, PluginSourceKind::Dev);

        let _ = fs::remove_dir_all(root);
    }
}
