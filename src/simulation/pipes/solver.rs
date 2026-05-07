use crate::simulation::PipeSimulationConfig;

use super::*;

#[derive(Clone, Copy, Debug, Default)]
struct VentPressureState {
    world_total: u32,
    world_pressure: f32,
    pipe_pressure: f32,
    raw_equilibrium_pipe_particles: u32,
    equilibrium_pipe_particles: u32,
    bootstrap_fill: bool,
}

pub(super) fn apply_pipe_network_step(
    structures: &PlacedStructureMap,
    pipe_gas: &mut PipeGasField,
    gas: &mut GasField,
    world: &WorldGrid,
    visual_state: &mut PipeFlowVisualState,
    config: &PipeSimulationConfig,
) -> bool {
    visual_state.transfers.clear();
    if pipe_gas.gas_count() == 0 {
        return false;
    }

    pipe_gas.sync_to_structures(structures);
    let runtime = PipeRuntime::from_structures(structures, pipe_gas);
    if runtime.nodes.is_empty() {
        return false;
    }

    let components = collect_pipe_components(&runtime);
    let mut world_changed = false;
    let mut pipe_changed = false;

    for component in components {
        if component.is_empty() {
            continue;
        }

        let starting_pipe_totals = component
            .iter()
            .map(|node_id| pipe_gas.total_amount_particles(*node_id))
            .collect::<Vec<_>>();
        let starting_pipe_species = component
            .iter()
            .map(|node_id| pipe_gas.node_species_counts(*node_id))
            .collect::<Vec<_>>();
        let vent_cells = component
            .iter()
            .map(|node_id| runtime.nodes[*node_id].vent_cell)
            .collect::<Vec<_>>();
        let vent_states = build_vent_pressure_states(
            config,
            pipe_gas.capacity_particles(),
            &starting_pipe_totals,
            &vent_cells,
            gas,
        );
        let (
            source_indices,
            component_target_fill,
            initial_world_to_pipe_requests,
            pipe_to_world_requests,
        ) = build_pressure_world_exchange_requests(
            config,
            pipe_gas.capacity_particles(),
            &starting_pipe_totals,
            &vent_states,
        );
        let has_active_sinks = pipe_to_world_requests.iter().any(|amount| *amount > 0);
        let profile =
            (!source_indices.is_empty() || has_active_sinks).then_some(PipeComponentVentProfile {
                inlet_indices: source_indices.clone(),
                outlet_indices: pipe_to_world_requests
                    .iter()
                    .enumerate()
                    .filter_map(|(index, amount)| (*amount > 0).then_some(index))
                    .collect(),
                target_fill: component_target_fill,
            });
        let raw_neighbor_requests = if !source_indices.is_empty() {
            build_demand_driven_pipe_transfer_plan(
                &runtime,
                &component,
                &starting_pipe_totals,
                &[],
                &initial_world_to_pipe_requests,
                &pipe_to_world_requests,
                profile.as_ref(),
            )
            .unwrap_or_else(|| {
                build_local_pipe_transfer_plan(
                    &runtime,
                    &component,
                    &starting_pipe_totals,
                    pipe_gas.capacity_particles(),
                )
            })
        } else if has_active_sinks {
            build_sink_oriented_pipe_transfer_plan(
                &runtime,
                &component,
                &starting_pipe_totals,
                &pipe_to_world_requests,
            )
        } else {
            build_local_pipe_transfer_plan(
                &runtime,
                &component,
                &starting_pipe_totals,
                pipe_gas.capacity_particles(),
            )
        };
        let neighbor_requests = raw_neighbor_requests
            .into_iter()
            .filter_map(|mut request| {
                if !has_active_sinks
                    && !source_indices.is_empty()
                    && component_target_fill > 0
                    && !source_indices.contains(&request.source_index)
                    && starting_pipe_totals[request.source_index] < component_target_fill
                {
                    return None;
                }
                request.amount = request.amount.min(config.edge_transfer_per_tick);
                (request.amount > 0).then_some(request)
            })
            .collect::<Vec<_>>();
        let world_to_pipe_requests = finalize_pressure_world_to_pipe_requests(
            config,
            pipe_gas.capacity_particles(),
            &starting_pipe_totals,
            &vent_states,
            &pipe_to_world_requests,
            &neighbor_requests,
            &initial_world_to_pipe_requests,
        );

        let mut next_pipe_species = starting_pipe_species.clone();
        let mut incoming_pipe_species = vec![vec![0u32; pipe_gas.gas_count()]; component.len()];
        let mut edge_transfers = Vec::<(PipeEdgePlan, Vec<u32>)>::new();

        for request in &neighbor_requests {
            if request.amount == 0 {
                continue;
            }
            let moved = remove_species_proportional_counts(
                &mut next_pipe_species[request.source_index],
                request.amount,
            );
            if moved.iter().any(|count| *count > 0) {
                edge_transfers.push((request.clone(), moved));
            }
        }

        for target_index in 0..component.len() {
            let incoming_edge_indices = edge_transfers
                .iter()
                .enumerate()
                .filter_map(|(edge_index, (request, species))| {
                    (request.target_index == target_index && species.iter().any(|count| *count > 0))
                        .then_some(edge_index)
                })
                .collect::<Vec<_>>();
            if incoming_edge_indices.is_empty() {
                continue;
            }

            let target_total_after_outgoing: u32 =
                next_pipe_species[target_index].iter().copied().sum();
            let target_capacity = pipe_gas
                .capacity_particles()
                .saturating_add(pipe_to_world_requests[target_index]);
            let free_for_incoming = target_capacity.saturating_sub(target_total_after_outgoing);
            let offered_totals = incoming_edge_indices
                .iter()
                .map(|edge_index| edge_transfers[*edge_index].1.iter().copied().sum::<u32>())
                .collect::<Vec<_>>();
            let accepted_totals = split_bounded_integer_requests(
                free_for_incoming.min(offered_totals.iter().copied().sum()),
                &offered_totals,
            );
            for (edge_index, accepted_total) in incoming_edge_indices
                .into_iter()
                .zip(accepted_totals.into_iter())
            {
                let (request, moved_species) = &edge_transfers[edge_index];
                let mut rejected_species = moved_species.clone();
                let accepted_species =
                    remove_species_proportional_counts(&mut rejected_species, accepted_total);
                for (gas_index, amount) in accepted_species.iter().copied().enumerate() {
                    incoming_pipe_species[target_index][gas_index] =
                        incoming_pipe_species[target_index][gas_index].saturating_add(amount);
                }
                if rejected_species.iter().any(|count| *count > 0) {
                    let _ = add_species_counts_limited(
                        &mut next_pipe_species[request.source_index],
                        &rejected_species,
                        pipe_gas.capacity_particles(),
                    );
                }

                let source_node_id = component[request.source_index];
                let target_node_id = component[request.target_index];
                if accepted_species.iter().any(|count| *count > 0) {
                    visual_state.transfers.push(PipeTransferRecord {
                        from: runtime.nodes[source_node_id].visual_cell,
                        to: runtime.nodes[target_node_id].visual_cell,
                        total_amount: accepted_species.iter().copied().sum(),
                        gas_counts: accepted_species,
                        visual_path: transfer_visual_path(
                            pipe_gas.keys[source_node_id],
                            pipe_gas.keys[target_node_id],
                            structures,
                        ),
                    });
                }
            }
        }

        for target_index in 0..component.len() {
            if incoming_pipe_species[target_index]
                .iter()
                .all(|count| *count == 0)
            {
                continue;
            }
            let _ = add_species_counts_limited(
                &mut next_pipe_species[target_index],
                &incoming_pipe_species[target_index],
                pipe_gas
                    .capacity_particles()
                    .saturating_add(pipe_to_world_requests[target_index]),
            );
        }

        for (component_index, node_id) in component.iter().copied().enumerate() {
            let pipe_out = pipe_to_world_requests[component_index];
            if pipe_out > 0 {
                let Some(cell) = vent_cells[component_index] else {
                    continue;
                };
                let moved = remove_species_proportional_counts(
                    &mut next_pipe_species[component_index],
                    pipe_out,
                );
                if moved.iter().any(|count| *count > 0) {
                    for (gas_index, amount) in moved.iter().copied().enumerate() {
                        if amount > 0 {
                            let _ = gas
                                .add_particles_no_impulse(cell.x, cell.y, gas_index, amount, world);
                        }
                    }
                    world_changed = true;
                }
            }

            let world_in = world_to_pipe_requests[component_index];
            if world_in > 0 {
                let Some(cell) = vent_cells[component_index] else {
                    continue;
                };
                let removed_counts =
                    gas.remove_particles_proportional_counts(cell.x, cell.y, world_in, world);
                if removed_counts.iter().any(|count| *count > 0) {
                    add_species_counts_limited(
                        &mut next_pipe_species[component_index],
                        &removed_counts,
                        pipe_gas.capacity_particles(),
                    );
                    world_changed = true;
                }
            }

            pipe_gas.set_species_counts_exact(node_id, &next_pipe_species[component_index]);
            pipe_changed |=
                next_pipe_species[component_index] != starting_pipe_species[component_index];
        }
    }

    if world_changed {
        gas.recompute_total_density_buffer(world);
    }

    pipe_changed || world_changed
}

pub(super) fn world_pressure(config: &PipeSimulationConfig, world_particles: u32) -> f32 {
    let _ = config;
    world_particles as f32
}

pub(super) fn pipe_pressure(config: &PipeSimulationConfig, pipe_particles: u32) -> f32 {
    pipe_particles as f32 * config.cell_volume_ratio.max(1.0)
}

fn raw_equilibrium_pipe_particles_for_world(
    config: &PipeSimulationConfig,
    world_particles: u32,
) -> u32 {
    let ratio = config.cell_volume_ratio.max(1.0);
    (world_particles as f32 / ratio).round() as u32
}

fn build_vent_pressure_states(
    config: &PipeSimulationConfig,
    capacity_particles: u32,
    starting_pipe_totals: &[u32],
    vent_cells: &[Option<UVec2>],
    gas: &GasField,
) -> Vec<Option<VentPressureState>> {
    starting_pipe_totals
        .iter()
        .enumerate()
        .map(|(index, pipe_total)| {
            let cell = vent_cells.get(index).and_then(|cell| *cell)?;
            let world_total = gas.total_amount_rounded(cell.x, cell.y);
            let world_pressure = world_pressure(config, world_total);
            let pipe_pressure = pipe_pressure(config, *pipe_total);
            let raw_equilibrium_pipe_particles =
                raw_equilibrium_pipe_particles_for_world(config, world_total);
            let equilibrium_pipe_particles = raw_equilibrium_pipe_particles.min(capacity_particles);
            Some(VentPressureState {
                world_total,
                world_pressure,
                pipe_pressure,
                raw_equilibrium_pipe_particles,
                equilibrium_pipe_particles,
                bootstrap_fill: *pipe_total == 0
                    && equilibrium_pipe_particles == capacity_particles,
            })
        })
        .collect()
}

fn build_pressure_world_exchange_requests(
    config: &PipeSimulationConfig,
    capacity_particles: u32,
    starting_pipe_totals: &[u32],
    vent_states: &[Option<VentPressureState>],
) -> (Vec<usize>, u32, Vec<u32>, Vec<u32>) {
    let mut source_indices = Vec::new();
    let mut world_to_pipe = vec![0u32; starting_pipe_totals.len()];
    let mut pipe_to_world = vec![0u32; starting_pipe_totals.len()];
    let component_target_fill = component_target_fill(capacity_particles, vent_states);

    for (index, state) in vent_states.iter().enumerate() {
        let Some(state) = state else {
            continue;
        };
        let delta = state.world_pressure - state.pipe_pressure;
        if state.equilibrium_pipe_particles > 0 && delta >= -config.pressure_epsilon {
            source_indices.push(index);
        }
        if delta > config.pressure_epsilon {
            let raise_by = state
                .equilibrium_pipe_particles
                .saturating_sub(starting_pipe_totals[index]);
            if raise_by > 0 {
                world_to_pipe[index] = if state.bootstrap_fill {
                    capacity_particles.min(state.world_total)
                } else {
                    config
                        .vent_transfer_per_tick
                        .min(raise_by)
                        .min(state.world_total)
                };
            }
        } else if delta < -config.pressure_epsilon {
            let drain_by =
                starting_pipe_totals[index].saturating_sub(state.equilibrium_pipe_particles);
            if drain_by > 0 {
                pipe_to_world[index] = config.vent_transfer_per_tick.min(drain_by);
            }
        }
    }

    (
        source_indices,
        component_target_fill,
        world_to_pipe,
        pipe_to_world,
    )
}

fn component_target_fill(
    capacity_particles: u32,
    vent_states: &[Option<VentPressureState>],
) -> u32 {
    let connected = vent_states
        .iter()
        .filter_map(|state| state.as_ref())
        .collect::<Vec<_>>();
    if connected.is_empty() {
        return capacity_particles;
    }
    if connected.len() == 1 {
        return connected[0]
            .equilibrium_pipe_particles
            .min(capacity_particles);
    }
    let total: u64 = connected
        .iter()
        .map(|state| u64::from(state.raw_equilibrium_pipe_particles))
        .sum();
    ((total as f32 / connected.len() as f32).round() as u32).min(capacity_particles)
}

fn finalize_pressure_world_to_pipe_requests(
    config: &PipeSimulationConfig,
    capacity_particles: u32,
    starting_pipe_totals: &[u32],
    vent_states: &[Option<VentPressureState>],
    pipe_to_world_requests: &[u32],
    neighbor_requests: &[PipeEdgePlan],
    initial_world_to_pipe_requests: &[u32],
) -> Vec<u32> {
    let mut world_to_pipe = initial_world_to_pipe_requests.to_vec();
    let mut outgoing_by_node = vec![0u32; starting_pipe_totals.len()];
    for request in neighbor_requests {
        outgoing_by_node[request.source_index] =
            outgoing_by_node[request.source_index].saturating_add(request.amount);
    }
    for (index, amount) in pipe_to_world_requests.iter().copied().enumerate() {
        outgoing_by_node[index] = outgoing_by_node[index].saturating_add(amount);
    }

    for (index, state) in vent_states.iter().enumerate() {
        let Some(state) = state else {
            continue;
        };
        if state.world_pressure + config.pressure_epsilon < state.pipe_pressure {
            continue;
        }
        let projected_total = starting_pipe_totals[index].saturating_sub(outgoing_by_node[index]);
        let room_to_target = state
            .equilibrium_pipe_particles
            .saturating_sub(projected_total)
            .min(capacity_particles.saturating_sub(projected_total));
        if room_to_target == 0 {
            continue;
        }
        let refill = if state.bootstrap_fill && projected_total == 0 {
            capacity_particles.min(state.world_total)
        } else {
            config
                .vent_transfer_per_tick
                .min(room_to_target)
                .min(state.world_total)
        };
        world_to_pipe[index] = world_to_pipe[index].max(refill);
    }

    world_to_pipe
}
