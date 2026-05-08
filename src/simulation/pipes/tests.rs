use super::{
    apply_pipe_network_step, bridge_port_cells, pipe_cell_display_blocks_with_transfers,
    pressure::{pipe_pressure_pa, world_pressure_pa},
    scenarios::build_reference_pipe_scenarios,
    PipeCellDisplayBlock, PipeContainerKind, PipeFlowVisualState, PipeFluxField, PipeGasField,
};
use crate::{
    config::{GasDefinition, GasRegistry},
    simulation::{
        apply_gas_structures_pre_step, do_one_substep, BlockSyncState, GasSimulationConfig,
        PipeSimulationConfig, SimulationStep,
    },
    world::{
        grid::WorldGrid,
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

fn simulation_config() -> GasSimulationConfig {
    GasSimulationConfig::default()
}

fn pipe_config() -> PipeSimulationConfig {
    simulation_config().pipe
}

fn relative_diff(a: f32, b: f32) -> f32 {
    let denom = a.abs().max(b.abs()).max(1.0);
    (a - b).abs() / denom
}

fn pressures_match_within(a: f32, b: f32, tolerance: f32) -> bool {
    relative_diff(a, b) <= tolerance
}

fn pressure_spread(values: &[f32]) -> f32 {
    let mut min_value = f32::INFINITY;
    let mut max_value = f32::NEG_INFINITY;
    for value in values {
        min_value = min_value.min(*value);
        max_value = max_value.max(*value);
    }
    if values.is_empty() {
        0.0
    } else {
        max_value - min_value
    }
}

fn pipe_node_total_at(pipe_gas: &PipeGasField, kind: PipeContainerKind, cell: UVec2) -> u32 {
    let node_id = pipe_gas
        .snapshot_state()
        .nodes
        .iter()
        .position(|node| node.key.kind == kind && node.key.anchor == cell)
        .expect("pipe node at cell");
    pipe_gas.total_amount_particles(node_id)
}

fn pipe_pressure_at(
    pipe_gas: &PipeGasField,
    config: &PipeSimulationConfig,
    kind: PipeContainerKind,
    cell: UVec2,
) -> f32 {
    pipe_pressure_pa(config, pipe_node_total_at(pipe_gas, kind, cell))
}

fn world_pressure_at(
    gas: &crate::simulation::gas::GasField,
    config: &PipeSimulationConfig,
    cell: UVec2,
) -> f32 {
    world_pressure_pa(config, gas.total_amount_rounded(cell.x, cell.y))
}

fn mean_world_pressure_in_rect(
    gas: &crate::simulation::gas::GasField,
    config: &PipeSimulationConfig,
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
) -> f32 {
    let mut sum = 0.0;
    let mut count = 0u32;
    for y in top..=bottom {
        for x in left..=right {
            sum += world_pressure_at(gas, config, UVec2::new(x, y));
            count += 1;
        }
    }
    sum / count.max(1) as f32
}

fn has_transfer_between(visuals: &PipeFlowVisualState, a: UVec2, b: UVec2) -> bool {
    visuals.transfers.iter().any(|transfer| {
        transfer.total_amount > 0
            && ((transfer.from == a && transfer.to == b)
                || (transfer.from == b && transfer.to == a))
    })
}

struct ScenarioHarness {
    world: WorldGrid,
    structures: PlacedStructureMap,
    gas: crate::simulation::gas::GasField,
    pipe_gas: PipeGasField,
    pipe_flux: PipeFluxField,
    visuals: PipeFlowVisualState,
    config: GasSimulationConfig,
    block_sync: BlockSyncState,
    step: SimulationStep,
}

impl ScenarioHarness {
    fn from_slot(
        slot: crate::simulation::pipes::scenarios::PipeScenarioSlot,
        config: GasSimulationConfig,
    ) -> Self {
        Self {
            world: slot.world,
            structures: slot.structures,
            gas: slot.gas,
            pipe_gas: slot.pipe_gas,
            pipe_flux: PipeFluxField::default(),
            visuals: PipeFlowVisualState::default(),
            config,
            block_sync: BlockSyncState,
            step: SimulationStep(0),
        }
    }

    fn tick(&mut self) {
        let _ = apply_pipe_network_step(
            &self.structures,
            &mut self.pipe_gas,
            &mut self.pipe_flux,
            &mut self.gas,
            &self.world,
            &mut self.visuals,
            &self.config.pipe,
        );
        let _ = apply_gas_structures_pre_step(&self.structures, &mut self.gas, &self.world);
        do_one_substep(
            &mut self.block_sync,
            &mut self.gas,
            &self.world,
            &self.config,
            &mut self.step,
        );
    }

    fn run_until<F>(&mut self, max_ticks: usize, mut stop: F) -> usize
    where
        F: FnMut(&Self, usize) -> bool,
    {
        for tick in 1..=max_ticks {
            self.tick();
            if stop(self, tick) {
                return tick;
            }
        }
        max_ticks
    }
}

fn scenario_slot(index: usize) -> crate::simulation::pipes::scenarios::PipeScenarioSlot {
    build_reference_pipe_scenarios(&registry(), &pipe_config())
        .expect("scenario build")
        .into_iter()
        .nth(index)
        .expect("scenario index")
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
fn topology_change_resets_flux_but_preserves_pipe_gas() {
    let registry = registry();
    let world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    assert!(structures.place_pipe(10, 10, &world));
    assert!(structures.place_pipe(11, 10, &world));

    let mut pipe_gas = PipeGasField::from_registry(&registry);
    pipe_gas.sync_to_structures(&structures);
    pipe_gas.add_species_counts(0, &[120, 0, 0]);
    pipe_gas.add_species_counts(1, &[80, 0, 0]);

    let mut pipe_flux = PipeFluxField::default();
    let runtime = super::PipeRuntime::from_structures(&structures, &pipe_gas);
    pipe_flux.sync_to_runtime(&runtime, &pipe_gas);
    pipe_flux.set_signed_flux(pipe_gas.keys[0], pipe_gas.keys[1], 42.0);

    assert!(structures.place_pipe(12, 10, &world));
    let before_total: u32 = (0..pipe_gas.node_count())
        .map(|node_id| pipe_gas.total_amount_particles(node_id))
        .sum();
    pipe_gas.sync_to_structures(&structures);
    let runtime_after = super::PipeRuntime::from_structures(&structures, &pipe_gas);
    pipe_flux.sync_to_runtime(&runtime_after, &pipe_gas);
    let after_total: u32 = (0..pipe_gas.node_count())
        .map(|node_id| pipe_gas.total_amount_particles(node_id))
        .sum();

    assert!(pipe_flux
        .flux_by_edge
        .values()
        .all(|value| value.abs() <= f32::EPSILON));
    assert_eq!(before_total, after_total);
}

#[test]
fn scenario_one_long_pipe_reaches_dense_front_and_low_gradient() {
    let config = simulation_config();
    let mut harness = ScenarioHarness::from_slot(scenario_slot(0), config);
    let segment_cells = (24..79).map(|x| UVec2::new(x, 51)).collect::<Vec<_>>();
    let source_cell = UVec2::new(23, 51);
    let mut first_reach_tick = vec![None; segment_cells.len()];
    let stop_tick = harness.run_until(700, |state, tick| {
        let source_pressure = world_pressure_at(&state.gas, &state.config.pipe, source_cell);
        for (index, cell) in segment_cells.iter().copied().enumerate() {
            let pressure = pipe_pressure_at(
                &state.pipe_gas,
                &state.config.pipe,
                PipeContainerKind::Pipe,
                cell,
            );
            if first_reach_tick[index].is_none() && pressure >= source_pressure * 0.95 {
                first_reach_tick[index] = Some(tick);
            }
        }
        first_reach_tick.iter().all(Option::is_some)
    });

    let final_profile = (23..=79)
        .map(|x| {
            pipe_pressure_at(
                &harness.pipe_gas,
                &harness.config.pipe,
                PipeContainerKind::Pipe,
                UVec2::new(x, 51),
            )
        })
        .collect::<Vec<_>>();
    let final_source_pressure = world_pressure_at(&harness.gas, &harness.config.pipe, source_cell);

    assert!(
        first_reach_tick.iter().all(Option::is_some),
        "not every segment reached 95% of inlet pressure within {stop_tick} ticks: reach={first_reach_tick:?}, source={final_source_pressure}, profile={final_profile:?}"
    );

    let source_pressure = final_source_pressure;
    let final_pressures = final_profile;
    for cell in &segment_cells {
        let pressure = pipe_pressure_at(
            &harness.pipe_gas,
            &harness.config.pipe,
            PipeContainerKind::Pipe,
            *cell,
        );
        assert!(
            pressure >= source_pressure * 0.95,
            "segment {cell:?} stayed below dense-front target: pressure={pressure}, source={source_pressure}, profile={final_pressures:?}"
        );
    }

    let pressures = final_pressures;
    for window in pressures.windows(2) {
        assert!(
            relative_diff(window[0], window[1]) <= 0.01,
            "neighboring pressure gradient exceeded 1%: {pressures:?}"
        );
    }

    let max_tick = first_reach_tick
        .iter()
        .flatten()
        .copied()
        .max()
        .unwrap_or(0) as f32;
    let avg_ticks_per_cell = max_tick / segment_cells.len() as f32;
    assert!(
        avg_ticks_per_cell <= 10.0,
        "dense-front propagation was too slow: avg_ticks_per_cell={avg_ticks_per_cell}, ticks={first_reach_tick:?}"
    );
}

#[test]
fn scenario_two_equalizes_two_rooms_and_pipe() {
    let config = simulation_config();
    let mut harness = ScenarioHarness::from_slot(scenario_slot(1), config);
    let ticks = harness.run_until(20_000, |state, _| {
        let left = mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 7, 44, 23, 58);
        let right = mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 79, 44, 95, 58);
        pressures_match_within(left, right, 0.03)
    });

    let left = mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 7, 44, 23, 58);
    let right = mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 79, 44, 95, 58);
    assert!(
        pressures_match_within(left, right, 0.03),
        "scenario 2 room pressures did not converge after {ticks} ticks: left={left}, right={right}"
    );

    let target = (left + right) * 0.5;
    for x in 23..=79 {
        let pressure = pipe_pressure_at(
            &harness.pipe_gas,
            &harness.config.pipe,
            PipeContainerKind::Pipe,
            UVec2::new(x, 51),
        );
        assert!(
            pressures_match_within(pressure, target, 0.03),
            "scenario 2 pipe segment at x={x} did not match room pressure: pipe={pressure}, room={target}"
        );
    }
}

#[test]
fn scenario_two_equalizes_two_rooms_and_pipe_smoke() {
    let config = simulation_config();
    let mut harness = ScenarioHarness::from_slot(scenario_slot(1), config);
    let left_initial =
        mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 7, 44, 23, 58);
    let right_initial =
        mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 79, 44, 95, 58);
    let initial_diff = (left_initial - right_initial).abs();
    let ticks = harness.run_until(1_200, |state, _| {
        let left = mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 7, 44, 23, 58);
        let right = mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 79, 44, 95, 58);
        let center = pipe_pressure_at(
            &state.pipe_gas,
            &state.config.pipe,
            PipeContainerKind::Pipe,
            UVec2::new(51, 51),
        );
        (left - right).abs() <= initial_diff * 0.95 && center >= right_initial
    });

    let left = mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 7, 44, 23, 58);
    let right = mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 79, 44, 95, 58);
    let center = pipe_pressure_at(
        &harness.pipe_gas,
        &harness.config.pipe,
        PipeContainerKind::Pipe,
        UVec2::new(51, 51),
    );
    let final_diff = (left - right).abs();
    assert!(
        final_diff <= initial_diff * 0.95,
        "scenario 2 smoke did not reduce room pressure gap enough after {ticks} ticks: initial_diff={initial_diff}, final_diff={final_diff}, left={left}, right={right}"
    );
    assert!(
        center >= right_initial,
        "scenario 2 smoke did not pressurize the middle of the pipe above the low-pressure room after {ticks} ticks: center={center}, right_initial={right_initial}"
    );
}

#[test]
fn scenario_three_equalizes_and_uses_both_parallel_routes() {
    let config = simulation_config();
    let mut harness = ScenarioHarness::from_slot(scenario_slot(2), config);
    let mut direct_route_seen = false;
    let mut branch_route_seen = false;
    harness.run_until(20_000, |state, _| {
        direct_route_seen |=
            has_transfer_between(&state.visuals, UVec2::new(50, 53), UVec2::new(51, 53));
        branch_route_seen |=
            has_transfer_between(&state.visuals, UVec2::new(50, 49), UVec2::new(51, 49));
        let left = mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 7, 44, 23, 62);
        let right = mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 79, 44, 95, 62);
        direct_route_seen && branch_route_seen && pressures_match_within(left, right, 0.03)
    });

    let left = mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 7, 44, 23, 62);
    let right = mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 79, 44, 95, 62);
    assert!(direct_route_seen, "expected flow through the direct branch");
    assert!(
        branch_route_seen,
        "expected flow through the longer P-branch"
    );
    assert!(
        pressures_match_within(left, right, 0.03),
        "scenario 3 room pressures did not converge: left={left}, right={right}"
    );
}

#[test]
fn scenario_three_equalizes_and_uses_both_parallel_routes_smoke() {
    let config = simulation_config();
    let mut harness = ScenarioHarness::from_slot(scenario_slot(2), config);
    let left_initial =
        mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 7, 44, 23, 62);
    let right_initial =
        mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 79, 44, 95, 62);
    let initial_diff = (left_initial - right_initial).abs();
    let mut direct_route_seen = false;
    let mut branch_route_seen = false;
    let ticks = harness.run_until(1_400, |state, _| {
        direct_route_seen |=
            has_transfer_between(&state.visuals, UVec2::new(50, 53), UVec2::new(51, 53));
        branch_route_seen |=
            has_transfer_between(&state.visuals, UVec2::new(50, 49), UVec2::new(51, 49));
        let left = mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 7, 44, 23, 62);
        let right = mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 79, 44, 95, 62);
        direct_route_seen && branch_route_seen && (left - right).abs() <= initial_diff * 0.95
    });

    let left = mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 7, 44, 23, 62);
    let right = mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 79, 44, 95, 62);
    let final_diff = (left - right).abs();
    assert!(
        direct_route_seen,
        "scenario 3 smoke expected flow through the direct branch"
    );
    assert!(
        branch_route_seen,
        "scenario 3 smoke expected flow through the longer P-branch"
    );
    assert!(
        final_diff <= initial_diff * 0.95,
        "scenario 3 smoke did not reduce room pressure gap enough after {ticks} ticks: initial_diff={initial_diff}, final_diff={final_diff}, left={left}, right={right}"
    );
}

#[test]
fn scenario_four_dead_end_pipe_fills_almost_to_source_pressure() {
    let config = simulation_config();
    let mut harness = ScenarioHarness::from_slot(scenario_slot(3), config);
    let source_cell = UVec2::new(27, 51);
    let stop_tick = harness.run_until(700, |state, _| {
        let source_pressure = world_pressure_at(&state.gas, &state.config.pipe, source_cell);
        (27..=85).all(|x| {
            pipe_pressure_at(
                &state.pipe_gas,
                &state.config.pipe,
                PipeContainerKind::Pipe,
                UVec2::new(x, 51),
            ) >= source_pressure * 0.95
        })
    });

    let source_pressure = world_pressure_at(&harness.gas, &harness.config.pipe, source_cell);
    for x in 27..=85 {
        let pressure = pipe_pressure_at(
            &harness.pipe_gas,
            &harness.config.pipe,
            PipeContainerKind::Pipe,
            UVec2::new(x, 51),
        );
        assert!(
            pressure >= source_pressure * 0.95,
            "dead-end segment x={x} stayed under 95% of source pressure after {stop_tick} ticks: pipe={pressure}, source={source_pressure}"
        );
    }
}

#[test]
fn scenario_five_star_equalizes_three_rooms_and_all_rays_flow_smoke() {
    let config = simulation_config();
    let mut harness = ScenarioHarness::from_slot(scenario_slot(4), config);
    let initial_pressures = [
        mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 44, 7, 58, 23),
        mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 7, 44, 23, 58),
        mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 79, 44, 95, 58),
    ];
    let initial_spread = pressure_spread(&initial_pressures);
    let mut top_seen = false;
    let mut left_seen = false;
    let mut right_seen = false;
    let ticks = harness.run_until(1_400, |state, _| {
        top_seen |= has_transfer_between(&state.visuals, UVec2::new(51, 30), UVec2::new(51, 31));
        left_seen |= has_transfer_between(&state.visuals, UVec2::new(30, 51), UVec2::new(31, 51));
        right_seen |= has_transfer_between(&state.visuals, UVec2::new(71, 51), UVec2::new(72, 51));
        let current_pressures = [
            mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 44, 7, 58, 23),
            mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 7, 44, 23, 58),
            mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 79, 44, 95, 58),
        ];
        top_seen
            && left_seen
            && right_seen
            && pressure_spread(&current_pressures) <= initial_spread * 0.95
    });

    let final_pressures = [
        mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 44, 7, 58, 23),
        mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 7, 44, 23, 58),
        mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 79, 44, 95, 58),
    ];
    let final_spread = pressure_spread(&final_pressures);
    assert!(
        top_seen,
        "scenario 5 smoke expected flow through the top ray"
    );
    assert!(
        left_seen,
        "scenario 5 smoke expected flow through the left ray"
    );
    assert!(
        right_seen,
        "scenario 5 smoke expected flow through the right ray"
    );
    assert!(
        final_spread <= initial_spread * 0.95,
        "scenario 5 smoke did not reduce room pressure spread enough after {ticks} ticks: initial_spread={initial_spread}, final_spread={final_spread}, pressures={final_pressures:?}"
    );
}

#[test]
fn scenario_five_star_equalizes_three_rooms_and_all_rays_flow() {
    let config = simulation_config();
    let mut harness = ScenarioHarness::from_slot(scenario_slot(4), config);
    let mut top_seen = false;
    let mut left_seen = false;
    let mut right_seen = false;
    harness.run_until(20_000, |state, _| {
        top_seen |= has_transfer_between(&state.visuals, UVec2::new(51, 30), UVec2::new(51, 31));
        left_seen |= has_transfer_between(&state.visuals, UVec2::new(30, 51), UVec2::new(31, 51));
        right_seen |= has_transfer_between(&state.visuals, UVec2::new(71, 51), UVec2::new(72, 51));
        let top = mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 44, 7, 58, 23);
        let left = mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 7, 44, 23, 58);
        let right = mean_world_pressure_in_rect(&state.gas, &state.config.pipe, 79, 44, 95, 58);
        top_seen
            && left_seen
            && right_seen
            && pressures_match_within(top, left, 0.03)
            && pressures_match_within(left, right, 0.03)
    });

    let top = mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 44, 7, 58, 23);
    let left = mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 7, 44, 23, 58);
    let right = mean_world_pressure_in_rect(&harness.gas, &harness.config.pipe, 79, 44, 95, 58);
    let center = pipe_pressure_at(
        &harness.pipe_gas,
        &harness.config.pipe,
        PipeContainerKind::Pipe,
        UVec2::new(51, 51),
    );

    assert!(top_seen, "expected flow through the top ray");
    assert!(left_seen, "expected flow through the left ray");
    assert!(right_seen, "expected flow through the right ray");
    assert!(
        pressures_match_within(top, left, 0.03) && pressures_match_within(left, right, 0.03),
        "scenario 5 room pressures did not converge: top={top}, left={left}, right={right}"
    );
    assert!(
        pressures_match_within(center, (top + left + right) / 3.0, 0.03),
        "scenario 5 center pressure did not match shared room pressure: center={center}, rooms=({}, {}, {})",
        top,
        left,
        right
    );
}
