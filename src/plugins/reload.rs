use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    config::GasRegistry,
    plugins::{
        rebuild_plugin_registry_from_enabled_set, EnabledPluginSet, PluginBootstrapConfig,
        PluginBootstrapOutput, PluginId, PluginRegistryState, PluginSourceFingerprint,
        PluginSourceKind, PluginSourceRecord, PluginSourceRegistry,
    },
};

static RELOAD_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Request parameters for one manual plugin registry reload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginReloadRequest {
    pub force: bool,
}

impl PluginReloadRequest {
    /// Builds the default manual reload request used by the Plugins menu.
    pub fn manual() -> Self {
        Self { force: true }
    }
}

/// Error returned when a plugin reload cannot be completed safely.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginReloadError {
    WorldLoaded,
    Registry(String),
    Config(String),
}

impl fmt::Display for PluginReloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorldLoaded => {
                write!(f, "Hot reload is available only before a world is loaded.")
            }
            Self::Registry(message) | Self::Config(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for PluginReloadError {}

/// Successful atomic reload result and the resources ready to be swapped in.
#[derive(Clone, Debug)]
pub struct PluginReloadReport {
    pub generation: u64,
    pub changed_sources: Vec<PluginId>,
    pub message: String,
    pub output: PluginBootstrapOutput,
    pub gas_registry: GasRegistry,
}

/// Rebuilds plugin registries and config-dependent gas registry without mutating active state.
pub fn reload_plugin_registry(
    request: &PluginReloadRequest,
    config: &PluginBootstrapConfig,
    enabled_set: &EnabledPluginSet,
    has_world: bool,
    current_sources: &PluginSourceRegistry,
    current_registry_state: &PluginRegistryState,
) -> Result<PluginReloadReport, PluginReloadError> {
    if has_world {
        return Err(PluginReloadError::WorldLoaded);
    }

    let output = rebuild_plugin_registry_from_enabled_set(config, enabled_set.clone());
    let gas_registry = crate::config::GameConfig::load_gas_registry_from_default_location(
        &output.content_registry,
    )
    .map_err(PluginReloadError::Config)?;
    let changed_sources = changed_plugin_sources(current_sources, &output.source_registry);
    let generation = RELOAD_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let message = reload_message(
        request,
        &changed_sources,
        current_registry_state,
        generation,
    );

    Ok(PluginReloadReport {
        generation,
        changed_sources,
        message,
        output,
        gas_registry,
    })
}

fn reload_message(
    request: &PluginReloadRequest,
    changed_sources: &[PluginId],
    current_registry_state: &PluginRegistryState,
    generation: u64,
) -> String {
    if !request.force && changed_sources.is_empty() {
        return format!("Plugins unchanged at generation {}.", generation);
    }
    let previous_count = current_registry_state.entries.len();
    format!(
        "Plugins reloaded: {} source(s) changed. Generation {}. Previous entries: {}.",
        changed_sources.len(),
        generation,
        previous_count
    )
}

fn changed_plugin_sources(
    old_sources: &PluginSourceRegistry,
    new_sources: &PluginSourceRegistry,
) -> Vec<PluginId> {
    let old_fingerprints = old_sources
        .entries()
        .iter()
        .filter_map(source_fingerprint_key)
        .collect::<BTreeMap<_, _>>();
    let mut changed = BTreeSet::new();
    for source in new_sources.entries() {
        let Some((key, fingerprint)) = source_fingerprint_key(source) else {
            continue;
        };
        if old_fingerprints.get(&key) != Some(&fingerprint) {
            changed.insert(key.plugin_id);
        }
    }
    changed.into_iter().collect()
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SourceFingerprintKey {
    plugin_id: PluginId,
    source_kind: PluginSourceKind,
    source_name: String,
}

fn source_fingerprint_key(
    source: &PluginSourceRecord,
) -> Option<(SourceFingerprintKey, PluginSourceFingerprint)> {
    if source.fingerprint.entries().is_empty() {
        return None;
    }
    let plugin_id = source.plugin_id.clone()?;
    Some((
        SourceFingerprintKey {
            plugin_id,
            source_kind: source.source_kind,
            source_name: source.source_name.clone(),
        },
        source.fingerprint.clone(),
    ))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
        thread,
        time::Duration,
    };

    use crate::plugins::{
        registry::{bootstrap_plugin_registry, PluginBootstrapConfig},
        reload::{reload_plugin_registry, PluginReloadError, PluginReloadRequest},
        PluginId, PluginRuntimeStatus,
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

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
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

    fn build_sample_content_plugin_cdylib() -> PathBuf {
        let crate_root = repo_root()
            .join("src")
            .join("plugins")
            .join("flux_stage7_sample_content_plugin");
        let status = Command::new(cargo_binary())
            .arg("build")
            .arg("--manifest-path")
            .arg(crate_root.join("Cargo.toml"))
            .current_dir(repo_root())
            .status()
            .expect("spawn cargo build for sample content plugin");
        assert!(status.success(), "sample content plugin build failed");
        crate_root
            .join("target")
            .join("debug")
            .join("flux_stage7_sample_content_plugin.dll")
    }

    fn create_working_sample_content_plugin_directory(root_dir: &Path) {
        let crate_root = repo_root()
            .join("src")
            .join("plugins")
            .join("flux_stage7_sample_content_plugin");
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

    fn enabled_state() -> String {
        r#"
schema_version = 1

[plugins]
flux.sample_content = true
"#
        .to_string()
    }

    fn pause_for_mtime_tick() {
        thread::sleep(Duration::from_millis(20));
    }

    #[test]
    fn plugin_reload_rejects_loaded_world_with_clear_message() {
        let root = make_temp_root("flux_reload_world_loaded");
        let config = PluginBootstrapConfig::from_repo_root(&root, true);
        let output = bootstrap_plugin_registry(&config);

        let error = reload_plugin_registry(
            &PluginReloadRequest::manual(),
            &config,
            &output.enabled_set,
            true,
            &output.source_registry,
            &output.registry_state,
        )
        .expect_err("reload must be blocked while a world is loaded");

        assert_eq!(error, PluginReloadError::WorldLoaded);
        assert!(error.to_string().contains("before a world is loaded"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_reload_success_reports_changed_generation_after_manifest_edit() {
        let root = make_temp_root("flux_reload_manifest");
        let plugin_root = root.join("plugins_dev/flux.sample_content");
        create_working_sample_content_plugin_directory(&plugin_root);
        fs::write(root.join("plugin_state.toml"), enabled_state()).expect("write state");
        let config = PluginBootstrapConfig::from_repo_root(&root, true);
        let output = bootstrap_plugin_registry(&config);

        pause_for_mtime_tick();
        let manifest_path = plugin_root.join("manifest.toml");
        let manifest = fs::read_to_string(&manifest_path)
            .expect("read manifest")
            .replace(
                "Flux Stage7 Sample Content Plugin",
                "Flux Stage8 Reloaded Sample Content Plugin",
            );
        fs::write(&manifest_path, manifest).expect("edit manifest");

        let report = reload_plugin_registry(
            &PluginReloadRequest::manual(),
            &config,
            &output.enabled_set,
            false,
            &output.source_registry,
            &output.registry_state,
        )
        .expect("reload should succeed");

        assert!(report.generation > 0);
        assert!(report
            .changed_sources
            .iter()
            .any(|id| id.as_str() == "flux.sample_content"));
        assert!(report.message.contains("Plugins reloaded"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_reload_detects_config_and_dll_fingerprint_changes() {
        let root = make_temp_root("flux_reload_fingerprint");
        let plugin_root = root.join("plugins_dev/flux.sample_content");
        create_working_sample_content_plugin_directory(&plugin_root);
        fs::write(root.join("plugin_state.toml"), enabled_state()).expect("write state");
        let config = PluginBootstrapConfig::from_repo_root(&root, true);
        let first = bootstrap_plugin_registry(&config);

        pause_for_mtime_tick();
        fs::write(plugin_root.join("config/sample.toml"), "# changed\n").expect("edit config");
        let second = reload_plugin_registry(
            &PluginReloadRequest::manual(),
            &config,
            &first.enabled_set,
            false,
            &first.source_registry,
            &first.registry_state,
        )
        .expect("config reload");
        assert!(second
            .changed_sources
            .iter()
            .any(|id| id.as_str() == "flux.sample_content"));

        pause_for_mtime_tick();
        fs::write(
            plugin_root.join("bin/flux_stage7_sample_content_plugin.dll"),
            build_sample_content_plugin_cdylib()
                .read_bytes()
                .expect("read dll"),
        )
        .expect("rewrite dll");
        let third = reload_plugin_registry(
            &PluginReloadRequest::manual(),
            &config,
            &second.output.enabled_set,
            false,
            &second.output.source_registry,
            &second.output.registry_state,
        )
        .expect("dll reload");
        assert!(third
            .changed_sources
            .iter()
            .any(|id| id.as_str() == "flux.sample_content"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_reload_failure_leaves_previous_output_owned_by_caller() {
        let root = make_temp_root("flux_reload_failure");
        let plugin_root = root.join("plugins_dev/flux.sample_content");
        create_working_sample_content_plugin_directory(&plugin_root);
        fs::write(root.join("plugin_state.toml"), enabled_state()).expect("write state");
        let config = PluginBootstrapConfig::from_repo_root(&root, true);
        let output = bootstrap_plugin_registry(&config);
        assert!(output
            .content_registry
            .substances()
            .keys()
            .any(|id| id.as_str() == "flux.sample_content.substance.neon"));

        fs::write(plugin_root.join("manifest.toml"), "not = [valid toml").expect("break manifest");
        let report = reload_plugin_registry(
            &PluginReloadRequest::manual(),
            &config,
            &output.enabled_set,
            false,
            &output.source_registry,
            &output.registry_state,
        )
        .expect("broken source becomes registry error, not a panic");

        assert!(report.output.registry_state.entries.iter().any(|entry| {
            entry.status == PluginRuntimeStatus::Error
                && entry
                    .error_message
                    .as_deref()
                    .unwrap_or_default()
                    .contains("failed to parse manifest.toml")
        }));
        assert!(output
            .content_registry
            .substances()
            .keys()
            .any(|id| id.as_str() == "flux.sample_content.substance.neon"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_reload_disabled_plugin_does_not_register_content_and_default_stays_locked() {
        let root = make_temp_root("flux_reload_disabled");
        let plugin_root = root.join("plugins_dev/flux.sample_content");
        create_working_sample_content_plugin_directory(&plugin_root);
        let config = PluginBootstrapConfig::from_repo_root(&root, true);
        let output = bootstrap_plugin_registry(&config);

        let report = reload_plugin_registry(
            &PluginReloadRequest::manual(),
            &config,
            &output.enabled_set,
            false,
            &output.source_registry,
            &output.registry_state,
        )
        .expect("reload");

        assert!(!report
            .output
            .content_registry
            .provider_plugins()
            .iter()
            .any(|id| id.as_str() == "flux.sample_content"));
        let default_entry = report
            .output
            .registry_state
            .entries
            .iter()
            .find(|entry| entry.plugin_id.as_ref() == Some(&PluginId::default_plugin()))
            .expect("default plugin entry");
        assert_eq!(default_entry.status, PluginRuntimeStatus::Enabled);
        assert!(default_entry.locked);

        let _ = fs::remove_dir_all(root);
    }

    trait ReadBytes {
        fn read_bytes(self) -> Result<Vec<u8>, std::io::Error>;
    }

    impl ReadBytes for PathBuf {
        fn read_bytes(self) -> Result<Vec<u8>, std::io::Error> {
            fs::read(self)
        }
    }
}
