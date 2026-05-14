use std::{
    collections::{BTreeSet, HashSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use naga_oil::compose::{
    get_preprocessor_data, ComposableModuleDescriptor, Composer, NagaModuleDescriptor,
    ShaderLanguage, ShaderType,
};

use crate::wgsl_imports::{collect_wgsl_files, ModuleResolver};
use crate::{cargo_binary, WgslToolError};

/// Colored report of one full WGSL validation pass.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WgslValidationReport {
    files: Vec<WgslFileValidation>,
}

impl WgslValidationReport {
    fn new(files: Vec<WgslFileValidation>) -> Self {
        Self { files }
    }

    pub(crate) fn total_files(&self) -> usize {
        self.files.len()
    }

    pub(crate) fn changed_files(&self) -> usize {
        self.files.iter().filter(|file| file.changed).count()
    }

    pub(crate) fn ok_files(&self) -> usize {
        self.files.iter().filter(|file| file.status.is_ok()).count()
    }

    pub(crate) fn error_files(&self) -> usize {
        self.total_files().saturating_sub(self.ok_files())
    }

    fn has_errors(&self) -> bool {
        self.error_files() > 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WgslFileValidation {
    relative_path: PathBuf,
    changed: bool,
    status: WgslValidationStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum WgslValidationStatus {
    Ok,
    Error(String),
}

impl WgslValidationStatus {
    fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }
}

/// Validates all WGSL files, prints a colored report and returns an error when at least one file is invalid.
pub(crate) fn run_validate_wgsl_command(repo_root: &Path) -> Result<(), WgslToolError> {
    let report = validate_all_wgsl_shaders(repo_root)?;
    print_validation_report(&report);
    if report.has_errors() {
        Err(WgslToolError::new(format!(
            "WGSL validation failed: {} file(s) have errors",
            report.error_files()
        )))
    } else {
        Ok(())
    }
}

fn validate_all_wgsl_shaders(repo_root: &Path) -> Result<WgslValidationReport, WgslToolError> {
    let all_files = collect_all_wgsl_paths(repo_root)?;
    let changed_files = collect_changed_wgsl_paths(repo_root)?;

    let mut files = Vec::with_capacity(all_files.len());
    for relative_path in all_files {
        files.push(WgslFileValidation {
            changed: changed_files.contains(&relative_path),
            status: validate_shader_file(repo_root, &relative_path),
            relative_path,
        });
    }

    Ok(WgslValidationReport::new(files))
}

fn validate_shader_file(repo_root: &Path, relative_path: &Path) -> WgslValidationStatus {
    let absolute_path = repo_root.join(relative_path);
    let source = match fs::read_to_string(&absolute_path) {
        Ok(source) => source,
        Err(error) => {
            return WgslValidationStatus::Error(format!(
                "failed to read WGSL shader '{}': {}",
                absolute_path.display(),
                error
            ));
        }
    };

    match validate_with_cargo_wgsl(repo_root, &source) {
        Ok(()) => WgslValidationStatus::Ok,
        Err(cargo_wgsl_error) => {
            if !has_preprocessor_directives(&source) {
                return WgslValidationStatus::Error(cargo_wgsl_error);
            }
            match validate_with_naga_oil(repo_root, relative_path, &absolute_path, &source) {
                Ok(()) => WgslValidationStatus::Ok,
                Err(fallback_error) => WgslValidationStatus::Error(format!(
                    "cargo-wgsl output:\n{}\n\nnaga_oil fallback output:\n{}",
                    cargo_wgsl_error, fallback_error
                )),
            }
        }
    }
}

fn validate_with_cargo_wgsl(repo_root: &Path, source: &str) -> Result<(), String> {
    let mut child = Command::new(cargo_binary())
        .arg("wgsl")
        .arg("--stdin")
        .current_dir(repo_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn 'cargo wgsl --stdin': {error}"))?;

    let Some(mut stdin) = child.stdin.take() else {
        return Err("failed to open stdin for 'cargo wgsl --stdin'".to_string());
    };
    stdin
        .write_all(source.as_bytes())
        .map_err(|error| format!("failed to write WGSL source to cargo-wgsl stdin: {error}"))?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to wait for cargo-wgsl: {error}"))?;
    if output.status.success() {
        return Ok(());
    }

    Err(combine_process_output(&output.stdout, &output.stderr))
}

fn validate_with_naga_oil(
    repo_root: &Path,
    relative_path: &Path,
    absolute_path: &Path,
    source: &str,
) -> Result<(), String> {
    let mut composer = Composer::default();
    let mut resolver = ModuleResolver::new(repo_root).map_err(|error| error.to_string())?;
    let mut visiting = HashSet::new();
    let mut added_modules = HashSet::new();

    let (_, imports, _) = get_preprocessor_data(source);
    for import in imports {
        add_import_recursive(
            &mut composer,
            &mut resolver,
            &import.import,
            absolute_path,
            &mut visiting,
            &mut added_modules,
        )?;
    }

    let file_path = normalize_path(relative_path);
    if let Err(error) = composer.make_naga_module(NagaModuleDescriptor {
        source,
        file_path: &file_path,
        shader_type: ShaderType::Wgsl,
        ..Default::default()
    }) {
        return Err(error.emit_to_string(&composer));
    }

    Ok(())
}

fn add_import_recursive(
    composer: &mut Composer,
    resolver: &mut ModuleResolver,
    module_name: &str,
    importer_file: &Path,
    visiting: &mut HashSet<String>,
    added_modules: &mut HashSet<String>,
) -> Result<(), String> {
    if added_modules.contains(module_name) {
        return Ok(());
    }
    if !visiting.insert(module_name.to_string()) {
        return Err(format!("cyclic WGSL imports detected at '{module_name}'"));
    }

    let result = (|| -> Result<(), String> {
        let module_path = resolver
            .resolve_import(module_name, importer_file)
            .map_err(|error| error.to_string())?;
        let module_source = fs::read_to_string(&module_path).map_err(|error| {
            format!(
                "failed to read WGSL module '{}': {error}",
                module_path.display()
            )
        })?;
        let (_, imports, _) = get_preprocessor_data(&module_source);
        for import in imports {
            add_import_recursive(
                composer,
                resolver,
                &import.import,
                &module_path,
                visiting,
                added_modules,
            )?;
        }

        let module_file_path = normalize_path(&module_path);
        if let Err(error) = composer.add_composable_module(ComposableModuleDescriptor {
            source: &module_source,
            file_path: &module_file_path,
            language: ShaderLanguage::Wgsl,
            ..Default::default()
        }) {
            return Err(error.emit_to_string(composer));
        }

        Ok(())
    })();

    visiting.remove(module_name);
    if result.is_ok() {
        added_modules.insert(module_name.to_string());
    }
    result
}

fn collect_all_wgsl_paths(repo_root: &Path) -> Result<Vec<PathBuf>, WgslToolError> {
    let mut absolute_paths = Vec::new();
    collect_wgsl_files(repo_root, &mut absolute_paths)?;

    let mut relative_paths = absolute_paths
        .into_iter()
        .filter_map(|path| path.strip_prefix(repo_root).ok().map(Path::to_path_buf))
        .collect::<Vec<_>>();
    relative_paths.sort();
    Ok(relative_paths)
}

fn collect_changed_wgsl_paths(repo_root: &Path) -> Result<BTreeSet<PathBuf>, WgslToolError> {
    let tracked = run_git_paths(
        repo_root,
        &[
            "diff",
            "--name-only",
            "--diff-filter=ACMR",
            "HEAD",
            "--",
            "*.wgsl",
        ],
    )?;
    let untracked = run_git_paths(
        repo_root,
        &["ls-files", "--others", "--exclude-standard", "--", "*.wgsl"],
    )?;

    let mut unique_paths = BTreeSet::new();
    for path in tracked.into_iter().chain(untracked) {
        if !path.as_os_str().is_empty() && repo_root.join(&path).is_file() {
            unique_paths.insert(path);
        }
    }
    Ok(unique_paths)
}

fn run_git_paths(repo_root: &Path, args: &[&str]) -> Result<Vec<PathBuf>, WgslToolError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|error| {
            WgslToolError::new(format!("failed to run git {}: {}", args.join(" "), error))
        })?;
    if !output.status.success() {
        return Err(WgslToolError::new(format!(
            "git {} failed: {}",
            args.join(" "),
            combine_process_output(&output.stdout, &output.stderr)
        )));
    }

    Ok(parse_newline_paths(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn parse_newline_paths(value: &str) -> Vec<PathBuf> {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn has_preprocessor_directives(source: &str) -> bool {
    source
        .lines()
        .any(|line| line.trim_start().starts_with('#'))
}

fn combine_process_output(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => "process exited with non-zero status and no output".to_string(),
        (false, true) => stdout,
        (true, false) => stderr,
        (false, false) => format!("{stdout}\n{stderr}"),
    }
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn print_validation_report(report: &WgslValidationReport) {
    println!("{}", style("WGSL validation report", "1;36"));
    println!();

    for file in &report.files {
        let changed = if file.changed {
            style("CHANGED", "33")
        } else {
            style("UNCHANGED", "90")
        };
        let status = if file.status.is_ok() {
            style("OK", "32")
        } else {
            style("NOT OK", "31")
        };
        println!("{}  {}  {}", changed, status, file.relative_path.display());
    }

    if report.files.is_empty() {
        println!("{}", style("No WGSL files found in repository.", "33"));
    }

    let errors = report
        .files
        .iter()
        .filter_map(|file| match &file.status {
            WgslValidationStatus::Error(error) => Some((&file.relative_path, error)),
            WgslValidationStatus::Ok => None,
        })
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        println!();
        println!("{}", style("WGSL errors:", "1;31"));
        for (relative_path, error) in errors {
            println!(
                "{}",
                style(&format!("--- {}", relative_path.display()), "31")
            );
            println!("{error}");
            println!();
        }
    }

    let summary = format!(
        "Summary: total={}, changed={}, ok={}, errors={}",
        report.total_files(),
        report.changed_files(),
        report.ok_files(),
        report.error_files()
    );
    let summary_style = if report.has_errors() { "1;31" } else { "1;32" };
    println!("{}", style(&summary, summary_style));
}

fn style(text: &str, ansi_code: &str) -> String {
    format!("\x1b[{}m{}\x1b[0m", ansi_code, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_newline_path_lists() {
        let parsed = parse_newline_paths("a.wgsl\n b/c.wgsl \n\n");
        assert_eq!(
            parsed,
            vec![PathBuf::from("a.wgsl"), PathBuf::from("b/c.wgsl")]
        );
    }

    #[test]
    fn detects_preprocessor_directives() {
        assert!(!has_preprocessor_directives("fn main() {}\n"));
        assert!(has_preprocessor_directives(
            "#import bevy_sprite::mesh2d_vertex_output::VertexOutput\n"
        ));
    }

    #[test]
    fn report_counts_reflect_file_statuses() {
        let report = WgslValidationReport::new(vec![
            WgslFileValidation {
                relative_path: PathBuf::from("a.wgsl"),
                changed: true,
                status: WgslValidationStatus::Ok,
            },
            WgslFileValidation {
                relative_path: PathBuf::from("b.wgsl"),
                changed: false,
                status: WgslValidationStatus::Error("bad".to_string()),
            },
        ]);
        assert_eq!(report.total_files(), 2);
        assert_eq!(report.changed_files(), 1);
        assert_eq!(report.ok_files(), 1);
        assert_eq!(report.error_files(), 1);
        assert!(report.has_errors());
    }
}
