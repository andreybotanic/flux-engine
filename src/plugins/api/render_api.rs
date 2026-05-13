pub use flux_plugin_sdk::{
    BlendNode, MaterialNode, OverlayBlendMode, OverlayEntitySpriteOverride, OverlayEntityStyle,
    OverlayGraph, OverlayGraphError, OverlayImageInstance, OverlayMaterialDescriptor,
    OverlayMaterialId, OverlayMaterialParam, OverlayMaterialParamValue, OverlayNode, OverlayNodeId,
    OverlayNodeKind, OverlayOutput, OverlayPlacement, OverlaySelectorExpr, RenderEntitiesNode,
    RenderFreeGasNode, RenderImageNode, Selector,
};

/// Defines who controls rendering for one plugin overlay.
///
/// # Variants
/// - `CoreDefault`: The engine keeps its normal overlay rendering behavior.
/// - `PluginControlled`: The plugin fully defines rendering via overlay graph output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayRenderPolicy {
    CoreDefault,
    PluginControlled,
}
