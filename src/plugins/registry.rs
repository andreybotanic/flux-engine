use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use bevy::prelude::Resource;

use crate::plugins::{
    content::ContentRegistry,
    default_plugin::default_content_registry,
    source::{
        discover_plugin_sources, DiscoveredPluginSource, PluginSourceDiscovery, PluginSourceKind,
        RejectedPluginSource,
    },
    state::{EnabledPluginSet, PluginRegistryEntry, PluginRegistryState, PluginRuntimeStatus},
    PluginId, PluginManifest, PluginRuntimeRegistration, PluginVersion, SubstanceRegistry,
};

/// Startup configuration used by the stage-2 plugin bootstrap.
#[derive(Resource, Clone, Debug)]
pub struct PluginBootstrapConfig {
    pub packaged_plugins_root: PathBuf,
    pub dev_plugins_root: PathBuf,
    pub state_file_path: PathBuf,
    pub dev_mode: bool,
}

impl PluginBootstrapConfig {
    /// Builds the default bootstrap config from one repository root.
    pub fn from_repo_root(repo_root: &Path, dev_mode: bool) -> Self {
        Self {
            packaged_plugins_root: repo_root.join("plugins"),
            dev_plugins_root: repo_root.join("plugins_dev"),
            state_file_path: repo_root.join("plugin_state.toml"),
            dev_mode,
        }
    }
}

/// One discovered source record kept for future UI layers.
#[derive(Clone, Debug)]
pub struct PluginSourceRecord {
    pub plugin_id: Option<PluginId>,
    pub display_name: Option<String>,
    pub version: Option<PluginVersion>,
    pub source_kind: PluginSourceKind,
    pub source_name: String,
    pub source_path: Option<PathBuf>,
    pub content: bool,
    pub locked: bool,
    pub error_message: Option<String>,
}

/// Runtime registry of every discovered plugin source.
#[derive(Resource, Clone, Debug, Default)]
pub struct PluginSourceRegistry {
    sources: Vec<PluginSourceRecord>,
}

impl PluginSourceRegistry {
    /// Returns the discovered source records in deterministic order.
    pub fn entries(&self) -> &[PluginSourceRecord] {
        &self.sources
    }
}

/// One enabled plugin that passed stage-2 bootstrap and is ready for future content registration.
#[derive(Clone, Debug)]
pub struct LoadedPluginMetadata {
    pub plugin_id: PluginId,
    pub display_name: String,
    pub version: PluginVersion,
    pub source_kind: PluginSourceKind,
    pub content: bool,
    pub locked: bool,
    pub source_name: String,
    pub source_path: Option<PathBuf>,
    pub manifest: Option<PluginManifest>,
    pub registration: PluginRuntimeRegistration,
}

/// Runtime registry of enabled plugins that completed bootstrap successfully.
#[derive(Resource, Clone, Debug, Default)]
pub struct LoadedPluginRegistry {
    loaded_plugins: Vec<LoadedPluginMetadata>,
}

impl LoadedPluginRegistry {
    /// Returns the loaded plugin metadata in deterministic order.
    pub fn entries(&self) -> &[LoadedPluginMetadata] {
        &self.loaded_plugins
    }
}

/// Output of the stage-2 bootstrap pipeline.
#[derive(Clone, Debug)]
pub struct PluginBootstrapOutput {
    pub source_registry: PluginSourceRegistry,
    pub loaded_registry: LoadedPluginRegistry,
    pub enabled_set: EnabledPluginSet,
    pub content_registry: ContentRegistry,
    pub registry_state: PluginRegistryState,
}

#[derive(Default)]
struct SourceGroup {
    packaged: Option<DiscoveredPluginSource>,
    dev: Option<DiscoveredPluginSource>,
}

#[derive(Default)]
struct RejectedSourceGroup {
    packaged: Vec<RejectedPluginSource>,
    dev: Vec<RejectedPluginSource>,
}

/// Runs the full stage-2 plugin discovery and state bootstrap pipeline.
pub fn bootstrap_plugin_registry(config: &PluginBootstrapConfig) -> PluginBootstrapOutput {
    let mut warnings = Vec::new();
    let enabled_state = match EnabledPluginSet::load_from_path(&config.state_file_path) {
        Ok(load_result) => {
            warnings.extend(load_result.warnings);
            load_result.enabled_set
        }
        Err(error) => {
            warnings.push(error);
            let mut enabled_set = EnabledPluginSet::default();
            enabled_set.enforce_default_plugin();
            enabled_set
        }
    };

    let discovery =
        discover_plugin_sources(&config.packaged_plugins_root, &config.dev_plugins_root);
    let source_registry = build_source_registry(&discovery);
    let (loaded_registry, content_registry, registry_state) =
        build_runtime_registries(&discovery, enabled_state.clone(), warnings, config.dev_mode);

    PluginBootstrapOutput {
        source_registry,
        loaded_registry,
        enabled_set: enabled_state,
        content_registry,
        registry_state,
    }
}

/// Rebuilds plugin registries from an already edited enabled set.
pub fn rebuild_plugin_registry_from_enabled_set(
    config: &PluginBootstrapConfig,
    mut enabled_set: EnabledPluginSet,
) -> PluginBootstrapOutput {
    enabled_set.enforce_default_plugin();
    let discovery =
        discover_plugin_sources(&config.packaged_plugins_root, &config.dev_plugins_root);
    let source_registry = build_source_registry(&discovery);
    let (loaded_registry, content_registry, registry_state) =
        build_runtime_registries(&discovery, enabled_set.clone(), Vec::new(), config.dev_mode);

    PluginBootstrapOutput {
        source_registry,
        loaded_registry,
        enabled_set,
        content_registry,
        registry_state,
    }
}

fn build_source_registry(discovery: &PluginSourceDiscovery) -> PluginSourceRegistry {
    let mut sources = vec![default_source_record()];
    sources.extend(
        discovery
            .packaged_sources
            .iter()
            .map(discovered_source_record)
            .collect::<Vec<_>>(),
    );
    sources.extend(
        discovery
            .dev_sources
            .iter()
            .map(discovered_source_record)
            .collect::<Vec<_>>(),
    );
    sources.extend(
        discovery
            .rejected_sources
            .iter()
            .map(rejected_source_record)
            .collect::<Vec<_>>(),
    );
    sources.sort_by(source_record_sort_key);
    PluginSourceRegistry { sources }
}

fn build_runtime_registries(
    discovery: &PluginSourceDiscovery,
    enabled_set: EnabledPluginSet,
    warnings: Vec<String>,
    dev_mode: bool,
) -> (LoadedPluginRegistry, ContentRegistry, PluginRegistryState) {
    let default_plugin_id = PluginId::default_plugin();
    let mut source_groups = BTreeMap::<PluginId, SourceGroup>::new();
    for source in &discovery.packaged_sources {
        source_groups
            .entry(source.plugin_id.clone())
            .or_default()
            .packaged = Some(source.clone());
    }
    for source in &discovery.dev_sources {
        source_groups
            .entry(source.plugin_id.clone())
            .or_default()
            .dev = Some(source.clone());
    }

    let mut rejected_groups = BTreeMap::<PluginId, RejectedSourceGroup>::new();
    let mut anonymous_rejections = Vec::new();
    for rejected in &discovery.rejected_sources {
        if let Some(plugin_id) = rejected.plugin_id.clone() {
            let group = rejected_groups.entry(plugin_id).or_default();
            match rejected.source_kind {
                PluginSourceKind::Packaged => group.packaged.push(rejected.clone()),
                PluginSourceKind::Dev => group.dev.push(rejected.clone()),
                PluginSourceKind::Builtin => group.packaged.push(rejected.clone()),
            }
        } else {
            anonymous_rejections.push(rejected.clone());
        }
    }

    let mut loaded_plugins = vec![default_loaded_plugin()];
    let mut content_registry = default_content_registry();
    let mut entries = vec![default_registry_entry()];

    let mut plugin_ids = BTreeSet::new();
    plugin_ids.extend(source_groups.keys().cloned());
    plugin_ids.extend(rejected_groups.keys().cloned());
    plugin_ids.extend(enabled_set.iter().cloned());
    plugin_ids.remove(&default_plugin_id);

    for plugin_id in plugin_ids {
        let source_group = source_groups.remove(&plugin_id).unwrap_or_default();
        let rejected_group = rejected_groups.remove(&plugin_id).unwrap_or_default();
        let selected_kind = select_source_kind(&source_group, &rejected_group, dev_mode);
        let is_enabled = enabled_set.is_enabled(&plugin_id);
        let mut loaded_plugin = None;

        let mut registry_entry = match selected_kind {
            Some(PluginSourceKind::Packaged) => {
                if let Some(source) = source_group.packaged {
                    if is_enabled {
                        loaded_plugin = Some(loaded_metadata_from_source(&source, false));
                    }
                    resolved_registry_entry(source, is_enabled, false)
                } else {
                    rejected_registry_entry(
                        plugin_id,
                        rejected_group.packaged,
                        PluginSourceKind::Packaged,
                    )
                }
            }
            Some(PluginSourceKind::Dev) => {
                if let Some(source) = source_group.dev {
                    if is_enabled {
                        loaded_plugin = Some(loaded_metadata_from_source(&source, false));
                    }
                    resolved_registry_entry(source, is_enabled, false)
                } else {
                    rejected_registry_entry(plugin_id, rejected_group.dev, PluginSourceKind::Dev)
                }
            }
            Some(PluginSourceKind::Builtin) | None if is_enabled => {
                missing_registry_entry(plugin_id)
            }
            Some(PluginSourceKind::Builtin) | None => continue,
        };

        if let Some(loaded) = loaded_plugin {
            match register_loaded_plugin_content(&mut content_registry, &loaded) {
                Ok(()) => loaded_plugins.push(loaded),
                Err(error) => {
                    registry_entry.status = PluginRuntimeStatus::Error;
                    registry_entry.error_message = Some(error);
                }
            }
        }
        entries.push(registry_entry);
    }

    for rejected in anonymous_rejections {
        entries.push(anonymous_rejected_registry_entry(rejected));
    }

    loaded_plugins.sort_by(loaded_plugin_sort_key);
    entries.sort_by(registry_entry_sort_key);
    let registry_state = PluginRegistryState { entries, warnings };

    (
        LoadedPluginRegistry { loaded_plugins },
        content_registry,
        registry_state,
    )
}

fn select_source_kind(
    source_group: &SourceGroup,
    rejected_group: &RejectedSourceGroup,
    dev_mode: bool,
) -> Option<PluginSourceKind> {
    let packaged_present = source_group.packaged.is_some() || !rejected_group.packaged.is_empty();
    let dev_present = source_group.dev.is_some() || !rejected_group.dev.is_empty();

    if dev_mode {
        if dev_present {
            return Some(PluginSourceKind::Dev);
        }
        if packaged_present {
            return Some(PluginSourceKind::Packaged);
        }
        return None;
    }

    if packaged_present {
        return Some(PluginSourceKind::Packaged);
    }
    if dev_present {
        return Some(PluginSourceKind::Dev);
    }
    None
}

fn default_source_record() -> PluginSourceRecord {
    PluginSourceRecord {
        plugin_id: Some(PluginId::default_plugin()),
        display_name: Some("Flux Default".to_string()),
        version: Some(default_plugin_version()),
        source_kind: PluginSourceKind::Builtin,
        source_name: "builtin".to_string(),
        source_path: None,
        content: true,
        locked: true,
        error_message: None,
    }
}

fn discovered_source_record(source: &DiscoveredPluginSource) -> PluginSourceRecord {
    PluginSourceRecord {
        plugin_id: Some(source.plugin_id.clone()),
        display_name: Some(source.display_name.clone()),
        version: Some(source.version.clone()),
        source_kind: source.source_kind,
        source_name: source.source_name.clone(),
        source_path: Some(source.source_path.clone()),
        content: source.manifest.content,
        locked: false,
        error_message: None,
    }
}

fn rejected_source_record(source: &RejectedPluginSource) -> PluginSourceRecord {
    PluginSourceRecord {
        plugin_id: source.plugin_id.clone(),
        display_name: None,
        version: None,
        source_kind: source.source_kind,
        source_name: source.source_name.clone(),
        source_path: Some(source.source_path.clone()),
        content: false,
        locked: false,
        error_message: Some(source.error.to_string()),
    }
}

fn default_loaded_plugin() -> LoadedPluginMetadata {
    LoadedPluginMetadata {
        plugin_id: PluginId::default_plugin(),
        display_name: "Flux Default".to_string(),
        version: default_plugin_version(),
        source_kind: PluginSourceKind::Builtin,
        content: true,
        locked: true,
        source_name: "builtin".to_string(),
        source_path: None,
        manifest: None,
        registration: PluginRuntimeRegistration::default(),
    }
}

fn default_registry_entry() -> PluginRegistryEntry {
    PluginRegistryEntry {
        plugin_id: Some(PluginId::default_plugin()),
        display_name: "Flux Default".to_string(),
        version: Some(default_plugin_version()),
        source_kind: Some(PluginSourceKind::Builtin),
        status: PluginRuntimeStatus::Enabled,
        locked: true,
        content: true,
        source_name: "builtin".to_string(),
        source_path: None,
        error_message: None,
    }
}

fn resolved_registry_entry(
    source: DiscoveredPluginSource,
    enabled: bool,
    locked: bool,
) -> PluginRegistryEntry {
    PluginRegistryEntry {
        plugin_id: Some(source.plugin_id.clone()),
        display_name: source.display_name.clone(),
        version: Some(source.version.clone()),
        source_kind: Some(source.source_kind),
        status: if enabled {
            PluginRuntimeStatus::Enabled
        } else {
            PluginRuntimeStatus::Disabled
        },
        locked,
        content: source.manifest.content,
        source_name: source.source_name.clone(),
        source_path: Some(source.source_path.clone()),
        error_message: None,
    }
}

fn rejected_registry_entry(
    plugin_id: PluginId,
    rejected_sources: Vec<RejectedPluginSource>,
    source_kind: PluginSourceKind,
) -> PluginRegistryEntry {
    let first = rejected_sources
        .first()
        .expect("rejected sources must exist for an error entry");
    let error_message = rejected_sources
        .iter()
        .map(|source| source.error.to_string())
        .collect::<Vec<_>>()
        .join(" | ");
    PluginRegistryEntry {
        plugin_id: Some(plugin_id.clone()),
        display_name: plugin_id.as_str().to_string(),
        version: None,
        source_kind: Some(source_kind),
        status: PluginRuntimeStatus::Error,
        locked: false,
        content: false,
        source_name: first.source_name.clone(),
        source_path: Some(first.source_path.clone()),
        error_message: Some(error_message),
    }
}

fn missing_registry_entry(plugin_id: PluginId) -> PluginRegistryEntry {
    PluginRegistryEntry {
        plugin_id: Some(plugin_id.clone()),
        display_name: plugin_id.as_str().to_string(),
        version: None,
        source_kind: None,
        status: PluginRuntimeStatus::Missing,
        locked: false,
        content: false,
        source_name: plugin_id.as_str().to_string(),
        source_path: None,
        error_message: Some(
            "plugin is enabled in plugin_state.toml but no packaged or dev source was found"
                .to_string(),
        ),
    }
}

fn anonymous_rejected_registry_entry(rejected: RejectedPluginSource) -> PluginRegistryEntry {
    PluginRegistryEntry {
        plugin_id: None,
        display_name: rejected.source_name.clone(),
        version: None,
        source_kind: Some(rejected.source_kind),
        status: PluginRuntimeStatus::Error,
        locked: false,
        content: false,
        source_name: rejected.source_name,
        source_path: Some(rejected.source_path),
        error_message: Some(rejected.error.to_string()),
    }
}

fn loaded_metadata_from_source(
    source: &DiscoveredPluginSource,
    locked: bool,
) -> LoadedPluginMetadata {
    LoadedPluginMetadata {
        plugin_id: source.plugin_id.clone(),
        display_name: source.display_name.clone(),
        version: source.version.clone(),
        source_kind: source.source_kind,
        content: source.manifest.content,
        locked,
        source_name: source.source_name.clone(),
        source_path: Some(source.source_path.clone()),
        manifest: Some(source.manifest.clone()),
        registration: source.registration.clone(),
    }
}

fn register_loaded_plugin_content(
    content_registry: &mut ContentRegistry,
    loaded: &LoadedPluginMetadata,
) -> Result<(), String> {
    if !loaded.content {
        return Ok(());
    }

    let mut next_substances = content_registry
        .substances()
        .values()
        .cloned()
        .collect::<Vec<_>>();
    next_substances.extend(loaded.registration.gas_substances.iter().cloned());
    SubstanceRegistry::new(next_substances).map_err(|error| {
        format!(
            "plugin '{}' content registration failed: {}",
            loaded.plugin_id, error
        )
    })?;

    content_registry.register_provider_plugin(loaded.plugin_id.clone());
    for substance in &loaded.registration.gas_substances {
        content_registry.register_substance(substance.clone());
    }
    Ok(())
}

fn default_plugin_version() -> PluginVersion {
    PluginVersion::parse(env!("CARGO_PKG_VERSION")).expect("crate version must stay valid")
}

fn source_record_sort_key(
    left: &PluginSourceRecord,
    right: &PluginSourceRecord,
) -> std::cmp::Ordering {
    let left_group = if left.plugin_id.as_ref() == Some(&PluginId::default_plugin()) {
        0
    } else {
        1
    };
    let right_group = if right.plugin_id.as_ref() == Some(&PluginId::default_plugin()) {
        0
    } else {
        1
    };
    left_group
        .cmp(&right_group)
        .then_with(|| source_label(left).cmp(&source_label(right)))
        .then_with(|| left.source_name.cmp(&right.source_name))
}

fn loaded_plugin_sort_key(
    left: &LoadedPluginMetadata,
    right: &LoadedPluginMetadata,
) -> std::cmp::Ordering {
    let left_group = if left.plugin_id == PluginId::default_plugin() {
        0
    } else {
        1
    };
    let right_group = if right.plugin_id == PluginId::default_plugin() {
        0
    } else {
        1
    };
    left_group
        .cmp(&right_group)
        .then_with(|| left.plugin_id.cmp(&right.plugin_id))
}

fn registry_entry_sort_key(
    left: &PluginRegistryEntry,
    right: &PluginRegistryEntry,
) -> std::cmp::Ordering {
    let left_group = if left.plugin_id.as_ref() == Some(&PluginId::default_plugin()) {
        0
    } else {
        1
    };
    let right_group = if right.plugin_id.as_ref() == Some(&PluginId::default_plugin()) {
        0
    } else {
        1
    };
    left_group
        .cmp(&right_group)
        .then_with(|| left.label().cmp(right.label()))
}

fn source_label(source: &PluginSourceRecord) -> String {
    source
        .plugin_id
        .as_ref()
        .map(|plugin_id| plugin_id.as_str().to_string())
        .unwrap_or_else(|| source.source_name.clone())
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

    use crate::plugins::{
        registry::{bootstrap_plugin_registry, PluginBootstrapConfig},
        state::PluginRuntimeStatus,
        PluginId,
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
        fs::create_dir_all(root.join("plugins")).expect("create plugins dir");
        fs::create_dir_all(root.join("plugins_dev")).expect("create plugins_dev dir");
        root
    }

    fn sample_plugin_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("crates")
            .join("flux_stage1_sample_plugin")
    }

    fn sample_content_plugin_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("crates")
            .join("flux_stage7_sample_content_plugin")
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
        build_plugin_cdylib(&crate_root, "flux_stage1_sample_plugin.dll")
    }

    fn build_sample_content_plugin_cdylib() -> PathBuf {
        let crate_root = sample_content_plugin_root();
        build_plugin_cdylib(&crate_root, "flux_stage7_sample_content_plugin.dll")
    }

    fn build_plugin_cdylib(crate_root: &Path, dll_name: &str) -> PathBuf {
        let status = Command::new(cargo_binary())
            .arg("build")
            .arg("--manifest-path")
            .arg(crate_root.join("Cargo.toml"))
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
            .status()
            .expect("spawn cargo build for sample plugin");
        assert!(status.success(), "sample plugin cargo build failed");

        crate_root.join("target").join("debug").join(dll_name)
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

    fn create_working_sample_content_plugin_directory(root_dir: &Path) {
        let crate_root = sample_content_plugin_root();
        let dll_path = build_sample_content_plugin_cdylib();
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
        fs::copy(
            dll_path,
            root_dir.join("bin/flux_stage7_sample_content_plugin.dll"),
        )
        .expect("copy dll");
    }

    fn create_manifest_only_archive(archive_path: &Path, plugin_id: &str) {
        let file = File::create(archive_path).expect("create archive");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        let manifest = format!(
            r#"id = "{plugin_id}"
display_name = "Duplicate Test"
version = "1.0.0"
api_version = 2
dll = "bin/test.dll"
configs = "config"
assets = "assets"
content = false
"#
        );
        writer
            .start_file("manifest.toml", options)
            .expect("start manifest");
        writer
            .write_all(manifest.as_bytes())
            .expect("write manifest");
        writer.finish().expect("finish archive");
    }

    fn bootstrap_from_root(
        root: &Path,
        dev_mode: bool,
    ) -> crate::plugins::registry::PluginBootstrapOutput {
        bootstrap_plugin_registry(&PluginBootstrapConfig::from_repo_root(root, dev_mode))
    }

    #[test]
    fn plugin_registry_default_plugin_is_enabled_and_locked() {
        let root = make_temp_root("flux_registry_default");
        let output = bootstrap_from_root(&root, false);
        let default_entry = output
            .registry_state
            .entries
            .iter()
            .find(|entry| entry.plugin_id.as_ref() == Some(&PluginId::default_plugin()))
            .expect("default entry");

        assert_eq!(default_entry.status, PluginRuntimeStatus::Enabled);
        assert!(default_entry.locked);
        assert!(output.enabled_set.is_enabled(&PluginId::default_plugin()));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_registry_cannot_disable_default_plugin_from_state_file() {
        let root = make_temp_root("flux_registry_default_state");
        fs::write(
            root.join("plugin_state.toml"),
            r#"
schema_version = 1

[plugins]
flux.default = false
"#,
        )
        .expect("write plugin state");

        let output = bootstrap_from_root(&root, false);
        assert!(output.enabled_set.is_enabled(&PluginId::default_plugin()));
        assert_eq!(output.registry_state.warnings.len(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_registry_dev_source_overrides_packaged_only_in_dev_mode() {
        let root = make_temp_root("flux_registry_priority");
        create_working_sample_plugin_archive(&root.join("plugins/sample.fluxplugin"));
        create_working_sample_plugin_directory(&root.join("plugins_dev/flux.sample_stage1"));
        fs::write(
            root.join("plugin_state.toml"),
            r#"
schema_version = 1

[plugins]
flux.sample_stage1 = true
"#,
        )
        .expect("write plugin state");

        let packaged_output = bootstrap_from_root(&root, false);
        let packaged_entry = packaged_output
            .loaded_registry
            .entries()
            .iter()
            .find(|entry| entry.plugin_id.as_str() == "flux.sample_stage1")
            .expect("packaged entry");
        assert_eq!(
            packaged_entry.source_kind,
            crate::plugins::source::PluginSourceKind::Packaged
        );

        let dev_output = bootstrap_from_root(&root, true);
        let dev_entry = dev_output
            .loaded_registry
            .entries()
            .iter()
            .find(|entry| entry.plugin_id.as_str() == "flux.sample_stage1")
            .expect("dev entry");
        assert_eq!(
            dev_entry.source_kind,
            crate::plugins::source::PluginSourceKind::Dev
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_registry_enabled_missing_plugin_is_reported_as_missing() {
        let root = make_temp_root("flux_registry_missing");
        fs::write(
            root.join("plugin_state.toml"),
            r#"
schema_version = 1

[plugins]
missing.plugin = true
"#,
        )
        .expect("write plugin state");

        let output = bootstrap_from_root(&root, false);
        let missing_entry = output
            .registry_state
            .entries
            .iter()
            .find(|entry| entry.plugin_id.as_ref().map(PluginId::as_str) == Some("missing.plugin"))
            .expect("missing entry");

        assert_eq!(missing_entry.status, PluginRuntimeStatus::Missing);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_registry_duplicate_packaged_plugin_id_becomes_error() {
        let root = make_temp_root("flux_registry_duplicate");
        create_manifest_only_archive(&root.join("plugins/alpha.fluxplugin"), "dup.test");
        create_manifest_only_archive(&root.join("plugins/beta.fluxplugin"), "dup.test");

        let output = bootstrap_from_root(&root, false);
        let duplicate_entry = output
            .registry_state
            .entries
            .iter()
            .find(|entry| entry.plugin_id.as_ref().map(PluginId::as_str) == Some("dup.test"))
            .expect("duplicate entry");

        assert_eq!(duplicate_entry.status, PluginRuntimeStatus::Error);
        assert!(duplicate_entry
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("duplicate packaged plugin id"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_registry_non_content_plugin_does_not_enter_content_registry() {
        let root = make_temp_root("flux_registry_non_content");
        create_working_sample_plugin_archive(&root.join("plugins/sample.fluxplugin"));
        fs::write(
            root.join("plugin_state.toml"),
            r#"
schema_version = 1

[plugins]
flux.sample_stage1 = true
"#,
        )
        .expect("write plugin state");

        let output = bootstrap_from_root(&root, false);
        assert!(!output
            .content_registry
            .provider_plugins()
            .contains(&PluginId::parse("flux.sample_stage1").expect("valid id")));
        assert!(output
            .content_registry
            .provider_plugins()
            .contains(&PluginId::default_plugin()));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_registry_enabled_content_plugin_registers_external_gas() {
        let root = make_temp_root("flux_registry_content_gas");
        create_working_sample_content_plugin_directory(
            &root.join("plugins_dev/flux.sample_content"),
        );
        fs::write(
            root.join("plugin_state.toml"),
            r#"
schema_version = 1

[plugins]
flux.sample_content = true
"#,
        )
        .expect("write plugin state");

        let output = bootstrap_from_root(&root, true);
        assert!(output
            .content_registry
            .provider_plugins()
            .contains(&PluginId::parse("flux.sample_content").expect("valid id")));
        assert!(output
            .content_registry
            .substances()
            .keys()
            .any(|id| id.as_str() == "flux.sample_content.substance.neon"));

        let registry = crate::config::GameConfig::load_gas_registry_from_default_location(
            &output.content_registry,
        )
        .expect("gas registry with plugin substance");
        assert_eq!(registry.index_of("neon"), Some(1));
        assert_eq!(
            registry.stable_id_by_index(1).map(|id| id.as_str()),
            Some("flux.sample_content.substance.neon")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_registry_main_menu_status_is_not_empty_without_external_plugins() {
        let root = make_temp_root("flux_registry_status");
        let output = bootstrap_from_root(&root, false);

        let status = output
            .registry_state
            .main_menu_status_text()
            .expect("status text");
        assert!(status.contains("flux.default"));

        let _ = fs::remove_dir_all(root);
    }
}
