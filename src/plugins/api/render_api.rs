use bevy::prelude::*;

use crate::{
    plugins::ContentId,
    world::structures::{PlacedStructureId, StructureKind},
};

/// Defines who controls rendering for one plugin overlay.
///
/// # Variants
/// - `CoreDefault`: The engine keeps its normal overlay rendering behavior.
/// - `PluginControlled`: The plugin is responsible for supplying the visible overlay frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayRenderPolicy {
    CoreDefault,
    PluginControlled,
}

/// Display style for one world cell in a plugin-controlled overlay.
///
/// # Variants
/// - `Hidden`: The cell is hidden in the plugin-controlled overlay frame.
/// - `Normal`: The cell uses the engine's default overlay rendering.
/// - `Filled`: The cell is rendered as a filled color block with explicit alpha.
/// - `Outline`: The cell is rendered as an outline with explicit color and alpha.
/// - `Sprite`: The cell is rendered with a plugin-provided sprite asset override.
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
/// - `Hidden`: The structure is hidden in the plugin-controlled overlay frame.
/// - `Normal`: The structure uses the engine's default overlay rendering.
/// - `OutlineOnly`: The structure footprint is rendered as an outline only.
/// - `Filled`: The structure footprint is rendered as a filled color block.
/// - `SpriteOverride`: The structure is rendered with a plugin-provided sprite asset override.
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
/// - `Hidden`: Gas is hidden for the target cell.
/// - `Normal`: Gas uses the engine's default overlay rendering.
/// - `Custom`: Gas is rendered with a plugin-provided color and intensity.
#[derive(Clone, Debug, PartialEq)]
pub enum GasRenderStyle {
    Hidden,
    Normal,
    Custom { color: Color, intensity: f32 },
}

/// Extra draw command emitted by a plugin overlay.
///
/// # Variants
/// - `Rect`: Draws a filled rectangle aligned to one world cell.
/// - `Outline`: Draws an outlined rectangle aligned to one world cell.
/// - `Sprite`: Draws a sprite aligned to one world cell.
/// - `Line`: Draws a world-space line between two points.
/// - `Text`: Draws text anchored to one world cell.
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
/// - `cell`: Target world cell that receives the style override.
/// - `style`: Cell rendering style to apply in the overlay frame.
#[derive(Clone, Debug, PartialEq)]
pub struct CellStyleEntry {
    pub cell: UVec2,
    pub style: CellRenderStyle,
}

/// Per-structure style entry for an overlay frame.
///
/// # Fields
/// - `structure_id`: Runtime id of the structure receiving the style override.
/// - `kind`: Registered structure kind of the styled structure.
/// - `style`: Structure rendering style to apply in the overlay frame.
/// - `z_order`: Explicit ordering value used for overlay composition.
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
/// - `cell`: Target world cell whose gas rendering is overridden.
/// - `style`: Gas rendering style to apply for that cell.
#[derive(Clone, Debug, PartialEq)]
pub struct GasStyleEntry {
    pub cell: UVec2,
    pub style: GasRenderStyle,
}

/// Render frame returned by a plugin for one overlay refresh.
///
/// # Fields
/// - `overlay_id`: Stable content id of the overlay producing this frame.
/// - `policy`: Rendering ownership mode used for this frame.
/// - `background`: Optional full-frame background color drawn before cell content.
/// - `world_alpha`: Alpha multiplier applied to the core world rendering pass.
/// - `show_core_gas`: Whether core gas rendering remains visible underneath plugin styling.
/// - `cell_styles`: Per-cell visual overrides emitted by the plugin.
/// - `structure_styles`: Per-structure visual overrides emitted by the plugin.
/// - `gas_styles`: Per-cell gas visual overrides emitted by the plugin.
/// - `draw_commands`: Extra draw primitives layered on top of the frame.
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
