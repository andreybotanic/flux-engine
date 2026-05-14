use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use crate::XtaskError;

const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
const PNG_COLOR_TYPE_OFFSET: usize = 25;
const DEFAULT_KTX_BIN_FALLBACK: &str = r"C:\Program Files\KTX-Software\bin\ktx.exe";
const SKIPPED_CORE_PNG_RELATIVE: [&str; 2] = [
    "assets/sprites/ui/main_menu_background.png",
    "assets/sprites/world/backdrop_noise.png",
];

const SPRITE_SOURCE_DIRS: [&str; 2] = ["assets/sprites", "src/plugins/default_plugin/assets"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PngColorType {
    Rgb,
    Rgba,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpritePngAsset {
    png_path: PathBuf,
    ktx2_path: PathBuf,
    color_type: PngColorType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpriteKtxIssue {
    png_path: PathBuf,
    ktx2_path: PathBuf,
    kind: SpriteKtxIssueKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpriteKtxIssueKind {
    Missing,
    Outdated,
}

/// Summary of one `generate-sprite-ktx` execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpriteKtxGenerateReport {
    scanned_png_count: usize,
    generated_ktx2_count: usize,
}

impl SpriteKtxGenerateReport {
    /// Returns how many PNG sources were scanned.
    pub(crate) fn scanned_png_count(&self) -> usize {
        self.scanned_png_count
    }

    /// Returns how many `.ktx2` files were regenerated in this run.
    pub(crate) fn generated_ktx2_count(&self) -> usize {
        self.generated_ktx2_count
    }
}

/// Summary of one `check-sprite-ktx` execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpriteKtxCheckReport {
    scanned_png_count: usize,
}

impl SpriteKtxCheckReport {
    /// Returns how many PNG sources were scanned.
    pub(crate) fn scanned_png_count(&self) -> usize {
        self.scanned_png_count
    }
}

/// Generates `.ktx2` textures with mipmaps for built-in sprite PNG assets.
pub(crate) fn generate_sprite_ktx(repo_root: &Path) -> Result<SpriteKtxGenerateReport, XtaskError> {
    let assets = collect_sprite_png_assets(repo_root)?;
    let ktx_bin = resolve_ktx_binary()?;
    let mut generated = 0usize;

    for asset in &assets {
        if !needs_regeneration(asset)? {
            continue;
        }
        run_ktx_create(&ktx_bin, asset, repo_root)?;
        generated += 1;
    }

    Ok(SpriteKtxGenerateReport {
        scanned_png_count: assets.len(),
        generated_ktx2_count: generated,
    })
}

/// Checks that built-in sprite `.ktx2` files exist and are up to date against PNG sources.
pub(crate) fn check_sprite_ktx(repo_root: &Path) -> Result<SpriteKtxCheckReport, XtaskError> {
    let assets = collect_sprite_png_assets(repo_root)?;
    let mut issues = Vec::new();

    for asset in &assets {
        let Some(kind) = check_issue_kind(asset)? else {
            continue;
        };
        issues.push(SpriteKtxIssue {
            png_path: asset.png_path.clone(),
            ktx2_path: asset.ktx2_path.clone(),
            kind,
        });
    }

    if !issues.is_empty() {
        return Err(XtaskError::new(render_check_failure(&issues)));
    }

    Ok(SpriteKtxCheckReport {
        scanned_png_count: assets.len(),
    })
}

fn collect_sprite_png_assets(repo_root: &Path) -> Result<Vec<SpritePngAsset>, XtaskError> {
    let mut collected = Vec::new();
    for relative_dir in SPRITE_SOURCE_DIRS {
        let root = repo_root.join(relative_dir);
        collect_sprite_png_assets_from(repo_root, root.as_path(), &mut collected)?;
    }
    collected.sort_by(|left, right| left.png_path.cmp(&right.png_path));
    Ok(collected)
}

fn collect_sprite_png_assets_from(
    repo_root: &Path,
    root: &Path,
    collected: &mut Vec<SpritePngAsset>,
) -> Result<(), XtaskError> {
    if !root.exists() {
        return Ok(());
    }
    if !root.is_dir() {
        return Err(XtaskError::new(format!(
            "sprite source root '{}' is not a directory",
            root.display()
        )));
    }

    for entry in fs::read_dir(root).map_err(|error| {
        XtaskError::new(format!(
            "failed to read sprite source directory '{}': {}",
            root.display(),
            error
        ))
    })? {
        let entry = entry.map_err(|error| {
            XtaskError::new(format!(
                "failed to read entry in '{}': {}",
                root.display(),
                error
            ))
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_sprite_png_assets_from(repo_root, path.as_path(), collected)?;
            continue;
        }
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("png"))
            != Some(true)
        {
            continue;
        }
        if is_skipped_core_png(repo_root, path.as_path()) {
            continue;
        }
        let color_type = read_png_color_type(path.as_path())?;
        collected.push(SpritePngAsset {
            ktx2_path: path.with_extension("ktx2"),
            png_path: path,
            color_type,
        });
    }
    Ok(())
}

fn is_skipped_core_png(repo_root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(repo_root) else {
        return false;
    };
    SKIPPED_CORE_PNG_RELATIVE
        .iter()
        .any(|skip| relative.components().eq(Path::new(skip).components()))
}

fn read_png_color_type(path: &Path) -> Result<PngColorType, XtaskError> {
    let bytes = fs::read(path).map_err(|error| {
        XtaskError::new(format!(
            "failed to read PNG source '{}': {}",
            path.display(),
            error
        ))
    })?;
    if bytes.len() <= PNG_COLOR_TYPE_OFFSET {
        return Err(XtaskError::new(format!(
            "PNG source '{}' is too small or invalid",
            path.display()
        )));
    }
    if bytes[..8] != PNG_SIGNATURE {
        return Err(XtaskError::new(format!(
            "sprite source '{}' is not a PNG file",
            path.display()
        )));
    }
    if &bytes[12..16] != b"IHDR" {
        return Err(XtaskError::new(format!(
            "PNG source '{}' is missing IHDR chunk",
            path.display()
        )));
    }
    match bytes[PNG_COLOR_TYPE_OFFSET] {
        2 => Ok(PngColorType::Rgb),
        6 => Ok(PngColorType::Rgba),
        other => Err(XtaskError::new(format!(
            "unsupported PNG color type {} in '{}'; supported color types: RGB(2), RGBA(6)",
            other,
            path.display()
        ))),
    }
}

fn needs_regeneration(asset: &SpritePngAsset) -> Result<bool, XtaskError> {
    if !asset.ktx2_path.exists() {
        return Ok(true);
    }
    let png_modified = file_modified(asset.png_path.as_path())?;
    let ktx_modified = file_modified(asset.ktx2_path.as_path())?;
    Ok(ktx_modified < png_modified)
}

fn check_issue_kind(asset: &SpritePngAsset) -> Result<Option<SpriteKtxIssueKind>, XtaskError> {
    if !asset.ktx2_path.exists() {
        return Ok(Some(SpriteKtxIssueKind::Missing));
    }
    let png_modified = file_modified(asset.png_path.as_path())?;
    let ktx_modified = file_modified(asset.ktx2_path.as_path())?;
    if ktx_modified < png_modified {
        return Ok(Some(SpriteKtxIssueKind::Outdated));
    }
    Ok(None)
}

fn file_modified(path: &Path) -> Result<SystemTime, XtaskError> {
    fs::metadata(path)
        .map_err(|error| {
            XtaskError::new(format!("failed to stat '{}': {}", path.display(), error))
        })?
        .modified()
        .map_err(|error| {
            XtaskError::new(format!(
                "failed to read modification time for '{}': {}",
                path.display(),
                error
            ))
        })
}

fn run_ktx_create(
    ktx_bin: &Path,
    asset: &SpritePngAsset,
    repo_root: &Path,
) -> Result<(), XtaskError> {
    if asset.ktx2_path.exists() {
        fs::remove_file(asset.ktx2_path.as_path()).map_err(|error| {
            XtaskError::new(format!(
                "failed to remove stale KTX2 '{}': {}",
                asset.ktx2_path.display(),
                error
            ))
        })?;
    }

    let mut command = Command::new(ktx_bin);
    command
        .arg("create")
        .arg("--format")
        .arg("R8G8B8A8_SRGB")
        .arg("--generate-mipmap");
    if asset.color_type == PngColorType::Rgb {
        command.arg("--input-swizzle").arg("rgb1");
    }
    command
        .arg(asset.png_path.as_os_str())
        .arg(asset.ktx2_path.as_os_str())
        .current_dir(repo_root);

    let status = command.status().map_err(|error| {
        XtaskError::new(format!(
            "failed to launch '{}' while converting '{}' -> '{}': {}",
            ktx_bin.display(),
            asset.png_path.display(),
            asset.ktx2_path.display(),
            error
        ))
    })?;
    if status.success() {
        return Ok(());
    }

    Err(XtaskError::new(format!(
        "ktx create failed for '{}' -> '{}' with status {}",
        asset.png_path.display(),
        asset.ktx2_path.display(),
        status
    )))
}

fn render_check_failure(issues: &[SpriteKtxIssue]) -> String {
    let mut lines = vec![
        "sprite KTX check failed; run `cargo generate-sprite-ktx` to refresh generated files:"
            .to_string(),
    ];
    for issue in issues {
        let kind = match issue.kind {
            SpriteKtxIssueKind::Missing => "missing",
            SpriteKtxIssueKind::Outdated => "outdated",
        };
        lines.push(format!(
            "- {}: '{}' (source '{}')",
            kind,
            issue.ktx2_path.display(),
            issue.png_path.display()
        ));
    }
    lines.join("\n")
}

fn resolve_ktx_binary() -> Result<PathBuf, XtaskError> {
    let fallback = PathBuf::from(DEFAULT_KTX_BIN_FALLBACK);
    resolve_ktx_binary_with_path(std::env::var_os("PATH").as_deref(), fallback.as_path())
}

fn resolve_ktx_binary_with_path(
    path_value: Option<&OsStr>,
    fallback: &Path,
) -> Result<PathBuf, XtaskError> {
    if let Some(path) = find_executable_in_path("ktx.exe", path_value)
        .or_else(|| find_executable_in_path("ktx", path_value))
    {
        return Ok(path);
    }
    if fallback.exists() {
        return Ok(fallback.to_path_buf());
    }
    Err(XtaskError::new(format!(
        "ktx CLI not found. Add it to PATH or install KTX-Software and use fallback path '{}'.",
        fallback.display()
    )))
}

fn find_executable_in_path(name: &str, path_value: Option<&OsStr>) -> Option<PathBuf> {
    let path_dirs = std::env::split_paths(path_value?);
    for directory in path_dirs {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::time::Duration;

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
        fs::create_dir_all(root.join("assets/sprites")).expect("create assets sprites");
        fs::create_dir_all(root.join("src/plugins/default_plugin/assets"))
            .expect("create plugin assets");
        root
    }

    fn write_fake_png(path: &Path, color_type: u8) {
        let mut bytes = vec![0_u8; 33];
        bytes[..8].copy_from_slice(&PNG_SIGNATURE);
        bytes[8..12].copy_from_slice(&13_u32.to_be_bytes());
        bytes[12..16].copy_from_slice(b"IHDR");
        bytes[PNG_COLOR_TYPE_OFFSET] = color_type;
        fs::create_dir_all(path.parent().expect("png parent")).expect("create png parent");
        fs::write(path, bytes).expect("write fake png");
    }

    #[test]
    fn collect_scans_only_target_sprite_roots() {
        let root = temp_repo("flux_xtask_sprite_ktx_collect");
        let in_scope = root.join("assets/sprites/ui/icon.png");
        let skipped_main_menu = root.join("assets/sprites/ui/main_menu_background.png");
        let skipped_backdrop = root.join("assets/sprites/world/backdrop_noise.png");
        let in_scope_plugin = root.join("src/plugins/default_plugin/assets/world/tile.png");
        let out_of_scope = root.join("assets/other/ignored.png");
        write_fake_png(in_scope.as_path(), 6);
        write_fake_png(skipped_main_menu.as_path(), 6);
        write_fake_png(skipped_backdrop.as_path(), 6);
        write_fake_png(in_scope_plugin.as_path(), 2);
        write_fake_png(out_of_scope.as_path(), 6);

        let collected = collect_sprite_png_assets(root.as_path()).expect("collect sprite pngs");
        let png_paths = collected
            .iter()
            .map(|item| item.png_path.clone())
            .collect::<Vec<_>>();
        assert_eq!(png_paths.len(), 2);
        assert!(png_paths.contains(&in_scope));
        assert!(png_paths.contains(&in_scope_plugin));
        assert!(!png_paths.contains(&skipped_main_menu));
        assert!(!png_paths.contains(&skipped_backdrop));
        assert!(!png_paths.contains(&out_of_scope));
        let ktx_paths = collected
            .iter()
            .map(|item| item.ktx2_path.clone())
            .collect::<Vec<_>>();
        assert!(ktx_paths.contains(&in_scope.with_extension("ktx2")));
        assert!(ktx_paths.contains(&in_scope_plugin.with_extension("ktx2")));
        assert!(!ktx_paths.contains(&skipped_main_menu.with_extension("ktx2")));
        assert!(!ktx_paths.contains(&skipped_backdrop.with_extension("ktx2")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn color_type_parser_supports_rgb_and_rgba() {
        let root = temp_repo("flux_xtask_sprite_ktx_color");
        let rgb = root.join("assets/sprites/rgb.png");
        let rgba = root.join("assets/sprites/rgba.png");
        write_fake_png(rgb.as_path(), 2);
        write_fake_png(rgba.as_path(), 6);

        assert_eq!(
            read_png_color_type(rgb.as_path()).expect("rgb parse"),
            PngColorType::Rgb
        );
        assert_eq!(
            read_png_color_type(rgba.as_path()).expect("rgba parse"),
            PngColorType::Rgba
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn color_type_parser_rejects_unsupported_type() {
        let root = temp_repo("flux_xtask_sprite_ktx_bad_color");
        let bad = root.join("assets/sprites/bad.png");
        write_fake_png(bad.as_path(), 0);
        let error = read_png_color_type(bad.as_path()).expect_err("must fail on grayscale");
        assert!(error.to_string().contains("unsupported PNG color type"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn check_reports_missing_or_outdated_outputs() {
        let root = temp_repo("flux_xtask_sprite_ktx_check");
        let png = root.join("assets/sprites/ui/icon.png");
        let ktx = png.with_extension("ktx2");
        write_fake_png(png.as_path(), 6);

        let missing = check_sprite_ktx(root.as_path()).expect_err("missing ktx should fail");
        assert!(missing.to_string().contains("missing"));

        fs::write(ktx.as_path(), "old").expect("write old ktx");
        std::thread::sleep(Duration::from_millis(1200));
        let mut bytes = fs::read(png.as_path()).expect("read png");
        bytes[31] = 1;
        fs::write(png.as_path(), bytes).expect("touch png");
        let outdated = check_sprite_ktx(root.as_path()).expect_err("outdated ktx should fail");
        assert!(outdated.to_string().contains("outdated"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ktx_binary_resolution_prefers_path_executable() {
        let root = temp_repo("flux_xtask_sprite_ktx_bin");
        let fake_bin_dir = root.join("fake_bin");
        fs::create_dir_all(&fake_bin_dir).expect("create fake bin");
        let fake_ktx = fake_bin_dir.join("ktx.exe");
        fs::write(&fake_ktx, "not an executable").expect("write fake ktx");

        let resolved = resolve_ktx_binary_with_path(
            Some(OsString::from(fake_bin_dir.as_os_str()).as_os_str()),
            root.join("missing/ktx.exe").as_path(),
        )
        .expect("resolve ktx");
        assert_eq!(resolved, fake_ktx);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ktx_binary_resolution_uses_fallback_when_path_missing() {
        let root = temp_repo("flux_xtask_sprite_ktx_fallback");
        let fallback = root.join("tools/ktx.exe");
        fs::create_dir_all(fallback.parent().expect("fallback parent"))
            .expect("create fallback parent");
        fs::write(&fallback, "fake fallback").expect("write fallback ktx");

        let resolved =
            resolve_ktx_binary_with_path(None, fallback.as_path()).expect("resolve from fallback");
        assert_eq!(resolved, fallback);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ktx_binary_resolution_returns_clear_error_when_unavailable() {
        let root = temp_repo("flux_xtask_sprite_ktx_missing");
        let missing_fallback = root.join("missing/ktx.exe");
        let error = resolve_ktx_binary_with_path(None, missing_fallback.as_path())
            .expect_err("missing ktx should fail");
        assert!(error.to_string().contains("ktx CLI not found"));
        assert!(error
            .to_string()
            .contains(&missing_fallback.display().to_string()));

        let _ = fs::remove_dir_all(root);
    }
}
