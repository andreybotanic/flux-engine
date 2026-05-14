use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use naga_oil::compose::get_preprocessor_data;

use crate::WgslToolError;

const WGSL_EXTENSION: &str = "wgsl";
const SKIPPED_DIRECTORIES: &[&str] = &[".git", "target"];

#[derive(Default)]
pub(crate) struct ModuleResolver {
    repo_root: PathBuf,
    repo_wgsl_files: Vec<PathBuf>,
    project_custom_modules: HashMap<String, PathBuf>,
    bevy_custom_modules: HashMap<String, PathBuf>,
    scanned_bevy_prefixes: HashSet<String>,
    cargo_registry_roots: Vec<PathBuf>,
}

impl ModuleResolver {
    pub(crate) fn new(repo_root: &Path) -> Result<Self, WgslToolError> {
        let mut repo_wgsl_files = Vec::new();
        collect_wgsl_files(repo_root, &mut repo_wgsl_files)?;
        repo_wgsl_files.sort();

        Ok(Self {
            repo_root: repo_root.to_path_buf(),
            repo_wgsl_files,
            project_custom_modules: scan_custom_modules(repo_root)?,
            bevy_custom_modules: HashMap::new(),
            scanned_bevy_prefixes: HashSet::new(),
            cargo_registry_roots: discover_cargo_registry_roots(),
        })
    }

    pub(crate) fn resolve_import(
        &mut self,
        module_name: &str,
        importer_file: &Path,
    ) -> Result<PathBuf, WgslToolError> {
        if let Some(path) = self.resolve_quoted_import(module_name, importer_file) {
            return Ok(path);
        }

        if let Some(path) = self.project_custom_modules.get(module_name) {
            return Ok(path.clone());
        }
        if let Some(path) = self.bevy_custom_modules.get(module_name) {
            return Ok(path.clone());
        }

        if let Some(prefix) = custom_module_prefix(module_name) {
            if prefix.starts_with("bevy_") {
                self.scan_bevy_prefix(prefix)?;
                if let Some(path) = self.bevy_custom_modules.get(module_name) {
                    return Ok(path.clone());
                }
            }
        }

        Err(WgslToolError::new(format!(
            "unable to resolve WGSL import '{module_name}'"
        )))
    }

    fn resolve_quoted_import(&self, module_name: &str, importer_file: &Path) -> Option<PathBuf> {
        let quoted = extract_quoted_module_path(module_name)?;
        let importer_dir = importer_file.parent().unwrap_or(importer_file);
        let normalized_quoted = quoted
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string();
        let candidates = [
            importer_dir.join(quoted),
            self.repo_root.join(quoted),
            self.repo_root.join("assets").join(quoted),
        ];
        if let Some(path) = candidates.into_iter().find(|path| path.is_file()) {
            return Some(path);
        }

        self.repo_wgsl_files
            .iter()
            .filter(|path| normalize_path(path).ends_with(&normalized_quoted))
            .min_by_key(|path| path.components().count())
            .cloned()
    }

    fn scan_bevy_prefix(&mut self, prefix: &str) -> Result<(), WgslToolError> {
        if self.scanned_bevy_prefixes.contains(prefix) {
            return Ok(());
        }

        let crate_prefix = format!("{prefix}-");
        for registry_root in &self.cargo_registry_roots {
            let mut candidates = fs::read_dir(registry_root)
                .map_err(|error| {
                    WgslToolError::new(format!(
                        "failed to read cargo registry root '{}': {}",
                        registry_root.display(),
                        error
                    ))
                })?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_dir()
                        && path
                            .file_name()
                            .and_then(OsStr::to_str)
                            .map(|name| name.starts_with(&crate_prefix))
                            .unwrap_or(false)
                })
                .collect::<Vec<_>>();
            candidates.sort();
            candidates.reverse();

            for candidate in candidates {
                let custom_modules = scan_custom_modules(&candidate)?;
                for (module_name, path) in custom_modules {
                    self.bevy_custom_modules.entry(module_name).or_insert(path);
                }
            }
        }

        self.scanned_bevy_prefixes.insert(prefix.to_string());
        Ok(())
    }
}

pub(crate) fn collect_wgsl_files(
    root: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), WgslToolError> {
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(root).map_err(|error| {
        WgslToolError::new(format!(
            "failed to read directory '{}': {}",
            root.display(),
            error
        ))
    })? {
        let entry = entry.map_err(|error| WgslToolError::new(error.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            let directory_name = entry.file_name().to_string_lossy().to_string();
            if SKIPPED_DIRECTORIES
                .iter()
                .any(|blocked| directory_name.eq_ignore_ascii_case(blocked))
            {
                continue;
            }
            collect_wgsl_files(&path, output)?;
            continue;
        }
        if path.extension().and_then(OsStr::to_str) == Some(WGSL_EXTENSION) {
            output.push(path);
        }
    }
    Ok(())
}

fn custom_module_prefix(module_name: &str) -> Option<&str> {
    if module_name.starts_with('"') {
        return None;
    }
    module_name
        .split("::")
        .next()
        .filter(|value| !value.is_empty())
}

fn extract_quoted_module_path(module_name: &str) -> Option<&str> {
    if !module_name.starts_with('"') {
        return None;
    }
    let end = module_name[1..].find('"')?;
    Some(&module_name[1..1 + end])
}

fn discover_cargo_registry_roots() -> Vec<PathBuf> {
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".cargo")));
    let Some(cargo_home) = cargo_home else {
        return Vec::new();
    };

    let src_root = cargo_home.join("registry").join("src");
    if !src_root.is_dir() {
        return Vec::new();
    }

    let mut roots = fs::read_dir(&src_root)
        .ok()
        .into_iter()
        .flat_map(|iter| iter.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    if roots.is_empty() {
        roots.push(src_root);
    }
    roots
}

fn scan_custom_modules(root: &Path) -> Result<HashMap<String, PathBuf>, WgslToolError> {
    let mut wgsl_files = Vec::new();
    collect_wgsl_files(root, &mut wgsl_files)?;

    let mut modules = HashMap::new();
    for file in wgsl_files {
        let source = fs::read_to_string(&file).map_err(|error| {
            WgslToolError::new(format!(
                "failed to read WGSL source '{}': {}",
                file.display(),
                error
            ))
        })?;
        let (import_path, _, _) = get_preprocessor_data(&source);
        if let Some(import_path) = import_path {
            modules.entry(import_path).or_insert(file);
        }
    }
    Ok(modules)
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
