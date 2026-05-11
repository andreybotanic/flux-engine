use std::{
    collections::HashMap,
    fmt, fs,
    path::{Path, PathBuf},
};

use bevy::prelude::Resource;

use crate::plugins::{
    id::PluginId,
    loader::{read_packaged_plugin_candidate, validate_packaged_plugin_candidate},
    source::{is_packaged_plugin_path, PackagedPluginSource},
    PluginManifest,
};

/// Structured stage-1 plugin contract error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginContractError {
    Io(String),
    Manifest(String),
    Archive(String),
    Dll(String),
    Abi(String),
    DuplicateId(String),
}

impl fmt::Display for PluginContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message)
            | Self::Manifest(message)
            | Self::Archive(message)
            | Self::Dll(message)
            | Self::Abi(message)
            | Self::DuplicateId(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for PluginContractError {}

/// One plugin archive that passed contract validation.
#[derive(Clone, Debug)]
pub struct AcceptedPluginContract {
    pub source_name: String,
    pub plugin_id: PluginId,
    pub manifest: PluginManifest,
}

impl AcceptedPluginContract {
    /// Returns a short startup log line for one accepted plugin package.
    pub fn log_line(&self) -> String {
        format!(
            "Plugin '{}' accepted: plugin id '{}' passed manifest and ABI validation.",
            self.source_name, self.plugin_id
        )
    }
}

/// One plugin archive that was rejected during startup validation.
#[derive(Clone, Debug)]
pub struct RejectedPluginContract {
    pub source_name: String,
    pub plugin_id: Option<PluginId>,
    pub error: PluginContractError,
}

impl RejectedPluginContract {
    /// Returns a short, user-facing rejection summary.
    pub fn status_line(&self) -> String {
        format!(
            "Plugin '{}' rejected: {}.",
            self.source_name,
            self.error.to_string().trim_end_matches('.')
        )
    }
}

/// Startup summary of external packaged plugin contract validation.
#[derive(Resource, Clone, Debug, Default)]
pub struct PluginStartupDiagnostics {
    pub accepted_plugins: Vec<AcceptedPluginContract>,
    pub rejected_plugins: Vec<RejectedPluginContract>,
}

impl PluginStartupDiagnostics {
    /// Returns the main-menu status text for startup plugin errors.
    pub fn main_menu_status_text(&self) -> Option<String> {
        match self.rejected_plugins.as_slice() {
            [] => None,
            [single] => Some(single.status_line()),
            many => Some(format!(
                "{} plugins rejected. First: {}",
                many.len(),
                many[0].status_line()
            )),
        }
    }

    /// Writes every rejection to stderr so startup logs keep full detail.
    pub fn log_to_stderr(&self) {
        for accepted in &self.accepted_plugins {
            eprintln!("{}", accepted.log_line());
        }
        for rejected in &self.rejected_plugins {
            eprintln!("{}", rejected.status_line());
        }
    }
}

/// Scans `plugins/*.fluxplugin`, validates their stage-1 contract and returns diagnostics.
pub fn scan_packaged_plugin_contracts(plugins_root: &Path) -> PluginStartupDiagnostics {
    let mut diagnostics = PluginStartupDiagnostics::default();
    if !plugins_root.exists() {
        return diagnostics;
    }

    let archives = match collect_packaged_archives(plugins_root) {
        Ok(archives) => archives,
        Err(error) => {
            diagnostics.rejected_plugins.push(RejectedPluginContract {
                source_name: plugins_root.display().to_string(),
                plugin_id: None,
                error,
            });
            return diagnostics;
        }
    };

    let mut parsed_candidates = Vec::new();
    for archive_path in archives {
        let source = PackagedPluginSource::new(archive_path.clone());
        match read_packaged_plugin_candidate(&archive_path) {
            Ok(candidate) => parsed_candidates.push(candidate),
            Err(error) => diagnostics.rejected_plugins.push(RejectedPluginContract {
                source_name: source.display_name(),
                plugin_id: None,
                error,
            }),
        }
    }

    let (unique_candidates, duplicate_rejections) = reject_duplicate_plugin_ids(parsed_candidates);
    diagnostics.rejected_plugins.extend(duplicate_rejections);

    for candidate in unique_candidates {
        match validate_packaged_plugin_candidate(&candidate) {
            Ok(accepted) => diagnostics.accepted_plugins.push(AcceptedPluginContract {
                source_name: accepted.source.display_name(),
                plugin_id: accepted.manifest.id.clone(),
                manifest: accepted.manifest,
            }),
            Err(error) => diagnostics.rejected_plugins.push(RejectedPluginContract {
                source_name: candidate.source.display_name(),
                plugin_id: Some(candidate.manifest.id.clone()),
                error,
            }),
        }
    }

    diagnostics
}

fn collect_packaged_archives(plugins_root: &Path) -> Result<Vec<PathBuf>, PluginContractError> {
    let entries = fs::read_dir(plugins_root).map_err(|error| {
        PluginContractError::Io(format!(
            "failed to read plugins root '{}': {}",
            plugins_root.display(),
            error
        ))
    })?;

    let mut archives = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            PluginContractError::Io(format!(
                "failed to read one entry from '{}': {}",
                plugins_root.display(),
                error
            ))
        })?;
        let path = entry.path();
        if is_packaged_plugin_path(&path) {
            archives.push(path);
        }
    }

    archives.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .cmp(right.file_name().unwrap_or_default())
    });
    Ok(archives)
}

fn reject_duplicate_plugin_ids(
    candidates: Vec<crate::plugins::loader::PackagedPluginCandidate>,
) -> (
    Vec<crate::plugins::loader::PackagedPluginCandidate>,
    Vec<RejectedPluginContract>,
) {
    let mut grouped =
        HashMap::<PluginId, Vec<crate::plugins::loader::PackagedPluginCandidate>>::new();
    for candidate in candidates {
        grouped
            .entry(candidate.manifest.id.clone())
            .or_default()
            .push(candidate);
    }

    let mut unique = Vec::new();
    let mut rejected = Vec::new();
    let mut ordered_groups = grouped.into_iter().collect::<Vec<_>>();
    ordered_groups.sort_by(|(left_id, _), (right_id, _)| left_id.as_str().cmp(right_id.as_str()));

    for (plugin_id, group) in ordered_groups {
        if group.len() == 1 {
            unique.push(group.into_iter().next().expect("single candidate"));
            continue;
        }

        let sources = group
            .iter()
            .map(|candidate| candidate.source.display_name())
            .collect::<Vec<_>>();
        let message = format!(
            "duplicate plugin id '{}' is declared by {}",
            plugin_id,
            sources.join(", ")
        );

        for candidate in group {
            rejected.push(RejectedPluginContract {
                source_name: candidate.source.display_name(),
                plugin_id: Some(candidate.manifest.id.clone()),
                error: PluginContractError::DuplicateId(message.clone()),
            });
        }
    }

    unique.sort_by(|left, right| left.source.display_name().cmp(&right.source.display_name()));
    (unique, rejected)
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

    use super::scan_packaged_plugin_contracts;

    fn make_temp_plugins_root(prefix: &str) -> PathBuf {
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
        fs::create_dir_all(&root).expect("create temp plugins root");
        root
    }

    fn create_packaged_plugin_archive(archive_path: &Path, manifest_id: &str) {
        let file = File::create(archive_path).expect("create archive");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        let manifest = format!(
            r#"id = "{manifest_id}"
display_name = "Duplicate Test"
version = "1.0.0"
api_version = 4
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
        assert!(dll_path.is_file(), "sample plugin DLL must exist");

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

        writer.finish().expect("finish working plugin archive");
    }

    #[test]
    fn plugin_contract_duplicate_plugin_ids_are_rejected() {
        let root = make_temp_plugins_root("flux_plugin_dupe");
        create_packaged_plugin_archive(&root.join("alpha.fluxplugin"), "dup.test");
        create_packaged_plugin_archive(&root.join("beta.fluxplugin"), "dup.test");

        let diagnostics = scan_packaged_plugin_contracts(&root);
        assert!(diagnostics.accepted_plugins.is_empty());
        assert_eq!(diagnostics.rejected_plugins.len(), 2);
        assert!(diagnostics.rejected_plugins.iter().all(|entry| entry
            .error
            .to_string()
            .contains("duplicate plugin id 'dup.test'")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_contract_working_sample_plugin_is_accepted() {
        let root = make_temp_plugins_root("flux_plugin_working");
        create_working_sample_plugin_archive(&root.join("sample.fluxplugin"));

        let diagnostics = scan_packaged_plugin_contracts(&root);
        assert_eq!(diagnostics.accepted_plugins.len(), 1);
        assert!(diagnostics.rejected_plugins.is_empty());
        assert_eq!(
            diagnostics.accepted_plugins[0].plugin_id.as_str(),
            "flux.sample_stage1"
        );
        assert!(diagnostics.main_menu_status_text().is_none());

        let _ = fs::remove_dir_all(root);
    }
}
