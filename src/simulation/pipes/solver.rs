use std::collections::{HashSet, VecDeque};

use crate::simulation::PipeSimulationConfig;

use super::{
    pressure::{pipe_pressure_pa, world_pressure_pa},
    *,
};

#[derive(Clone, Copy, Debug)]
enum VentRequestDirection {
    WorldToPipe,
    PipeToWorld,
}

#[derive(Clone, Copy, Debug)]
struct DirectedPipeRequest {
    source_index: usize,
    target_index: usize,
    requested_amount: u32,
    planned_flux: f32,
}

#[derive(Clone, Debug)]
struct VentRequest {
    node_index: usize,
    requested_amount: u32,
    direction: VentRequestDirection,
    world_cells: Vec<UVec2>,
}

#[derive(Clone, Copy, Debug)]
struct ComponentEdge {
    source_local: usize,
    target_local: usize,
    source_global: usize,
    target_global: usize,
}

const COMPONENT_NONLINEAR_ITERS: usize = 6;
const COMPONENT_LINEAR_ITERS: usize = 48;
const COMPONENT_SOLVE_EPSILON: f32 = 0.05;
const COMPONENT_WEIGHT_CAP: f32 = 8.0;

pub(super) fn apply_pipe_network_step(
    structures: &PlacedStructureMap,
    pipe_gas: &mut PipeGasField,
    pipe_flux: &mut PipeFluxField,
    gas: &mut GasField,
    world: &WorldGrid,
    visual_state: &mut PipeFlowVisualState,
    config: &PipeSimulationConfig,
) -> bool {
    visual_state.transfers.clear();
    if pipe_gas.gas_count() == 0 {
        pipe_flux.clear_all();
        return false;
    }

    pipe_gas.sync_to_structures(structures);
    let runtime = PipeRuntime::from_structures(structures, pipe_gas);
    if runtime.nodes.is_empty() {
        pipe_flux.clear_all();
        return false;
    }
    pipe_flux.sync_to_runtime(&runtime, pipe_gas);

    let starting_pipe_species = (0..pipe_gas.node_count())
        .map(|node_id| pipe_gas.node_species_counts(node_id))
        .collect::<Vec<_>>();
    let starting_pipe_totals = (0..pipe_gas.node_count())
        .map(|node_id| pipe_gas.total_amount_particles(node_id))
        .collect::<Vec<_>>();

    let mut pipe_requests =
        build_pipe_edge_requests(&runtime, pipe_gas, pipe_flux, &starting_pipe_totals, config);
    let accepted_pipe_requests =
        scale_pipe_requests_by_available_mass(&starting_pipe_totals, &mut pipe_requests);
    let outbound_pipe_totals =
        project_pipe_totals_after_outgoing(starting_pipe_totals.len(), &starting_pipe_totals, &accepted_pipe_requests);
    let after_pipe_totals =
        project_pipe_totals_after_requests(starting_pipe_totals.len(), &starting_pipe_totals, &accepted_pipe_requests);
    let mut vent_requests =
        build_vent_requests(&runtime, &outbound_pipe_totals, &after_pipe_totals, gas, world, config);
    scale_vent_requests_by_available_mass(&outbound_pipe_totals, &mut vent_requests);

    let mut next_pipe_species = starting_pipe_species.clone();
    let mut incoming_pipe_species = vec![vec![0u32; pipe_gas.gas_count()]; pipe_gas.node_count()];
    let mut world_changed = false;
    let mut pipe_changed = false;

    for request in &accepted_pipe_requests {
        if request.requested_amount == 0 {
            continue;
        }
        let moved = remove_species_proportional_counts(
            &mut next_pipe_species[request.source_index],
            request.requested_amount,
        );
        if moved.iter().all(|count| *count == 0) {
            continue;
        }
        add_species_counts(&mut incoming_pipe_species[request.target_index], &moved);

        visual_state.transfers.push(PipeTransferRecord {
            from: runtime.nodes[request.source_index].visual_cell,
            to: runtime.nodes[request.target_index].visual_cell,
            total_amount: moved.iter().copied().sum(),
            gas_counts: moved,
            visual_path: transfer_visual_path(
                pipe_gas.keys[request.source_index],
                pipe_gas.keys[request.target_index],
                structures,
            ),
        });
    }

    for (node_id, incoming) in incoming_pipe_species.iter().enumerate() {
        if incoming.iter().all(|count| *count == 0) {
            continue;
        }
        add_species_counts(&mut next_pipe_species[node_id], incoming);
    }

    for request in &vent_requests {
        if request.requested_amount == 0 {
            continue;
        }
        let Some(_cell) = runtime.nodes[request.node_index].vent_cell else {
            continue;
        };
        match request.direction {
            VentRequestDirection::PipeToWorld => {
                let moved = remove_species_proportional_counts(
                    &mut next_pipe_species[request.node_index],
                    request.requested_amount,
                );
                if moved.iter().all(|count| *count == 0) {
                    continue;
                }
                world_changed |= add_particles_to_world_cells(gas, &request.world_cells, &moved, world);
            }
            VentRequestDirection::WorldToPipe => {
                let removed =
                    remove_particles_from_world_cells(gas, &request.world_cells, request.requested_amount, world);
                if removed.iter().all(|count| *count == 0) {
                    continue;
                }
                add_species_counts(&mut next_pipe_species[request.node_index], &removed);
                world_changed = true;
            }
        }
    }

    for node_id in 0..pipe_gas.node_count() {
        pipe_changed |= next_pipe_species[node_id] != starting_pipe_species[node_id];
        pipe_gas.set_species_counts_exact(node_id, &next_pipe_species[node_id]);
    }

    for request in &pipe_requests {
        let requested = request.requested_amount.max(1) as f32;
        let actual = accepted_pipe_requests
            .iter()
            .find(|accepted| {
                accepted.source_index == request.source_index
                    && accepted.target_index == request.target_index
            })
            .map(|accepted| accepted.requested_amount as f32)
            .unwrap_or(0.0);
        let scaled_flux = if requested <= f32::EPSILON {
            request.planned_flux
        } else {
            request.planned_flux * (actual / requested)
        };
        pipe_flux.set_signed_flux(
            pipe_gas.keys[request.source_index],
            pipe_gas.keys[request.target_index],
            scaled_flux,
        );
    }

    if world_changed {
        gas.recompute_total_density_buffer(world);
    }

    pipe_changed || world_changed
}

fn build_pipe_edge_requests(
    runtime: &PipeRuntime,
    pipe_gas: &PipeGasField,
    pipe_flux: &PipeFluxField,
    starting_pipe_totals: &[u32],
    config: &PipeSimulationConfig,
) -> Vec<DirectedPipeRequest> {
    let mut requests = Vec::new();
    let components = collect_runtime_components(runtime);
    for component_nodes in components {
        if component_nodes.len() < 2 {
            continue;
        }

        let mut local_by_global = vec![usize::MAX; runtime.nodes.len()];
        for (local_index, global_index) in component_nodes.iter().copied().enumerate() {
            local_by_global[global_index] = local_index;
        }
        let component_edges = collect_component_edges(runtime, &component_nodes, &local_by_global);
        if component_edges.is_empty() {
            continue;
        }

        let starting_component = component_nodes
            .iter()
            .map(|node_index| starting_pipe_totals[*node_index] as f32)
            .collect::<Vec<_>>();
        let adjacency = build_component_adjacency(component_nodes.len(), &component_edges);
        let solved = solve_component_particles(&starting_component, &component_edges, &adjacency, config);
        let mut planned_fluxes = component_edges
            .iter()
            .copied()
            .map(|edge| {
                let source_pressure = pipe_pressure_pa_f32(config, solved[edge.source_local]);
                let target_pressure = pipe_pressure_pa_f32(config, solved[edge.target_local]);
                let drive = signed_log_pressure_ratio(source_pressure, target_pressure, config);
                let previous_flux = pipe_flux.signed_flux(
                    pipe_gas.keys[edge.source_global],
                    pipe_gas.keys[edge.target_global],
                );
                ((previous_flux + config.pipe_flux_gain * drive)
                    * config.pipe_flux_damping.clamp(0.0, 1.0))
                    .clamp(
                        -config.max_pipe_flux_particles_per_tick.max(0.0),
                        config.max_pipe_flux_particles_per_tick.max(0.0),
                    )
            })
            .collect::<Vec<_>>();
        smooth_component_fluxes(&component_edges, &mut planned_fluxes);

        for (edge, planned_flux) in component_edges.iter().copied().zip(planned_fluxes) {
            requests.push(directed_request_from_flux(edge, planned_flux));
        }
    }

    requests
}

fn collect_runtime_components(runtime: &PipeRuntime) -> Vec<Vec<usize>> {
    let mut components = Vec::new();
    let mut visited = vec![false; runtime.nodes.len()];

    for start_index in 0..runtime.nodes.len() {
        if visited[start_index] {
            continue;
        }
        visited[start_index] = true;
        let mut queue = VecDeque::from([start_index]);
        let mut component = Vec::new();
        while let Some(node_index) = queue.pop_front() {
            component.push(node_index);
            for neighbor in runtime.nodes[node_index].neighbors.iter().copied() {
                if visited[neighbor] {
                    continue;
                }
                visited[neighbor] = true;
                queue.push_back(neighbor);
            }
        }
        component.sort_unstable();
        components.push(component);
    }

    components
}

fn collect_component_edges(
    runtime: &PipeRuntime,
    component_nodes: &[usize],
    local_by_global: &[usize],
) -> Vec<ComponentEdge> {
    let mut edges = Vec::new();
    let mut seen = HashSet::new();
    for source_global in component_nodes.iter().copied() {
        for target_global in runtime.nodes[source_global].neighbors.iter().copied() {
            if source_global >= target_global {
                continue;
            }
            if local_by_global[target_global] == usize::MAX {
                continue;
            }
            if !seen.insert((source_global, target_global)) {
                continue;
            }
            edges.push(ComponentEdge {
                source_local: local_by_global[source_global],
                target_local: local_by_global[target_global],
                source_global,
                target_global,
            });
        }
    }
    edges
}

fn build_component_adjacency(
    node_count: usize,
    edges: &[ComponentEdge],
) -> Vec<Vec<(usize, usize)>> {
    let mut adjacency = vec![Vec::new(); node_count];
    for (edge_index, edge) in edges.iter().copied().enumerate() {
        adjacency[edge.source_local].push((edge.target_local, edge_index));
        adjacency[edge.target_local].push((edge.source_local, edge_index));
    }
    adjacency
}

fn solve_component_particles(
    starting_component: &[f32],
    edges: &[ComponentEdge],
    adjacency: &[Vec<(usize, usize)>],
    config: &PipeSimulationConfig,
) -> Vec<f32> {
    let mut solved = starting_component.to_vec();
    let mut weights = component_edge_weights(config, &solved, edges);

    for _ in 0..COMPONENT_NONLINEAR_ITERS {
        let next = solve_linear_component(starting_component, adjacency, &weights);
        let node_delta = solved
            .iter()
            .zip(next.iter())
            .map(|(previous, current)| (previous - current).abs())
            .fold(0.0, f32::max);
        solved = next;

        let next_weights = component_edge_weights(config, &solved, edges);
        let weight_delta = weights
            .iter()
            .zip(next_weights.iter())
            .map(|(previous, current)| (previous - current).abs())
            .fold(0.0, f32::max);
        weights = next_weights;

        if node_delta <= COMPONENT_SOLVE_EPSILON && weight_delta <= COMPONENT_SOLVE_EPSILON {
            break;
        }
    }

    solved
}

fn solve_linear_component(
    starting_component: &[f32],
    adjacency: &[Vec<(usize, usize)>],
    weights: &[f32],
) -> Vec<f32> {
    let mut solved = starting_component.to_vec();
    for _ in 0..COMPONENT_LINEAR_ITERS {
        let mut max_delta = 0.0f32;
        for node_index in 0..solved.len() {
            let mut diagonal = 1.0f32;
            let mut rhs = starting_component[node_index];
            for (neighbor_index, edge_index) in adjacency[node_index].iter().copied() {
                let weight = weights[edge_index];
                diagonal += weight;
                rhs += weight * solved[neighbor_index];
            }
            let next_value = (rhs / diagonal).max(0.0);
            max_delta = max_delta.max((solved[node_index] - next_value).abs());
            solved[node_index] = next_value;
        }
        if max_delta <= COMPONENT_SOLVE_EPSILON {
            break;
        }
    }
    solved
}

fn component_edge_weights(
    config: &PipeSimulationConfig,
    solved_particles: &[f32],
    edges: &[ComponentEdge],
) -> Vec<f32> {
    edges
        .iter()
        .map(|edge| {
            let source_particles = solved_particles[edge.source_local].max(0.0);
            let target_particles = solved_particles[edge.target_local].max(0.0);
            let source_pressure = pipe_pressure_pa_f32(config, source_particles);
            let target_pressure = pipe_pressure_pa_f32(config, target_particles);
            edge_weight_from_pressures(
                config,
                source_particles,
                target_particles,
                source_pressure,
                target_pressure,
            )
        })
        .collect()
}

fn smooth_component_fluxes(edges: &[ComponentEdge], fluxes: &mut [f32]) {
    if edges.len() < 2 {
        return;
    }

    for _ in 0..4 {
        let previous = fluxes.to_vec();
        for edge_index in 0..edges.len() {
            let mut shared = Vec::new();
            for other_index in 0..edges.len() {
                if edge_index == other_index {
                    continue;
                }
                let edge = edges[edge_index];
                let other = edges[other_index];
                let shares_node = edge.source_local == other.source_local
                    || edge.source_local == other.target_local
                    || edge.target_local == other.source_local
                    || edge.target_local == other.target_local;
                if !shares_node {
                    continue;
                }
                if previous[edge_index].signum() != 0.0
                    && previous[other_index].signum() != 0.0
                    && previous[edge_index].signum() != previous[other_index].signum()
                {
                    continue;
                }
                shared.push(previous[other_index]);
            }

            if shared.is_empty() {
                continue;
            }
            let neighbor_average = shared.iter().copied().sum::<f32>() / shared.len() as f32;
            fluxes[edge_index] = previous[edge_index] * 0.85 + neighbor_average * 0.15;
        }
    }
}

fn edge_weight_from_pressures(
    config: &PipeSimulationConfig,
    source_particles: f32,
    target_particles: f32,
    source_pressure: f32,
    target_pressure: f32,
) -> f32 {
    let particle_delta = (source_particles - target_particles).abs();
    if particle_delta <= 1.0 {
        let avg_pressure = ((source_pressure + target_pressure) * 0.5)
            .max(config.cell_particle_pressure_pa * config.cell_volume_ratio)
            .max(config.pressure_epsilon_pa.max(1e-6));
        return (config.pipe_flux_gain.max(0.0)
            * config.cell_particle_pressure_pa
            * config.cell_volume_ratio
            / avg_pressure)
            .clamp(0.0, COMPONENT_WEIGHT_CAP);
    }

    let drive = signed_log_pressure_ratio(source_pressure, target_pressure, config).abs();
    (config.pipe_flux_gain.max(0.0) * drive / particle_delta).clamp(0.0, COMPONENT_WEIGHT_CAP)
}

fn directed_request_from_flux(edge: ComponentEdge, signed_flux: f32) -> DirectedPipeRequest {
    let requested_amount = signed_flux.abs().round() as u32;
    if signed_flux >= 0.0 {
        DirectedPipeRequest {
            source_index: edge.source_global,
            target_index: edge.target_global,
            requested_amount,
            planned_flux: signed_flux,
        }
    } else {
        DirectedPipeRequest {
            source_index: edge.target_global,
            target_index: edge.source_global,
            requested_amount,
            planned_flux: -signed_flux,
        }
    }
}

fn build_vent_requests(
    runtime: &PipeRuntime,
    outbound_pipe_totals: &[u32],
    after_pipe_totals: &[u32],
    gas: &GasField,
    world: &WorldGrid,
    config: &PipeSimulationConfig,
) -> Vec<VentRequest> {
    let mut requests = Vec::new();
    for (node_index, node) in runtime.nodes.iter().enumerate() {
        let Some(cell) = node.vent_cell else {
            continue;
        };
        let world_cells = collect_vent_world_cells(world, cell);
        let (world_pressure, _) = sampled_world_reservoir(gas, config, &world_cells);
        let inbound_pipe_pressure = pipe_pressure_pa(config, after_pipe_totals[node_index]);
        if world_pressure - inbound_pipe_pressure > config.pressure_epsilon_pa.max(0.0) {
            let amount = vent_requested_amount(config, world_pressure, inbound_pipe_pressure)
                .min(pipe_equalization_gap_particles(config, after_pipe_totals[node_index], world_pressure));
            if amount > 0 {
                requests.push(VentRequest {
                    node_index,
                    requested_amount: amount,
                    direction: VentRequestDirection::WorldToPipe,
                    world_cells: world_cells.clone(),
                });
            }
            continue;
        }

        let outbound_pipe_pressure = pipe_pressure_pa(config, outbound_pipe_totals[node_index]);
        if outbound_pipe_pressure - world_pressure > config.pressure_epsilon_pa.max(0.0) {
            let amount = vent_requested_amount(config, outbound_pipe_pressure, world_pressure)
                .min(pipe_pressure_excess_particles(config, outbound_pipe_totals[node_index], world_pressure));
            if amount > 0 {
                requests.push(VentRequest {
                    node_index,
                    requested_amount: amount,
                    direction: VentRequestDirection::PipeToWorld,
                    world_cells,
                });
            }
        }
    }
    requests
}

fn scale_pipe_requests_by_available_mass(
    starting_pipe_totals: &[u32],
    pipe_requests: &mut [DirectedPipeRequest],
) -> Vec<DirectedPipeRequest> {
    let mut accepted_pipe = Vec::new();
    for node_index in 0..starting_pipe_totals.len() {
        let mut request_kinds = Vec::new();
        let mut request_amounts = Vec::new();

        for (pipe_index, request) in pipe_requests.iter().enumerate() {
            if request.source_index == node_index && request.requested_amount > 0 {
                request_kinds.push((true, pipe_index));
                request_amounts.push(request.requested_amount);
            }
        }
        if request_amounts.is_empty() {
            continue;
        }

        let accepted_total = starting_pipe_totals[node_index].min(request_amounts.iter().copied().sum());
        let accepted_split = split_bounded_integer_requests(accepted_total, &request_amounts);
        for ((_is_pipe, index), accepted_amount) in request_kinds.into_iter().zip(accepted_split) {
            let request = pipe_requests[index];
            if accepted_amount > 0 {
                accepted_pipe.push(DirectedPipeRequest {
                    requested_amount: accepted_amount,
                    ..request
                });
            }
        }
    }

    accepted_pipe
}

fn project_pipe_totals_after_requests(
    node_count: usize,
    starting_pipe_totals: &[u32],
    accepted_pipe_requests: &[DirectedPipeRequest],
) -> Vec<u32> {
    let mut projected = starting_pipe_totals.to_vec();
    for request in accepted_pipe_requests {
        if request.requested_amount == 0 {
            continue;
        }
        projected[request.source_index] =
            projected[request.source_index].saturating_sub(request.requested_amount);
        projected[request.target_index] =
            projected[request.target_index].saturating_add(request.requested_amount);
    }
    projected.resize(node_count, 0);
    projected
}

fn project_pipe_totals_after_outgoing(
    node_count: usize,
    starting_pipe_totals: &[u32],
    accepted_pipe_requests: &[DirectedPipeRequest],
) -> Vec<u32> {
    let mut projected = starting_pipe_totals.to_vec();
    for request in accepted_pipe_requests {
        if request.requested_amount == 0 {
            continue;
        }
        projected[request.source_index] =
            projected[request.source_index].saturating_sub(request.requested_amount);
    }
    projected.resize(node_count, 0);
    projected
}

fn scale_vent_requests_by_available_mass(
    projected_pipe_totals: &[u32],
    vent_requests: &mut [VentRequest],
) {
    for request in vent_requests.iter_mut() {
        if matches!(request.direction, VentRequestDirection::WorldToPipe) {
            continue;
        }
        request.requested_amount = request
            .requested_amount
            .min(projected_pipe_totals[request.node_index]);
    }
}

fn signed_log_pressure_ratio(
    source_pressure: f32,
    target_pressure: f32,
    config: &PipeSimulationConfig,
) -> f32 {
    let floor = config.pressure_epsilon_pa.max(1e-6);
    ((source_pressure.max(0.0) + floor) / (target_pressure.max(0.0) + floor)).ln()
}

fn vent_requested_amount(
    config: &PipeSimulationConfig,
    upstream_pressure: f32,
    downstream_pressure: f32,
) -> u32 {
    if upstream_pressure <= config.pressure_epsilon_pa.max(1e-6) {
        return 0;
    }

    let upstream = upstream_pressure.max(config.pressure_epsilon_pa.max(1e-6));
    let ratio = (downstream_pressure.max(0.0) / upstream).clamp(0.0, 1.0);
    let choke_ratio = config.vent_choked_pressure_ratio.clamp(0.0, 0.999);
    let flow_scale = if ratio <= choke_ratio {
        1.0
    } else {
        ((1.0 - ratio) / (1.0 - choke_ratio)).clamp(0.0, 1.0).sqrt()
    };
    let amount = (config.vent_discharge_coefficient.max(0.0) * upstream.sqrt() * flow_scale)
        .min(config.max_vent_flux_particles_per_tick.max(0.0));
    amount.round() as u32
}

fn add_species_counts(target: &mut [u32], offered: &[u32]) {
    for (index, slot) in target.iter_mut().enumerate() {
        *slot = slot.saturating_add(offered.get(index).copied().unwrap_or(0));
    }
}

fn pipe_pressure_pa_f32(config: &PipeSimulationConfig, particles: f32) -> f32 {
    particles.max(0.0) * config.cell_particle_pressure_pa * config.cell_volume_ratio
}

fn pipe_particle_pressure_pa(config: &PipeSimulationConfig) -> f32 {
    (config.cell_particle_pressure_pa * config.cell_volume_ratio).max(1e-6)
}

fn pipe_particles_for_pressure(config: &PipeSimulationConfig, pressure: f32) -> u32 {
    (pressure.max(0.0) / pipe_particle_pressure_pa(config)).ceil() as u32
}

fn pipe_equalization_gap_particles(
    config: &PipeSimulationConfig,
    current_pipe_particles: u32,
    target_pressure: f32,
) -> u32 {
    pipe_particles_for_pressure(config, target_pressure).saturating_sub(current_pipe_particles)
}

fn pipe_pressure_excess_particles(
    config: &PipeSimulationConfig,
    current_pipe_particles: u32,
    target_pressure: f32,
) -> u32 {
    current_pipe_particles.saturating_sub(pipe_particles_for_pressure(config, target_pressure))
}

fn collect_vent_world_cells(world: &WorldGrid, center: UVec2) -> Vec<UVec2> {
    if world.is_solid(center.x, center.y) {
        return vec![center];
    }

    let mut cells = Vec::new();
    let mut queue = VecDeque::from([center]);
    let mut visited = HashSet::from([center]);
    while let Some(cell) = queue.pop_front() {
        cells.push(cell);
        for neighbor in orthogonal_neighbors(cell) {
            if visited.contains(&neighbor) || world.is_solid(neighbor.x, neighbor.y) {
                continue;
            }
            visited.insert(neighbor);
            queue.push_back(neighbor);
        }
    }

    cells
}

fn sampled_world_reservoir(
    gas: &GasField,
    config: &PipeSimulationConfig,
    world_cells: &[UVec2],
) -> (f32, u32) {
    let total_particles: u32 = world_cells
        .iter()
        .map(|cell| gas.total_amount_rounded(cell.x, cell.y))
        .sum();
    let average_particles = if world_cells.is_empty() {
        0
    } else {
        total_particles / world_cells.len() as u32
    };
    (world_pressure_pa(config, average_particles), total_particles)
}

fn remove_particles_from_world_cells(
    gas: &mut GasField,
    world_cells: &[UVec2],
    requested_amount: u32,
    world: &WorldGrid,
) -> Vec<u32> {
    if requested_amount == 0 || world_cells.is_empty() {
        return vec![0; gas.gas_count()];
    }
    let requests = world_cells
        .iter()
        .map(|cell| gas.total_amount_rounded(cell.x, cell.y))
        .collect::<Vec<_>>();
    let total_available: u32 = requests.iter().copied().sum();
    let accepted = split_bounded_integer_requests(requested_amount.min(total_available), &requests);
    let mut removed_total = vec![0u32; gas.gas_count()];
    for (cell, amount) in world_cells.iter().zip(accepted) {
        if amount == 0 {
            continue;
        }
        let removed = gas.remove_particles_proportional_counts(cell.x, cell.y, amount, world);
        add_species_counts(&mut removed_total, &removed);
    }
    removed_total
}

fn add_particles_to_world_cells(
    gas: &mut GasField,
    world_cells: &[UVec2],
    species_counts: &[u32],
    world: &WorldGrid,
) -> bool {
    if world_cells.is_empty() || species_counts.iter().all(|count| *count == 0) {
        return false;
    }
    let equal_weights = vec![1.0; world_cells.len()];
    let mut changed = false;
    for (gas_index, amount) in species_counts.iter().copied().enumerate() {
        if amount == 0 {
            continue;
        }
        let split = split_integer_by_weights(amount, &equal_weights);
        for (cell, cell_amount) in world_cells.iter().zip(split) {
            if cell_amount == 0 {
                continue;
            }
            if gas.add_particles_no_impulse(cell.x, cell.y, gas_index, cell_amount, world) > 0 {
                changed = true;
            }
        }
    }
    changed
}
