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
            render_item_page(&mut contents, &item);
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
    use std::path::{Path, PathBuf};

    use super::{check_plugin_sdk_docs, generated_plugin_sdk_files};

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has repo parent")
            .to_path_buf()
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
        assert!(summary
            .contents
            .contains("[Constants](generated/constants.md)"));
        assert!(summary
            .contents
            .contains("[`FluxRuntimeHost`](generated/structures/fluxruntimehost.md)"));
        assert!(summary
            .contents
            .contains("[`PluginEventKind`](generated/enums/plugineventkind.md)"));
        assert!(summary.contents.contains(
            "[`FLUX_PLUGIN_CREATE_EXPORT_NAME`](generated/constants/flux-plugin-create-export-name.md)"
        ));
        assert!(summary
            .contents
            .contains("[`FluxRuntimeHost::new`](generated/methods/fluxruntimehost-new.md)"));
        assert!(summary.contents.contains(
            "[`PluginEventKind::WorldCreated`](generated/events/plugineventkind-worldcreated.md)"
        ));
    }

    #[test]
    fn generated_structure_pages_include_field_table() {
        let files = generated_plugin_sdk_files(&repo_root()).expect("generate SDK docs in memory");
        let page = files
            .iter()
            .find(|file| file.relative_path == Path::new("generated/structures/fluxruntimehost.md"))
            .expect("FluxRuntimeHost page");

        assert!(page.contents.contains("## Fields"));
        assert!(page.contents.contains("set_cell_material"));
        assert!(page.contents.contains("Option < FluxSetCellMaterialFn >"));
    }

    #[test]
    fn generated_method_pages_include_arguments_and_return_value() {
        let files = generated_plugin_sdk_files(&repo_root()).expect("generate SDK docs in memory");
        let page = files
            .iter()
            .find(|file| {
                file.relative_path == Path::new("generated/methods/worldapi-get-cell-info.md")
            })
            .expect("WorldApi::get_cell_info page");

        assert!(page.contents.contains("## Arguments"));
        assert!(page.contents.contains("cell"));
        assert!(page.contents.contains("## Return Value"));
        assert!(page
            .contents
            .contains("Result < CellInfo , WorldApiError >"));
    }

    #[test]
    fn tracked_sdk_docs_are_current() {
        check_plugin_sdk_docs(&repo_root()).expect("tracked generated SDK docs must be current");
    }
}
