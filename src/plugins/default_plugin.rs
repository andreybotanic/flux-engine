use std::collections::BTreeMap;

use bevy::prelude::{IVec2, Resource, UVec2};

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
        PluginId,
    },
    render::OverlayMode,
    world::{
        grid::CellMaterial,
        structures::{
            LayerCellSpec, LayerCollisionKind, LayerKind, LayerMarkerKind, StructureDescriptor,
            StructureKind, StructureLayer, StructureRotation,
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
            CellMaterial::Boundary,
            "boundary.toml",
            "Boundary",
            "sprites/world/tile_boundary.png",
            None,
            1,
        ),
        cell_descriptor_for(
            CELL_BRICK_ID,
            CellMaterial::Brick,
            "brick.toml",
            "Brick",
            "sprites/world/tile_brick.png",
            Some("sprites/world/silhouette_brick.png"),
            2,
        ),
        cell_descriptor_for(
            CELL_METAL_ID,
            CellMaterial::Metal,
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
            StructureKind::Pipe,
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
            StructureKind::Vent,
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
            StructureKind::GasSource,
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
            StructureKind::GasSink,
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
            StructureKind::GasPipeBridge,
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
        overlay_descriptor(OVERLAY_PIPES_ID, OverlayMode::Pipes, "Pipes", "F3"),
    ]
}

/// Returns the stable content id for a legacy cell material.
pub fn cell_material_content_id(material: CellMaterial) -> ContentId {
    content_id(match material {
        CellMaterial::Boundary => CELL_BOUNDARY_ID,
        CellMaterial::Brick => CELL_BRICK_ID,
        CellMaterial::Metal => CELL_METAL_ID,
    })
}

/// Returns the legacy cell material represented by a stable content id.
pub fn cell_material_from_content_id(id: &ContentId) -> Option<CellMaterial> {
    match id.as_str() {
        CELL_BOUNDARY_ID => Some(CellMaterial::Boundary),
        CELL_BRICK_ID => Some(CellMaterial::Brick),
        CELL_METAL_ID => Some(CellMaterial::Metal),
        _ => None,
    }
}

/// Returns the stable content id for a legacy structure kind.
pub fn structure_kind_content_id(kind: StructureKind) -> ContentId {
    content_id(match kind {
        StructureKind::Pipe => ENTITY_PIPE_ID,
        StructureKind::Vent => ENTITY_VENT_ID,
        StructureKind::GasSource => ENTITY_GAS_SOURCE_ID,
        StructureKind::GasSink => ENTITY_GAS_SINK_ID,
        StructureKind::GasPipeBridge => ENTITY_GAS_PIPE_BRIDGE_ID,
    })
}

/// Returns the legacy structure kind represented by a stable content id.
pub fn structure_kind_from_content_id(id: &ContentId) -> Option<StructureKind> {
    match id.as_str() {
        ENTITY_PIPE_ID => Some(StructureKind::Pipe),
        ENTITY_VENT_ID => Some(StructureKind::Vent),
        ENTITY_GAS_SOURCE_ID => Some(StructureKind::GasSource),
        ENTITY_GAS_SINK_ID => Some(StructureKind::GasSink),
        ENTITY_GAS_PIPE_BRIDGE_ID => Some(StructureKind::GasPipeBridge),
        _ => None,
    }
}

/// Returns the stable content id for a legacy overlay mode.
pub fn overlay_mode_content_id(mode: OverlayMode) -> ContentId {
    content_id(match mode {
        OverlayMode::Main => OVERLAY_MAIN_ID,
        OverlayMode::Gas => OVERLAY_GAS_ID,
        OverlayMode::Pipes => OVERLAY_PIPES_ID,
    })
}

/// Returns the legacy overlay mode represented by a stable content id.
pub fn overlay_mode_from_content_id(id: &ContentId) -> Option<OverlayMode> {
    match id.as_str() {
        OVERLAY_MAIN_ID => Some(OverlayMode::Main),
        OVERLAY_GAS_ID => Some(OverlayMode::Gas),
        OVERLAY_PIPES_ID => Some(OverlayMode::Pipes),
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
    match material {
        CellMaterial::Boundary => "Boundary",
        CellMaterial::Brick => "Brick",
        CellMaterial::Metal => "Metal",
    }
}

/// Returns the label registered for a legacy structure kind.
pub fn structure_label(kind: StructureKind) -> &'static str {
    match kind {
        StructureKind::Pipe => "Pipe",
        StructureKind::Vent => "Vent",
        StructureKind::GasSource => "Gas Source",
        StructureKind::GasSink => "Gas Sink",
        StructureKind::GasPipeBridge => "Bridge",
    }
}

/// Returns the sprite path registered for a legacy cell material.
pub fn cell_sprite_path(material: CellMaterial) -> &'static str {
    match material {
        CellMaterial::Boundary => "sprites/world/tile_boundary.png",
        CellMaterial::Brick => "sprites/world/tile_brick.png",
        CellMaterial::Metal => "sprites/world/tile_metal.png",
    }
}

/// Returns the silhouette sprite path registered for a legacy cell material.
pub fn cell_silhouette_path(material: CellMaterial) -> Option<&'static str> {
    match material {
        CellMaterial::Boundary => None,
        CellMaterial::Brick => Some("sprites/world/silhouette_brick.png"),
        CellMaterial::Metal => Some("sprites/world/silhouette_metal.png"),
    }
}

/// Returns the primary sprite path registered for a legacy structure kind.
pub fn structure_sprite_path(kind: StructureKind) -> &'static str {
    match kind {
        StructureKind::Pipe => "sprites/world/pipe_mask_00.png",
        StructureKind::Vent => "sprites/world/tile_vent.png",
        StructureKind::GasSource => "sprites/world/tile_gas_source.png",
        StructureKind::GasSink => "sprites/world/tile_gas_sink.png",
        StructureKind::GasPipeBridge => "sprites/world/bridge.png",
    }
}

/// Returns the silhouette sprite path registered for a legacy structure kind.
pub fn structure_silhouette_path(kind: StructureKind) -> Option<&'static str> {
    match kind {
        StructureKind::Pipe => Some("sprites/world/pipe_silhouette_mask_00.png"),
        StructureKind::Vent => Some("sprites/world/silhouette_vent.png"),
        StructureKind::GasSource => Some("sprites/world/tile_gas_source.png"),
        StructureKind::GasSink => Some("sprites/world/tile_gas_sink.png"),
        StructureKind::GasPipeBridge => Some("sprites/world/bridge_silhouette.png"),
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

include!("default_plugin_descriptors_block.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_plugin_content_ids_roundtrip_legacy_enums() {
        for material in [
            CellMaterial::Boundary,
            CellMaterial::Brick,
            CellMaterial::Metal,
        ] {
            let id = cell_material_content_id(material);
            assert_eq!(cell_material_from_content_id(&id), Some(material));
        }

        for kind in [
            StructureKind::Pipe,
            StructureKind::Vent,
            StructureKind::GasSource,
            StructureKind::GasSink,
            StructureKind::GasPipeBridge,
        ] {
            let id = structure_kind_content_id(kind);
            assert_eq!(structure_kind_from_content_id(&id), Some(kind));
        }

        for mode in [OverlayMode::Main, OverlayMode::Gas, OverlayMode::Pipes] {
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
        let descriptor = structure_content_descriptor(StructureKind::GasPipeBridge);
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
        assert!(orders.contains(&(StructureKind::Pipe, 10)));
        assert!(orders.contains(&(StructureKind::GasPipeBridge, 20)));
        assert!(orders.contains(&(StructureKind::Vent, 30)));
        assert!(orders.contains(&(StructureKind::GasSource, 40)));
        assert!(orders.contains(&(StructureKind::GasSink, 50)));
    }
}
