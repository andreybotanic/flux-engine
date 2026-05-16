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
            CellContentDescriptor, ContentId, ContentRegistry, EntityCategoryDescriptor,
            LegacyStorageDescriptor, OverlayContentDescriptor, SpriteMetadata,
            StructureContentDescriptor,
        },
        PluginId, SubstanceDefinition, SubstanceId,
    },
    render::OverlayMode,
    world::{
        grid::CellMaterial,
        structures::{
            LayerCellSpec, LayerCollisionKind, LayerMarkerKind, PackedState, StructureDescriptor,
            StructureKind, StructureLayer, StructureRotation, APPEARANCE_LAYER,
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
    for descriptor in default_entity_category_descriptors() {
        registry.register_entity_category(descriptor);
    }
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

/// Returns every entity category descriptor registered by `flux.default`.
pub fn default_entity_category_descriptors() -> Vec<EntityCategoryDescriptor> {
    vec![
        EntityCategoryDescriptor {
            id: content_id(CATEGORY_CELLS_ID),
            plugin_id: PluginId::default_plugin(),
            label: "Cells".to_string(),
            icon_path: "sprites/ui/tool_build.ktx2".to_string(),
        },
        EntityCategoryDescriptor {
            id: content_id(CATEGORY_GASES_ID),
            plugin_id: PluginId::default_plugin(),
            label: "Gases".to_string(),
            icon_path: "sprites/ui/tool_gases.ktx2".to_string(),
        },
    ]
}

/// Returns the stable category id for the default `Cells` category.
pub fn cells_category_content_id() -> ContentId {
    content_id(CATEGORY_CELLS_ID)
}

/// Returns the stable category id for the default `Gases` category.
pub fn gases_category_content_id() -> ContentId {
    content_id(CATEGORY_GASES_ID)
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
    let mut descriptors = vec![
        cell_descriptor_for(
            CELL_BOUNDARY_ID,
            boundary_cell_material(),
            "boundary.toml",
            "Boundary",
            "flux_default://world/tile_boundary.ktx2",
            None,
            None,
            1,
        ),
        cell_descriptor_for(
            CELL_BRICK_ID,
            brick_cell_material(),
            "brick.toml",
            "Brick",
            "flux_default://world/tile_brick.ktx2",
            Some("flux_default://world/silhouette_brick.ktx2"),
            Some(CATEGORY_CELLS_ID),
            2,
        ),
        cell_descriptor_for(
            CELL_METAL_ID,
            metal_cell_material(),
            "metal.toml",
            "Metal",
            "flux_default://world/tile_metal.ktx2",
            Some("flux_default://world/silhouette_metal.ktx2"),
            Some(CATEGORY_CELLS_ID),
            3,
        ),
    ];

    let extra_cells = [
        (CELL_BRICK_01_ID, "Brick Alpha", true),
        (CELL_BRICK_02_ID, "Brick Beta", true),
        (CELL_BRICK_03_ID, "Brick Gamma", true),
        (CELL_BRICK_04_ID, "Brick Delta", true),
        (CELL_BRICK_05_ID, "Brick Epsilon", true),
        (CELL_BRICK_06_ID, "Brick Zeta", true),
        (CELL_BRICK_07_ID, "Brick Eta", true),
        (CELL_BRICK_08_ID, "Brick Theta", true),
        (CELL_BRICK_09_ID, "Brick Iota", true),
        (CELL_BRICK_10_ID, "Brick Kappa", true),
        (CELL_METAL_01_ID, "Metal Alpha", false),
        (CELL_METAL_02_ID, "Metal Beta", false),
        (CELL_METAL_03_ID, "Metal Gamma", false),
        (CELL_METAL_04_ID, "Metal Delta", false),
        (CELL_METAL_05_ID, "Metal Epsilon", false),
        (CELL_METAL_06_ID, "Metal Zeta", false),
        (CELL_METAL_07_ID, "Metal Eta", false),
        (CELL_METAL_08_ID, "Metal Theta", false),
        (CELL_METAL_09_ID, "Metal Iota", false),
        (CELL_METAL_10_ID, "Metal Kappa", false),
    ];
    for (index, (id, label, is_brick)) in extra_cells.iter().enumerate() {
        let (config_file, image, silhouette) = if *is_brick {
            (
                "brick.toml",
                "flux_default://world/tile_brick.ktx2",
                Some("flux_default://world/silhouette_brick.ktx2"),
            )
        } else {
            (
                "metal.toml",
                "flux_default://world/tile_metal.ktx2",
                Some("flux_default://world/silhouette_metal.ktx2"),
            )
        };
        descriptors.push(cell_descriptor_for(
            id,
            CellMaterial::new(id),
            config_file,
            label,
            image,
            silhouette,
            Some(CATEGORY_CELLS_ID),
            (4 + index) as u8,
        ));
    }
    descriptors
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
            "flux_default://world/pipe_mask_00.ktx2",
            Some("flux_default://world/pipe_silhouette_mask_00.ktx2"),
            None,
            vec![StructureRotation::Deg0],
            Some(CATEGORY_GASES_ID),
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
            "flux_default://world/tile_vent.ktx2",
            Some("flux_default://world/silhouette_vent.ktx2"),
            Some("flux_default://world/gas_in_out.ktx2"),
            vec![StructureRotation::Deg0],
            Some(CATEGORY_GASES_ID),
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
            "flux_default://world/tile_gas_source.ktx2",
            Some("flux_default://world/tile_gas_source.ktx2"),
            None,
            vec![StructureRotation::Deg0],
            None,
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
            "flux_default://world/tile_gas_sink.ktx2",
            Some("flux_default://world/tile_gas_sink.ktx2"),
            None,
            vec![StructureRotation::Deg0],
            None,
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
            "flux_default://world/bridge.ktx2",
            Some("flux_default://world/bridge_silhouette.ktx2"),
            Some("flux_default://world/gas_in_out.ktx2"),
            vec![StructureRotation::Deg0, StructureRotation::Deg90],
            Some(CATEGORY_GASES_ID),
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
        _ => extra_cell_materials()
            .into_iter()
            .find(|material| material.as_str() == id.as_str()),
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
    if let Some(extra_label) = extra_cell_label(material) {
        return extra_label;
    }
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
    if is_brick_family_cell(material) {
        return "flux_default://world/tile_brick.ktx2";
    }
    if is_metal_family_cell(material) {
        return "flux_default://world/tile_metal.ktx2";
    }
    match material.as_str() {
        CELL_BOUNDARY_ID => "flux_default://world/tile_boundary.ktx2",
        _ => "flux_default://world/tile_brick.ktx2",
    }
}

/// Returns the silhouette sprite path registered for a legacy cell material.
pub fn cell_silhouette_path(material: CellMaterial) -> Option<&'static str> {
    if is_brick_family_cell(material) {
        return Some("flux_default://world/silhouette_brick.ktx2");
    }
    if is_metal_family_cell(material) {
        return Some("flux_default://world/silhouette_metal.ktx2");
    }
    match material.as_str() {
        CELL_BOUNDARY_ID => None,
        _ => None,
    }
}

/// Returns the UI tool icon path registered for a legacy cell material.
pub fn cell_tool_icon_path(material: CellMaterial) -> &'static str {
    if is_brick_family_cell(material) {
        return "flux_default://ui/tool_brick.ktx2";
    }
    if is_metal_family_cell(material) {
        return "flux_default://ui/tool_metal.ktx2";
    }
    match material.as_str() {
        _ => "flux_default://ui/tool_brick.ktx2",
    }
}

/// Returns the primary sprite path registered for a legacy structure kind.
pub fn structure_sprite_path(kind: StructureKind) -> &'static str {
    match kind.as_str() {
        ENTITY_PIPE_ID => "flux_default://world/pipe_mask_00.ktx2",
        ENTITY_VENT_ID => "flux_default://world/tile_vent.ktx2",
        ENTITY_GAS_SOURCE_ID => "flux_default://world/tile_gas_source.ktx2",
        ENTITY_GAS_SINK_ID => "flux_default://world/tile_gas_sink.ktx2",
        ENTITY_GAS_PIPE_BRIDGE_ID => "flux_default://world/bridge.ktx2",
        _ => "flux_default://world/pipe_mask_00.ktx2",
    }
}

/// Returns the silhouette sprite path registered for a legacy structure kind.
pub fn structure_silhouette_path(kind: StructureKind) -> Option<&'static str> {
    match kind.as_str() {
        ENTITY_PIPE_ID => Some("flux_default://world/pipe_silhouette_mask_00.ktx2"),
        ENTITY_VENT_ID => Some("flux_default://world/silhouette_vent.ktx2"),
        ENTITY_GAS_SOURCE_ID => Some("flux_default://world/tile_gas_source.ktx2"),
        ENTITY_GAS_SINK_ID => Some("flux_default://world/tile_gas_sink.ktx2"),
        ENTITY_GAS_PIPE_BRIDGE_ID => Some("flux_default://world/bridge_silhouette.ktx2"),
        _ => None,
    }
}

/// Returns the UI tool icon path registered for a legacy structure kind.
pub fn structure_tool_icon_path(kind: StructureKind) -> &'static str {
    match kind.as_str() {
        ENTITY_PIPE_ID => "flux_default://ui/tool_pipe.ktx2",
        ENTITY_VENT_ID => "flux_default://ui/tool_vent.ktx2",
        ENTITY_GAS_SOURCE_ID => "flux_default://ui/tool_gas_source.ktx2",
        ENTITY_GAS_SINK_ID => "flux_default://ui/tool_gas_sink.ktx2",
        ENTITY_GAS_PIPE_BRIDGE_ID => "flux_default://ui/tool_bridge.ktx2",
        _ => "flux_default://ui/tool_pipe.ktx2",
    }
}

/// Returns the overlay sprite path registered for pipe connection markers.
pub fn pipe_connection_overlay_sprite_path() -> &'static str {
    "flux_default://world/gas_in_out.ktx2"
}

/// Returns the registered pipe mask sprite path for one connection mask.
pub fn pipe_mask_sprite_path(mask: u8) -> String {
    format!("flux_default://world/pipe_mask_{mask:02}.ktx2")
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
        crate::world::grid::CellKind::Solid(material) => extra_cell_materials()
            .iter()
            .position(|candidate| *candidate == material)
            .map(|index| (4 + index) as u8),
    }
}

/// Returns the primary sprite path for a concrete structure packed state.
pub fn structure_state_sprite_path(kind: StructureKind, state: PackedState) -> String {
    if is_pipe_structure(kind) {
        let mask = (state.value() & 0b1111) as u8;
        return pipe_mask_sprite_path(mask);
    }
    structure_sprite_path(kind).to_string()
}

/// Returns the sprite transform for a concrete structure packed state.
pub fn structure_state_sprite_transform(
    kind: StructureKind,
    state: PackedState,
) -> flux_plugin_sdk::EntitySpriteTransform {
    if is_gas_pipe_bridge_structure(kind) && (state.value() & 1) == 1 {
        return flux_plugin_sdk::EntitySpriteTransform::Rot90;
    }
    flux_plugin_sdk::EntitySpriteTransform::None
}

/// Decodes one legacy numeric world-cell code.
pub fn legacy_cell_kind_from_code(code: u8) -> Option<crate::world::grid::CellKind> {
    match code {
        0 => Some(crate::world::grid::CellKind::Empty),
        1 => Some(crate::world::grid::CellKind::Solid(boundary_cell_material())),
        2 => Some(crate::world::grid::CellKind::Solid(brick_cell_material())),
        3 => Some(crate::world::grid::CellKind::Solid(metal_cell_material())),
        _ => extra_cell_materials()
            .get(code.saturating_sub(4) as usize)
            .copied()
            .map(crate::world::grid::CellKind::Solid),
    }
}

fn extra_cell_label(material: CellMaterial) -> Option<&'static str> {
    const EXTRA_LABELS: [&str; 20] = [
        "Brick Alpha",
        "Brick Beta",
        "Brick Gamma",
        "Brick Delta",
        "Brick Epsilon",
        "Brick Zeta",
        "Brick Eta",
        "Brick Theta",
        "Brick Iota",
        "Brick Kappa",
        "Metal Alpha",
        "Metal Beta",
        "Metal Gamma",
        "Metal Delta",
        "Metal Epsilon",
        "Metal Zeta",
        "Metal Eta",
        "Metal Theta",
        "Metal Iota",
        "Metal Kappa",
    ];
    let index = extra_cell_materials()
        .iter()
        .position(|candidate| *candidate == material)?;
    EXTRA_LABELS.get(index).copied()
}

fn is_brick_family_cell(material: CellMaterial) -> bool {
    material == brick_cell_material()
        || matches!(
            material.as_str(),
            CELL_BRICK_01_ID
                | CELL_BRICK_02_ID
                | CELL_BRICK_03_ID
                | CELL_BRICK_04_ID
                | CELL_BRICK_05_ID
                | CELL_BRICK_06_ID
                | CELL_BRICK_07_ID
                | CELL_BRICK_08_ID
                | CELL_BRICK_09_ID
                | CELL_BRICK_10_ID
        )
}

fn is_metal_family_cell(material: CellMaterial) -> bool {
    material == metal_cell_material()
        || matches!(
            material.as_str(),
            CELL_METAL_01_ID
                | CELL_METAL_02_ID
                | CELL_METAL_03_ID
                | CELL_METAL_04_ID
                | CELL_METAL_05_ID
                | CELL_METAL_06_ID
                | CELL_METAL_07_ID
                | CELL_METAL_08_ID
                | CELL_METAL_09_ID
                | CELL_METAL_10_ID
        )
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
