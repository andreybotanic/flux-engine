use super::{
    apply_pipe_network_step, bridge_port_cells, offer_based_intake_particles_for_test,
    pipe_cell_display_blocks_with_transfers, PipeCellDisplayBlock, PipeContainerKind,
    PipeFlowVisualState, PipeFluxField, PipeGasField, PipeSimulationConfig, PipeTransferRecord,
    PipeTransferVisualPath,
};
use crate::{
    config::{GasDefinition, GasRegistry},
    simulation::gas::GasField,
    world::{
        grid::{WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
        structures::{PlacedStructureMap, StructureRotation},
    },
};
use bevy::prelude::*;

fn registry() -> GasRegistry {
    GasRegistry::new(vec![
        GasDefinition {
            id: "h2".to_string(),
            label: "Hydrogen".to_string(),
            color: [0.7, 0.8, 1.0],
            molecular_mass: 2.016,
        },
        GasDefinition {
            id: "o2".to_string(),
            label: "Oxygen".to_string(),
            color: [0.5, 0.8, 1.0],
            molecular_mass: 31.998,
        },
        GasDefinition {
            id: "co2".to_string(),
            label: "Carbon Dioxide".to_string(),
            color: [0.8, 0.8, 0.8],
            molecular_mass: 44.009,
        },
    ])
    .expect("test registry")
}

fn pipe_config() -> PipeSimulationConfig {
    PipeSimulationConfig::default()
}

fn pipe_node_id(pipe_gas: &PipeGasField, kind: PipeContainerKind, cell: UVec2) -> usize {
    pipe_gas
        .snapshot_state()
        .nodes
        .iter()
        .position(|node| node.key.kind == kind && node.key.anchor == cell)
        .expect("node id")
}

fn pipe_node_total(pipe_gas: &PipeGasField, kind: PipeContainerKind, cell: UVec2) -> u32 {
    let node = pipe_node_id(pipe_gas, kind, cell);
    pipe_gas.total_amount_particles(node)
}

fn run_pipe_ticks(
    steps: usize,
    world: &WorldGrid,
    structures: &PlacedStructureMap,
    gas: &mut GasField,
    pipe_gas: &mut PipeGasField,
    pipe_flux: &mut PipeFluxField,
    visuals: &mut PipeFlowVisualState,
    config: &PipeSimulationConfig,
) {
    for _ in 0..steps {
        let _ =
            apply_pipe_network_step(structures, pipe_gas, pipe_flux, gas, world, visuals, config);
    }
}

fn run_pipe_ticks_with_observer(
    steps: usize,
    world: &WorldGrid,
    structures: &PlacedStructureMap,
    gas: &mut GasField,
    pipe_gas: &mut PipeGasField,
    pipe_flux: &mut PipeFluxField,
    visuals: &mut PipeFlowVisualState,
    config: &PipeSimulationConfig,
    mut on_tick: impl FnMut(&PipeFlowVisualState),
) {
    for _ in 0..steps {
        let _ =
            apply_pipe_network_step(structures, pipe_gas, pipe_flux, gas, world, visuals, config);
        on_tick(visuals);
    }
}

fn total_world_mass(gas: &GasField, gas_index: usize) -> u64 {
    let mut total = 0u64;
    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            total = total.saturating_add(u64::from(gas.amount_particles(x, y, gas_index)));
        }
    }
    total
}

fn total_pipe_mass(pipe_gas: &PipeGasField, gas_index: usize) -> u64 {
    (0..pipe_gas.node_count())
        .map(|node_id| u64::from(pipe_gas.amount_particles(node_id, gas_index)))
        .sum()
}

fn set_area_pressure(
    gas: &mut GasField,
    gas_index: usize,
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    pressure_pa: f32,
    config: &PipeSimulationConfig,
) {
    let particles = pressure_pa / config.cell_particle_pressure_pa.max(1e-6);
    for y in top..=bottom {
        for x in left..=right {
            gas.set_amount(x, y, gas_index, particles);
        }
    }
}

fn sum_area_particles(
    gas: &GasField,
    gas_index: usize,
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
) -> u64 {
    let mut total = 0u64;
    for y in top..=bottom {
        for x in left..=right {
            total = total.saturating_add(u64::from(gas.amount_particles(x, y, gas_index)));
        }
    }
    total
}

fn build_multi_vent_stress_network(
    pressure_pa: f32,
    config: &PipeSimulationConfig,
) -> (
    WorldGrid,
    PlacedStructureMap,
    GasField,
    PipeGasField,
    PipeFluxField,
    PipeFlowVisualState,
) {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();

    let mut ensure_pipe = |x: u32, y: u32| {
        if !structures.has_pipe_at(x, y) {
            assert!(structures.place_pipe(x, y, &world));
        }
    };

    for x in 20..=80 {
        ensure_pipe(x, 50);
    }
    for y in 30..=70 {
        ensure_pipe(50, y);
    }
    for x in 35..=65 {
        ensure_pipe(x, 40);
        ensure_pipe(x, 60);
    }
    for y in 40..=60 {
        ensure_pipe(35, y);
        ensure_pipe(65, y);
    }
    for x in 81..=90 {
        ensure_pipe(x, 50);
    }

    let vent_cells = [
        UVec2::new(20, 50),
        UVec2::new(80, 50),
        UVec2::new(50, 30),
        UVec2::new(50, 70),
        UVec2::new(35, 50),
        UVec2::new(65, 50),
    ];
    for cell in vent_cells {
        assert!(structures.place_vent(cell.x, cell.y, &world));
    }

    let mut gas = GasField::from_registry(&registry);
    let vent_pressures = [
        (UVec2::new(20, 50), pressure_pa),
        (UVec2::new(80, 50), pressure_pa * 0.1),
        (UVec2::new(50, 30), pressure_pa * 0.6),
        (UVec2::new(50, 70), pressure_pa * 0.2),
        (UVec2::new(35, 50), pressure_pa * 0.8),
        (UVec2::new(65, 50), pressure_pa * 0.3),
    ];
    let world = world;
    for (cell, local_pressure) in vent_pressures {
        set_area_pressure(
            &mut gas,
            gas_index,
            cell.x,
            cell.y,
            cell.x,
            cell.y,
            local_pressure,
            config,
        );
    }
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let pipe_particle_pressure =
        (config.cell_particle_pressure_pa * config.cell_volume_ratio).max(1e-6);
    for (cell, local_pressure) in vent_pressures {
        let seed_particles = (local_pressure / pipe_particle_pressure).ceil() as u32;
        if seed_particles == 0 {
            continue;
        }
        let node_id = pipe_node_id(&pipe_gas, PipeContainerKind::Pipe, cell);
        pipe_gas.add_species_counts(node_id, &[seed_particles, 0, 0]);
    }

    (
        world,
        structures,
        gas,
        pipe_gas,
        PipeFluxField::default(),
        PipeFlowVisualState::default(),
    )
}

fn has_transfer_between(visuals: &PipeFlowVisualState, a: UVec2, b: UVec2) -> bool {
    visuals.transfers.iter().any(|transfer| {
        transfer.total_amount > 0
            && ((transfer.from == a && transfer.to == b)
                || (transfer.from == b && transfer.to == a))
    })
}

fn has_transfer_touching_cell(visuals: &PipeFlowVisualState, cell: UVec2) -> bool {
    visuals
        .transfers
        .iter()
        .any(|transfer| transfer.total_amount > 0 && (transfer.from == cell || transfer.to == cell))
}

fn has_directional_transfer(visuals: &PipeFlowVisualState, from: UVec2, to: UVec2) -> bool {
    visuals
        .transfers
        .iter()
        .any(|transfer| transfer.total_amount > 0 && transfer.from == from && transfer.to == to)
}

#[test]
fn bridge_placement_exposes_two_port_cells() {
    assert_eq!(
        bridge_port_cells(UVec2::new(10, 12), StructureRotation::Deg0),
        vec![UVec2::new(10, 12), UVec2::new(12, 12)]
    );
    assert_eq!(
        bridge_port_cells(UVec2::new(10, 12), StructureRotation::Deg90),
        vec![UVec2::new(10, 12), UVec2::new(10, 14)]
    );
}

#[test]
fn cell_with_bridge_and_pipe_returns_two_display_blocks() {
    let registry = registry();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    assert!(structures.place_pipe(20, 20, &world));
    assert!(structures.place_pipe(21, 20, &world));
    assert!(structures.place_pipe(22, 20, &world));
    assert!(structures
        .place_bridge(UVec2::new(20, 20), StructureRotation::Deg0, &world)
        .is_some());

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let snapshot = pipe_gas.snapshot_state();
    let pipe_node = snapshot
        .nodes
        .iter()
        .position(|node| {
            node.key.kind == PipeContainerKind::Pipe && node.key.anchor == UVec2::new(21, 20)
        })
        .expect("pipe node");
    let bridge_node = snapshot
        .nodes
        .iter()
        .position(|node| node.key.kind == PipeContainerKind::BridgePipe)
        .expect("bridge node");
    pipe_gas.add_species_counts(pipe_node, &[10, 0, 0]);
    pipe_gas.add_species_counts(bridge_node, &[0, 6, 0]);

    let blocks = pipe_cell_display_blocks_with_transfers(
        &structures,
        &pipe_gas,
        &PipeFlowVisualState::default(),
        21,
        20,
        true,
    );
    assert_eq!(
        blocks,
        vec![
            PipeCellDisplayBlock {
                kind: PipeContainerKind::BridgePipe,
                species_counts: vec![0, 6, 0],
                total_particles: 6,
            },
            PipeCellDisplayBlock {
                kind: PipeContainerKind::Pipe,
                species_counts: vec![10, 0, 0],
                total_particles: 10,
            },
        ]
    );
}

#[test]
fn pipe_snapshot_restore_accepts_amounts_far_above_legacy_capacity() {
    let registry = registry();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    assert!(structures.place_pipe(8, 8, &world));

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    pipe_gas.add_species_counts(0, &[25_000, 7_500, 0]);

    let snapshot = pipe_gas.snapshot_state();
    let mut restored = PipeGasField::from_registry(&registry);
    restored
        .restore_state(&snapshot, &structures)
        .expect("restore without capacity clamp");

    assert_eq!(restored.total_amount_particles(0), 32_500);
    assert_eq!(restored.amount_particles(0, 0), 25_000);
    assert_eq!(restored.amount_particles(0, 1), 7_500);
}

#[test]
fn blocked_branch_without_downstream_demand_stops_all_incoming_flow() {
    let registry = registry();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    for x in 10..=12 {
        assert!(structures.place_pipe(x, 10, &world));
    }

    let mut gas = GasField::from_registry(&registry);
    let mut pipe_gas = PipeGasField::from_registry(&registry);
    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();
    let config = pipe_config();

    pipe_gas.sync_to_structures(&structures);
    let source = pipe_node_id(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(10, 10));
    pipe_gas.add_species_counts(source, &[1_000, 0, 0]);

    run_pipe_ticks(
        (config.pipe_step_interval_ticks as usize) * 3,
        &world,
        &structures,
        &mut gas,
        &mut pipe_gas,
        &mut pipe_flux,
        &mut visuals,
        &config,
    );

    assert_eq!(
        pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(10, 10)),
        1_000
    );
    assert_eq!(
        pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(11, 10)),
        0
    );
    assert_eq!(
        pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(12, 10)),
        0
    );
    assert!(visuals.transfers.is_empty());
}

#[test]
fn blocked_branch_with_alternative_sink_keeps_upstream_flowing() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();

    for cell in [
        UVec2::new(10, 10),
        UVec2::new(11, 10),
        UVec2::new(12, 10),
        UVec2::new(11, 11),
        UVec2::new(11, 12),
    ] {
        assert!(structures.place_pipe(cell.x, cell.y, &world));
    }
    assert!(structures.place_vent(11, 12, &world));

    let mut gas = GasField::from_registry(&registry);
    let config = pipe_config();
    set_area_pressure(&mut gas, gas_index, 9, 11, 13, 14, 0.0, &config);
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let source = pipe_node_id(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(10, 10));
    pipe_gas.add_species_counts(source, &[2_000, 0, 0]);

    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();
    let mut saw_source_to_junction = false;
    let mut saw_junction_to_sink_branch = false;

    run_pipe_ticks_with_observer(
        (config.pipe_step_interval_ticks as usize) * 8,
        &world,
        &structures,
        &mut gas,
        &mut pipe_gas,
        &mut pipe_flux,
        &mut visuals,
        &config,
        |state| {
            saw_source_to_junction |=
                has_transfer_between(state, UVec2::new(10, 10), UVec2::new(11, 10));
            saw_junction_to_sink_branch |=
                has_transfer_between(state, UVec2::new(11, 10), UVec2::new(11, 11));
        },
    );

    assert!(
        saw_source_to_junction,
        "expected upstream branch to keep moving"
    );
    assert!(
        saw_junction_to_sink_branch,
        "expected branch with outlet vent to receive flow"
    );
    assert_eq!(
        pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(12, 10)),
        0,
        "dead-end branch should stay blocked"
    );
}

#[test]
fn segment_keeps_single_direction_inside_hop() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    for cell in [
        UVec2::new(20, 20),
        UVec2::new(21, 20),
        UVec2::new(22, 20),
        UVec2::new(23, 20),
        UVec2::new(22, 21),
        UVec2::new(22, 22),
    ] {
        assert!(structures.place_pipe(cell.x, cell.y, &world));
    }
    assert!(structures.place_vent(20, 20, &world));
    assert!(structures.place_vent(23, 20, &world));
    assert!(structures.place_vent(22, 20, &world));
    assert!(structures.place_vent(22, 22, &world));

    let mut gas = GasField::from_registry(&registry);
    let config = pipe_config();
    set_area_pressure(&mut gas, gas_index, 20, 20, 20, 20, 1_000.0, &config);
    set_area_pressure(&mut gas, gas_index, 23, 20, 23, 20, 0.0, &config);
    set_area_pressure(&mut gas, gas_index, 22, 22, 22, 22, 0.0, &config);
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let source = pipe_node_id(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(20, 20));
    pipe_gas.add_species_counts(source, &[4_000, 0, 0]);

    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();
    let mut saw_segment_flow = false;
    let mut saw_mixed_directions = false;

    run_pipe_ticks_with_observer(
        (config.pipe_step_interval_ticks as usize) * 8,
        &world,
        &structures,
        &mut gas,
        &mut pipe_gas,
        &mut pipe_flux,
        &mut visuals,
        &config,
        |state| {
            let mut saw_forward = false;
            let mut saw_backward = false;
            for transfer in state.transfers.iter().filter(|transfer| {
                matches!(
                    (transfer.from, transfer.to),
                    (UVec2 { x: 20, y: 20 }, UVec2 { x: 21, y: 20 })
                        | (UVec2 { x: 21, y: 20 }, UVec2 { x: 20, y: 20 })
                        | (UVec2 { x: 21, y: 20 }, UVec2 { x: 22, y: 20 })
                        | (UVec2 { x: 22, y: 20 }, UVec2 { x: 21, y: 20 })
                )
            }) {
                if transfer.total_amount == 0 {
                    continue;
                }
                saw_segment_flow = true;
                if transfer.to.x > transfer.from.x {
                    saw_forward = true;
                } else {
                    saw_backward = true;
                }
            }
            if saw_forward && saw_backward {
                saw_mixed_directions = true;
            }
        },
    );

    assert!(saw_segment_flow, "expected segment transfers to appear");
    assert!(
        !saw_mixed_directions,
        "segment must not contain opposite transfer directions inside one hop"
    );
}

#[test]
fn non_boundary_cells_shift_full_mass_to_next_cell_on_next_hop() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    let y = 18u32;
    let start_x = 40u32;
    let end_x = 46u32;

    for x in start_x..=end_x {
        assert!(structures.place_pipe(x, y, &world));
    }
    assert!(structures.place_vent(start_x, y, &world));
    assert!(structures.place_vent(end_x, y, &world));

    let mut config = pipe_config();
    config.max_vent_flux_particles_per_tick = 0.0;

    let mut gas = GasField::from_registry(&registry);
    set_area_pressure(
        &mut gas, gas_index, start_x, y, start_x, y, 100_000.0, &config,
    );
    set_area_pressure(&mut gas, gas_index, end_x, y, end_x, y, 0.0, &config);
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let seeded = [30u32, 18u32, 11u32, 7u32, 3u32];
    for (offset, amount) in seeded.into_iter().enumerate() {
        let node = pipe_node_id(
            &pipe_gas,
            PipeContainerKind::Pipe,
            UVec2::new(start_x + offset as u32, y),
        );
        pipe_gas.add_species_counts(node, &[amount, 0, 0]);
    }

    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();
    let _ = apply_pipe_network_step(
        &structures,
        &mut pipe_gas,
        &mut pipe_flux,
        &mut gas,
        &world,
        &mut visuals,
        &config,
    );
    assert!(
        visuals
            .transfers
            .iter()
            .filter(|transfer| transfer.total_amount > 0)
            .all(
                |transfer| transfer.to.x == transfer.from.x + 1 && transfer.to.y == transfer.from.y
            ),
        "expected all planned conveyor transfers to move strictly in +X direction"
    );

    let prev = (start_x..=end_x)
        .map(|x| pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(x, y)))
        .collect::<Vec<_>>();

    run_pipe_ticks(
        config.pipe_step_interval_ticks as usize,
        &world,
        &structures,
        &mut gas,
        &mut pipe_gas,
        &mut pipe_flux,
        &mut visuals,
        &config,
    );

    let next = (start_x..=end_x)
        .map(|x| pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(x, y)))
        .collect::<Vec<_>>();
    for index in 1..(next.len() - 1) {
        assert_eq!(
            next[index],
            prev[index - 1],
            "cell shift mismatch at index={index}: expected full mass transfer from previous cell"
        );
    }
}

#[test]
fn flow_resumes_after_downstream_unblock() {
    let registry = registry();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    for x in 30..=32 {
        assert!(structures.place_pipe(x, 15, &world));
    }

    let mut gas = GasField::from_registry(&registry);
    let mut pipe_gas = PipeGasField::from_registry(&registry);
    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();
    let config = pipe_config();

    pipe_gas.sync_to_structures(&structures);
    let source = pipe_node_id(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(30, 15));
    pipe_gas.add_species_counts(source, &[1_500, 0, 0]);

    run_pipe_ticks(
        (config.pipe_step_interval_ticks as usize) * 3,
        &world,
        &structures,
        &mut gas,
        &mut pipe_gas,
        &mut pipe_flux,
        &mut visuals,
        &config,
    );
    assert_eq!(
        pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(30, 15)),
        1_500
    );

    assert!(structures.place_vent(32, 15, &world));
    run_pipe_ticks(
        (config.pipe_step_interval_ticks as usize) * 6,
        &world,
        &structures,
        &mut gas,
        &mut pipe_gas,
        &mut pipe_flux,
        &mut visuals,
        &config,
    );

    assert!(
        pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(30, 15)) < 1_500,
        "upstream node should start draining after outlet appears"
    );
}

#[test]
fn room_to_empty_room_pipe_front_fills_without_hop_gaps() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let config = pipe_config();
    let mut world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    let y = 25u32;
    let start_x = 7u32;
    let end_x = 25u32;

    let source_room_min = UVec2::new(6, 24);
    let source_room_max = UVec2::new(8, 26);
    let source_walls_min = UVec2::new(5, 23);
    let source_walls_max = UVec2::new(9, 27);
    let sink_room_min = UVec2::new(24, 24);
    let sink_room_max = UVec2::new(26, 26);
    let sink_walls_min = UVec2::new(23, 23);
    let sink_walls_max = UVec2::new(27, 27);

    for x in start_x..=end_x {
        assert!(structures.place_pipe(x, y, &world));
    }
    assert!(structures.place_vent(start_x, y, &world));
    assert!(structures.place_vent(end_x, y, &world));

    for y_wall in source_walls_min.y..=source_walls_max.y {
        for x_wall in source_walls_min.x..=source_walls_max.x {
            let is_interior = x_wall >= source_room_min.x
                && x_wall <= source_room_max.x
                && y_wall >= source_room_min.y
                && y_wall <= source_room_max.y;
            if is_interior {
                continue;
            }
            let _ = world.set_solid_with_material(
                x_wall,
                y_wall,
                crate::plugins::default_plugin::brick_cell_material(),
            );
        }
    }
    for y_wall in sink_walls_min.y..=sink_walls_max.y {
        for x_wall in sink_walls_min.x..=sink_walls_max.x {
            let is_interior = x_wall >= sink_room_min.x
                && x_wall <= sink_room_max.x
                && y_wall >= sink_room_min.y
                && y_wall <= sink_room_max.y;
            if is_interior {
                continue;
            }
            let _ = world.set_solid_with_material(
                x_wall,
                y_wall,
                crate::plugins::default_plugin::brick_cell_material(),
            );
        }
    }

    let mut gas = GasField::from_registry(&registry);
    set_area_pressure(
        &mut gas,
        gas_index,
        source_room_min.x,
        source_room_min.y,
        source_room_max.x,
        source_room_max.y,
        100_000.0,
        &config,
    );
    set_area_pressure(
        &mut gas,
        gas_index,
        sink_room_min.x,
        sink_room_min.y,
        sink_room_max.x,
        sink_room_max.y,
        0.0,
        &config,
    );
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();

    let observed_hops = 6usize;
    let ticks_to_run = observed_hops * config.pipe_step_interval_ticks as usize;
    let mut front_lengths = Vec::new();
    let mut source_totals = Vec::new();

    for _ in 0..ticks_to_run {
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut pipe_flux,
            &mut gas,
            &world,
            &mut visuals,
            &config,
        );

        if visuals.flow_progress().abs() > f32::EPSILON {
            continue;
        }

        let front_len = (start_x..=end_x)
            .take_while(|x| {
                pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(*x, y)) > 0
            })
            .count();
        front_lengths.push(front_len);
        source_totals.push(pipe_node_total(
            &pipe_gas,
            PipeContainerKind::Pipe,
            UVec2::new(start_x, y),
        ));

        if front_lengths.len() == observed_hops {
            break;
        }
    }

    assert_eq!(
        front_lengths.len(),
        observed_hops,
        "expected exactly one front sample per hop tick"
    );
    assert_eq!(front_lengths[0], 1, "first hop must seed inlet segment");
    for idx in 1..front_lengths.len() {
        assert_eq!(
            front_lengths[idx],
            front_lengths[idx - 1] + 1,
            "pipe front must advance by one segment each hop without gaps (hop #{idx})",
        );
    }
    for (idx, source_total) in source_totals.iter().enumerate() {
        assert!(
            *source_total > 100,
            "inlet segment must keep receiving gas each hop (hop #{idx}, total={source_total})",
        );
    }
}

#[test]
fn first_hop_vent_intake_is_visible_in_flow_transfers() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let config = pipe_config();
    let mut world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    let y = 40u32;
    let start_x = 40u32;
    let end_x = 46u32;

    let source_room_min = UVec2::new(39, 39);
    let source_room_max = UVec2::new(41, 41);
    let source_walls_min = UVec2::new(38, 38);
    let source_walls_max = UVec2::new(42, 42);
    let sink_room_min = UVec2::new(45, 39);
    let sink_room_max = UVec2::new(47, 41);
    let sink_walls_min = UVec2::new(44, 38);
    let sink_walls_max = UVec2::new(48, 42);

    for x in start_x..=end_x {
        assert!(structures.place_pipe(x, y, &world));
    }
    assert!(structures.place_vent(start_x, y, &world));
    assert!(structures.place_vent(end_x, y, &world));

    for y_wall in source_walls_min.y..=source_walls_max.y {
        for x_wall in source_walls_min.x..=source_walls_max.x {
            let is_interior = x_wall >= source_room_min.x
                && x_wall <= source_room_max.x
                && y_wall >= source_room_min.y
                && y_wall <= source_room_max.y;
            if is_interior {
                continue;
            }
            let _ = world.set_solid_with_material(
                x_wall,
                y_wall,
                crate::plugins::default_plugin::brick_cell_material(),
            );
        }
    }
    for y_wall in sink_walls_min.y..=sink_walls_max.y {
        for x_wall in sink_walls_min.x..=sink_walls_max.x {
            let is_interior = x_wall >= sink_room_min.x
                && x_wall <= sink_room_max.x
                && y_wall >= sink_room_min.y
                && y_wall <= sink_room_max.y;
            if is_interior {
                continue;
            }
            let _ = world.set_solid_with_material(
                x_wall,
                y_wall,
                crate::plugins::default_plugin::brick_cell_material(),
            );
        }
    }

    let mut gas = GasField::from_registry(&registry);
    set_area_pressure(
        &mut gas,
        gas_index,
        source_room_min.x,
        source_room_min.y,
        source_room_max.x,
        source_room_max.y,
        100_000.0,
        &config,
    );
    set_area_pressure(
        &mut gas,
        gas_index,
        sink_room_min.x,
        sink_room_min.y,
        sink_room_max.x,
        sink_room_max.y,
        0.0,
        &config,
    );
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();

    let _ = apply_pipe_network_step(
        &structures,
        &mut pipe_gas,
        &mut pipe_flux,
        &mut gas,
        &world,
        &mut visuals,
        &config,
    );

    let inlet_cell = UVec2::new(start_x, y);
    assert!(
        pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, inlet_cell) > 0,
        "first hop should place gas into inlet segment"
    );
    assert!(
        has_transfer_touching_cell(&visuals, inlet_cell),
        "first hop inlet gas must stay visible in flow transfers"
    );
}

#[test]
fn vent_intake_reads_only_its_own_world_cell() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let config = pipe_config();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    let y = 45u32;
    let start_x = 45u32;
    let end_x = 49u32;

    for x in start_x..=end_x {
        assert!(structures.place_pipe(x, y, &world));
    }
    assert!(structures.place_vent(start_x, y, &world));
    assert!(structures.place_vent(end_x, y, &world));

    let mut gas = GasField::from_registry(&registry);
    let room_left = start_x - 1;
    let room_right = start_x + 1;
    let room_top = y - 1;
    let room_bottom = y + 1;
    set_area_pressure(
        &mut gas,
        gas_index,
        room_left,
        room_top,
        room_right,
        room_bottom,
        100_000.0,
        &config,
    );
    gas.set_amount(start_x, y, gas_index, 0.0);
    gas.recompute_total_density_buffer(&world);

    let neighbor_cell = UVec2::new(start_x - 1, y);
    let neighbor_before = gas.amount_particles(neighbor_cell.x, neighbor_cell.y, gas_index);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();

    let _ = apply_pipe_network_step(
        &structures,
        &mut pipe_gas,
        &mut pipe_flux,
        &mut gas,
        &world,
        &mut visuals,
        &config,
    );

    let inlet_cell = UVec2::new(start_x, y);
    assert_eq!(
        pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, inlet_cell),
        0,
        "vent must not intake from neighboring world cells when its own cell is empty"
    );
    assert_eq!(
        gas.amount_particles(neighbor_cell.x, neighbor_cell.y, gas_index),
        neighbor_before,
        "neighbor world cell should stay unchanged on the first hop"
    );
}

#[test]
fn vent_output_writes_only_its_own_world_cell() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let config = pipe_config();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    let vent_cell = UVec2::new(60, 45);

    assert!(structures.place_pipe(vent_cell.x, vent_cell.y, &world));
    assert!(structures.place_vent(vent_cell.x, vent_cell.y, &world));

    let mut gas = GasField::from_registry(&registry);
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let vent_node = pipe_node_id(&pipe_gas, PipeContainerKind::Pipe, vent_cell);
    pipe_gas.add_species_counts(vent_node, &[2_000, 0, 0]);

    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();

    let _ = apply_pipe_network_step(
        &structures,
        &mut pipe_gas,
        &mut pipe_flux,
        &mut gas,
        &world,
        &mut visuals,
        &config,
    );

    let center_particles = gas.amount_particles(vent_cell.x, vent_cell.y, gas_index);
    let neighbor_particles = gas.amount_particles(vent_cell.x + 1, vent_cell.y, gas_index);
    assert_eq!(
        pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, vent_cell),
        0,
        "vented pipe segment must be drained into the vent buffer before world release"
    );
    assert!(
        center_particles > 0,
        "vent output must emit gas into the world cell with the vent"
    );
    assert_eq!(
        neighbor_particles, 0,
        "vent output must not spread directly into neighboring world cells"
    );
}

#[test]
fn sink_vent_drains_arrived_hop_mass_before_world_release() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let config = pipe_config();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    let y = 40u32;
    let start_x = 36u32;
    let end_x = 43u32;

    for x in start_x..=end_x {
        assert!(structures.place_pipe(x, y, &world));
    }
    assert!(structures.place_vent(start_x, y, &world));
    assert!(structures.place_vent(end_x, y, &world));

    let mut gas = GasField::from_registry(&registry);
    set_area_pressure(
        &mut gas, gas_index, start_x, y, start_x, y, 250_000.0, &config,
    );
    set_area_pressure(&mut gas, gas_index, end_x, y, end_x, y, 0.0, &config);
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();

    let sink_cell = UVec2::new(end_x, y);
    let sink_upstream = UVec2::new(end_x - 1, y);
    let mut saw_commit_into_sink = false;
    let mut sink_world_gained = false;
    let mut sink_drained_on_arrival_hops = true;

    let ticks = (config.pipe_step_interval_ticks as usize) * 18;
    for _ in 0..ticks {
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut pipe_flux,
            &mut gas,
            &world,
            &mut visuals,
            &config,
        );
        if visuals.flow_progress().abs() > f32::EPSILON {
            continue;
        }
        let committed_into_sink = visuals.previous_transfers.iter().any(|transfer| {
            transfer.total_amount > 0 && transfer.from == sink_upstream && transfer.to == sink_cell
        });
        if committed_into_sink {
            saw_commit_into_sink = true;
            if pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, sink_cell) > 0 {
                sink_drained_on_arrival_hops = false;
            }
        }
        if gas.amount_particles(sink_cell.x, sink_cell.y, gas_index) > 0 {
            sink_world_gained = true;
        }
    }

    assert!(
        saw_commit_into_sink,
        "expected committed conveyor mass to reach the sink vent"
    );
    assert!(
        sink_world_gained,
        "sink vent must release arrived gas into its world cell"
    );
    assert!(
        sink_drained_on_arrival_hops,
        "sink vent pipe segment must be drained on hops where mass arrives from upstream"
    );
}

#[test]
fn intake_stage_recomputes_segment_direction_when_room_pressures_flip() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let config = pipe_config();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    let y = 28u32;
    let start_x = 22u32;
    let end_x = 27u32;

    for x in start_x..=end_x {
        assert!(structures.place_pipe(x, y, &world));
    }
    assert!(structures.place_vent(start_x, y, &world));
    assert!(structures.place_vent(end_x, y, &world));

    let mut gas = GasField::from_registry(&registry);
    set_area_pressure(
        &mut gas, gas_index, start_x, y, start_x, y, 200_000.0, &config,
    );
    set_area_pressure(&mut gas, gas_index, end_x, y, end_x, y, 0.0, &config);
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();

    let left_edge_from = UVec2::new(start_x, y);
    let left_edge_to = UVec2::new(start_x + 1, y);
    let right_edge_from = UVec2::new(end_x, y);
    let right_edge_to = UVec2::new(end_x - 1, y);

    let mut saw_forward_before_flip = false;
    let warmup_ticks = config.pipe_step_interval_ticks as usize * 4;
    for _ in 0..warmup_ticks {
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut pipe_flux,
            &mut gas,
            &world,
            &mut visuals,
            &config,
        );
        if visuals.flow_progress().abs() > f32::EPSILON {
            continue;
        }
        saw_forward_before_flip |= has_directional_transfer(&visuals, left_edge_from, left_edge_to);
    }
    assert!(
        saw_forward_before_flip,
        "expected initial flow from left room toward right room before pressure flip"
    );

    set_area_pressure(&mut gas, gas_index, start_x, y, start_x, y, 0.0, &config);
    set_area_pressure(&mut gas, gas_index, end_x, y, end_x, y, 200_000.0, &config);
    gas.recompute_total_density_buffer(&world);

    let mut saw_reversed_after_flip = false;
    let mut saw_old_direction_after_flip = false;
    let observe_ticks = config.pipe_step_interval_ticks as usize * 4;
    for _ in 0..observe_ticks {
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut pipe_flux,
            &mut gas,
            &world,
            &mut visuals,
            &config,
        );
        if visuals.flow_progress().abs() > f32::EPSILON {
            continue;
        }
        let has_reverse = has_directional_transfer(&visuals, right_edge_from, right_edge_to);
        let has_old = has_directional_transfer(&visuals, left_edge_from, left_edge_to);
        if has_reverse {
            saw_reversed_after_flip = true;
            if has_old {
                saw_old_direction_after_flip = true;
            }
        }
    }

    assert!(
        saw_reversed_after_flip,
        "expected segment direction to reverse after room pressures flipped"
    );
    assert!(
        !saw_old_direction_after_flip,
        "segment must not keep old intake direction once reverse direction is established"
    );
}

#[test]
fn segment_does_not_plan_reverse_while_external_gradient_sign_is_stable() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let config = pipe_config();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    let y = 42u32;
    let start_x = 41u32;
    let end_x = 54u32;

    for x in start_x..=end_x {
        assert!(structures.place_pipe(x, y, &world));
    }
    assert!(structures.place_vent(start_x, y, &world));
    assert!(structures.place_vent(end_x, y, &world));

    let mut gas = GasField::from_registry(&registry);
    set_area_pressure(
        &mut gas, gas_index, start_x, y, start_x, y, 200_000.0, &config,
    );
    set_area_pressure(&mut gas, gas_index, end_x, y, end_x, y, 0.0, &config);
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();

    let left_cell = UVec2::new(start_x, y);
    let left_next = UVec2::new(start_x + 1, y);
    let right_cell = UVec2::new(end_x, y);

    let mut checked_hops = 0usize;
    let ticks = config.pipe_step_interval_ticks as usize * 8;
    for tick in 0..ticks {
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut pipe_flux,
            &mut gas,
            &world,
            &mut visuals,
            &config,
        );
        if visuals.flow_progress().abs() > f32::EPSILON {
            continue;
        }

        let left_pressure = gas.total_amount_rounded(left_cell.x, left_cell.y) as f32
            * config.cell_particle_pressure_pa.max(1e-6);
        let right_pressure = gas.total_amount_rounded(right_cell.x, right_cell.y) as f32
            * config.cell_particle_pressure_pa.max(1e-6);
        if left_pressure <= right_pressure + config.pressure_epsilon_pa.max(0.0) {
            continue;
        }
        checked_hops += 1;
        assert!(
            !has_directional_transfer(&visuals, left_next, left_cell),
            "reverse source-edge plan detected at tick={} (left_pressure={}Pa, right_pressure={}Pa)",
            tick + 1,
            left_pressure,
            right_pressure
        );
    }

    assert!(
        checked_hops > 0,
        "expected at least one hop where external gradient stayed left->right"
    );
}

#[test]
fn intake_offer_uses_only_external_vent_pressures_on_three_vent_branch() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let config = pipe_config();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();

    let left_vent = UVec2::new(31, 40);
    let junction = UVec2::new(35, 40);
    let top_vent = UVec2::new(35, 35);
    let bottom_vent = UVec2::new(35, 45);

    let mut ensure_pipe = |x: u32, y: u32| {
        if !structures.has_pipe_at(x, y) {
            assert!(structures.place_pipe(x, y, &world));
        }
    };
    for x in left_vent.x..=junction.x {
        ensure_pipe(x, left_vent.y);
    }
    for y in top_vent.y..=junction.y {
        ensure_pipe(junction.x, y);
    }
    for y in junction.y..=bottom_vent.y {
        ensure_pipe(junction.x, y);
    }
    assert!(structures.place_vent(left_vent.x, left_vent.y, &world));
    assert!(structures.place_vent(top_vent.x, top_vent.y, &world));
    assert!(structures.place_vent(bottom_vent.x, bottom_vent.y, &world));

    let mut gas = GasField::from_registry(&registry);
    set_area_pressure(
        &mut gas,
        gas_index,
        left_vent.x,
        left_vent.y,
        left_vent.x,
        left_vent.y,
        100.0,
        &config,
    );
    set_area_pressure(
        &mut gas, gas_index, top_vent.x, top_vent.y, top_vent.x, top_vent.y, 0.0, &config,
    );
    set_area_pressure(
        &mut gas,
        gas_index,
        bottom_vent.x,
        bottom_vent.y,
        bottom_vent.x,
        bottom_vent.y,
        50.0,
        &config,
    );
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();

    let ticks = config.pipe_step_interval_ticks as usize * 2;
    let mut hop_transfers = Vec::<Vec<(UVec2, UVec2, u32)>>::new();
    let mut hop_world_masses = Vec::<(u32, u32, u32)>::new();
    for _ in 0..ticks {
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut pipe_flux,
            &mut gas,
            &world,
            &mut visuals,
            &config,
        );
        if visuals.flow_progress().abs() > f32::EPSILON {
            continue;
        }
        hop_transfers.push(
            visuals
                .transfers
                .iter()
                .filter(|transfer| transfer.total_amount > 0)
                .map(|transfer| (transfer.from, transfer.to, transfer.total_amount))
                .collect(),
        );
        hop_world_masses.push((
            gas.amount_particles(left_vent.x, left_vent.y, gas_index),
            gas.amount_particles(bottom_vent.x, bottom_vent.y, gas_index),
            gas.amount_particles(top_vent.x, top_vent.y, gas_index),
        ));
        if hop_transfers.len() == 2 {
            break;
        }
    }

    let (left_after_first_hop, bottom_after_first_hop, top_after_first_hop) = hop_world_masses
        .first()
        .copied()
        .expect("expected first hop world snapshot");
    assert_eq!(
        left_after_first_hop,
        470,
        "left vent intake must follow external vent offers with linear delta/25 mapping and min-one rule"
    );
    assert_eq!(
        bottom_after_first_hop, 250,
        "neutral-offer bottom vent must not intake on the first hop"
    );
    assert_eq!(
        top_after_first_hop, 0,
        "sink vent must not intake from world when its offer is positive"
    );

    assert!(
        hop_transfers.first().is_some_and(|hop| hop
            .iter()
            .any(|(from, to, _)| *from == left_vent && *to == UVec2::new(32, 40))),
        "first hop must start conveyor flow from the negative-offer vent"
    );
    assert!(
        hop_transfers.first().is_some_and(|hop| hop
            .iter()
            .all(|(from, to, _)| !(*from == bottom_vent && *to == UVec2::new(35, 44)))),
        "first hop must not start flow from neutral-offer vent"
    );
    assert!(
        hop_transfers.get(1).is_some_and(|hop| hop
            .iter()
            .all(|(from, to, _)| !(*from == junction && *to == UVec2::new(35, 41)))),
        "branch with neutral offer must stay without forward transfer on the second hop"
    );
}

#[test]
fn intake_split_uses_outgoing_requests_not_sink_count() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let config = pipe_config();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();

    let source_vent = UVec2::new(30, 30);
    let junction = UVec2::new(31, 30);
    let sink_top = UVec2::new(32, 29);
    let sink_bottom = UVec2::new(32, 31);

    let mut ensure_pipe = |x: u32, y: u32| {
        if !structures.has_pipe_at(x, y) {
            assert!(structures.place_pipe(x, y, &world));
        }
    };
    for cell in [
        source_vent,
        junction,
        UVec2::new(32, 30),
        sink_top,
        sink_bottom,
    ] {
        ensure_pipe(cell.x, cell.y);
    }
    assert!(structures.place_vent(source_vent.x, source_vent.y, &world));
    assert!(structures.place_vent(sink_top.x, sink_top.y, &world));
    assert!(structures.place_vent(sink_bottom.x, sink_bottom.y, &world));

    let mut gas = GasField::from_registry(&registry);
    set_area_pressure(
        &mut gas,
        gas_index,
        source_vent.x,
        source_vent.y,
        source_vent.x,
        source_vent.y,
        100.0,
        &config,
    );
    set_area_pressure(
        &mut gas, gas_index, sink_top.x, sink_top.y, sink_top.x, sink_top.y, 0.0, &config,
    );
    set_area_pressure(
        &mut gas,
        gas_index,
        sink_bottom.x,
        sink_bottom.y,
        sink_bottom.x,
        sink_bottom.y,
        0.0,
        &config,
    );
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();

    let ticks = config.pipe_step_interval_ticks as usize;
    let mut source_world_after_first_hop = None;
    for _ in 0..ticks {
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut pipe_flux,
            &mut gas,
            &world,
            &mut visuals,
            &config,
        );
        if visuals.flow_progress().abs() > f32::EPSILON {
            continue;
        }
        source_world_after_first_hop =
            Some(gas.amount_particles(source_vent.x, source_vent.y, gas_index));
        break;
    }

    assert_eq!(
        source_world_after_first_hop,
        Some(460),
        "n must count outgoing requests from source vent only: offer=200 Pa and n=1 should apply one outbound split"
    );
    assert!(
        has_transfer_between(&visuals, source_vent, junction),
        "source vent must issue one outgoing request through its only outgoing edge"
    );
}

#[test]
fn low_pressure_offer_uses_linear_intake_mapping() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let config = pipe_config();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    let source = UVec2::new(18, 18);
    let sink = UVec2::new(19, 18);

    assert!(structures.place_pipe(source.x, source.y, &world));
    assert!(structures.place_pipe(sink.x, sink.y, &world));
    assert!(structures.place_vent(source.x, source.y, &world));
    assert!(structures.place_vent(sink.x, sink.y, &world));

    let mut gas = GasField::from_registry(&registry);
    set_area_pressure(
        &mut gas, gas_index, source.x, source.y, source.x, source.y, 24.0, &config,
    );
    set_area_pressure(
        &mut gas, gas_index, sink.x, sink.y, sink.x, sink.y, 0.0, &config,
    );
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();

    let mut source_world_after_first_hop = None;
    for _ in 0..config.pipe_step_interval_ticks as usize {
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut pipe_flux,
            &mut gas,
            &world,
            &mut visuals,
            &config,
        );
        if visuals.flow_progress().abs() > f32::EPSILON {
            continue;
        }
        source_world_after_first_hop = Some(gas.amount_particles(source.x, source.y, gas_index));
        break;
    }

    assert_eq!(
        source_world_after_first_hop,
        Some(115),
        "linear intake mapping should remove round(delta/25) particles from source vent cell"
    );
}

#[test]
fn linear_intake_curve_matches_baseline_rules() {
    let config = pipe_config();
    let p1 = offer_based_intake_particles_for_test(&config, 1.0, 1);
    let p4 = offer_based_intake_particles_for_test(&config, 4.0, 1);
    let p5 = offer_based_intake_particles_for_test(&config, 5.0, 1);
    let p10 = offer_based_intake_particles_for_test(&config, 10.0, 1);
    let p30 = offer_based_intake_particles_for_test(&config, 30.0, 1);
    let p50 = offer_based_intake_particles_for_test(&config, 50.0, 1);
    let p1k = offer_based_intake_particles_for_test(&config, 1_000.0, 1);
    let p10m = offer_based_intake_particles_for_test(&config, 10_000_000.0, 1);
    let p100m = offer_based_intake_particles_for_test(&config, 100_000_000.0, 1);
    let p40_n1 = offer_based_intake_particles_for_test(&config, 40.0, 1);
    let p40_n2 = offer_based_intake_particles_for_test(&config, 40.0, 2);

    assert_eq!(
        p1, 1,
        "delta=1Pa equals 5 particles, so min-one rule must apply"
    );
    assert_eq!(
        p4, 1,
        "delta=4Pa should round to one particle after Pa->particles conversion"
    );
    assert_eq!(p5, 1, "delta=5Pa should keep one particle");
    assert_eq!(
        p10, 2,
        "delta=10Pa should map to two particles on delta_particles/25 baseline"
    );
    assert_eq!(
        p30, 6,
        "delta=30Pa should map to six particles on delta_particles/25 baseline"
    );
    assert_eq!(
        p50, 10,
        "delta=50Pa should map to ten particles on delta_particles/25 baseline"
    );
    assert_eq!(
        p1k, 200,
        "delta=1000Pa should map to 200 particles for delta_particles/25 baseline"
    );
    assert_eq!(p10m, 200_000, "intake must clamp to hard 200000 cap");
    assert!(
        p100m == 200_000,
        "very large delta must stay clamped at hard 200000 cap (got={p100m})"
    );
    assert!(
        p40_n1 > p40_n2,
        "reduced delta must use delta/n_outgoing, so larger n lowers intake (n1={p40_n1}, n2={p40_n2})"
    );
}

#[test]
fn delta_30_single_outgoing_path_intake_is_at_least_one() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let config = pipe_config();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    let source = UVec2::new(26, 26);
    let sink = UVec2::new(27, 26);

    assert!(structures.place_pipe(source.x, source.y, &world));
    assert!(structures.place_pipe(sink.x, sink.y, &world));
    assert!(structures.place_vent(source.x, source.y, &world));
    assert!(structures.place_vent(sink.x, sink.y, &world));

    let mut gas = GasField::from_registry(&registry);
    set_area_pressure(
        &mut gas, gas_index, source.x, source.y, source.x, source.y, 30.0, &config,
    );
    set_area_pressure(
        &mut gas, gas_index, sink.x, sink.y, sink.x, sink.y, 0.0, &config,
    );
    gas.recompute_total_density_buffer(&world);

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    let mut pipe_flux = PipeFluxField::default();
    let mut visuals = PipeFlowVisualState::default();
    let initial = gas.amount_particles(source.x, source.y, gas_index);
    for _ in 0..config.pipe_step_interval_ticks as usize {
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut pipe_flux,
            &mut gas,
            &world,
            &mut visuals,
            &config,
        );
        if visuals.flow_progress().abs() <= f32::EPSILON {
            break;
        }
    }

    let removed = initial.saturating_sub(gas.amount_particles(source.x, source.y, gas_index));
    assert!(
        removed >= 1,
        "expected at least 1 intake particle for delta=30, n=1 due to min-one rule; got {removed}"
    );
}

#[test]
fn source_room_vent_intake_occurs_on_every_hop_for_pressure_tiers() {
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let config = pipe_config();

    for tier_pa in [100.0, 1_000.0, 1_000_000.0] {
        let mut world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let y = 35u32;
        let start_x = 11u32;
        let end_x = 29u32;

        let source_room_min = UVec2::new(10, 34);
        let source_room_max = UVec2::new(12, 36);
        let source_walls_min = UVec2::new(9, 33);
        let source_walls_max = UVec2::new(13, 37);
        let sink_room_min = UVec2::new(28, 34);
        let sink_room_max = UVec2::new(30, 36);
        let sink_walls_min = UVec2::new(27, 33);
        let sink_walls_max = UVec2::new(31, 37);

        for x in start_x..=end_x {
            assert!(structures.place_pipe(x, y, &world));
        }
        assert!(structures.place_vent(start_x, y, &world));
        assert!(structures.place_vent(end_x, y, &world));

        for y_wall in source_walls_min.y..=source_walls_max.y {
            for x_wall in source_walls_min.x..=source_walls_max.x {
                let is_interior = x_wall >= source_room_min.x
                    && x_wall <= source_room_max.x
                    && y_wall >= source_room_min.y
                    && y_wall <= source_room_max.y;
                if is_interior {
                    continue;
                }
                let _ = world.set_solid_with_material(
                    x_wall,
                    y_wall,
                    crate::plugins::default_plugin::brick_cell_material(),
                );
            }
        }
        for y_wall in sink_walls_min.y..=sink_walls_max.y {
            for x_wall in sink_walls_min.x..=sink_walls_max.x {
                let is_interior = x_wall >= sink_room_min.x
                    && x_wall <= sink_room_max.x
                    && y_wall >= sink_room_min.y
                    && y_wall <= sink_room_max.y;
                if is_interior {
                    continue;
                }
                let _ = world.set_solid_with_material(
                    x_wall,
                    y_wall,
                    crate::plugins::default_plugin::brick_cell_material(),
                );
            }
        }

        let mut gas = GasField::from_registry(&registry);
        set_area_pressure(
            &mut gas,
            gas_index,
            source_room_min.x,
            source_room_min.y,
            source_room_max.x,
            source_room_max.y,
            tier_pa,
            &config,
        );
        set_area_pressure(
            &mut gas,
            gas_index,
            sink_room_min.x,
            sink_room_min.y,
            sink_room_max.x,
            sink_room_max.y,
            0.0,
            &config,
        );
        gas.recompute_total_density_buffer(&world);

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let mut pipe_flux = PipeFluxField::default();
        let mut visuals = PipeFlowVisualState::default();
        let observed_hops = 6usize;
        let ticks_to_run = observed_hops * config.pipe_step_interval_ticks as usize;

        let mut source_room_mass_per_hop = Vec::new();
        let mut source_edge_flow_per_hop = Vec::new();
        for _ in 0..ticks_to_run {
            let _ = apply_pipe_network_step(
                &structures,
                &mut pipe_gas,
                &mut pipe_flux,
                &mut gas,
                &world,
                &mut visuals,
                &config,
            );
            if visuals.flow_progress().abs() > f32::EPSILON {
                continue;
            }
            source_room_mass_per_hop.push(sum_area_particles(
                &gas,
                gas_index,
                source_room_min.x,
                source_room_min.y,
                source_room_max.x,
                source_room_max.y,
            ));
            source_edge_flow_per_hop.push(has_transfer_between(
                &visuals,
                UVec2::new(start_x, y),
                UVec2::new(start_x + 1, y),
            ));
            if source_room_mass_per_hop.len() == observed_hops {
                break;
            }
        }

        assert_eq!(
            source_room_mass_per_hop.len(),
            observed_hops,
            "expected one source-room sample per hop for tier {tier_pa} Pa"
        );
        for hop_idx in 1..source_room_mass_per_hop.len() {
            let prev = source_room_mass_per_hop[hop_idx - 1];
            let current = source_room_mass_per_hop[hop_idx];
            assert!(
                current < prev,
                "source room must lose gas on each hop (tier={tier_pa} Pa, hop={}, prev={}, current={})",
                hop_idx + 1,
                prev,
                current
            );
        }
        let first_flow_hop = source_edge_flow_per_hop
            .iter()
            .position(|has_flow| *has_flow)
            .expect("expected at least one source-edge transfer hop");
        assert!(
            first_flow_hop <= 1,
            "source-edge transfer should appear no later than hop-2 (tier={tier_pa} Pa, first_flow_hop={})",
            first_flow_hop + 1
        );
        let mut saw_gap_after_start = false;
        for hop_idx in first_flow_hop..source_edge_flow_per_hop.len() {
            if !source_edge_flow_per_hop[hop_idx] {
                saw_gap_after_start = true;
            }
        }
        assert!(
            !saw_gap_after_start,
            "source-edge transfer must stay continuous after it starts (tier={tier_pa} Pa, per_hop={:?})",
            source_edge_flow_per_hop
        );
    }
}

fn run_multi_vent_pressure_tier_stress(tier_pa: f32, ticks: usize) {
    let config = pipe_config();
    let registry = registry();
    let gas_index = registry.index_of("h2").expect("h2 index");
    let (world, structures, mut gas, mut pipe_gas, mut pipe_flux, mut visuals) =
        build_multi_vent_stress_network(tier_pa, &config);

    let initial_mass = total_world_mass(&gas, gas_index) + total_pipe_mass(&pipe_gas, gas_index);
    let mut saw_any_transfer = false;

    run_pipe_ticks_with_observer(
        ticks,
        &world,
        &structures,
        &mut gas,
        &mut pipe_gas,
        &mut pipe_flux,
        &mut visuals,
        &config,
        |state| {
            saw_any_transfer |= state
                .transfers
                .iter()
                .any(|transfer| transfer.total_amount > 0);
        },
    );

    let final_mass = total_world_mass(&gas, gas_index) + total_pipe_mass(&pipe_gas, gas_index);
    let drift = initial_mass.abs_diff(final_mass) as f64;
    let rel_drift = drift / (initial_mass.max(1) as f64);

    assert!(
        saw_any_transfer,
        "expected active flow for tier {tier_pa} Pa (pipe_mass={}, node_count={})",
        total_pipe_mass(&pipe_gas, gas_index),
        pipe_gas.node_count()
    );
    assert!(
        rel_drift <= 0.02,
        "mass drift is too large for tier {tier_pa} Pa: initial={initial_mass}, final={final_mass}, rel_drift={rel_drift}"
    );

    for node_id in 0..pipe_gas.node_count() {
        for gas_slot in 0..pipe_gas.gas_count() {
            let amount = pipe_gas.amount_particles(node_id, gas_slot);
            assert!(
                amount <= u32::MAX,
                "invalid pipe amount at node {node_id}, gas {gas_slot}"
            );
        }
    }

    assert_eq!(
        pipe_node_total(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(90, 50)),
        0,
        "dead-end branch in stress network should stay blocked at tier {tier_pa} Pa"
    );
}

#[test]
fn multi_vent_pressure_tiers_stress_smoke() {
    for tier_pa in [100.0, 1_000.0, 1_000_000.0] {
        run_multi_vent_pressure_tier_stress(tier_pa, 140);
    }
}

#[test]
fn multi_vent_pressure_tiers_stress_full() {
    for tier_pa in [100.0, 1_000.0, 1_000_000.0] {
        run_multi_vent_pressure_tier_stress(tier_pa, 900);
    }
}

#[test]
fn flow_visual_records_include_bridge_arc_path() {
    let flow_state = PipeFlowVisualState {
        transfers: vec![PipeTransferRecord {
            from: UVec2::new(5, 5),
            from_kind: PipeContainerKind::BridgePipe,
            to: UVec2::new(6, 5),
            to_kind: PipeContainerKind::BridgePipe,
            gas_counts: vec![10, 0, 0],
            total_amount: 10,
            visual_path: PipeTransferVisualPath::BridgeArc {
                bridge_origin: UVec2::new(5, 5),
                bridge_rotation: StructureRotation::Deg0,
            },
        }],
        ..Default::default()
    };

    assert_eq!(flow_state.transfers.len(), 1);
    assert!(matches!(
        flow_state.transfers[0].visual_path,
        PipeTransferVisualPath::BridgeArc { .. }
    ));
}

#[test]
fn flow_progress_reaches_destination_within_interval() {
    let mut flow_state = PipeFlowVisualState::default();
    let mut observed_progress = Vec::new();
    let mut hop_ticks = Vec::new();
    for _ in 0..10 {
        hop_ticks.push(flow_state.begin_tick(10));
        observed_progress.push(flow_state.flow_progress());
    }
    hop_ticks.push(flow_state.begin_tick(10));
    observed_progress.push(flow_state.flow_progress());

    assert!(hop_ticks[0], "first tick of interval must execute hop");
    assert!(
        hop_ticks[1..10].iter().all(|is_hop| !*is_hop),
        "intermediate ticks must only advance interpolation",
    );
    assert!(
        hop_ticks[10],
        "next interval must start with a new hop tick"
    );
    assert!((observed_progress[0] - 0.0).abs() < 1e-6);
    assert!((observed_progress[9] - 0.9).abs() < 1e-6);
    assert!((observed_progress[10] - 0.0).abs() < 1e-6);
}

#[test]
fn hop_boundary_render_uses_current_hop_start_frame() {
    let mut flow_state = PipeFlowVisualState::default();
    flow_state.transfers = vec![PipeTransferRecord {
        from: UVec2::new(1, 1),
        from_kind: PipeContainerKind::Pipe,
        to: UVec2::new(2, 1),
        to_kind: PipeContainerKind::Pipe,
        gas_counts: vec![5, 0, 0],
        total_amount: 5,
        visual_path: PipeTransferVisualPath::Straight,
    }];
    assert!(flow_state.begin_tick(10));
    flow_state.begin_hop_recording();
    flow_state.transfers = vec![PipeTransferRecord {
        from: UVec2::new(2, 1),
        from_kind: PipeContainerKind::Pipe,
        to: UVec2::new(3, 1),
        to_kind: PipeContainerKind::Pipe,
        gas_counts: vec![5, 0, 0],
        total_amount: 5,
        visual_path: PipeTransferVisualPath::Straight,
    }];
    let (render_transfers, render_progress) = flow_state.render_view();
    assert_eq!(
        render_transfers[0].from,
        UVec2::new(2, 1),
        "hop boundary must render current hop transfer to avoid first-frame skip"
    );
    assert_eq!(
        render_transfers[0].to,
        UVec2::new(3, 1),
        "hop boundary must use current hop destination"
    );
    assert!(
        (render_progress - 0.0).abs() < 1e-6,
        "hop boundary should start with progress=0 for the new transfer"
    );

    assert!(!flow_state.begin_tick(10));
    let (render_transfers_next_tick, render_progress_next_tick) = flow_state.render_view();
    assert_eq!(
        render_transfers_next_tick[0].from,
        UVec2::new(2, 1),
        "after boundary tick renderer should switch to current hop transfer"
    );
    assert!(
        render_progress_next_tick > 0.0 && render_progress_next_tick < 1.0,
        "current hop should continue with regular interpolation after boundary"
    );
}
