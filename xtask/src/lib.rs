use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fmt, fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::Command,
};

use flux_engine::plugins::{
    validate_archive_entry_path, validate_expanded_plugin_root, validate_packaged_plugin_archive,
    PluginId, PluginManifest, PACKAGED_PLUGIN_EXTENSION,
};
use zip::{write::SimpleFileOptions, ZipWriter};

mod plugin_sdk_docs;
mod sprite_ktx;
mod target_cleanup;

const MANIFEST_RELATIVE: &str = "package_template/manifest.toml";
const EXPANDED_OUTPUT_ROOT: &str = "target/plugins/expanded";
const PACKAGE_OUTPUT_ROOT: &str = "target/plugins/packages";
const DEV_PLUGIN_ROOT: &str = "plugins_dev";

/// Error returned by the plugin build helper.
#[derive(Debug)]
pub struct XtaskError(String);

impl XtaskError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for XtaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for XtaskError {}

impl From<std::io::Error> for XtaskError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

impl From<zip::result::ZipError> for XtaskError {
    fn from(error: zip::result::ZipError) -> Self {
        Self(error.to_string())
    }
}

/// One plugin project discovered under `src/plugins/<plugin_crate>/`.
#[derive(Clone, Debug)]
pub struct PluginProject {
    pub plugin_id: PluginId,
    pub crate_name: String,
    pub crate_root: PathBuf,
    pub package_template_root: PathBuf,
    pub manifest: PluginManifest,
}

/// Runs the xtask CLI using process arguments and the current directory.
pub fn run_from_env() -> Result<(), XtaskError> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let repo_root = std::env::current_dir()?;
    run_cli(&repo_root, &args)
}

/// Runs one xtask command against an explicit repository root.
pub fn run_cli(repo_root: &Path, args: &[String]) -> Result<(), XtaskError> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(usage_error());
    };
    match command {
        "build-plugin" => {
            let request = parse_build_plugin_args(args)?;
            if request.install_dev {
                let output = build_plugin_for_dev_environment(repo_root, request.plugin_id)?;
                println!("Built dev plugin at {}", output.display());
                println!("Use Reload in Main Menu -> Plugins to refresh the running game.");
            } else {
                let output = build_plugin(repo_root, request.plugin_id)?;
                println!("Built expanded plugin at {}", output.display());
            }
            Ok(())
        }
        "pack-plugin" => {
            let plugin_id = required_plugin_id(args)?;
            let output = pack_plugin(repo_root, plugin_id)?;
            println!("Packed plugin archive at {}", output.display());
            Ok(())
        }
        "build-all-plugins" => {
            let outputs = build_all_plugins(repo_root)?;
            for output in outputs {
                println!("Packed plugin archive at {}", output.display());
            }
            Ok(())
        }
        "generate-plugin-sdk-docs" => {
            plugin_sdk_docs::generate_plugin_sdk_docs(repo_root)?;
            println!("Generated Plugin SDK docs.");
            Ok(())
        }
        "check-plugin-sdk-docs" => {
            plugin_sdk_docs::check_plugin_sdk_docs(repo_root)?;
            println!("Plugin SDK docs are up to date.");
            Ok(())
        }
        "build-plugin-sdk-docs" => {
            let output = plugin_sdk_docs::build_plugin_sdk_docs(repo_root)?;
            println!("Built Plugin SDK docs at {}", output.display());
            Ok(())
        }
        "generate-sprite-ktx" => {
            let report = sprite_ktx::generate_sprite_ktx(repo_root)?;
            println!(
                "Generated KTX2 sprites: {} updated from {} PNG sources.",
                report.generated_ktx2_count(),
                report.scanned_png_count()
            );
            Ok(())
        }
        "check-sprite-ktx" => {
            let report = sprite_ktx::check_sprite_ktx(repo_root)?;
            println!(
                "Sprite KTX2 assets are up to date for {} PNG sources.",
                report.scanned_png_count()
            );
            Ok(())
        }
        "clean-target" => {
            let report = target_cleanup::clean_target(repo_root)?;
            println!(
                "Cleaned target/: removed {} entries, freed {:.3} GiB.",
                report.removed_entries(),
                report.removed_bytes() as f64 / 1024.0 / 1024.0 / 1024.0
            );
            Ok(())
        }
        "clean-target-hard" => {
            let report = target_cleanup::clean_target_hard(repo_root)?;
            println!(
                "Hard-cleaned target/: removed {} entries, freed {:.3} GiB.",
                report.removed_entries(),
                report.removed_bytes() as f64 / 1024.0 / 1024.0 / 1024.0
            );
            Ok(())
        }
        "build-release" => {
            let report = target_cleanup::build_release(repo_root)?;
            println!(
                "Cleaned target/: removed {} entries, freed {:.3} GiB.",
                report.removed_entries(),
                report.removed_bytes() as f64 / 1024.0 / 1024.0 / 1024.0
            );
            println!("Built release application.");
            Ok(())
        }
        _ => Err(usage_error()),
    }
}

/// Discovers plugin projects in deterministic `PluginId` order.
pub fn discover_plugin_projects(repo_root: &Path) -> Result<Vec<PluginProject>, XtaskError> {
    let projects_root = repo_root.join("src").join("plugins");
    if !projects_root.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();
    for entry in fs::read_dir(&projects_root).map_err(|error| {
        XtaskError::new(format!(
            "failed to read in-project plugins root '{}': {}",
            projects_root.display(),
            error
        ))
    })? {
        let entry = entry?;
        let crate_root = entry.path();
        if !crate_root.is_dir() {
            continue;
        }
        let manifest_path = crate_root.join(MANIFEST_RELATIVE);
        if !manifest_path.is_file() {
            continue;
        }
        let manifest_bytes = fs::read(&manifest_path).map_err(|error| {
            XtaskError::new(format!(
                "failed to read plugin manifest '{}': {}",
                manifest_path.display(),
                error
            ))
        })?;
        let manifest = PluginManifest::from_bytes(&manifest_bytes).map_err(|error| {
            XtaskError::new(format!(
                "plugin manifest '{}' is invalid: {}",
                manifest_path.display(),
                error
            ))
        })?;
        let crate_name = crate_root
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
            .to_string();
        projects.push(PluginProject {
            plugin_id: manifest.id.clone(),
            crate_name,
            package_template_root: crate_root.join("package_template"),
            crate_root,
            manifest,
        });
    }

    reject_duplicate_plugin_ids(&projects)?;
    projects.sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));
    Ok(projects)
}

/// Builds one plugin DLL and writes its expanded package output.
pub fn build_plugin(repo_root: &Path, plugin_id: &str) -> Result<PathBuf, XtaskError> {
    let project = find_plugin_project(repo_root, plugin_id)?;
    build_plugin_dll(repo_root, &project)?;
    let output_root = expanded_plugin_root(repo_root, &project.plugin_id);
    recreate_dir(&output_root)?;
    copy_package_template(&project, &output_root)?;
    copy_plugin_dll(&project, &output_root)?;
    validate_expanded_plugin_root(&output_root).map_err(|error| {
        XtaskError::new(format!(
            "expanded plugin '{}' failed runtime validation: {}",
            output_root.display(),
            error
        ))
    })?;
    Ok(output_root)
}

/// Builds one plugin and installs the expanded output into `plugins_dev/<plugin_id>`.
pub fn build_plugin_for_dev_environment(
    repo_root: &Path,
    plugin_id: &str,
) -> Result<PathBuf, XtaskError> {
    let expanded_root = build_plugin(repo_root, plugin_id)?;
    let plugin_id = PluginId::parse(plugin_id).map_err(|error| {
        XtaskError::new(format!("invalid plugin id '{}': {}", plugin_id, error))
    })?;
    let dev_root = dev_plugin_root(repo_root, &plugin_id);
    replace_dev_plugin_directory(&expanded_root, &dev_root)?;
    validate_expanded_plugin_root(&dev_root).map_err(|error| {
        XtaskError::new(format!(
            "dev plugin '{}' failed runtime validation after install: {}",
            dev_root.display(),
            error
        ))
    })?;
    Ok(dev_root)
}

/// Builds and packs one plugin into a `.fluxplugin` archive.
pub fn pack_plugin(repo_root: &Path, plugin_id: &str) -> Result<PathBuf, XtaskError> {
    let project = find_plugin_project(repo_root, plugin_id)?;
    let expanded_root = build_plugin(repo_root, plugin_id)?;
    let package_root = repo_root.join(PACKAGE_OUTPUT_ROOT);
    fs::create_dir_all(&package_root)?;
    let archive_path = package_root.join(format!(
        "{}.{}",
        project.plugin_id.as_str(),
        PACKAGED_PLUGIN_EXTENSION
    ));
    let files = package_file_list(&expanded_root)?;
    let file = fs::File::create(&archive_path).map_err(|error| {
        XtaskError::new(format!(
            "failed to create plugin archive '{}': {}",
            archive_path.display(),
            error
        ))
    })?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default();
    for relative in files {
        let entry_name = relative.to_string_lossy().replace('\\', "/");
        validate_archive_entry_path(&entry_name).map_err(|error| {
            XtaskError::new(format!(
                "archive entry '{}' is invalid: {}",
                entry_name, error
            ))
        })?;
        writer.start_file(entry_name, options)?;
        let mut bytes = Vec::new();
        fs::File::open(expanded_root.join(&relative))?.read_to_end(&mut bytes)?;
        writer.write_all(&bytes)?;
    }
    writer.finish()?;
    validate_packaged_plugin_archive(&archive_path).map_err(|error| {
        XtaskError::new(format!(
            "packaged plugin '{}' failed runtime validation: {}",
            archive_path.display(),
            error
        ))
    })?;
    Ok(archive_path)
}

/// Builds and packs all plugin projects in deterministic order.
pub fn build_all_plugins(repo_root: &Path) -> Result<Vec<PathBuf>, XtaskError> {
    discover_plugin_projects(repo_root)?
        .into_iter()
        .map(|project| pack_plugin(repo_root, project.plugin_id.as_str()))
        .collect()
}

/// Returns `true` when an expanded package path is safe to include in an archive.
pub fn package_path_allowed(relative: &Path) -> bool {
    if relative.components().any(forbidden_component) {
        return false;
    }
    let Some(first) = relative.components().next() else {
        return false;
    };
    let first = first.as_os_str().to_string_lossy();
    first == "manifest.toml" || first == "bin" || first == "config" || first == "assets"
}

fn required_plugin_id(args: &[String]) -> Result<&str, XtaskError> {
    if args.len() != 2 {
        return Err(usage_error());
    }
    Ok(args[1].as_str())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BuildPluginRequest<'a> {
    plugin_id: &'a str,
    install_dev: bool,
}

fn parse_build_plugin_args(args: &[String]) -> Result<BuildPluginRequest<'_>, XtaskError> {
    if args.len() == 2 {
        return Ok(BuildPluginRequest {
            plugin_id: args[1].as_str(),
            install_dev: false,
        });
    }
    if args.len() == 3 && args[2] == "--dev" {
        return Ok(BuildPluginRequest {
            plugin_id: args[1].as_str(),
            install_dev: true,
        });
    }
    if args.len() == 3 && args[1] == "--dev" {
        return Ok(BuildPluginRequest {
            plugin_id: args[2].as_str(),
            install_dev: true,
        });
    }
    Err(usage_error())
}

fn usage_error() -> XtaskError {
    XtaskError::new(
        "usage: cargo xtask build-plugin <plugin_id> [--dev] | pack-plugin <plugin_id> | build-all-plugins | generate-plugin-sdk-docs | check-plugin-sdk-docs | build-plugin-sdk-docs | generate-sprite-ktx | check-sprite-ktx | clean-target | clean-target-hard | build-release",
    )
}

fn find_plugin_project(repo_root: &Path, plugin_id: &str) -> Result<PluginProject, XtaskError> {
    discover_plugin_projects(repo_root)?
        .into_iter()
        .find(|project| project.plugin_id.as_str() == plugin_id)
        .ok_or_else(|| XtaskError::new(format!("unknown plugin id '{}'", plugin_id)))
}

fn reject_duplicate_plugin_ids(projects: &[PluginProject]) -> Result<(), XtaskError> {
    let mut by_id = BTreeMap::<&PluginId, Vec<&PluginProject>>::new();
    for project in projects {
        by_id.entry(&project.plugin_id).or_default().push(project);
    }
    let duplicates = by_id
        .into_iter()
        .filter(|(_, projects)| projects.len() > 1)
        .map(|(id, projects)| {
            let project_names = projects
                .iter()
                .map(|project| project.crate_name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} declared by {}", id, project_names)
        })
        .collect::<Vec<_>>();
    if duplicates.is_empty() {
        Ok(())
    } else {
        Err(XtaskError::new(format!(
            "duplicate plugin ids: {}",
            duplicates.join("; ")
        )))
    }
}

fn build_plugin_dll(repo_root: &Path, project: &PluginProject) -> Result<(), XtaskError> {
    let status = Command::new(cargo_binary())
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(project.crate_root.join("Cargo.toml"))
        .current_dir(repo_root)
        .status()
        .map_err(|error| XtaskError::new(format!("failed to spawn cargo build: {}", error)))?;
    if status.success() {
        Ok(())
    } else {
        Err(XtaskError::new(format!(
            "cargo build failed for plugin '{}' with status {}",
            project.plugin_id, status
        )))
    }
}

fn copy_package_template(project: &PluginProject, output_root: &Path) -> Result<(), XtaskError> {
    fs::copy(
        project.package_template_root.join("manifest.toml"),
        output_root.join("manifest.toml"),
    )?;
    for directory in [&project.manifest.configs, &project.manifest.assets] {
        let source = project.package_template_root.join(directory);
        let destination = output_root.join(directory);
        if source.exists() {
            copy_dir_recursive(&source, &destination)?;
        } else {
            fs::create_dir_all(&destination)?;
        }
    }
    Ok(())
}

fn copy_plugin_dll(project: &PluginProject, output_root: &Path) -> Result<(), XtaskError> {
    let Some(file_name) = project.manifest.dll.file_name() else {
        return Err(XtaskError::new(format!(
            "manifest dll path '{}' has no file name",
            project.manifest.dll.display()
        )));
    };
    let source = project
        .crate_root
        .join("target")
        .join("release")
        .join(file_name);
    if !source.is_file() {
        return Err(XtaskError::new(format!(
            "plugin DLL '{}' was not produced by release build",
            source.display()
        )));
    }
    let destination = output_root.join(&project.manifest.dll);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(())
}

fn package_file_list(expanded_root: &Path) -> Result<Vec<PathBuf>, XtaskError> {
    let mut files = Vec::new();
    collect_package_files(expanded_root, expanded_root, &mut files)?;
    files.sort();
    if !files.iter().any(|path| path == Path::new("manifest.toml")) {
        return Err(XtaskError::new("expanded plugin is missing manifest.toml"));
    }
    Ok(files)
}

fn collect_package_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), XtaskError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(root).map_err(|error| {
            XtaskError::new(format!(
                "failed to relativize '{}' against '{}': {}",
                path.display(),
                root.display(),
                error
            ))
        })?;
        if path.is_dir() {
            if package_path_allowed(relative) {
                collect_package_files(root, &path, files)?;
            }
            continue;
        }
        if package_path_allowed(relative) {
            files.push(relative.to_path_buf());
        }
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), XtaskError> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn recreate_dir(path: &Path) -> Result<(), XtaskError> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn replace_dev_plugin_directory(expanded_root: &Path, dev_root: &Path) -> Result<(), XtaskError> {
    let parent = dev_root.parent().ok_or_else(|| {
        XtaskError::new(format!(
            "dev plugin path '{}' has no parent directory",
            dev_root.display()
        ))
    })?;
    fs::create_dir_all(parent)?;
    let temp_root = parent.join(format!(
        ".tmp_{}_{}_{}",
        dev_root
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("plugin"),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| XtaskError::new(format!("failed to create temp id: {}", error)))?
            .as_nanos()
    ));
    if temp_root.exists() {
        fs::remove_dir_all(&temp_root)?;
    }
    copy_dir_recursive(expanded_root, &temp_root)?;
    if dev_root.exists() {
        fs::remove_dir_all(dev_root)?;
    }
    fs::rename(&temp_root, dev_root).or_else(|rename_error| {
        copy_dir_recursive(&temp_root, dev_root)?;
        fs::remove_dir_all(&temp_root)?;
        if dev_root.exists() {
            Ok(())
        } else {
            Err(rename_error.into())
        }
    })
}

fn expanded_plugin_root(repo_root: &Path, plugin_id: &PluginId) -> PathBuf {
    repo_root
        .join(EXPANDED_OUTPUT_ROOT)
        .join(plugin_id.as_str())
}

fn dev_plugin_root(repo_root: &Path, plugin_id: &PluginId) -> PathBuf {
    repo_root.join(DEV_PLUGIN_ROOT).join(plugin_id.as_str())
}

pub(crate) fn cargo_binary() -> PathBuf {
    if let Some(path) = std::env::var_os("CARGO") {
        return PathBuf::from(path);
    }
    let fallback = PathBuf::from(r"C:\Users\andreybotanic\.cargo\bin\cargo.exe");
    if fallback.exists() {
        return fallback;
    }
    PathBuf::from("cargo")
}

fn forbidden_component(component: Component<'_>) -> bool {
    let value = component.as_os_str().to_string_lossy().to_ascii_lowercase();
    matches!(
        value.as_str(),
        "target" | ".git" | ".hg" | ".svn" | ".idea" | ".vscode" | "__macosx"
    ) || value == ".env"
        || value.ends_with(".pem")
        || value.ends_with(".key")
        || value.ends_with(".p12")
        || value.contains("secret")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has repo parent")
            .to_path_buf()
    }

    fn temp_repo(prefix: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("src/plugins")).expect("create temp repo");
        root
    }

    fn write_project_manifest(root: &Path, crate_name: &str, plugin_id: &str) {
        let project = root.join("src/plugins").join(crate_name);
        fs::create_dir_all(project.join("package_template")).expect("create package template");
        fs::write(
            project.join("package_template/manifest.toml"),
            format!(
                r#"id = "{plugin_id}"
display_name = "Test Plugin"
version = "1.0.0"
api_version = 5
dll = "bin/test.dll"
configs = "config"
assets = "assets"
content = false
"#
            ),
        )
        .expect("write manifest");
    }

    #[test]
    fn discovers_real_plugin_projects_in_stable_order() {
        let projects = discover_plugin_projects(&repo_root()).expect("discover projects");
        let ids = projects
            .iter()
            .map(|project| project.plugin_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "flux.api_cell_demo",
                "flux.api_temperature_overlay",
                "flux.api_tick_demo",
                "flux.api_ui_save_demo",
                "flux.sample_content",
                "flux.sample_stage1"
            ]
        );
    }

    #[test]
    fn unknown_plugin_id_returns_error() {
        let error = build_plugin(&repo_root(), "missing.plugin").expect_err("must fail");
        assert!(error
            .to_string()
            .contains("unknown plugin id 'missing.plugin'"));
    }

    #[test]
    fn build_plugin_args_accept_optional_dev_flag() {
        let normal_args = [
            "build-plugin".to_string(),
            "flux.sample_content".to_string(),
        ];
        let normal = parse_build_plugin_args(&normal_args).expect("normal args");
        assert_eq!(
            normal,
            BuildPluginRequest {
                plugin_id: "flux.sample_content",
                install_dev: false
            }
        );

        let trailing_args = [
            "build-plugin".to_string(),
            "flux.sample_content".to_string(),
            "--dev".to_string(),
        ];
        let trailing = parse_build_plugin_args(&trailing_args).expect("trailing dev flag");
        assert_eq!(
            trailing,
            BuildPluginRequest {
                plugin_id: "flux.sample_content",
                install_dev: true
            }
        );

        let leading_args = [
            "build-plugin".to_string(),
            "--dev".to_string(),
            "flux.sample_content".to_string(),
        ];
        let leading = parse_build_plugin_args(&leading_args).expect("leading dev flag");
        assert_eq!(leading, trailing);
    }

    #[test]
    fn build_plugin_args_reject_unknown_extra_flags() {
        let error = parse_build_plugin_args(&[
            "build-plugin".to_string(),
            "flux.sample_content".to_string(),
            "--wat".to_string(),
        ])
        .expect_err("unknown flag must fail");
        assert!(error.to_string().contains("usage: cargo xtask"));
    }

    #[test]
    fn duplicate_plugin_ids_are_rejected() {
        let root = temp_repo("flux_xtask_dupe");
        write_project_manifest(&root, "a", "dup.plugin.test");
        write_project_manifest(&root, "b", "dup.plugin.test");

        let error = discover_plugin_projects(&root).expect_err("must reject duplicates");
        assert!(
            error.to_string().contains("duplicate plugin ids"),
            "unexpected duplicate test error: {}",
            error
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn package_path_filter_rejects_forbidden_entries() {
        assert!(package_path_allowed(Path::new("manifest.toml")));
        assert!(package_path_allowed(Path::new("assets/icon.png")));
        assert!(!package_path_allowed(Path::new("target/debug/plugin.dll")));
        assert!(!package_path_allowed(Path::new("assets/secret.key")));
        assert!(!package_path_allowed(Path::new(".env")));
    }

    #[test]
    fn dev_install_replaces_existing_directory_contents() {
        let root = temp_repo("flux_xtask_dev_install");
        let expanded = root.join("target/plugins/expanded/flux.sample");
        fs::create_dir_all(expanded.join("config")).expect("create expanded config");
        fs::write(expanded.join("manifest.toml"), "new").expect("write manifest");
        fs::write(expanded.join("config/sample.toml"), "config").expect("write config");

        let dev_root = root.join("plugins_dev/flux.sample");
        fs::create_dir_all(&dev_root).expect("create old dev root");
        fs::write(dev_root.join("stale.txt"), "stale").expect("write stale file");

        replace_dev_plugin_directory(&expanded, &dev_root).expect("replace dev root");

        assert_eq!(
            fs::read_to_string(dev_root.join("manifest.toml")).expect("read manifest"),
            "new"
        );
        assert_eq!(
            fs::read_to_string(dev_root.join("config/sample.toml")).expect("read config"),
            "config"
        );
        assert!(!dev_root.join("stale.txt").exists());

        let _ = fs::remove_dir_all(root);
    }
}
