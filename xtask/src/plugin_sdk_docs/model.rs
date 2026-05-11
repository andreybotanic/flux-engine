use std::path::PathBuf;

use crate::plugin_sdk_docs::parser::DocBlock;

pub(in crate::plugin_sdk_docs) const DOCS_ROOT: &str = "docs/plugin_sdk";
pub(in crate::plugin_sdk_docs) const DOCS_SRC_ROOT: &str = "docs/plugin_sdk/src";
pub(in crate::plugin_sdk_docs) const GENERATED_DIR: &str = "generated";

pub(in crate::plugin_sdk_docs) const SDK_SOURCES: &[SdkSource] = &[
    SdkSource::new("ABI v4", "src/plugins/abi.rs", SdkCategory::Abi),
    SdkSource::new(
        "ABI v4 Events",
        "src/plugins/abi_events.rs",
        SdkCategory::Abi,
    ),
    SdkSource::new("Manifest", "src/plugins/manifest.rs", SdkCategory::Manifest),
    SdkSource::new("Identifiers", "src/plugins/id.rs", SdkCategory::Manifest),
    SdkSource::new("Content", "src/plugins/content.rs", SdkCategory::Content),
    SdkSource::new(
        "Substances",
        "src/plugins/substances.rs",
        SdkCategory::Content,
    ),
    SdkSource::new("Events", "src/plugins/api/events.rs", SdkCategory::Event),
    SdkSource::new(
        "Runtime Registry",
        "src/plugins/api/runtime.rs",
        SdkCategory::EngineSide,
    ),
    SdkSource::new(
        "Render API",
        "src/plugins/api/render_api.rs",
        SdkCategory::EngineSide,
    ),
    SdkSource::new(
        "UI API",
        "src/plugins/api/ui_api.rs",
        SdkCategory::EngineSide,
    ),
    SdkSource::new(
        "World API",
        "src/plugins/api/world_api.rs",
        SdkCategory::EngineSide,
    ),
    SdkSource::new(
        "Save API",
        "src/plugins/api/save_api.rs",
        SdkCategory::EngineSide,
    ),
];

#[derive(Clone, Copy, Debug)]
/// One Rust source file that contributes items to the generated SDK reference.
pub(in crate::plugin_sdk_docs) struct SdkSource {
    pub(in crate::plugin_sdk_docs) title: &'static str,
    pub(in crate::plugin_sdk_docs) path: &'static str,
    pub(in crate::plugin_sdk_docs) category: SdkCategory,
}

impl SdkSource {
    /// Creates a static SDK source descriptor.
    const fn new(title: &'static str, path: &'static str, category: SdkCategory) -> Self {
        Self {
            title,
            path,
            category,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// High-level badge category attached to generated SDK items.
pub(in crate::plugin_sdk_docs) enum SdkCategory {
    Abi,
    Manifest,
    Content,
    Event,
    EngineSide,
}

impl SdkCategory {
    /// Returns the CSS badge token used by generated pages.
    pub(in crate::plugin_sdk_docs) fn badge(self) -> &'static str {
        match self {
            Self::Abi => "ABI",
            Self::Manifest => "manifest",
            Self::Content => "content",
            Self::Event => "event",
            Self::EngineSide => "engine-side",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// Top-level generated API group shown in mdBook navigation.
pub(in crate::plugin_sdk_docs) enum ApiGroup {
    Structures,
    Enums,
    Constants,
    Methods,
    Events,
}

impl ApiGroup {
    /// Returns the stable generated API group order.
    pub(in crate::plugin_sdk_docs) fn all() -> [Self; 5] {
        [
            Self::Structures,
            Self::Enums,
            Self::Constants,
            Self::Methods,
            Self::Events,
        ]
    }

    /// Returns the human-readable group label.
    pub(in crate::plugin_sdk_docs) fn label(self) -> &'static str {
        match self {
            Self::Structures => "Structures",
            Self::Enums => "Enums",
            Self::Constants => "Constants",
            Self::Methods => "Methods",
            Self::Events => "Events",
        }
    }

    /// Returns the directory that stores concrete pages for this group.
    pub(in crate::plugin_sdk_docs) fn directory_name(self) -> &'static str {
        match self {
            Self::Structures => "structures",
            Self::Enums => "enums",
            Self::Constants => "constants",
            Self::Methods => "methods",
            Self::Events => "events",
        }
    }

    /// Returns the generated index file name for this group.
    pub(in crate::plugin_sdk_docs) fn index_file_name(self) -> &'static str {
        match self {
            Self::Structures => "structures.md",
            Self::Enums => "enums.md",
            Self::Constants => "constants.md",
            Self::Methods => "methods.md",
            Self::Events => "events.md",
        }
    }
}

#[derive(Clone, Debug)]
/// One generated Markdown file relative to `docs/plugin_sdk/src`.
pub(in crate::plugin_sdk_docs) struct GeneratedFile {
    pub(in crate::plugin_sdk_docs) relative_path: PathBuf,
    pub(in crate::plugin_sdk_docs) contents: String,
}

#[derive(Clone, Debug)]
/// Fully collected documentation model for one generated API page.
pub(in crate::plugin_sdk_docs) struct ApiItemDoc {
    pub(in crate::plugin_sdk_docs) name: String,
    pub(in crate::plugin_sdk_docs) file_name: String,
    pub(in crate::plugin_sdk_docs) group: ApiGroup,
    pub(in crate::plugin_sdk_docs) kind: SdkItemKind,
    pub(in crate::plugin_sdk_docs) category: SdkCategory,
    pub(in crate::plugin_sdk_docs) source_path: String,
    pub(in crate::plugin_sdk_docs) source_title: String,
    pub(in crate::plugin_sdk_docs) docs: DocBlock,
    pub(in crate::plugin_sdk_docs) signature: Option<String>,
    pub(in crate::plugin_sdk_docs) fields: Vec<ApiFieldDoc>,
    pub(in crate::plugin_sdk_docs) variants: Vec<ApiVariantDoc>,
    pub(in crate::plugin_sdk_docs) arguments: Vec<ApiArgumentDoc>,
    pub(in crate::plugin_sdk_docs) return_value: Option<String>,
}

impl ApiItemDoc {
    /// Returns the relative link from a generated group index to this item.
    pub(in crate::plugin_sdk_docs) fn relative_link(&self) -> String {
        format!("{}/{}", self.group.directory_name(), self.file_name)
    }
}

#[derive(Clone, Debug)]
/// Public struct field rendered in a generated structure page.
pub(in crate::plugin_sdk_docs) struct ApiFieldDoc {
    pub(in crate::plugin_sdk_docs) name: String,
    pub(in crate::plugin_sdk_docs) ty: String,
    pub(in crate::plugin_sdk_docs) description: String,
}

#[derive(Clone, Debug)]
/// Enum variant rendered in a generated structure page.
pub(in crate::plugin_sdk_docs) struct ApiVariantDoc {
    pub(in crate::plugin_sdk_docs) name: String,
    pub(in crate::plugin_sdk_docs) payload: String,
    pub(in crate::plugin_sdk_docs) description: String,
}

#[derive(Clone, Debug)]
/// Method, callback or event payload argument rendered in generated pages.
pub(in crate::plugin_sdk_docs) struct ApiArgumentDoc {
    pub(in crate::plugin_sdk_docs) name: String,
    pub(in crate::plugin_sdk_docs) ty: String,
    pub(in crate::plugin_sdk_docs) description: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Fine-grained item kind shown next to an API item.
pub(in crate::plugin_sdk_docs) enum SdkItemKind {
    Struct,
    Enum,
    TypeAlias,
    Const,
    Function,
    Method,
    Event,
}

impl SdkItemKind {
    /// Returns the short kind label shown in generated pages.
    pub(in crate::plugin_sdk_docs) fn label(self) -> &'static str {
        match self {
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::TypeAlias => "callback",
            Self::Const => "const",
            Self::Function => "fn",
            Self::Method => "method",
            Self::Event => "event",
        }
    }
}
