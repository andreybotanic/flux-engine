use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use collector::collect_api_items;
use mdbook_driver::MDBook;
use model::{ApiGroup, ApiItemDoc, GeneratedFile, DOCS_ROOT, DOCS_SRC_ROOT, GENERATED_DIR};
use render::{render_index, render_item_page, render_summary};

use crate::XtaskError;

mod collector;
mod model;
mod parser;
mod render;

/// Rebuilds generated Plugin SDK Markdown files in-place.
pub fn generate_plugin_sdk_docs(repo_root: &Path) -> Result<(), XtaskError> {
    let files = generated_plugin_sdk_files(repo_root)?;
    let docs_src_root = repo_root.join(DOCS_SRC_ROOT);
    let generated_root = docs_src_root.join(GENERATED_DIR);
    if generated_root.exists() {
        fs::remove_dir_all(&generated_root)?;
    }
    fs::create_dir_all(&generated_root)?;
    for file in files {
        let path = docs_src_root.join(file.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, file.contents).map_err(|error| {
            XtaskError::new(format!(
                "failed to write generated SDK docs '{}': {}",
                path.display(),
                error
            ))
        })?;
    }
    Ok(())
}

/// Checks that generated Plugin SDK Markdown files are up to date.
pub fn check_plugin_sdk_docs(repo_root: &Path) -> Result<(), XtaskError> {
    let files = generated_plugin_sdk_files(repo_root)?;
    let docs_src_root = repo_root.join(DOCS_SRC_ROOT);
    let mut stale = Vec::new();
    for file in files {
        let path = docs_src_root.join(&file.relative_path);
        let current = fs::read_to_string(&path).unwrap_or_default();
        if current != file.contents {
            stale.push(path);
        }
    }
    if stale.is_empty() {
        return Ok(());
    }
    Err(XtaskError::new(format!(
        "Plugin SDK docs are stale; run `cargo xtask generate-plugin-sdk-docs`: {}",
        stale
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

/// Generates Plugin SDK docs and builds the mdBook site.
pub fn build_plugin_sdk_docs(repo_root: &Path) -> Result<PathBuf, XtaskError> {
    generate_plugin_sdk_docs(repo_root)?;
    let docs_root = repo_root.join(DOCS_ROOT);
    let book = MDBook::load(&docs_root).map_err(|error| {
        XtaskError::new(format!(
            "failed to load Plugin SDK mdBook '{}': {}",
            docs_root.display(),
            error
        ))
    })?;
    book.build().map_err(|error| {
        XtaskError::new(format!(
            "failed to build Plugin SDK mdBook '{}': {}",
            docs_root.display(),
            error
        ))
    })?;
    Ok(repo_root.join("target").join("plugin_sdk_docs"))
}

fn generated_plugin_sdk_files(repo_root: &Path) -> Result<Vec<GeneratedFile>, XtaskError> {
    let mut items = collect_api_items(repo_root)?;
    items.sort_by(|left, right| (left.group, &left.name).cmp(&(right.group, &right.name)));
    let grouped = grouped_items(&items);
    let mut files = Vec::new();

    let mut summary = String::new();
    render_summary(&mut summary, &summary_groups(&grouped));
    files.push(GeneratedFile {
        relative_path: PathBuf::from("SUMMARY.md"),
        contents: summary,
    });

    for group in ApiGroup::all() {
        let group_items = grouped.get(&group).cloned().unwrap_or_default();
        let mut index = String::new();
        render_index(&mut index, group.label(), &group_items);
        files.push(GeneratedFile {
            relative_path: PathBuf::from(GENERATED_DIR).join(group.index_file_name()),
            contents: index,
        });
        for item in group_items {
            let mut contents = String::new();
            render_item_page(&mut contents, &item, &items);
            files.push(GeneratedFile {
                relative_path: PathBuf::from(GENERATED_DIR)
                    .join(group.directory_name())
                    .join(&item.file_name),
                contents,
            });
        }
    }

    Ok(files)
}

fn grouped_items(items: &[ApiItemDoc]) -> BTreeMap<ApiGroup, Vec<ApiItemDoc>> {
    let mut grouped = BTreeMap::<ApiGroup, Vec<ApiItemDoc>>::new();
    for item in items {
        grouped.entry(item.group).or_default().push(item.clone());
    }
    grouped
}

fn summary_groups<'a>(
    grouped: &'a BTreeMap<ApiGroup, Vec<ApiItemDoc>>,
) -> Vec<(ApiGroup, Vec<&'a ApiItemDoc>)> {
    ApiGroup::all()
        .into_iter()
        .map(|group| {
            (
                group,
                grouped
                    .get(&group)
                    .map(|items| items.iter().collect())
                    .unwrap_or_default(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{check_plugin_sdk_docs, generated_plugin_sdk_files};

    struct TempRepo {
        root: PathBuf,
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has repo parent")
            .to_path_buf()
    }

    fn copy_tree(source: &Path, destination: &Path) {
        let metadata = fs::metadata(source).expect("source metadata");
        if metadata.is_dir() {
            fs::create_dir_all(destination).expect("create destination directory");
            for entry in fs::read_dir(source).expect("read directory") {
                let entry = entry.expect("directory entry");
                copy_tree(&entry.path(), &destination.join(entry.file_name()));
            }
            return;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).expect("create destination parent");
        }
        fs::copy(source, destination).expect("copy file");
    }

    fn temp_repo(label: &str) -> TempRepo {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time since epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "flux_plugin_sdk_docs_{}_{}_{}",
            label,
            std::process::id(),
            suffix
        ));
        fs::create_dir_all(&root).expect("create temp repo root");
        let repo = repo_root();
        copy_tree(&repo.join("src"), &root.join("src"));
        copy_tree(
            &repo.join("crates").join("flux_plugin_sdk"),
            &root.join("crates").join("flux_plugin_sdk"),
        );
        copy_tree(
            &repo
                .join("docs")
                .join("plugin_sdk")
                .join("src")
                .join("examples"),
            &root
                .join("docs")
                .join("plugin_sdk")
                .join("src")
                .join("examples"),
        );
        TempRepo { root }
    }

    #[test]
    fn generated_sdk_docs_group_api_by_kind() {
        let files = generated_plugin_sdk_files(&repo_root()).expect("generate SDK docs in memory");
        let summary = files
            .iter()
            .find(|file| file.relative_path == Path::new("SUMMARY.md"))
            .expect("summary generated file");

        assert!(summary.contents.contains("# Generated API"));
        assert!(summary
            .contents
            .contains("[Structures](generated/structures.md)"));
        assert!(summary.contents.contains("[Enums](generated/enums.md)"));
        assert!(summary.contents.contains("[Methods](generated/methods.md)"));
        assert!(summary.contents.contains("[Events](generated/events.md)"));
        assert!(summary
            .contents
            .contains("[`WorldApi`](generated/structures/worldapi.md)"));
        assert!(summary
            .contents
            .contains("[`Registrar`](generated/structures/registrar.md)"));
        assert!(summary
            .contents
            .contains("[`PluginEvent`](generated/enums/pluginevent.md)"));
        assert!(summary
            .contents
            .contains("[`WorldApi::width`](generated/methods/worldapi-width.md)"));
        assert!(summary.contents.contains(
            "[`PluginEvent::WorldCreated`](generated/events/pluginevent-worldcreated.md)"
        ));
        assert!(!summary.contents.contains("FluxRuntimeHost"));
        assert!(!summary.contents.contains("PluginRuntime"));
    }

    #[test]
    fn generated_structure_pages_hide_internal_runtime_details() {
        let files = generated_plugin_sdk_files(&repo_root()).expect("generate SDK docs in memory");
        let page = files
            .iter()
            .find(|file| file.relative_path == Path::new("generated/structures/worldapi.md"))
            .expect("WorldApi page");

        assert!(!page.contents.contains("## Fields"));
        assert!(page.contents.contains("## Methods"));
        assert!(page
            .contents
            .contains("[`WorldApi::width`](../methods/worldapi-width.md)"));
        assert!(!page.contents.contains("FluxRuntimeHost"));
    }

    #[test]
    fn generated_structure_pages_include_method_links() {
        let files = generated_plugin_sdk_files(&repo_root()).expect("generate SDK docs in memory");
        let page = files
            .iter()
            .find(|file| file.relative_path == Path::new("generated/structures/registrar.md"))
            .expect("Registrar page");

        assert!(page.contents.contains("## Methods"));
        assert!(page
            .contents
            .contains("[`Registrar::subscribe`](../methods/registrar-subscribe.md)"));
    }

    #[test]
    fn allowlisted_sources_only_generate_selected_types_and_methods() {
        let files = generated_plugin_sdk_files(&repo_root()).expect("generate SDK docs in memory");

        assert!(files
            .iter()
            .any(|file| file.relative_path == Path::new("generated/structures/worldapi.md")));
        assert!(files
            .iter()
            .any(|file| file.relative_path == Path::new("generated/methods/worldapi-width.md")));
        assert!(!files
            .iter()
            .any(|file| file.relative_path == Path::new("generated/structures/pluginruntime.md")));
        assert!(!files
            .iter()
            .any(|file| file.relative_path == Path::new("generated/structures/overlaystate.md")));
    }

    #[test]
    fn generated_method_pages_include_arguments_and_return_value() {
        let files = generated_plugin_sdk_files(&repo_root()).expect("generate SDK docs in memory");
        let page = files
            .iter()
            .find(|file| file.relative_path == Path::new("generated/methods/entityapi-place.md"))
            .expect("EntityApi::place page");

        assert!(page.contents.contains("## Arguments"));
        assert!(page.contents.contains("kind"));
        assert!(page.contents.contains("placement"));
        assert!(page.contents.contains("## Return Value"));
        assert!(page.contents.contains("EntityInstanceId"));
        assert!(page.contents.contains("PluginError"));
    }

    #[test]
    fn generated_event_pages_use_typed_payload_fields() {
        let files = generated_plugin_sdk_files(&repo_root()).expect("generate SDK docs in memory");
        let page = files
            .iter()
            .find(|file| {
                file.relative_path == Path::new("generated/events/pluginevent-mousedowncell.md")
            })
            .expect("MouseDownCell event page");

        assert!(page.contents.contains("| `button` |"));
        assert!(page.contents.contains("| `cell` |"));
        assert!(page.contents.contains("| `world_position` |"));
        assert!(!page.contents.contains("| `payload` |"));
    }

    #[test]
    fn internal_runtime_aliases_are_hidden_from_generated_docs() {
        let files = generated_plugin_sdk_files(&repo_root()).expect("generate SDK docs in memory");
        let methods_index = files
            .iter()
            .find(|file| file.relative_path == Path::new("generated/methods.md"))
            .expect("methods index page");

        assert!(!methods_index.contents.contains("FluxRuntimeHost"));
        assert!(!methods_index.contents.contains("PluginRuntime"));
        assert!(!files
            .iter()
            .any(|file| file.relative_path.to_string_lossy().contains("pluginruntime")));
        assert!(!files
            .iter()
            .any(|file| file.relative_path.to_string_lossy().contains("fluxruntimehost")));
    }

    #[test]
    fn missing_example_file_is_ignored() {
        let repo = temp_repo("missing_example");
        let missing = repo
            .root
            .join("docs")
            .join("plugin_sdk")
            .join("src")
            .join("examples")
            .join("events")
            .join("pluginevent-worldcreated.md");
        fs::remove_file(&missing).expect("remove example file");

        let files =
            generated_plugin_sdk_files(&repo.root).expect("missing examples should be optional");
        let page = files
            .iter()
            .find(|file| {
                file.relative_path == Path::new("generated/events/pluginevent-worldcreated.md")
            })
            .expect("WorldCreated event page");

        assert!(!page.contents.contains("## SDK Example"));
    }

    #[test]
    fn missing_field_description_uses_default_text() {
        let files = generated_plugin_sdk_files(&repo_root()).expect("generate SDK docs in memory");
        let page = files
            .iter()
            .find(|file| file.relative_path == Path::new("generated/structures/cellrect.md"))
            .expect("CellRect page");

        assert!(page.contents.contains("`min` field stored as"));
        assert!(page.contents.contains("CellRect"));
    }

    #[test]
    fn missing_variant_description_uses_default_text() {
        let files = generated_plugin_sdk_files(&repo_root()).expect("generate SDK docs in memory");
        let page = files
            .iter()
            .find(|file| file.relative_path == Path::new("generated/enums/placementcheck.md"))
            .expect("PlacementCheck page");

        assert!(page
            .contents
            .contains("`Allowed` variant of `PlacementCheck`."));
    }

    #[test]
    fn tracked_sdk_docs_are_current() {
        check_plugin_sdk_docs(&repo_root()).expect("tracked generated SDK docs must be current");
    }
}
