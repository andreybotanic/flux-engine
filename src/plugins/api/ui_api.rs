use crate::plugins::ContentId;

/// Declarative UI node exposed by the plugin API.
///
/// # Variants
/// Public variants of `UiNode` are listed in the Rust declaration and documented by the generated SDK reference.
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
/// Public fields of `HudBlock` are part of the generated SDK reference.
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
/// Public fields of `PanelDescriptor` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelDescriptor {
    pub id: ContentId,
    pub title: String,
    pub root: UiNode,
}

/// Plugin-provided tool description.
///
/// # Fields
/// Public fields of `ToolDescriptor` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub id: ContentId,
    pub label: String,
    pub icon_path: String,
    pub silhouette_path: Option<String>,
}
