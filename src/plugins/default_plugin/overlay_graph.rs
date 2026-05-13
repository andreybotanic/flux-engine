use bevy::prelude::{UVec2, Vec2, Vec3, Vec4};

use crate::{
    config::GasRegistry,
    plugins::default_plugin::{
        is_gas_pipe_bridge_structure,
        pipe_runtime::{
            pipe_cell_display_blocks_with_transfers, PipeContainerKind, PipeFlowVisualState,
            PipeGasField, PipeSimulationConfig, PipeTransferRecord, PipeTransferVisualPath,
        },
        OVERLAY_IMAGE_VENT_ICON_ID, OVERLAY_IMAGE_WHITE_ID, OVERLAY_MATERIAL_PIPE_HIGHLIGHT_ID,
    },
    world::{
        grid::{cell_center, world_origin, CELL_SIZE, WORLD_HEIGHT, WORLD_WIDTH},
        structures::{bridge_center_cell, PlacedStructureMap, StructureRotation},
    },
};

const NODE_BACKGROUND: &str = "background";
const NODE_DIM_WORLD: &str = "dim_world";
const NODE_BRIGHT_PIPES: &str = "bright_pipes";
const NODE_PIPE_HIGHLIGHT: &str = "pipe_highlight";
const NODE_PIPE_GAS: &str = "pipe_gas";
const NODE_FLOW_PACKETS: &str = "flow_packets";
const NODE_PORT_ICONS: &str = "port_icons";
const NODE_OUTPUT: &str = "compose";

pub(crate) fn build_pipes_overlay_graph(
    structures: &PlacedStructureMap,
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    pipe_config: &PipeSimulationConfig,
    gas_registry: &GasRegistry,
    flow_progress: f32,
    paused: bool,
) -> flux_plugin_sdk::OverlayGraph {
    let pipe_tag = content_tag("flux.default.tag.pipe-network");
    let background = overlay_node(
        NODE_BACKGROUND,
        Vec::new(),
        flux_plugin_sdk::OverlayNodeKind::RenderImage(flux_image_node(vec![full_world_tint(
            ColorRole::Background,
        )])),
    );
    let dim_world = overlay_node(
        NODE_DIM_WORLD,
        Vec::new(),
        flux_plugin_sdk::OverlayNodeKind::RenderEntities(flux_plugin_sdk::RenderEntitiesNode {
            selector: flux_plugin_sdk::Selector::not(flux_plugin_sdk::Selector::tag(
                pipe_tag.clone(),
            )),
            style: flux_plugin_sdk::OverlayEntityStyle {
                tint: Some(Vec4::new(1.0, 1.0, 1.0, 0.26)),
                sprite_override: None,
            },
        }),
    );
    let bright_pipes = overlay_node(
        NODE_BRIGHT_PIPES,
        Vec::new(),
        flux_plugin_sdk::OverlayNodeKind::RenderEntities(flux_plugin_sdk::RenderEntitiesNode {
            selector: flux_plugin_sdk::Selector::tag(pipe_tag),
            style: flux_plugin_sdk::OverlayEntityStyle {
                tint: Some(Vec4::new(0.98, 0.99, 1.0, 1.0)),
                sprite_override: None,
            },
        }),
    );
    let highlight = overlay_node(
        NODE_PIPE_HIGHLIGHT,
        vec![NODE_BRIGHT_PIPES],
        flux_plugin_sdk::OverlayNodeKind::Material(flux_plugin_sdk::MaterialNode {
            material_id: content_id(OVERLAY_MATERIAL_PIPE_HIGHLIGHT_ID),
            params: Vec::new(),
        }),
    );
    let pipe_gas_node = overlay_node(
        NODE_PIPE_GAS,
        Vec::new(),
        flux_plugin_sdk::OverlayNodeKind::RenderImage(flux_image_node(pipe_gas_instances(
            structures,
            pipe_gas,
            flow_state,
            pipe_config,
            gas_registry,
        ))),
    );
    let packets = overlay_node(
        NODE_FLOW_PACKETS,
        Vec::new(),
        flux_plugin_sdk::OverlayNodeKind::RenderImage(flux_image_node(flow_packet_instances(
            flow_state,
            pipe_config,
            gas_registry,
            flow_progress,
            paused,
        ))),
    );
    let icons = overlay_node(
        NODE_PORT_ICONS,
        Vec::new(),
        flux_plugin_sdk::OverlayNodeKind::RenderImage(flux_image_node(port_icon_instances(
            structures,
        ))),
    );
    let output = overlay_node(
        NODE_OUTPUT,
        vec![
            NODE_BACKGROUND,
            NODE_DIM_WORLD,
            NODE_BRIGHT_PIPES,
            NODE_PIPE_HIGHLIGHT,
            NODE_PIPE_GAS,
            NODE_FLOW_PACKETS,
            NODE_PORT_ICONS,
        ],
        flux_plugin_sdk::OverlayNodeKind::Blend(flux_plugin_sdk::BlendNode {
            mode: flux_plugin_sdk::OverlayBlendMode::AlphaOver,
        }),
    );

    flux_plugin_sdk::OverlayGraph {
        nodes: vec![
            background,
            dim_world,
            bright_pipes,
            highlight,
            pipe_gas_node,
            packets,
            icons,
            output,
        ],
        output: node_id(NODE_OUTPUT),
    }
}

fn pipe_gas_instances(
    structures: &PlacedStructureMap,
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    pipe_config: &PipeSimulationConfig,
    gas_registry: &GasRegistry,
) -> Vec<flux_plugin_sdk::OverlayImageInstance> {
    let mut instances = Vec::new();
    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            let blocks = pipe_cell_display_blocks_with_transfers(
                structures, pipe_gas, flow_state, x, y, false,
            );
            let total_slots = blocks.len().max(1);
            for (slot, block) in blocks.iter().enumerate() {
                if block.total_particles == 0 {
                    continue;
                }
                let outer_size =
                    pipe_overlay_slot_size(pipe_config, block.total_particles, total_slots);
                let inner_size = pipe_square_inner_size(outer_size);
                let offset = pipe_overlay_block_offset(
                    slot,
                    total_slots,
                    block.kind,
                    structures,
                    UVec2::new(x, y),
                );
                instances.push(cell_square_instance(x, y, offset, outer_size, Vec4::ONE));
                instances.push(cell_square_instance(
                    x,
                    y,
                    offset,
                    inner_size,
                    mixture_color(
                        block.total_particles,
                        &block.species_counts,
                        gas_registry,
                        0.92,
                    ),
                ));
            }
        }
    }
    instances
}

fn flow_packet_instances(
    flow_state: &PipeFlowVisualState,
    pipe_config: &PipeSimulationConfig,
    gas_registry: &GasRegistry,
    flow_progress: f32,
    paused: bool,
) -> Vec<flux_plugin_sdk::OverlayImageInstance> {
    if paused {
        return Vec::new();
    }
    let mut instances = Vec::new();
    for transfer in &flow_state.transfers {
        if transfer.total_amount < 5 {
            continue;
        }
        let outer_size = pipe_flow_square_size(pipe_config, transfer.total_amount);
        let inner_size = pipe_square_inner_size(outer_size);
        let position = flow_packet_position(transfer, flow_progress);
        let position_in_grid = world_position_to_grid_local(position);
        instances.push(grid_square_instance(
            position_in_grid,
            outer_size,
            Vec4::ONE,
        ));
        instances.push(grid_square_instance(
            position_in_grid,
            inner_size,
            mixture_color(
                transfer.total_amount,
                &transfer.gas_counts,
                gas_registry,
                0.84,
            ),
        ));
    }
    instances
}

fn port_icon_instances(
    structures: &PlacedStructureMap,
) -> Vec<flux_plugin_sdk::OverlayImageInstance> {
    let mut instances = Vec::new();
    for structure in structures.iter() {
        if crate::plugins::default_plugin::is_vent_structure(structure.kind) {
            instances.push(port_icon_at(structure.origin));
            continue;
        }
        if !is_gas_pipe_bridge_structure(structure.kind) {
            continue;
        }
        for cell in bridge_connection_cells(structure.origin, structure.rotation) {
            instances.push(port_icon_at(cell));
        }
    }
    instances
}

fn full_world_tint(role: ColorRole) -> flux_plugin_sdk::OverlayImageInstance {
    let tint = match role {
        ColorRole::Background => Vec4::new(0.0, 0.0, 0.0, 0.74),
    };
    flux_plugin_sdk::OverlayImageInstance {
        image: flux_plugin_sdk::OverlayImageSource::Asset(content_id(OVERLAY_IMAGE_WHITE_ID)),
        placement: flux_plugin_sdk::OverlayPlacement::GridLocal {
            position_in_grid: Vec2::ZERO,
            size_in_grid: Vec2::new(WORLD_WIDTH as f32, WORLD_HEIGHT as f32),
            rotation: 0.0,
            origin: Vec2::ZERO,
        },
        tint,
    }
}

fn cell_square_instance(
    x: u32,
    y: u32,
    offset_world: Vec2,
    size_world: f32,
    tint: Vec4,
) -> flux_plugin_sdk::OverlayImageInstance {
    flux_plugin_sdk::OverlayImageInstance {
        image: flux_plugin_sdk::OverlayImageSource::Asset(content_id(OVERLAY_IMAGE_WHITE_ID)),
        placement: flux_plugin_sdk::OverlayPlacement::CellLocal {
            cell: UVec2::new(x, y),
            anchor: Vec2::splat(0.5),
            offset_in_cell: offset_world / CELL_SIZE,
            size_in_cell: Vec2::splat(size_world / CELL_SIZE),
            rotation: 0.0,
        },
        tint,
    }
}

fn grid_square_instance(
    position_in_grid: Vec2,
    size_world: f32,
    tint: Vec4,
) -> flux_plugin_sdk::OverlayImageInstance {
    let size_in_grid = Vec2::splat(size_world / CELL_SIZE);
    flux_plugin_sdk::OverlayImageInstance {
        image: flux_plugin_sdk::OverlayImageSource::Asset(content_id(OVERLAY_IMAGE_WHITE_ID)),
        placement: flux_plugin_sdk::OverlayPlacement::GridLocal {
            position_in_grid,
            size_in_grid,
            rotation: 0.0,
            origin: Vec2::splat(0.5),
        },
        tint,
    }
}

fn world_position_to_grid_local(position: Vec2) -> Vec2 {
    (position - world_origin()) / CELL_SIZE
}

fn port_icon_at(cell: UVec2) -> flux_plugin_sdk::OverlayImageInstance {
    flux_plugin_sdk::OverlayImageInstance {
        image: flux_plugin_sdk::OverlayImageSource::Asset(content_id(OVERLAY_IMAGE_VENT_ICON_ID)),
        placement: flux_plugin_sdk::OverlayPlacement::CellLocal {
            cell,
            anchor: Vec2::splat(0.5),
            offset_in_cell: Vec2::ZERO,
            size_in_cell: Vec2::splat(0.92),
            rotation: 0.0,
        },
        tint: Vec4::ONE,
    }
}

fn mixture_color(
    total_particles: u32,
    species_counts: &[u32],
    gas_registry: &GasRegistry,
    alpha: f32,
) -> Vec4 {
    let total = total_particles.max(1) as f32;
    let mut weighted = Vec3::ZERO;
    for (gas_index, amount) in species_counts.iter().copied().enumerate() {
        if amount == 0 {
            continue;
        }
        if let Some(gas_def) = gas_registry.get(gas_index) {
            weighted += Vec3::from_array(gas_def.color) * amount as f32;
        }
    }
    let rgb = if weighted.length_squared() <= f32::EPSILON {
        Vec3::splat(0.9)
    } else {
        weighted / total
    };
    Vec4::new(rgb.x, rgb.y, rgb.z, alpha)
}

fn flow_packet_position(transfer: &PipeTransferRecord, t: f32) -> Vec2 {
    bridge_packet_position(transfer, t).unwrap_or_else(|| {
        cell_center(transfer.from.x, transfer.from.y)
            .lerp(cell_center(transfer.to.x, transfer.to.y), t)
    })
}

fn bridge_packet_position(transfer: &PipeTransferRecord, t: f32) -> Option<Vec2> {
    let PipeTransferVisualPath::BridgeArc {
        bridge_origin,
        bridge_rotation,
    } = transfer.visual_path
    else {
        return None;
    };
    let center = bridge_center_cell(bridge_origin, bridge_rotation)?;
    let [first_port, second_port] = bridge_connection_cells(bridge_origin, bridge_rotation);
    let curve_t = if transfer.from == first_port && transfer.to == center {
        t * 0.5
    } else if transfer.from == center && transfer.to == first_port {
        (1.0 - t) * 0.5
    } else if transfer.from == center && transfer.to == second_port {
        0.5 + t * 0.5
    } else if transfer.from == second_port && transfer.to == center {
        1.0 - t * 0.5
    } else {
        return None;
    };
    let bend = bridge_bend_direction(bridge_rotation) * (CELL_SIZE * 0.45);
    Some(quadratic_bezier_point(
        cell_center(first_port.x, first_port.y),
        cell_center(center.x, center.y) + bend,
        cell_center(second_port.x, second_port.y),
        curve_t,
    ))
}

fn quadratic_bezier_point(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
    let omt = 1.0 - t;
    p0 * omt * omt + p1 * 2.0 * omt * t + p2 * t * t
}

fn bridge_connection_cells(origin: UVec2, rotation: StructureRotation) -> [UVec2; 2] {
    match rotation {
        StructureRotation::Deg0 | StructureRotation::Deg180 => {
            [origin, UVec2::new(origin.x + 2, origin.y)]
        }
        StructureRotation::Deg90 | StructureRotation::Deg270 => {
            [origin, UVec2::new(origin.x, origin.y + 2)]
        }
    }
}

fn bridge_bend_direction(rotation: StructureRotation) -> Vec2 {
    match rotation {
        StructureRotation::Deg0 => Vec2::Y,
        StructureRotation::Deg90 => Vec2::NEG_X,
        StructureRotation::Deg180 => Vec2::NEG_Y,
        StructureRotation::Deg270 => Vec2::X,
    }
}

fn pipe_overlay_block_offset(
    slot: usize,
    total_slots: usize,
    kind: PipeContainerKind,
    structures: &PlacedStructureMap,
    cell: UVec2,
) -> Vec2 {
    let bridge_offset = structures
        .iter()
        .find_map(|structure| {
            (is_gas_pipe_bridge_structure(structure.kind)
                && bridge_center_cell(structure.origin, structure.rotation) == Some(cell))
            .then_some(bridge_bend_direction(structure.rotation) * (CELL_SIZE * 0.18))
        })
        .unwrap_or_else(|| pipe_overlay_slot_offset(slot, total_slots));
    match kind {
        PipeContainerKind::BridgePipe => bridge_offset,
        PipeContainerKind::Pipe if total_slots > 1 => -bridge_offset,
        PipeContainerKind::Pipe => Vec2::ZERO,
    }
}

fn pipe_overlay_slot_offset(slot: usize, total_slots: usize) -> Vec2 {
    if total_slots <= 1 {
        Vec2::ZERO
    } else if slot == 0 {
        Vec2::new(0.0, CELL_SIZE * 0.18)
    } else {
        Vec2::new(0.0, -CELL_SIZE * 0.18)
    }
}

fn pipe_overlay_slot_size(
    config: &PipeSimulationConfig,
    total_particles: u32,
    total_slots: usize,
) -> f32 {
    let base = pipe_gas_square_size(config, total_particles);
    if total_slots <= 1 {
        base
    } else {
        (base * 0.62).max(CELL_SIZE * 0.18)
    }
}

fn pipe_gas_square_size(config: &PipeSimulationConfig, total_particles: u32) -> f32 {
    scaled_pipe_square_size(config, total_particles, 0.22, 0.63)
}

fn pipe_flow_square_size(config: &PipeSimulationConfig, total_particles: u32) -> f32 {
    scaled_pipe_square_size(config, total_particles, 0.21, 0.504)
}

fn scaled_pipe_square_size(
    config: &PipeSimulationConfig,
    particles: u32,
    min_fraction: f32,
    max_fraction: f32,
) -> f32 {
    if particles == 0 {
        return 0.0;
    }
    let pressure =
        crate::plugins::default_plugin::pipe_runtime::pressure::pipe_pressure_pa(config, particles);
    let reference =
        crate::plugins::default_plugin::pipe_runtime::pressure::pipe_pressure_pa(config, 1_000)
            .max(1.0);
    let fill_ratio = ((pressure / (pressure + reference)).clamp(0.0, 1.0)).sqrt();
    CELL_SIZE * (min_fraction + (max_fraction - min_fraction) * fill_ratio)
}

fn pipe_square_inner_size(outer_size: f32) -> f32 {
    (outer_size - (CELL_SIZE / 64.0) * 2.0).max(0.0)
}

fn overlay_node(
    id: &str,
    depends_on: Vec<&str>,
    kind: flux_plugin_sdk::OverlayNodeKind,
) -> flux_plugin_sdk::OverlayNode {
    flux_plugin_sdk::OverlayNode {
        id: node_id(id),
        depends_on: depends_on.into_iter().map(node_id).collect(),
        kind,
    }
}

fn flux_image_node(
    instances: Vec<flux_plugin_sdk::OverlayImageInstance>,
) -> flux_plugin_sdk::RenderImageNode {
    flux_plugin_sdk::RenderImageNode { instances }
}

fn node_id(raw: &str) -> flux_plugin_sdk::OverlayNodeId {
    flux_plugin_sdk::OverlayNodeId::parse(raw).expect("default overlay node ids must stay valid")
}

fn content_id(raw: &str) -> flux_plugin_sdk::ContentId {
    flux_plugin_sdk::ContentId::parse(raw).expect("default overlay content ids must stay valid")
}

fn content_tag(raw: &str) -> flux_plugin_sdk::ContentTag {
    flux_plugin_sdk::ContentTag::parse(raw).expect("default overlay tags must stay valid")
}

enum ColorRole {
    Background,
}

#[cfg(test)]
mod tests {
    use super::{build_pipes_overlay_graph, flow_packet_instances, world_position_to_grid_local};
    use crate::{
        config::{GasDefinition, GasRegistry},
        plugins::default_plugin::pipe_runtime::{
            PipeFlowVisualState, PipeGasField, PipeSimulationConfig, PipeTransferRecord,
            PipeTransferVisualPath,
        },
        world::{grid::cell_center, structures::PlacedStructureMap},
    };
    use bevy::math::{UVec2, Vec2};

    fn test_registry() -> GasRegistry {
        GasRegistry::new(vec![GasDefinition {
            id: "h2".to_string(),
            label: "Hydrogen".to_string(),
            molecular_mass: 2.016,
            color: [0.7, 0.8, 1.0],
        }])
        .expect("valid gas registry")
    }

    #[test]
    fn world_position_to_grid_local_matches_cell_center_coordinates() {
        let cell = UVec2::new(10, 12);
        let position = cell_center(cell.x, cell.y);
        let grid = world_position_to_grid_local(position);
        let expected = Vec2::new(cell.x as f32 + 0.5, cell.y as f32 + 0.5);
        assert!((grid - expected).length() < 1e-4);
    }

    #[test]
    fn flow_packets_use_grid_local_coordinates_inside_world_bounds() {
        let flow_state = PipeFlowVisualState {
            transfers: vec![PipeTransferRecord {
                from: UVec2::new(3, 4),
                to: UVec2::new(4, 4),
                gas_counts: vec![8],
                total_amount: 8,
                visual_path: PipeTransferVisualPath::Straight,
            }],
        };
        let instances = flow_packet_instances(
            &flow_state,
            &PipeSimulationConfig::default(),
            &test_registry(),
            0.5,
            false,
        );
        assert_eq!(instances.len(), 2);
        for instance in instances {
            let flux_plugin_sdk::OverlayPlacement::GridLocal {
                position_in_grid, ..
            } = instance.placement
            else {
                panic!("flow packets must be rendered in grid-local space");
            };
            assert!(
                position_in_grid.x >= 0.0
                    && position_in_grid.x <= crate::world::grid::WORLD_WIDTH as f32
            );
            assert!(
                position_in_grid.y >= 0.0
                    && position_in_grid.y <= crate::world::grid::WORLD_HEIGHT as f32
            );
        }
    }

    #[test]
    fn pipes_overlay_graph_is_valid_while_paused_without_packets() {
        let registry = test_registry();
        let structures = PlacedStructureMap::default();
        let pipe_gas = PipeGasField::from_registry(&registry);
        let flow_state = PipeFlowVisualState::default();
        let graph = build_pipes_overlay_graph(
            &structures,
            &pipe_gas,
            &flow_state,
            &PipeSimulationConfig::default(),
            &registry,
            0.0,
            true,
        );
        graph
            .validate()
            .expect("paused overlay graph should stay valid even with empty packet/image layers");
    }
}
