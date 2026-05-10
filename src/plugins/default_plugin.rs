use std::collections::BTreeMap;

use bevy::prelude::{IVec2, Resource, UVec2};

pub mod pipe_runtime;

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
            LayerCellSpec, LayerCollisionKind, LayerKind, LayerMarkerKind, StructureDescriptor,
            StructureKind, StructureLayer, StructureRotation, APPEARANCE_LAYER,
        },
    },
};

/// Stable content id for the default boundary cell.
pub const CELL_BOUNDARY_ID: &str = "flux.default.cell.boundary";
/// Stable content id for the default brick cell.
pub const CELL_BRICK_ID: &str = "flux.default.cell.brick";
/// Stable content id for the default metal cell.
pub const CELL_METAL_ID: &str = "flux.default.cell.metal";
/// Stable content id for the default pipe entity.
pub const ENTITY_PIPE_ID: &str = "flux.default.entity.pipe";
/// Stable content id for the default vent entity.
pub const ENTITY_VENT_ID: &str = "flux.default.entity.vent";
/// Stable content id for the default gas source entity.
pub const ENTITY_GAS_SOURCE_ID: &str = "flux.default.entity.gas_source";
/// Stable content id for the default gas sink entity.
pub const ENTITY_GAS_SINK_ID: &str = "flux.default.entity.gas_sink";
/// Stable content id for the default gas pipe bridge entity.
pub const ENTITY_GAS_PIPE_BRIDGE_ID: &str = "flux.default.entity.gas_pipe_bridge";
/// Stable content id for the default main overlay mode.
pub const OVERLAY_MAIN_ID: &str = "flux.default.overlay.main";
/// Stable content id for the default gas overlay mode.
pub const OVERLAY_GAS_ID: &str = "flux.default.overlay.gas";
/// Stable content id for the default pipes overlay mode.
pub const OVERLAY_PIPES_ID: &str = "flux.default.overlay.pipes";
/// Stable substance id for default hydrogen gas.
pub const SUBSTANCE_H2_ID: &str = "flux.default.substance.h2";
/// Stable substance id for default oxygen gas.
pub const SUBSTANCE_O2_ID: &str = "flux.default.substance.o2";
/// Stable substance id for default carbon dioxide gas.
pub const SUBSTANCE_CO2_ID: &str = "flux.default.substance.co2";

const LAYER_GAS_PIPE_CONNECTIONS_ID: &str = "flux.default.layer.gas_pipe_connections";
const MARKER_GAS_PIPE_CONNECTION_BIDIRECTIONAL_ID: &str =
    "flux.default.marker.gas_pipe_connection_bidirectional";

/// Returns the default plugin boundary cell id wrapper.
pub const fn boundary_cell_material() -> CellMaterial {
    CellMaterial::new(CELL_BOUNDARY_ID)
}

/// Returns the default plugin brick cell id wrapper.
pub const fn brick_cell_material() -> CellMaterial {
    CellMaterial::new(CELL_BRICK_ID)
}

/// Returns the default plugin metal cell id wrapper.
pub const fn metal_cell_material() -> CellMaterial {
    CellMaterial::new(CELL_METAL_ID)
}

/// Returns the default plugin pipe structure id wrapper.
pub const fn pipe_structure_kind() -> StructureKind {
    StructureKind::new(ENTITY_PIPE_ID)
}

/// Returns the default plugin vent structure id wrapper.
pub const fn vent_structure_kind() -> StructureKind {
    StructureKind::new(ENTITY_VENT_ID)
}

/// Returns the default plugin gas source structure id wrapper.
pub const fn gas_source_structure_kind() -> StructureKind {
    StructureKind::new(ENTITY_GAS_SOURCE_ID)
}

/// Returns the default plugin gas sink structure id wrapper.
pub const fn gas_sink_structure_kind() -> StructureKind {
    StructureKind::new(ENTITY_GAS_SINK_ID)
}

/// Returns the default plugin gas pipe bridge structure id wrapper.
pub const fn gas_pipe_bridge_structure_kind() -> StructureKind {
    StructureKind::new(ENTITY_GAS_PIPE_BRIDGE_ID)
}

/// Returns the default plugin pipe overlay mode.
pub const fn pipes_overlay_mode() -> OverlayMode {
    OverlayMode::plugin(OVERLAY_PIPES_ID)
}

/// Returns true when the overlay mode is the default plugin pipe overlay.
pub fn is_pipes_overlay_mode(mode: OverlayMode) -> bool {
    matches!(mode, OverlayMode::Plugin(id) if id == OVERLAY_PIPES_ID)
}

/// Returns the default plugin pipe connection layer id.
pub const fn gas_pipe_connections_layer() -> LayerKind {
    LayerKind::new(LAYER_GAS_PIPE_CONNECTIONS_ID)
}

/// Returns a marker id for one default plugin content item.
pub const fn content_marker(id: &'static str) -> LayerMarkerKind {
    LayerMarkerKind::new(id)
}

/// Returns the marker used by default pipe connection cells.
pub const fn gas_pipe_connection_bidirectional_marker() -> LayerMarkerKind {
    LayerMarkerKind::new(MARKER_GAS_PIPE_CONNECTION_BIDIRECTIONAL_ID)
}

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
            "sprites/world/tile_boundary.png",
            None,
            1,
        ),
        cell_descriptor_for(
            CELL_BRICK_ID,
            brick_cell_material(),
            "brick.toml",
            "Brick",
            "sprites/world/tile_brick.png",
            Some("sprites/world/silhouette_brick.png"),
            2,
        ),
        cell_descriptor_for(
            CELL_METAL_ID,
            metal_cell_material(),
            "metal.toml",
            "Metal",
            "sprites/world/tile_metal.png",
            Some("sprites/world/silhouette_metal.png"),
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
            "sprites/world/pipe_mask_00.png",
            Some("sprites/world/pipe_silhouette_mask_00.png"),
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
            "sprites/world/tile_vent.png",
            Some("sprites/world/silhouette_vent.png"),
            Some("sprites/world/gas_in_out.png"),
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
            "sprites/world/tile_gas_source.png",
            Some("sprites/world/tile_gas_source.png"),
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
            "sprites/world/tile_gas_sink.png",
            Some("sprites/world/tile_gas_sink.png"),
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
            "sprites/world/bridge.png",
            Some("sprites/world/bridge_silhouette.png"),
            Some("sprites/world/gas_in_out.png"),
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
        CELL_BOUNDARY_ID => "sprites/world/tile_boundary.png",
        CELL_BRICK_ID => "sprites/world/tile_brick.png",
        CELL_METAL_ID => "sprites/world/tile_metal.png",
        _ => "sprites/world/tile_brick.png",
    }
}

/// Returns the silhouette sprite path registered for a legacy cell material.
pub fn cell_silhouette_path(material: CellMaterial) -> Option<&'static str> {
    match material.as_str() {
        CELL_BOUNDARY_ID => None,
        CELL_BRICK_ID => Some("sprites/world/silhouette_brick.png"),
        CELL_METAL_ID => Some("sprites/world/silhouette_metal.png"),
        _ => None,
    }
}

/// Returns the primary sprite path registered for a legacy structure kind.
pub fn structure_sprite_path(kind: StructureKind) -> &'static str {
    match kind.as_str() {
        ENTITY_PIPE_ID => "sprites/world/pipe_mask_00.png",
        ENTITY_VENT_ID => "sprites/world/tile_vent.png",
        ENTITY_GAS_SOURCE_ID => "sprites/world/tile_gas_source.png",
        ENTITY_GAS_SINK_ID => "sprites/world/tile_gas_sink.png",
        ENTITY_GAS_PIPE_BRIDGE_ID => "sprites/world/bridge.png",
        _ => "sprites/world/pipe_mask_00.png",
    }
}

/// Returns the silhouette sprite path registered for a legacy structure kind.
pub fn structure_silhouette_path(kind: StructureKind) -> Option<&'static str> {
    match kind.as_str() {
        ENTITY_PIPE_ID => Some("sprites/world/pipe_silhouette_mask_00.png"),
        ENTITY_VENT_ID => Some("sprites/world/silhouette_vent.png"),
        ENTITY_GAS_SOURCE_ID => Some("sprites/world/tile_gas_source.png"),
        ENTITY_GAS_SINK_ID => Some("sprites/world/tile_gas_sink.png"),
        ENTITY_GAS_PIPE_BRIDGE_ID => Some("sprites/world/bridge_silhouette.png"),
        _ => None,
    }
}

/// Returns the overlay sprite path registered for pipe connection markers.
pub fn pipe_connection_overlay_sprite_path() -> &'static str {
    "sprites/world/gas_in_out.png"
}

/// Returns the registered pipe mask sprite path for one connection mask.
pub fn pipe_mask_sprite_path(mask: u8) -> String {
    format!("sprites/world/pipe_mask_{mask:02}.png")
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

include!("default_plugin_descriptors_block.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_plugin_content_ids_roundtrip_legacy_enums() {
        for material in [
            crate::plugins::default_plugin::boundary_cell_material(),
            crate::plugins::default_plugin::brick_cell_material(),
            crate::plugins::default_plugin::metal_cell_material(),
        ] {
            let id = cell_material_content_id(material);
            assert_eq!(cell_material_from_content_id(&id), Some(material));
        }

        for kind in [
            crate::plugins::default_plugin::pipe_structure_kind(),
            crate::plugins::default_plugin::vent_structure_kind(),
            crate::plugins::default_plugin::gas_source_structure_kind(),
            crate::plugins::default_plugin::gas_sink_structure_kind(),
            crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
        ] {
            let id = structure_kind_content_id(kind);
            assert_eq!(structure_kind_from_content_id(&id), Some(kind));
        }

        for mode in [OverlayMode::Main, OverlayMode::Gas, pipes_overlay_mode()] {
            let id = overlay_mode_content_id(mode);
            assert_eq!(overlay_mode_from_content_id(&id), Some(mode));
        }
    }

    #[test]
    fn default_plugin_registry_contains_all_builtin_content() {
        let registry = default_content_registry();
        assert!(registry
            .provider_plugins()
            .contains(&PluginId::default_plugin()));
        assert_eq!(registry.cells().len(), 3);
        assert_eq!(registry.structures().len(), 5);
        assert_eq!(registry.overlays().len(), 3);
        assert_eq!(registry.substances().len(), 3);
        assert!(registry
            .substances()
            .contains_key(&SubstanceId::parse(SUBSTANCE_H2_ID).expect("h2 id")));
        assert!(registry
            .world_cell_hud()
            .expect("world hud")
            .block
            .substance_containers
            .iter()
            .any(|container| container.backing == ContainerBacking::WorldCell));
    }

    #[test]
    fn default_plugin_bridge_keeps_legacy_shape_and_rotations() {
        let descriptor = structure_content_descriptor(
            crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
        );
        assert_eq!(
            descriptor.allowed_rotations,
            vec![StructureRotation::Deg0, StructureRotation::Deg90]
        );
        assert_eq!(
            descriptor
                .layer_descriptor(StructureRotation::Deg0)
                .size_in_cells(),
            UVec2::new(3, 1)
        );
        assert_eq!(
            descriptor
                .layer_descriptor(StructureRotation::Deg90)
                .size_in_cells(),
            UVec2::new(1, 3)
        );
    }

    #[test]
    fn default_plugin_hud_order_matches_legacy_blocks() {
        let structures = default_structure_descriptors();
        let orders = structures
            .iter()
            .map(|descriptor| (descriptor.kind, descriptor.hud.sort_order))
            .collect::<Vec<_>>();
        assert!(orders.contains(&(crate::plugins::default_plugin::pipe_structure_kind(), 10)));
        assert!(orders.contains(&(
            crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
            20
        )));
        assert!(orders.contains(&(crate::plugins::default_plugin::vent_structure_kind(), 30)));
        assert!(orders.contains(&(
            crate::plugins::default_plugin::gas_source_structure_kind(),
            40
        )));
        assert!(orders.contains(&(
            crate::plugins::default_plugin::gas_sink_structure_kind(),
            50
        )));
    }
}
