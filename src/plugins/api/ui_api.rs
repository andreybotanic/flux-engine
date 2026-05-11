use crate::plugins::ContentId;

/// Declarative UI node exposed by the plugin API.
///
/// # Variants
/// - `Text`: Displays static text content.
/// - `Button`: Displays a clickable button identified by a stable content id.
/// - `Checkbox`: Displays a labelled boolean toggle.
/// - `Select`: Displays a labelled option selector with a stable selected index.
/// - `Slider`: Displays a labelled floating-point slider with explicit bounds.
/// - `Column`: Lays out child nodes vertically.
/// - `Row`: Lays out child nodes horizontally.
#[derive(Clone, Debug, PartialEq)]
pub enum UiNode {
    Text {
        text: String,
    },
    Button {
        id: ContentId,
        label: String,
    },
    Checkbox {
        id: ContentId,
        label: String,
        checked: bool,
    },
    Select {
        id: ContentId,
        label: String,
        options: Vec<String>,
        selected: usize,
    },
    Slider {
        id: ContentId,
        label: String,
        value: f32,
        min: f32,
        max: f32,
    },
    Column {
        children: Vec<UiNode>,
    },
    Row {
        children: Vec<UiNode>,
    },
}

/// Plugin-provided HUD block for a hovered world cell.
///
/// # Fields
/// - `id`: Stable content id of the HUD block entry.
/// - `title`: Block title shown in the HUD.
/// - `lines`: Text lines rendered inside the HUD block.
/// - `sort_order`: Ordering key used when multiple HUD blocks are combined.
#[derive(Clone, Debug, PartialEq)]
pub struct HudBlock {
    pub id: ContentId,
    pub title: String,
    pub lines: Vec<String>,
    pub sort_order: i32,
}

/// Plugin-provided panel description.
///
/// # Fields
/// - `id`: Stable content id of the panel.
/// - `title`: Human-readable panel title shown by the UI.
/// - `root`: Root declarative UI node used to build the panel contents.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelDescriptor {
    pub id: ContentId,
    pub title: String,
    pub root: UiNode,
}

/// Plugin-provided tool description.
///
/// # Fields
/// - `id`: Stable content id of the tool.
/// - `label`: Human-readable tool label shown in selectors and toolbars.
/// - `icon_path`: Relative asset path to the main tool icon.
/// - `silhouette_path`: Optional relative asset path to the tool silhouette icon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub id: ContentId,
    pub label: String,
    pub icon_path: String,
    pub silhouette_path: Option<String>,
}
