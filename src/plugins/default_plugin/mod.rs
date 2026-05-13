use std::collections::BTreeMap;

use bevy::prelude::{IVec2, Resource, UVec2};

mod overlay_graph;
pub mod pipe_runtime;
pub mod runtime_sdk;

mod ids;
pub use ids::*;
pub(crate) use overlay_graph::build_pipes_overlay_graph;

use crate::{
    config::{
        ConfiguredPipeNodeKind, ContainerBacking, HoverVisibility, HudBlockConfig,
        SubstanceContainerConfig, SubstanceKind, VisualPlacementConfig, WorldCellHudConfig,
    },
    plugins::{
        content::{
            CellContentDescriptor, ContentId, ContentRegistry, LegacyStorageDescriptor,
            OverlayContentDescriptor, SpriteMetadata, StructureContentDescriptor,
        },
        PluginId, SubstanceDefinition, SubstanceId,
    },
    render::OverlayMode,
    world::{
        grid::CellMaterial,
        structures::{
            LayerCellSpec, LayerCollisionKind, LayerMarkerKind, StructureDescriptor, StructureKind,
            StructureLayer, StructureRotation, APPEARANCE_LAYER,
        },
    },
};

/// Resource snapshot of the built-in default plugin content descriptors.
#[derive(Resource, Clone, Debug)]
pub struct DefaultPluginContent {
    registry: ContentRegistry,
}

impl DefaultPluginContent {
    /// Builds a fresh default plugin content snapshot.
    pub fn new() -> Self {
        Self {
            registry: default_content_registry(),
        }
    }

    /// Returns the default plugin content registry.
    pub fn registry(&self) -> &ContentRegistry {
        &self.registry
    }
}

impl Default for DefaultPluginContent {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds the content registry entries provided by the locked default plugin.
pub fn default_content_registry() -> ContentRegistry {
    let mut registry = ContentRegistry::default();
    register_default_content(&mut registry);
    registry
}

/// Registers the default plugin provider and all built-in content descriptors.
pub fn register_default_content(registry: &mut ContentRegistry) {
    registry.register_provider_plugin(PluginId::default_plugin());
    registry.set_world_cell_hud(default_world_cell_hud());
    for descriptor in default_cell_descriptors() {
        registry.register_cell(descriptor);
    }
    for descriptor in default_structure_descriptors() {
        registry.register_structure(descriptor);
    }
    for descriptor in default_overlay_descriptors() {
        registry.register_overlay(descriptor);
    }
    for definition in default_substance_definitions() {
        registry.register_substance(definition);
    }
}

/// Returns every built-in gas substance definition registered by `flux.default`.
pub fn default_substance_definitions() -> Vec<SubstanceDefinition> {
    vec![
        default_gas_substance(SUBSTANCE_H2_ID, "h2", "Hydrogen", 2.016, [0.82, 0.20, 0.90]),
        default_gas_substance(SUBSTANCE_O2_ID, "o2", "Oxygen", 31.998, [0.00, 0.86, 0.86]),
        default_gas_substance(
            SUBSTANCE_CO2_ID,
            "co2",
            "Carbon dioxide",
            44.009,
            [0.58, 0.58, 0.58],
        ),
    ]
}

/// Builds the stable default plugin substance id for a legacy short gas alias.
pub fn default_substance_id_for_alias(alias: &str) -> Result<SubstanceId, String> {
    if alias.trim().is_empty() || alias.contains('.') {
        return SubstanceId::parse(alias);
    }
    SubstanceId::parse(&format!("flux.default.substance.{alias}"))
}

/// Returns the default world-cell HUD descriptor.
pub fn default_world_cell_hud() -> WorldCellHudConfig {
    WorldCellHudConfig {
        label: "Cell".to_string(),
        block: HudBlockConfig {
            sort_order: 0,
            substance_containers: vec![SubstanceContainerConfig {
                substance: SubstanceKind::Gas,
                backing: ContainerBacking::WorldCell,
                visible_on_hover: HoverVisibility::SameCell,
            }],
        },
    }
}

/// Returns every cell material descriptor registered by `flux.default`.
pub fn default_cell_descriptors() -> Vec<CellContentDescriptor> {
    vec![
        cell_descriptor_for(
            CELL_BOUNDARY_ID,
            boundary_cell_material(),
            "boundary.toml",
            "Boundary",
            "flux_default://world/tile_boundary.png",
            None,
            1,
        ),
        cell_descriptor_for(
            CELL_BRICK_ID,
            brick_cell_material(),
            "brick.toml",
            "Brick",
            "flux_default://world/tile_brick.png",
            Some("flux_default://world/silhouette_brick.png"),
            2,
        ),
        cell_descriptor_for(
            CELL_METAL_ID,
            metal_cell_material(),
            "metal.toml",
            "Metal",
            "flux_default://world/tile_metal.png",
            Some("flux_default://world/silhouette_metal.png"),
            3,
        ),
    ]
}

/// Returns every structure descriptor registered by `flux.default`.
pub fn default_structure_descriptors() -> Vec<StructureContentDescriptor> {
    vec![
        structure_descriptor_for(
            ENTITY_PIPE_ID,
            pipe_structure_kind(),
            "pipe.toml",
            "Pipe",
            100,
            UVec2::ONE,
            "flux_default://world/pipe_mask_00.png",
            Some("flux_default://world/pipe_silhouette_mask_00.png"),
            None,
            vec![StructureRotation::Deg0],
            pipe_hud_block(),
            "Pipe",
        ),
        structure_descriptor_for(
            ENTITY_VENT_ID,
            vent_structure_kind(),
            "vent.toml",
            "Vent",
            120,
            UVec2::ONE,
            "flux_default://world/tile_vent.png",
            Some("flux_default://world/silhouette_vent.png"),
            Some("flux_default://world/gas_in_out.png"),
            vec![StructureRotation::Deg0],
            title_only_hud_block(30),
            "Vent",
        ),
        structure_descriptor_for(
            ENTITY_GAS_SOURCE_ID,
            gas_source_structure_kind(),
            "gas_source.toml",
            "Gas Source",
            130,
            UVec2::ONE,
            "flux_default://world/tile_gas_source.png",
            Some("flux_default://world/tile_gas_source.png"),
            None,
            vec![StructureRotation::Deg0],
            title_only_hud_block(40),
            "GasSource",
        ),
        structure_descriptor_for(
            ENTITY_GAS_SINK_ID,
            gas_sink_structure_kind(),
            "gas_sink.toml",
            "Gas Sink",
            130,
            UVec2::ONE,
            "flux_default://world/tile_gas_sink.png",
            Some("flux_default://world/tile_gas_sink.png"),
            None,
            vec![StructureRotation::Deg0],
            title_only_hud_block(50),
            "GasSink",
        ),
        structure_descriptor_for(
            ENTITY_GAS_PIPE_BRIDGE_ID,
            gas_pipe_bridge_structure_kind(),
            "gas_pipe_bridge.toml",
            "Bridge",
            110,
            UVec2::new(3, 1),
            "flux_default://world/bridge.png",
            Some("flux_default://world/bridge_silhouette.png"),
            Some("flux_default://world/gas_in_out.png"),
            vec![StructureRotation::Deg0, StructureRotation::Deg90],
            bridge_hud_block(),
            "GasPipeBridge",
        ),
    ]
}

/// Returns every overlay descriptor registered by `flux.default`.
pub fn default_overlay_descriptors() -> Vec<OverlayContentDescriptor> {
    vec![
        overlay_descriptor(OVERLAY_MAIN_ID, OverlayMode::Main, "Main", "F1"),
        overlay_descriptor(OVERLAY_GAS_ID, OverlayMode::Gas, "Gas", "F2"),
        overlay_descriptor(OVERLAY_PIPES_ID, pipes_overlay_mode(), "Pipes", "F3"),
    ]
}

/// Returns the stable content id for a legacy cell material.
pub fn cell_material_content_id(material: CellMaterial) -> ContentId {
    content_id(material.as_str())
}

/// Returns the legacy cell material represented by a stable content id.
pub fn cell_material_from_content_id(id: &ContentId) -> Option<CellMaterial> {
    match id.as_str() {
        CELL_BOUNDARY_ID => Some(boundary_cell_material()),
        CELL_BRICK_ID => Some(brick_cell_material()),
        CELL_METAL_ID => Some(metal_cell_material()),
        _ => None,
    }
}

/// Returns the stable content id for a legacy structure kind.
pub fn structure_kind_content_id(kind: StructureKind) -> ContentId {
    content_id(kind.as_str())
}

/// Returns the legacy structure kind represented by a stable content id.
pub fn structure_kind_from_content_id(id: &ContentId) -> Option<StructureKind> {
    match id.as_str() {
        ENTITY_PIPE_ID => Some(pipe_structure_kind()),
        ENTITY_VENT_ID => Some(vent_structure_kind()),
        ENTITY_GAS_SOURCE_ID => Some(gas_source_structure_kind()),
        ENTITY_GAS_SINK_ID => Some(gas_sink_structure_kind()),
        ENTITY_GAS_PIPE_BRIDGE_ID => Some(gas_pipe_bridge_structure_kind()),
        _ => None,
    }
}

/// Returns the stable content id for a legacy overlay mode.
pub fn overlay_mode_content_id(mode: OverlayMode) -> ContentId {
    content_id(match mode {
        OverlayMode::Main => OVERLAY_MAIN_ID,
        OverlayMode::Gas => OVERLAY_GAS_ID,
        OverlayMode::Plugin(id) => id,
    })
}

/// Returns the legacy overlay mode represented by a stable content id.
pub fn overlay_mode_from_content_id(id: &ContentId) -> Option<OverlayMode> {
    match id.as_str() {
        OVERLAY_MAIN_ID => Some(OverlayMode::Main),
        OVERLAY_GAS_ID => Some(OverlayMode::Gas),
        OVERLAY_PIPES_ID => Some(pipes_overlay_mode()),
        _ => None,
    }
}

/// Returns the registered default descriptor for a legacy cell material.
pub fn cell_content_descriptor(material: CellMaterial) -> CellContentDescriptor {
    default_cell_descriptors()
        .into_iter()
        .find(|descriptor| descriptor.material == material)
        .unwrap_or_else(|| panic!("missing default cell content descriptor for {:?}", material))
}

/// Returns the registered default descriptor for a legacy structure kind.
pub fn structure_content_descriptor(kind: StructureKind) -> StructureContentDescriptor {
    default_structure_descriptors()
        .into_iter()
        .find(|descriptor| descriptor.kind == kind)
        .unwrap_or_else(|| {
            panic!(
                "missing default structure content descriptor for {:?}",
                kind
            )
        })
}

/// Returns the registered default descriptor for a legacy overlay mode.
pub fn overlay_content_descriptor(mode: OverlayMode) -> OverlayContentDescriptor {
    default_overlay_descriptors()
        .into_iter()
        .find(|descriptor| descriptor.mode == mode)
        .unwrap_or_else(|| panic!("missing default overlay content descriptor for {:?}", mode))
}

/// Returns the layer descriptor for a legacy cell material.
pub fn cell_layer_descriptor(material: CellMaterial) -> StructureDescriptor {
    cell_content_descriptor(material).layer_descriptor
}

/// Returns the layer descriptor for a legacy structure kind and rotation.
pub fn structure_layer_descriptor(
    kind: StructureKind,
    rotation: StructureRotation,
) -> StructureDescriptor {
    structure_content_descriptor(kind)
        .layer_descriptor(rotation)
        .clone()
}

/// Returns the label registered for a legacy cell material.
pub fn cell_label(material: CellMaterial) -> &'static str {
    match material.as_str() {
        CELL_BOUNDARY_ID => "Boundary",
        CELL_BRICK_ID => "Brick",
        CELL_METAL_ID => "Metal",
        _ => "Cell",
    }
}

/// Returns the label registered for a legacy structure kind.
pub fn structure_label(kind: StructureKind) -> &'static str {
    match kind.as_str() {
        ENTITY_PIPE_ID => "Pipe",
        ENTITY_VENT_ID => "Vent",
        ENTITY_GAS_SOURCE_ID => "Gas Source",
        ENTITY_GAS_SINK_ID => "Gas Sink",
        ENTITY_GAS_PIPE_BRIDGE_ID => "Bridge",
        _ => "Structure",
    }
}

/// Returns the sprite path registered for a legacy cell material.
pub fn cell_sprite_path(material: CellMaterial) -> &'static str {
    match material.as_str() {
        CELL_BOUNDARY_ID => "flux_default://world/tile_boundary.png",
        CELL_BRICK_ID => "flux_default://world/tile_brick.png",
        CELL_METAL_ID => "flux_default://world/tile_metal.png",
        _ => "flux_default://world/tile_brick.png",
    }
}

/// Returns the silhouette sprite path registered for a legacy cell material.
pub fn cell_silhouette_path(material: CellMaterial) -> Option<&'static str> {
    match material.as_str() {
        CELL_BOUNDARY_ID => None,
        CELL_BRICK_ID => Some("flux_default://world/silhouette_brick.png"),
        CELL_METAL_ID => Some("flux_default://world/silhouette_metal.png"),
        _ => None,
    }
}

/// Returns the UI tool icon path registered for a legacy cell material.
pub fn cell_tool_icon_path(material: CellMaterial) -> &'static str {
    match material.as_str() {
        CELL_BRICK_ID => "flux_default://ui/tool_brick.png",
        CELL_METAL_ID => "flux_default://ui/tool_metal.png",
        _ => "flux_default://ui/tool_brick.png",
    }
}

/// Returns the primary sprite path registered for a legacy structure kind.
pub fn structure_sprite_path(kind: StructureKind) -> &'static str {
    match kind.as_str() {
        ENTITY_PIPE_ID => "flux_default://world/pipe_mask_00.png",
        ENTITY_VENT_ID => "flux_default://world/tile_vent.png",
        ENTITY_GAS_SOURCE_ID => "flux_default://world/tile_gas_source.png",
        ENTITY_GAS_SINK_ID => "flux_default://world/tile_gas_sink.png",
        ENTITY_GAS_PIPE_BRIDGE_ID => "flux_default://world/bridge.png",
        _ => "flux_default://world/pipe_mask_00.png",
    }
}

/// Returns the silhouette sprite path registered for a legacy structure kind.
pub fn structure_silhouette_path(kind: StructureKind) -> Option<&'static str> {
    match kind.as_str() {
        ENTITY_PIPE_ID => Some("flux_default://world/pipe_silhouette_mask_00.png"),
        ENTITY_VENT_ID => Some("flux_default://world/silhouette_vent.png"),
        ENTITY_GAS_SOURCE_ID => Some("flux_default://world/tile_gas_source.png"),
        ENTITY_GAS_SINK_ID => Some("flux_default://world/tile_gas_sink.png"),
        ENTITY_GAS_PIPE_BRIDGE_ID => Some("flux_default://world/bridge_silhouette.png"),
        _ => None,
    }
}

/// Returns the UI tool icon path registered for a legacy structure kind.
pub fn structure_tool_icon_path(kind: StructureKind) -> &'static str {
    match kind.as_str() {
        ENTITY_PIPE_ID => "flux_default://ui/tool_pipe.png",
        ENTITY_VENT_ID => "flux_default://ui/tool_vent.png",
        ENTITY_GAS_SOURCE_ID => "flux_default://ui/tool_gas_source.png",
        ENTITY_GAS_SINK_ID => "flux_default://ui/tool_gas_sink.png",
        ENTITY_GAS_PIPE_BRIDGE_ID => "flux_default://ui/tool_bridge.png",
        _ => "flux_default://ui/tool_pipe.png",
    }
}

/// Returns the overlay sprite path registered for pipe connection markers.
pub fn pipe_connection_overlay_sprite_path() -> &'static str {
    "flux_default://world/gas_in_out.png"
}

/// Returns the registered pipe mask sprite path for one connection mask.
pub fn pipe_mask_sprite_path(mask: u8) -> String {
    format!("flux_default://world/pipe_mask_{mask:02}.png")
}

/// Returns true when the cell kind is the default plugin boundary material.
pub fn is_boundary_cell_kind(cell: crate::world::grid::CellKind) -> bool {
    cell == crate::world::grid::CellKind::Solid(boundary_cell_material())
}

/// Encodes one world cell using the legacy numeric save/runtime code.
pub fn legacy_cell_kind_code(cell: crate::world::grid::CellKind) -> Option<u8> {
    match cell {
        crate::world::grid::CellKind::Empty => Some(0),
        crate::world::grid::CellKind::Solid(material) if material == boundary_cell_material() => {
            Some(1)
        }
        crate::world::grid::CellKind::Solid(material) if material == brick_cell_material() => {
            Some(2)
        }
        crate::world::grid::CellKind::Solid(material) if material == metal_cell_material() => {
            Some(3)
        }
        crate::world::grid::CellKind::Solid(_) => None,
    }
}

/// Decodes one legacy numeric world-cell code.
pub fn legacy_cell_kind_from_code(code: u8) -> Option<crate::world::grid::CellKind> {
    match code {
        0 => Some(crate::world::grid::CellKind::Empty),
        1 => Some(crate::world::grid::CellKind::Solid(boundary_cell_material())),
        2 => Some(crate::world::grid::CellKind::Solid(brick_cell_material())),
        3 => Some(crate::world::grid::CellKind::Solid(metal_cell_material())),
        _ => None,
    }
}

/// Encodes one default plugin structure kind using the legacy numeric save code.
pub fn legacy_structure_kind_code(kind: StructureKind) -> Option<u8> {
    match kind.as_str() {
        ENTITY_PIPE_ID => Some(0),
        ENTITY_VENT_ID => Some(1),
        ENTITY_GAS_SOURCE_ID => Some(2),
        ENTITY_GAS_SINK_ID => Some(3),
        ENTITY_GAS_PIPE_BRIDGE_ID => Some(4),
        _ => None,
    }
}

/// Decodes one legacy numeric structure code into a default plugin structure kind.
pub fn legacy_structure_kind_from_code(code: u8) -> Option<StructureKind> {
    match code {
        0 => Some(pipe_structure_kind()),
        1 => Some(vent_structure_kind()),
        2 => Some(gas_source_structure_kind()),
        3 => Some(gas_sink_structure_kind()),
        4 => Some(gas_pipe_bridge_structure_kind()),
        _ => None,
    }
}

/// Returns true when the id is the default pipe content item.
pub fn is_pipe_structure(kind: StructureKind) -> bool {
    kind == pipe_structure_kind()
}

/// Returns true when the id is the default vent content item.
pub fn is_vent_structure(kind: StructureKind) -> bool {
    kind == vent_structure_kind()
}

/// Returns true when the id is the default gas source content item.
pub fn is_gas_source_structure(kind: StructureKind) -> bool {
    kind == gas_source_structure_kind()
}

/// Returns true when the id is the default gas sink content item.
pub fn is_gas_sink_structure(kind: StructureKind) -> bool {
    kind == gas_sink_structure_kind()
}

/// Returns true when the id is the default gas pipe bridge content item.
pub fn is_gas_pipe_bridge_structure(kind: StructureKind) -> bool {
    kind == gas_pipe_bridge_structure_kind()
}

/// Returns true when a structure can be edited by the default source/sink editor.
pub fn is_editable_gas_structure(kind: StructureKind) -> bool {
    is_gas_source_structure(kind) || is_gas_sink_structure(kind)
}

/// Returns true when this structure blocks solid cell placement.
pub fn blocks_default_solid_placement(kind: StructureKind) -> bool {
    is_vent_structure(kind) || is_gas_source_structure(kind) || is_gas_sink_structure(kind)
}

/// Returns the deterministic legacy sort key for default plugin structures.
pub fn default_structure_sort_key(kind: StructureKind) -> u8 {
    legacy_structure_kind_code(kind).unwrap_or(u8::MAX)
}

include!("descriptors_block.rs");

#[cfg(test)]
mod tests;
