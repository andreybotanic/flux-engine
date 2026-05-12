use crate::plugins::ContentId;

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
