use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{cargo_binary, XtaskError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CleanupMode {
    Conservative,
    Aggressive,
}

/// Summary of one completed cleanup pass over `target/`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TargetCleanupReport {
    removed_entries: u64,
    removed_bytes: u64,
}

impl TargetCleanupReport {
    fn record(&mut self, bytes: u64) {
        self.removed_entries += 1;
        self.removed_bytes += bytes;
    }

    pub(crate) fn removed_entries(&self) -> u64 {
        self.removed_entries
    }

    pub(crate) fn removed_bytes(&self) -> u64 {
        self.removed_bytes
    }
}

/// Removes transient build artifacts from `target/` while preserving the live xtask binary.
pub(crate) fn clean_target(repo_root: &Path) -> Result<TargetCleanupReport, XtaskError> {
    clean_target_with_mode(repo_root, CleanupMode::Conservative)
}

/// Removes both transient artifacts and Cargo cache from `target/`.
pub(crate) fn clean_target_hard(repo_root: &Path) -> Result<TargetCleanupReport, XtaskError> {
    clean_target_with_mode(repo_root, CleanupMode::Aggressive)
}

fn clean_target_with_mode(
    repo_root: &Path,
    mode: CleanupMode,
) -> Result<TargetCleanupReport, XtaskError> {
    let target_root = repo_root.join("target");
    let mut preserved = Vec::new();
    if let Ok(current_exe) = std::env::current_exe() {
        preserved.push(current_exe.clone());
        if let Some(pdb_path) = sibling_pdb_path(&current_exe) {
            preserved.push(pdb_path);
        }
    }
    clean_target_directory(&target_root, &preserved, mode)
}

/// Runs conservative target cleanup and then performs `cargo build --release`.
pub(crate) fn build_release(repo_root: &Path) -> Result<TargetCleanupReport, XtaskError> {
    let report = clean_target(repo_root)?;
    let status = Command::new(cargo_binary())
        .arg("build")
        .arg("--release")
        .current_dir(repo_root)
        .status()
        .map_err(|error| {
            XtaskError::new(format!("failed to spawn cargo build --release: {error}"))
        })?;
    if status.success() {
        Ok(report)
    } else {
        Err(XtaskError::new(format!(
            "cargo build --release failed with status {status}"
        )))
    }
}

fn clean_target_directory(
    target_root: &Path,
    preserved_paths: &[PathBuf],
    mode: CleanupMode,
) -> Result<TargetCleanupReport, XtaskError> {
    if !target_root.exists() {
        return Ok(TargetCleanupReport::default());
    }

    let mut report = TargetCleanupReport::default();
    for entry in fs::read_dir(target_root).map_err(|error| {
        XtaskError::new(format!(
            "failed to read target directory '{}': {}",
            target_root.display(),
            error
        ))
    })? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        if file_name == OsStr::new("debug") {
            clean_debug_directory(&path, preserved_paths, &mut report, mode)?;
            continue;
        }
        if file_name == OsStr::new("release") {
            clean_release_directory(&path, &mut report, mode)?;
            continue;
        }
        if should_remove_target_root_entry(&path, &file_name) {
            remove_path(&path, &mut report)?;
        }
    }
    Ok(report)
}

fn clean_debug_directory(
    debug_root: &Path,
    preserved_paths: &[PathBuf],
    report: &mut TargetCleanupReport,
    mode: CleanupMode,
) -> Result<(), XtaskError> {
    if !debug_root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(debug_root).map_err(|error| {
        XtaskError::new(format!(
            "failed to read debug directory '{}': {}",
            debug_root.display(),
            error
        ))
    })? {
        let entry = entry?;
        let path = entry.path();
        if is_preserved_path(&path, preserved_paths) {
            continue;
        }
        if matches!(mode, CleanupMode::Aggressive) && path.is_dir() {
            prune_directory_contents(&path, preserved_paths, report)?;
            let _ = fs::remove_dir(&path);
        } else if should_remove_profile_file(&path, &entry.file_name()) {
            remove_path(&path, report)?;
        }
    }
    Ok(())
}

fn clean_release_directory(
    release_root: &Path,
    report: &mut TargetCleanupReport,
    mode: CleanupMode,
) -> Result<(), XtaskError> {
    if !release_root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(release_root).map_err(|error| {
        XtaskError::new(format!(
            "failed to read release directory '{}': {}",
            release_root.display(),
            error
        ))
    })? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        if path.is_dir() {
            if matches!(mode, CleanupMode::Aggressive) && should_remove_release_dir(&file_name) {
                remove_path(&path, report)?;
            }
            continue;
        }
        if matches!(mode, CleanupMode::Aggressive) {
            if should_remove_release_file_aggressive(&path, &file_name) {
                remove_path(&path, report)?;
            }
        } else if should_remove_profile_file(&path, &file_name) {
            remove_path(&path, report)?;
        }
    }
    Ok(())
}

fn should_remove_target_root_entry(path: &Path, file_name: &OsStr) -> bool {
    let file_name = file_name.to_string_lossy();
    if path.is_dir() {
        return file_name == "tmp"
            || file_name == "codex_runcheck"
            || file_name.starts_with("flycheck");
    }

    path.extension() == Some(OsStr::new("log"))
        || file_name == ".rustc_info.json"
        || file_name.starts_with("tmp_")
}

fn should_remove_release_dir(file_name: &OsStr) -> bool {
    matches!(
        file_name.to_string_lossy().as_ref(),
        "deps" | "build" | ".fingerprint" | "examples" | "incremental"
    )
}

fn should_remove_release_file_aggressive(path: &Path, file_name: &OsStr) -> bool {
    let file_name = file_name.to_string_lossy();
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .map(|value| value.to_ascii_lowercase());

    !matches!(extension.as_deref(), Some("exe") | Some("pdb"))
        || file_name.ends_with(".stdout.log")
        || file_name.ends_with(".stderr.log")
        || file_name.ends_with(".out.log")
        || file_name.ends_with(".err.log")
        || file_name.ends_with(".log")
}

fn should_remove_profile_file(path: &Path, file_name: &OsStr) -> bool {
    let file_name = file_name.to_string_lossy().to_ascii_lowercase();
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .map(|value| value.to_ascii_lowercase());
    matches!(extension.as_deref(), Some("log"))
        || file_name.ends_with(".stdout.log")
        || file_name.ends_with(".stderr.log")
        || file_name.ends_with(".out.log")
        || file_name.ends_with(".err.log")
        || file_name.starts_with("tmp_")
        || file_name.starts_with("codex_") && matches!(extension.as_deref(), Some("png"))
}

fn is_preserved_path(path: &Path, preserved_paths: &[PathBuf]) -> bool {
    preserved_paths.iter().any(|preserved| preserved == path)
}

fn sibling_pdb_path(exe_path: &Path) -> Option<PathBuf> {
    let extension = exe_path.extension()?.to_string_lossy().to_ascii_lowercase();
    if extension != "exe" {
        return None;
    }
    Some(exe_path.with_extension("pdb"))
}

fn remove_path(path: &Path, report: &mut TargetCleanupReport) -> Result<(), XtaskError> {
    if !path.exists() {
        return Ok(());
    }

    let bytes = path_size(path)?;
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|error| {
            XtaskError::new(format!(
                "failed to remove directory '{}': {}",
                path.display(),
                error
            ))
        })?;
    } else {
        fs::remove_file(path).map_err(|error| {
            XtaskError::new(format!(
                "failed to remove file '{}': {}",
                path.display(),
                error
            ))
        })?;
    }
    report.record(bytes);
    Ok(())
}

fn prune_directory_contents(
    root: &Path,
    preserved_paths: &[PathBuf],
    report: &mut TargetCleanupReport,
) -> Result<(), XtaskError> {
    for entry in fs::read_dir(root).map_err(|error| {
        XtaskError::new(format!(
            "failed to read directory while pruning '{}': {}",
            root.display(),
            error
        ))
    })? {
        let entry = entry?;
        let path = entry.path();
        if is_preserved_path(&path, preserved_paths) {
            continue;
        }
        if path.is_dir() {
            prune_directory_contents(&path, preserved_paths, report)?;
            let _ = fs::remove_dir(&path);
            continue;
        }
        let _ = remove_path(&path, report);
    }
    Ok(())
}

fn path_size(path: &Path) -> Result<u64, XtaskError> {
    let metadata = fs::metadata(path).map_err(|error| {
        XtaskError::new(format!(
            "failed to read metadata for '{}': {}",
            path.display(),
            error
        ))
    })?;
    if metadata.is_file() {
        return Ok(metadata.len());
    }

    let mut total = 0_u64;
    for entry in fs::read_dir(path).map_err(|error| {
        XtaskError::new(format!(
            "failed to read directory while sizing '{}': {}",
            path.display(),
            error
        ))
    })? {
        let entry = entry?;
        total = total.saturating_add(path_size(&entry.path())?);
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_target_root(prefix: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("target")).expect("create target root");
        root
    }

    fn write_bytes(path: &Path, len: usize) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, vec![b'x'; len]).expect("write bytes");
    }

    #[test]
    fn clean_target_removes_debug_artifacts_and_root_logs() {
        let root = temp_target_root("flux_xtask_clean_target");
        let target = root.join("target");
        write_bytes(&target.join("debug/deps/huge.bin"), 16);
        write_bytes(&target.join("debug/app.pdb"), 8);
        write_bytes(&target.join("release_keep.bin"), 4);
        write_bytes(&target.join("release_run_stdout.log"), 2);
        write_bytes(&target.join(".rustc_info.json"), 1);
        fs::create_dir_all(target.join("flycheck0")).expect("create flycheck dir");
        fs::create_dir_all(target.join("codex_runcheck")).expect("create codex dir");

        let report = clean_target_directory(&target, &[], CleanupMode::Conservative)
            .expect("cleanup target");

        assert!(report.removed_entries() >= 4);
        assert!(!target.join("release_run_stdout.log").exists());
        assert!(!target.join(".rustc_info.json").exists());
        assert!(!target.join("flycheck0").exists());
        assert!(!target.join("codex_runcheck").exists());
        assert!(target.join("debug/deps/huge.bin").exists());
        assert!(target.join("debug/app.pdb").exists());
        assert!(target.join("release_keep.bin").exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn conservative_cleanup_preserves_debug_cache() {
        let root = temp_target_root("flux_xtask_clean_preserve");
        let target = root.join("target");
        let preserved_exe = target.join("debug/xtask.exe");
        let preserved_pdb = target.join("debug/xtask.pdb");
        write_bytes(&preserved_exe, 5);
        write_bytes(&preserved_pdb, 7);
        write_bytes(&target.join("debug/other.exe"), 11);
        write_bytes(&target.join("debug/deps/other.rlib"), 13);
        write_bytes(&target.join("debug/debug_run_stdout.log"), 17);

        let report = clean_target_directory(
            &target,
            &[preserved_exe.clone(), preserved_pdb.clone()],
            CleanupMode::Conservative,
        )
        .expect("cleanup target");

        assert_eq!(report.removed_entries(), 1);
        assert_eq!(report.removed_bytes(), 17);
        assert!(preserved_exe.exists());
        assert!(preserved_pdb.exists());
        assert!(target.join("debug/other.exe").exists());
        assert!(target.join("debug/deps").exists());
        assert!(!target.join("debug/debug_run_stdout.log").exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hard_cleanup_removes_release_cache_but_keeps_release_binaries() {
        let root = temp_target_root("flux_xtask_clean_release");
        let target = root.join("target");
        write_bytes(&target.join("release/deps/libhuge.rlib"), 17);
        write_bytes(&target.join("release/build/script-output.bin"), 19);
        write_bytes(&target.join("release/.fingerprint/stamp"), 23);
        write_bytes(&target.join("release/flux_engine.exe"), 29);
        write_bytes(&target.join("release/flux_engine.pdb"), 31);
        write_bytes(&target.join("release/libflux_engine.rlib"), 37);
        write_bytes(&target.join("release/runtime_stdout.log"), 41);
        write_bytes(&target.join("release/codex_plugins_screen.png"), 43);

        let report =
            clean_target_directory(&target, &[], CleanupMode::Aggressive).expect("cleanup target");

        assert!(report.removed_entries() >= 6);
        assert!(!target.join("release/deps").exists());
        assert!(!target.join("release/build").exists());
        assert!(!target.join("release/.fingerprint").exists());
        assert!(!target.join("release/libflux_engine.rlib").exists());
        assert!(!target.join("release/runtime_stdout.log").exists());
        assert!(!target.join("release/codex_plugins_screen.png").exists());
        assert!(target.join("release/flux_engine.exe").exists());
        assert!(target.join("release/flux_engine.pdb").exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn conservative_cleanup_keeps_release_cache_and_removes_release_logs() {
        let root = temp_target_root("flux_xtask_clean_release_keep_cache");
        let target = root.join("target");
        write_bytes(&target.join("release/deps/libhuge.rlib"), 17);
        write_bytes(&target.join("release/build/script-output.bin"), 19);
        write_bytes(&target.join("release/flux_engine.exe"), 29);
        write_bytes(&target.join("release/runtime_stdout.log"), 31);
        write_bytes(&target.join("release/codex_plugins_screen.png"), 37);

        let report = clean_target_directory(&target, &[], CleanupMode::Conservative)
            .expect("cleanup target");

        assert_eq!(report.removed_entries(), 2);
        assert!(!target.join("release/runtime_stdout.log").exists());
        assert!(!target.join("release/codex_plugins_screen.png").exists());
        assert!(target.join("release/deps/libhuge.rlib").exists());
        assert!(target.join("release/build/script-output.bin").exists());
        assert!(target.join("release/flux_engine.exe").exists());

        let _ = fs::remove_dir_all(root);
    }
}
