use bevy::prelude::*;

use crate::{
    plugins::ContentId,
    world::structures::{PlacedStructureId, StructureKind},
};

/// Defines who controls rendering for one plugin overlay.
///
/// # Variants
/// Public variants of `OverlayRenderPolicy` are listed in the Rust declaration and documented by the generated SDK reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayRenderPolicy {
    CoreDefault,
    PluginControlled,
}

/// Display style for one world cell in a plugin-controlled overlay.
///
/// # Variants
/// Public variants of `CellRenderStyle` are listed in the Rust declaration and documented by the generated SDK reference.
#[derive(Clone, Debug, PartialEq)]
pub enum CellRenderStyle {
    Hidden,
    Normal,
    Filled { color: Color, alpha: f32 },
    Outline { color: Color, alpha: f32 },
    Sprite { asset_path: String, alpha: f32 },
}

/// Display style for one placed structure in a plugin-controlled overlay.
///
/// # Variants
/// Public variants of `StructureRenderStyle` are listed in the Rust declaration and documented by the generated SDK reference.
#[derive(Clone, Debug, PartialEq)]
pub enum StructureRenderStyle {
    Hidden,
    Normal,
    OutlineOnly { color: Color, alpha: f32 },
    Filled { color: Color, alpha: f32 },
    SpriteOverride { asset_path: String, alpha: f32 },
}

/// Display style for cell gas in a plugin-controlled overlay.
///
/// # Variants
/// Public variants of `GasRenderStyle` are listed in the Rust declaration and documented by the generated SDK reference.
#[derive(Clone, Debug, PartialEq)]
pub enum GasRenderStyle {
    Hidden,
    Normal,
    Custom { color: Color, intensity: f32 },
}

/// Extra draw command emitted by a plugin overlay.
///
/// # Variants
/// Public variants of `OverlayDrawCommand` are listed in the Rust declaration and documented by the generated SDK reference.
#[derive(Clone, Debug, PartialEq)]
pub enum OverlayDrawCommand {
    Rect {
        cell: UVec2,
        color: Color,
        z: f32,
    },
    Outline {
        cell: UVec2,
        color: Color,
        z: f32,
    },
    Sprite {
        cell: UVec2,
        asset_path: String,
        z: f32,
    },
    Line {
        from: Vec2,
        to: Vec2,
        color: Color,
        z: f32,
    },
    Text {
        cell: UVec2,
        text: String,
        color: Color,
        z: f32,
    },
}

/// Per-cell style entry for an overlay frame.
///
/// # Fields
/// Public fields of `CellStyleEntry` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq)]
pub struct CellStyleEntry {
    pub cell: UVec2,
    pub style: CellRenderStyle,
}

/// Per-structure style entry for an overlay frame.
///
/// # Fields
/// Public fields of `StructureStyleEntry` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq)]
pub struct StructureStyleEntry {
    pub structure_id: PlacedStructureId,
    pub kind: StructureKind,
    pub style: StructureRenderStyle,
    pub z_order: f32,
}

/// Per-cell gas style entry for an overlay frame.
///
/// # Fields
/// Public fields of `GasStyleEntry` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq)]
pub struct GasStyleEntry {
    pub cell: UVec2,
    pub style: GasRenderStyle,
}

/// Render frame returned by a plugin for one overlay refresh.
///
/// # Fields
/// Public fields of `OverlayFrame` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq)]
pub struct OverlayFrame {
    pub overlay_id: ContentId,
    pub policy: OverlayRenderPolicy,
    pub background: Option<Color>,
    pub world_alpha: f32,
    pub show_core_gas: bool,
    pub cell_styles: Vec<CellStyleEntry>,
    pub structure_styles: Vec<StructureStyleEntry>,
    pub gas_styles: Vec<GasStyleEntry>,
    pub draw_commands: Vec<OverlayDrawCommand>,
}

impl OverlayFrame {
    /// Builds an empty frame that keeps normal core rendering.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `core_default` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn core_default(overlay_id: ContentId) -> Self {
        Self {
            overlay_id,
            policy: OverlayRenderPolicy::CoreDefault,
            background: None,
            world_alpha: 1.0,
            show_core_gas: true,
            cell_styles: Vec::new(),
            structure_styles: Vec::new(),
            gas_styles: Vec::new(),
            draw_commands: Vec::new(),
        }
    }

    /// Builds an empty plugin-controlled overlay frame.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `plugin_controlled` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn plugin_controlled(overlay_id: ContentId) -> Self {
        Self {
            overlay_id,
            policy: OverlayRenderPolicy::PluginControlled,
            background: None,
            world_alpha: 1.0,
            show_core_gas: false,
            cell_styles: Vec::new(),
            structure_styles: Vec::new(),
            gas_styles: Vec::new(),
            draw_commands: Vec::new(),
        }
    }
}
