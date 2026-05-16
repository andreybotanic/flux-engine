use std::path::{Path, PathBuf};

use crate::{
    render::OverlayMode,
    world::{
        grid::CellMaterial,
        structures::{LayerKind, LayerMarkerKind, StructureKind},
    },
};

/// Stable content id for the default boundary cell.
pub const CELL_BOUNDARY_ID: &str = "flux.default.cell.boundary";
/// Stable content id for the default brick cell.
pub const CELL_BRICK_ID: &str = "flux.default.cell.brick";
/// Stable content id for the default metal cell.
pub const CELL_METAL_ID: &str = "flux.default.cell.metal";
/// Stable content id for the default `Cells` entity category.
pub const CATEGORY_CELLS_ID: &str = "flux.default.category.cells";
/// Stable content id for the default `Gases` entity category.
pub const CATEGORY_GASES_ID: &str = "flux.default.category.gases";
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
/// Stable content id for the default gas pump entity.
pub const ENTITY_GAS_PUMP_ID: &str = "flux.default.entity.gas_pump";
/// Stable content id for the default main overlay mode.
pub const OVERLAY_MAIN_ID: &str = "flux.default.overlay.main";
/// Stable content id for the default gas overlay mode.
pub const OVERLAY_GAS_ID: &str = "flux.default.overlay.gas";
/// Stable content id for the default pipes overlay mode.
pub const OVERLAY_PIPES_ID: &str = "flux.default.overlay.pipes";
/// Stable image id for the generic white square used by overlay graphs.
pub const OVERLAY_IMAGE_WHITE_ID: &str = "flux.default.overlay.image.white";
/// Stable image id for the vent/bridge port overlay icon used by `F3`.
pub const OVERLAY_IMAGE_VENT_ICON_ID: &str = "flux.default.overlay.image.vent_icon";
/// Stable image id for the one-way pump input port icon used by `F3`.
pub const OVERLAY_IMAGE_GAS_IN_ICON_ID: &str = "flux.default.overlay.image.gas_in_icon";
/// Stable image id for the one-way pump output port icon used by `F3`.
pub const OVERLAY_IMAGE_GAS_OUT_ICON_ID: &str = "flux.default.overlay.image.gas_out_icon";
/// Stable material id for the pipe highlight shader used by `F3`.
pub const OVERLAY_MATERIAL_PIPE_HIGHLIGHT_ID: &str = "flux.default.overlay.material.pipe_highlight";
/// Stable substance id for default hydrogen gas.
pub const SUBSTANCE_H2_ID: &str = "flux.default.substance.h2";
/// Stable substance id for default oxygen gas.
pub const SUBSTANCE_O2_ID: &str = "flux.default.substance.o2";
/// Stable substance id for default carbon dioxide gas.
pub const SUBSTANCE_CO2_ID: &str = "flux.default.substance.co2";

const LAYER_GAS_PIPE_CONNECTIONS_ID: &str = "flux.default.layer.gas_pipe_connections";
const MARKER_GAS_PIPE_CONNECTION_BIDIRECTIONAL_ID: &str =
    "flux.default.marker.gas_pipe_connection_bidirectional";
const MARKER_GAS_PIPE_CONNECTION_IN_ID: &str = "flux.default.marker.gas_pipe_connection_in";
const MARKER_GAS_PIPE_CONNECTION_OUT_ID: &str = "flux.default.marker.gas_pipe_connection_out";
/// Bevy asset source name used for assets owned by the built-in default plugin.
pub const DEFAULT_PLUGIN_ASSET_SOURCE: &str = "flux_default";
/// Repository-relative root for assets owned by the built-in default plugin.
pub const DEFAULT_PLUGIN_ASSET_ROOT_RELATIVE: &str = "src/plugins/default_plugin/assets";
/// Repository-relative root for configs owned by the built-in default plugin.
pub const DEFAULT_PLUGIN_CONFIG_ROOT_RELATIVE: &str = "src/plugins/default_plugin/config";

/// Returns the repository-local asset root for the built-in default plugin.
pub fn asset_root(repo_root: &Path) -> PathBuf {
    repo_root.join(DEFAULT_PLUGIN_ASSET_ROOT_RELATIVE)
}

/// Returns the repository-local config root for the built-in default plugin.
pub fn config_root(repo_root: &Path) -> PathBuf {
    repo_root.join(DEFAULT_PLUGIN_CONFIG_ROOT_RELATIVE)
}

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

/// Returns the default plugin gas pump structure id wrapper.
pub const fn gas_pump_structure_kind() -> StructureKind {
    StructureKind::new(ENTITY_GAS_PUMP_ID)
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

/// Returns the marker used by default one-way `gas_in` pipe connection cells.
pub const fn gas_pipe_connection_in_marker() -> LayerMarkerKind {
    LayerMarkerKind::new(MARKER_GAS_PIPE_CONNECTION_IN_ID)
}

/// Returns the marker used by default one-way `gas_out` pipe connection cells.
pub const fn gas_pipe_connection_out_marker() -> LayerMarkerKind {
    LayerMarkerKind::new(MARKER_GAS_PIPE_CONNECTION_OUT_ID)
}
