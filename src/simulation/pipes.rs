use std::collections::VecDeque;

use bevy::prelude::*;

use crate::{
    config::GasRegistry,
    simulation::gas::GasField,
    world::{
        grid::{linear_index, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
        pipes::PipeGrid,
    },
};

pub(crate) const PIPE_CELL_CAPACITY: u32 = 1_000;
/// Minimum rounded pressure delta between vents required to activate pipe flow.
pub(crate) const MIN_VENT_PRESSURE_DELTA_PARTICLES: u32 = 5;
const PIPE_LOCAL_TRANSFER_NUMERATOR: u32 = 1;
const PIPE_LOCAL_TRANSFER_DENOMINATOR: u32 = 2;

#[derive(Clone, Debug)]
/// Stores `PipeGasSnapshot` state.
pub struct PipeGasSnapshot {
    pub gas_count: usize,
    pub species: Vec<u32>,
}

#[derive(Resource, Clone)]
/// Stores `PipeGasField` state.
pub struct PipeGasField {
    cells: Vec<Vec<u32>>,
    gas_count: usize,
}

impl Default for PipeGasField {
    fn default() -> Self {
        Self {
            cells: vec![Vec::new(); (WORLD_WIDTH * WORLD_HEIGHT) as usize],
            gas_count: 0,
        }
    }
}

impl PipeGasField {
    /// Runs `from_registry` logic.
    pub fn from_registry(registry: &GasRegistry) -> Self {
        let gas_count = registry.count();
        let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        Self {
            cells: vec![vec![0; gas_count]; cells],
            gas_count,
        }
    }

    /// Runs `gas_count` logic.
    pub fn gas_count(&self) -> usize {
        self.gas_count
    }

    /// Runs `amount_particles` logic.
    pub fn amount_particles(&self, x: u32, y: u32, gas_index: usize) -> u32 {
        self.cells[linear_index(x, y)]
            .get(gas_index)
            .copied()
            .unwrap_or(0)
    }

    /// Runs `total_amount_particles` logic.
    pub fn total_amount_particles(&self, x: u32, y: u32) -> u32 {
        self.cells[linear_index(x, y)].iter().copied().sum::<u32>()
    }

    /// Runs `free_capacity` logic.
    pub fn free_capacity(&self, x: u32, y: u32) -> u32 {
        PIPE_CELL_CAPACITY.saturating_sub(self.total_amount_particles(x, y))
    }

    /// Runs `clear_cell` logic.
    pub fn clear_cell(&mut self, x: u32, y: u32) {
        let idx = linear_index(x, y);
        if self.gas_count == 0 {
            self.cells[idx].clear();
            return;
        }
        self.cells[idx].fill(0);
    }

    /// Runs `clear_all` logic.
    pub fn clear_all(&mut self) {
        for cell in &mut self.cells {
            if self.gas_count == 0 {
                cell.clear();
            } else {
                cell.fill(0);
            }
        }
    }

    fn species_counts(&self, x: u32, y: u32) -> Vec<u32> {
        self.cells[linear_index(x, y)].clone()
    }

    fn set_species_counts_exact(&mut self, x: u32, y: u32, species: &[u32]) {
        let idx = linear_index(x, y);
        if self.gas_count == 0 {
            self.cells[idx].clear();
            return;
        }
        self.cells[idx].fill(0);
        for gas_index in 0..self.gas_count {
            self.cells[idx][gas_index] = species.get(gas_index).copied().unwrap_or(0);
        }
    }

    /// Runs `remove_particles_proportional_counts` logic.
    pub fn remove_particles_proportional_counts(
        &mut self,
        x: u32,
        y: u32,
        amount: u32,
    ) -> Vec<u32> {
        if amount == 0 || self.gas_count == 0 {
            return vec![0; self.gas_count];
        }
        let idx = linear_index(x, y);
        let cell = &mut self.cells[idx];
        let total: u64 = cell.iter().map(|&value| u64::from(value)).sum();
        if total == 0 {
            return vec![0; self.gas_count];
        }

        let remove = u64::from(amount).min(total);
        if remove == total {
            let removed = cell.clone();
            cell.fill(0);
            return removed;
        }

        let mut removed = vec![0u32; self.gas_count];
        let mut remainders = vec![0u64; self.gas_count];
        let mut removed_base = 0u64;
        for gas_index in 0..self.gas_count {
            let numerator = u128::from(cell[gas_index]) * u128::from(remove);
            let base = (numerator / u128::from(total)) as u64;
            removed[gas_index] = base.min(u64::from(cell[gas_index])) as u32;
            remainders[gas_index] = (numerator % u128::from(total)) as u64;
            removed_base = removed_base.saturating_add(u64::from(removed[gas_index]));
        }

        let mut remaining = remove.saturating_sub(removed_base);
        while remaining > 0 {
            let mut best_index = None;
            let mut best_remainder = 0u64;
            for gas_index in 0..self.gas_count {
                if removed[gas_index] >= cell[gas_index] {
                    continue;
                }
                if best_index.is_none() || remainders[gas_index] > best_remainder {
                    best_index = Some(gas_index);
                    best_remainder = remainders[gas_index];
                }
            }
            let Some(best) = best_index else {
                break;
            };
            removed[best] = removed[best].saturating_add(1);
            remainders[best] = 0;
            remaining -= 1;
        }

        for gas_index in 0..self.gas_count {
            cell[gas_index] = cell[gas_index].saturating_sub(removed[gas_index]);
        }
        removed
    }

    /// Runs `add_species_counts_limited` logic.
    pub fn add_species_counts_limited(&mut self, x: u32, y: u32, offered: &[u32]) -> Vec<u32> {
        let mut accepted = vec![0u32; self.gas_count];
        if self.gas_count == 0 || offered.is_empty() {
            return accepted;
        }
        let idx = linear_index(x, y);
        let total_offered: u32 = offered.iter().take(self.gas_count).copied().sum();
        let accept_total = self.free_capacity(x, y).min(total_offered);
        if accept_total == 0 {
            return accepted;
        }

        if accept_total == total_offered {
            for gas_index in 0..self.gas_count {
                accepted[gas_index] = offered.get(gas_index).copied().unwrap_or(0);
                self.cells[idx][gas_index] =
                    self.cells[idx][gas_index].saturating_add(accepted[gas_index]);
            }
            return accepted;
        }

        let weights = offered
            .iter()
            .take(self.gas_count)
            .map(|&value| value as f32)
            .collect::<Vec<_>>();
        accepted = split_integer_by_weights(accept_total, &weights);
        for gas_index in 0..self.gas_count {
            accepted[gas_index] =
                accepted[gas_index].min(offered.get(gas_index).copied().unwrap_or(0));
        }
        let mut current_total: u32 = accepted.iter().copied().sum();
        while current_total < accept_total {
            let mut changed = false;
            for gas_index in 0..self.gas_count {
                let available = offered.get(gas_index).copied().unwrap_or(0);
                if accepted[gas_index] < available {
                    accepted[gas_index] = accepted[gas_index].saturating_add(1);
                    current_total += 1;
                    changed = true;
                    if current_total == accept_total {
                        break;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        for gas_index in 0..self.gas_count {
            self.cells[idx][gas_index] =
                self.cells[idx][gas_index].saturating_add(accepted[gas_index]);
        }
        accepted
    }

    /// Runs `snapshot_state` logic.
    pub fn snapshot_state(&self) -> PipeGasSnapshot {
        let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        let mut species = Vec::with_capacity(cells * self.gas_count);
        for idx in 0..cells {
            for gas_index in 0..self.gas_count {
                species.push(self.cells[idx][gas_index]);
            }
        }
        PipeGasSnapshot {
            gas_count: self.gas_count,
            species,
        }
    }

    /// Runs `restore_state` logic.
    pub fn restore_state(&mut self, snapshot: &PipeGasSnapshot) -> Result<(), String> {
        let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        if snapshot.gas_count != self.gas_count {
            return Err(format!(
                "Pipe gas snapshot gas_count mismatch: got {}, expected {}",
                snapshot.gas_count, self.gas_count
            ));
        }
        if snapshot.species.len() != cells * self.gas_count {
            return Err(format!(
                "Pipe gas snapshot length mismatch: got {}, expected {}",
                snapshot.species.len(),
                cells * self.gas_count
            ));
        }

        for idx in 0..cells {
            let base = idx * self.gas_count;
            let total: u32 = snapshot.species[base..base + self.gas_count]
                .iter()
                .copied()
                .sum();
            if total > PIPE_CELL_CAPACITY {
                return Err(format!(
                    "Pipe gas snapshot exceeds capacity at cell index {}: {} > {}",
                    idx, total, PIPE_CELL_CAPACITY
                ));
            }
            for gas_index in 0..self.gas_count {
                self.cells[idx][gas_index] = snapshot.species[base + gas_index];
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
/// Stores `PipeTransferRecord` state.
pub struct PipeTransferRecord {
    pub from: UVec2,
    pub to: UVec2,
    pub gas_counts: Vec<u32>,
    pub total_amount: u32,
}

#[derive(Resource, Default, Clone)]
/// Stores `PipeFlowVisualState` state.
pub struct PipeFlowVisualState {
    pub transfers: Vec<PipeTransferRecord>,
}

/// Builds display-ready per-gas counts for a pipe cell.
///
/// The result combines persistent `PipeGasField` contents with transient flow packets
/// from the latest tick so HUD/overlay can show gas presence even while it is actively moving.
#[cfg(test)]
pub(crate) fn pipe_cell_display_species_counts(
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    x: u32,
    y: u32,
) -> Vec<u32> {
    pipe_cell_display_species_counts_with_transfers(pipe_gas, flow_state, x, y, true)
}

/// Builds display-ready per-gas counts for a pipe cell with optional transient flow packets.
pub(crate) fn pipe_cell_display_species_counts_with_transfers(
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    x: u32,
    y: u32,
    include_transfers: bool,
) -> Vec<u32> {
    let gas_count = pipe_gas.gas_count();
    let mut displayed = (0..gas_count)
        .map(|gas_index| pipe_gas.amount_particles(x, y, gas_index))
        .collect::<Vec<_>>();
    if !include_transfers {
        return displayed;
    }
    let mut incoming = vec![0u32; gas_count];
    let mut outgoing = vec![0u32; gas_count];

    for transfer in &flow_state.transfers {
        if transfer.to == UVec2::new(x, y) {
            for gas_index in 0..gas_count {
                incoming[gas_index] = incoming[gas_index]
                    .saturating_add(transfer.gas_counts.get(gas_index).copied().unwrap_or(0));
            }
        }
        if transfer.from == UVec2::new(x, y) {
            for gas_index in 0..gas_count {
                outgoing[gas_index] = outgoing[gas_index]
                    .saturating_add(transfer.gas_counts.get(gas_index).copied().unwrap_or(0));
            }
        }
    }

    for gas_index in 0..gas_count {
        displayed[gas_index] = displayed[gas_index]
            .max(incoming[gas_index])
            .max(outgoing[gas_index]);
    }

    displayed
}

/// Sums display-ready gas counts for a pipe cell.
#[cfg(test)]
pub(crate) fn pipe_cell_display_total_particles(
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    x: u32,
    y: u32,
) -> u32 {
    pipe_cell_display_total_particles_with_transfers(pipe_gas, flow_state, x, y, true)
}

/// Sums display-ready gas counts for a pipe cell with optional transient flow packets.
pub(crate) fn pipe_cell_display_total_particles_with_transfers(
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    x: u32,
    y: u32,
    include_transfers: bool,
) -> u32 {
    pipe_cell_display_species_counts_with_transfers(pipe_gas, flow_state, x, y, include_transfers)
        .into_iter()
        .sum()
}

/// Runs `apply_pipe_network_step` logic.
pub fn apply_pipe_network_step(
    layout: &PipeGrid,
    pipe_gas: &mut PipeGasField,
    gas: &mut GasField,
    world: &WorldGrid,
    visual_state: &mut PipeFlowVisualState,
) -> bool {
    visual_state.transfers.clear();
    if pipe_gas.gas_count() == 0 {
        return false;
    }

    let components = collect_pipe_components(layout);
    let mut world_changed = false;
    let mut pipe_changed = false;

    for component in components {
        if component.is_empty() {
            continue;
        }
        let starting_pipe_totals = component
            .iter()
            .map(|cell| pipe_gas.total_amount_particles(cell.x, cell.y))
            .collect::<Vec<_>>();
        let starting_pipe_species = component
            .iter()
            .map(|cell| pipe_gas.species_counts(cell.x, cell.y))
            .collect::<Vec<_>>();
        let starting_world_totals = component
            .iter()
            .map(|cell| {
                let pipe_cell = layout.cell(cell.x, cell.y);
                if pipe_cell.has_pipe && pipe_cell.has_vent {
                    Some(gas.total_amount_rounded(cell.x, cell.y))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let mut neighbor_requests = Vec::<PipeNeighborRequest>::new();
        let mut world_to_pipe_requests = vec![0u32; component.len()];
        let mut pipe_to_world_requests = vec![0u32; component.len()];
        let mut accepted_outgoing = vec![0u32; component.len()];

        for (source_index, cell) in component.iter().copied().enumerate() {
            let source_total = starting_pipe_totals[source_index];
            let mut outgoing_requests = Vec::<OutgoingRequest>::new();

            for neighbor in layout.connected_neighbors(cell.x, cell.y) {
                let Some(target_index) = component
                    .iter()
                    .position(|candidate| *candidate == neighbor)
                else {
                    continue;
                };
                let target_total = starting_pipe_totals[target_index];
                if source_total <= target_total {
                    continue;
                }
                let delta = source_total - target_total;
                outgoing_requests.push(OutgoingRequest {
                    target: OutgoingTarget::Pipe(target_index),
                    pressure_delta: delta,
                });
            }

            if let Some(world_total) = starting_world_totals[source_index] {
                if source_total > world_total {
                    let delta = source_total - world_total;
                    if delta >= MIN_VENT_PRESSURE_DELTA_PARTICLES {
                        outgoing_requests.push(OutgoingRequest {
                            target: OutgoingTarget::World,
                            pressure_delta: delta,
                        });
                    }
                } else if world_total > source_total {
                    let delta = world_total - source_total;
                    if delta >= MIN_VENT_PRESSURE_DELTA_PARTICLES {
                        world_to_pipe_requests[source_index] =
                            desired_local_transfer_amount(delta).min(world_total);
                    }
                }
            }

            if outgoing_requests.is_empty() || source_total == 0 {
                continue;
            }

            let combined_delta = combined_outgoing_delta(
                &outgoing_requests
                    .iter()
                    .map(|request| request.pressure_delta)
                    .collect::<Vec<_>>(),
            );
            let desired_total = desired_local_transfer_amount(combined_delta);
            if desired_total == 0 {
                continue;
            }
            let capped_total = desired_total.min(source_total);
            if capped_total == 0 {
                continue;
            }
            let split = split_integer_by_weights(
                capped_total,
                &outgoing_requests
                    .iter()
                    .map(|request| request.pressure_delta as f32)
                    .collect::<Vec<_>>(),
            );

            for (request, accepted_amount) in outgoing_requests.into_iter().zip(split.into_iter()) {
                if accepted_amount == 0 {
                    continue;
                }
                accepted_outgoing[source_index] =
                    accepted_outgoing[source_index].saturating_add(accepted_amount);
                match request.target {
                    OutgoingTarget::Pipe(target_index) => {
                        neighbor_requests.push(PipeNeighborRequest {
                            source_index,
                            target_index,
                            amount: accepted_amount,
                        });
                    }
                    OutgoingTarget::World => {
                        pipe_to_world_requests[source_index] = accepted_amount;
                    }
                }
            }
        }

        let mut capped_neighbor_requests = vec![0u32; neighbor_requests.len()];
        let mut capped_world_to_pipe = vec![0u32; component.len()];
        for target_index in 0..component.len() {
            let start_total = starting_pipe_totals[target_index];
            let room = PIPE_CELL_CAPACITY
                .saturating_sub(start_total)
                .saturating_add(accepted_outgoing[target_index]);

            let incoming_neighbors = neighbor_requests
                .iter()
                .enumerate()
                .filter_map(|(request_index, request)| {
                    (request.target_index == target_index)
                        .then_some((request_index, request.amount))
                })
                .collect::<Vec<_>>();
            let total_incoming_neighbors: u32 =
                incoming_neighbors.iter().map(|(_, amount)| *amount).sum();
            let world_in = world_to_pipe_requests[target_index];
            let total_requested_in = total_incoming_neighbors.saturating_add(world_in);
            if total_requested_in == 0 {
                continue;
            }
            if total_requested_in <= room {
                for (request_index, amount) in incoming_neighbors {
                    capped_neighbor_requests[request_index] = amount;
                }
                capped_world_to_pipe[target_index] = world_in;
                continue;
            }

            let mut request_amounts = incoming_neighbors
                .iter()
                .map(|(_, amount)| *amount)
                .collect::<Vec<_>>();
            if world_in > 0 {
                request_amounts.push(world_in);
            }
            let capped = split_integer_by_weights(
                room,
                &request_amounts
                    .iter()
                    .map(|amount| *amount as f32)
                    .collect::<Vec<_>>(),
            );
            let neighbor_count = incoming_neighbors.len();
            for ((request_index, _), accepted_amount) in
                incoming_neighbors.into_iter().zip(capped.iter().copied())
            {
                capped_neighbor_requests[request_index] = accepted_amount;
            }
            if world_in > 0 {
                capped_world_to_pipe[target_index] =
                    capped.get(neighbor_count).copied().unwrap_or(0);
            }
        }

        let mut next_component_species = starting_pipe_species.clone();
        let mut pipe_remaining = starting_pipe_species.clone();
        let mut removed_from_pipe = vec![vec![0u32; pipe_gas.gas_count()]; component.len()];
        let mut world_to_pipe_applied = false;
        let mut pipe_to_world_applied = false;
        let mut moved_between_pipes = false;

        let mut sorted_neighbor_requests = neighbor_requests
            .into_iter()
            .enumerate()
            .map(|(request_index, request)| (request_index, request))
            .collect::<Vec<_>>();
        sorted_neighbor_requests
            .sort_by_key(|(_, request)| (request.source_index, request.target_index));
        for (request_index, request) in sorted_neighbor_requests {
            let amount = capped_neighbor_requests[request_index];
            if amount == 0 {
                continue;
            }
            let gas_counts = remove_particles_proportional_from_counts(
                &mut pipe_remaining[request.source_index],
                amount,
            );
            let total_amount: u32 = gas_counts.iter().copied().sum();
            if total_amount == 0 {
                continue;
            }
            add_species_counts(&mut removed_from_pipe[request.source_index], &gas_counts);
            add_species_counts(
                &mut next_component_species[request.target_index],
                &gas_counts,
            );
            moved_between_pipes = true;
            pipe_changed = true;
            visual_state.transfers.push(PipeTransferRecord {
                from: component[request.source_index],
                to: component[request.target_index],
                gas_counts,
                total_amount,
            });
        }

        for (source_index, amount) in pipe_to_world_requests.iter().copied().enumerate() {
            if amount == 0 {
                continue;
            }
            let gas_counts = remove_particles_proportional_from_counts(
                &mut pipe_remaining[source_index],
                amount,
            );
            let total_amount: u32 = gas_counts.iter().copied().sum();
            if total_amount == 0 {
                continue;
            }
            add_species_counts(&mut removed_from_pipe[source_index], &gas_counts);
            let added_to_world =
                add_pipe_mix_to_world(gas, world, component[source_index], &gas_counts);
            if added_to_world > 0 {
                world_changed = true;
                pipe_changed = true;
                pipe_to_world_applied = true;
            }
        }

        for (source_index, removed_mix) in removed_from_pipe.iter().enumerate() {
            subtract_species_counts(&mut next_component_species[source_index], removed_mix);
        }

        for (target_index, amount) in capped_world_to_pipe.iter().copied().enumerate() {
            if amount == 0 {
                continue;
            }
            let world_mix = gas.remove_particles_proportional_counts(
                component[target_index].x,
                component[target_index].y,
                amount,
                world,
            );
            let accepted_total: u32 = world_mix.iter().copied().sum();
            if accepted_total == 0 {
                continue;
            }
            add_species_counts(&mut next_component_species[target_index], &world_mix);
            world_changed = true;
            pipe_changed = true;
            world_to_pipe_applied = true;
        }

        for (index, cell) in component.iter().copied().enumerate() {
            pipe_gas.set_species_counts_exact(cell.x, cell.y, &next_component_species[index]);
        }

        if !moved_between_pipes && !pipe_to_world_applied && !world_to_pipe_applied {
            continue;
        }
    }

    if world_changed {
        gas.recompute_total_density_buffer(world);
    }

    world_changed || pipe_changed
}

fn desired_local_transfer_amount(delta: u32) -> u32 {
    if delta == 0 {
        return 0;
    }
    ((u64::from(delta) * u64::from(PIPE_LOCAL_TRANSFER_NUMERATOR))
        / u64::from(PIPE_LOCAL_TRANSFER_DENOMINATOR))
    .min(u64::from(u32::MAX)) as u32
}

fn combined_outgoing_delta(deltas: &[u32]) -> u32 {
    if deltas.is_empty() {
        return 0;
    }
    let sum = deltas.iter().copied().map(u64::from).sum::<u64>();
    (sum / deltas.len() as u64).min(u64::from(u32::MAX)) as u32
}

fn remove_particles_proportional_from_counts(counts: &mut [u32], amount: u32) -> Vec<u32> {
    if amount == 0 || counts.is_empty() {
        return vec![0; counts.len()];
    }
    let gas_count = counts.len();
    let total: u64 = counts.iter().map(|&value| u64::from(value)).sum();
    if total == 0 {
        return vec![0; gas_count];
    }

    let remove = u64::from(amount).min(total);
    if remove == total {
        let removed = counts.to_vec();
        counts.fill(0);
        return removed;
    }

    let mut removed = vec![0u32; gas_count];
    let mut remainders = vec![0u64; gas_count];
    let mut removed_base = 0u64;
    for gas_index in 0..gas_count {
        let numerator = u128::from(counts[gas_index]) * u128::from(remove);
        let base = (numerator / u128::from(total)) as u64;
        removed[gas_index] = base.min(u64::from(counts[gas_index])) as u32;
        remainders[gas_index] = (numerator % u128::from(total)) as u64;
        removed_base = removed_base.saturating_add(u64::from(removed[gas_index]));
    }

    let mut remaining = remove.saturating_sub(removed_base);
    while remaining > 0 {
        let mut best_index = None;
        let mut best_remainder = 0u64;
        for gas_index in 0..gas_count {
            if removed[gas_index] >= counts[gas_index] {
                continue;
            }
            if best_index.is_none() || remainders[gas_index] > best_remainder {
                best_index = Some(gas_index);
                best_remainder = remainders[gas_index];
            }
        }
        let Some(best) = best_index else {
            break;
        };
        removed[best] = removed[best].saturating_add(1);
        remainders[best] = 0;
        remaining -= 1;
    }

    for gas_index in 0..gas_count {
        counts[gas_index] = counts[gas_index].saturating_sub(removed[gas_index]);
    }
    removed
}

fn collect_pipe_components(layout: &PipeGrid) -> Vec<Vec<UVec2>> {
    let mut visited = vec![false; (WORLD_WIDTH * WORLD_HEIGHT) as usize];
    let mut components = Vec::new();
    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            let idx = linear_index(x, y);
            if visited[idx] || !layout.has_pipe(x, y) {
                continue;
            }
            visited[idx] = true;
            let mut queue = VecDeque::from([UVec2::new(x, y)]);
            let mut component = Vec::new();
            while let Some(cell) = queue.pop_front() {
                component.push(cell);
                for neighbor in layout.connected_neighbors(cell.x, cell.y) {
                    let neighbor_idx = linear_index(neighbor.x, neighbor.y);
                    if visited[neighbor_idx] {
                        continue;
                    }
                    visited[neighbor_idx] = true;
                    queue.push_back(neighbor);
                }
            }
            components.push(component);
        }
    }
    components
}

fn add_pipe_mix_to_world(
    gas: &mut GasField,
    world: &WorldGrid,
    cell: UVec2,
    offered: &[u32],
) -> u32 {
    let mut added_total = 0u32;
    for (gas_index, amount) in offered.iter().copied().enumerate() {
        if amount == 0 {
            continue;
        }
        added_total = added_total
            .saturating_add(gas.add_particles_no_impulse(cell.x, cell.y, gas_index, amount, world));
    }
    added_total
}

fn add_species_counts(target: &mut [u32], added: &[u32]) {
    for (target_value, added_value) in target.iter_mut().zip(added.iter().copied()) {
        *target_value = target_value.saturating_add(added_value);
    }
}

fn subtract_species_counts(target: &mut [u32], removed: &[u32]) {
    for (target_value, removed_value) in target.iter_mut().zip(removed.iter().copied()) {
        *target_value = target_value.saturating_sub(removed_value);
    }
}

fn split_integer_by_weights(total: u32, weights: &[f32]) -> Vec<u32> {
    if total == 0 || weights.is_empty() {
        return vec![0; weights.len()];
    }
    let positive_total = weights
        .iter()
        .map(|weight| weight.max(0.0))
        .sum::<f32>()
        .max(f32::EPSILON);
    let mut out = vec![0u32; weights.len()];
    let mut remainders = vec![0.0f32; weights.len()];
    let mut base_sum = 0u32;

    for (index, weight) in weights.iter().copied().enumerate() {
        let raw = total as f32 * weight.max(0.0) / positive_total;
        let base = raw.floor() as u32;
        out[index] = base;
        remainders[index] = raw - base as f32;
        base_sum = base_sum.saturating_add(base);
    }

    let mut remaining = total.saturating_sub(base_sum);
    while remaining > 0 {
        let mut best_index = 0usize;
        let mut best_remainder = -1.0f32;
        for (index, remainder) in remainders.iter().copied().enumerate() {
            if remainder > best_remainder {
                best_index = index;
                best_remainder = remainder;
            }
        }
        out[best_index] = out[best_index].saturating_add(1);
        remainders[best_index] = -1.0;
        remaining -= 1;
    }

    out
}

#[derive(Clone, Copy)]
struct PipeNeighborRequest {
    source_index: usize,
    target_index: usize,
    amount: u32,
}

#[derive(Clone, Copy)]
struct OutgoingRequest {
    target: OutgoingTarget,
    pressure_delta: u32,
}

#[derive(Clone, Copy)]
enum OutgoingTarget {
    Pipe(usize),
    World,
}

#[cfg(test)]
mod tests {
    use super::{
        apply_pipe_network_step, combined_outgoing_delta, desired_local_transfer_amount,
        pipe_cell_display_species_counts, pipe_cell_display_species_counts_with_transfers,
        pipe_cell_display_total_particles, pipe_cell_display_total_particles_with_transfers,
        PipeFlowVisualState, PipeGasField, MIN_VENT_PRESSURE_DELTA_PARTICLES, PIPE_CELL_CAPACITY,
    };
    use crate::{
        config::{GasDefinition, GasRegistry},
        simulation::gas::GasField,
        world::{
            gas_structures::GasStructureGrid,
            grid::{WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
            pipes::PipeGrid,
        },
    };
    use bevy::prelude::UVec2;

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
                label: "CO2".to_string(),
                color: [0.9, 0.6, 0.4],
                molecular_mass: 44.009,
            },
        ])
        .expect("test registry")
    }

    fn connect(grid: &mut PipeGrid, a: UVec2, b: UVec2) {
        assert!(grid.add_connection(a, b));
    }

    fn build_line(
        grid: &mut PipeGrid,
        world: &WorldGrid,
        structures: &GasStructureGrid,
        xs: &[u32],
        y: u32,
    ) {
        for &x in xs {
            assert!(grid.set_pipe(x, y, world, structures));
        }
        for pair in xs.windows(2) {
            connect(grid, UVec2::new(pair[0], y), UVec2::new(pair[1], y));
        }
    }

    fn total_world_and_pipe_particles(gas: &GasField, pipe_gas: &PipeGasField) -> u64 {
        let mut total = 0u64;
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                total = total
                    .saturating_add(gas.total_amount_particles(x, y))
                    .saturating_add(u64::from(pipe_gas.total_amount_particles(x, y)));
            }
        }
        total
    }

    #[test]
    fn single_vent_pipe_draws_gas_into_first_segment() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        build_line(&mut layout, &world, &structures, &[10, 11, 12], 10);
        assert!(layout.set_vent(10, 10, &world, &structures));
        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(10, 10, 0, 80.0);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();

        assert!(apply_pipe_network_step(
            &layout,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals
        ));
        assert_eq!(pipe_gas.total_amount_particles(10, 10), 40);
        assert_eq!(pipe_gas.total_amount_particles(11, 10), 0);
        assert_eq!(gas.total_amount_rounded(10, 10), 40);
        assert!(visuals.transfers.is_empty());
    }

    #[test]
    fn straight_pipe_front_advances_without_multi_hop() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        build_line(&mut layout, &world, &structures, &[20, 21, 22, 23], 20);
        assert!(layout.set_vent(20, 20, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(20, 20, 0, 120.0);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();

        assert!(apply_pipe_network_step(
            &layout,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals
        ));
        assert!(visuals.transfers.is_empty());
        assert_eq!(pipe_gas.total_amount_particles(20, 20), 60);
        assert_eq!(pipe_gas.total_amount_particles(21, 20), 0);

        assert!(apply_pipe_network_step(
            &layout,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals
        ));
        assert!(visuals.transfers.iter().any(
            |transfer| transfer.from == UVec2::new(20, 20) && transfer.to == UVec2::new(21, 20)
        ));
        assert!(!visuals.transfers.iter().any(
            |transfer| transfer.from == UVec2::new(21, 20) && transfer.to == UVec2::new(22, 20)
        ));
        assert_eq!(pipe_gas.total_amount_particles(22, 20), 0);

        let mut reached_third_segment = false;
        for _ in 0..6 {
            let _ = apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
            reached_third_segment |= pipe_gas.total_amount_particles(22, 20) > 0
                || visuals.transfers.iter().any(|transfer| {
                    transfer.from == UVec2::new(21, 20) && transfer.to == UVec2::new(22, 20)
                });
        }
        assert!(reached_third_segment);
    }

    #[test]
    fn two_vents_exchange_gas_through_local_deltas() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        build_line(&mut layout, &world, &structures, &[30, 31, 32], 30);
        assert!(layout.set_vent(30, 30, &world, &structures));
        assert!(layout.set_vent(32, 30, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(30, 30, 0, 120.0);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();

        let mut right_world_received = false;
        for _ in 0..12 {
            let changed =
                apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
            right_world_received |= gas.total_amount_rounded(32, 30) > 0;
            if !changed {
                break;
            }
        }
        assert!(right_world_received);
    }

    #[test]
    fn t_junction_splits_flow_by_downstream_deficit() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        for cell in [UVec2::new(40, 40), UVec2::new(41, 40), UVec2::new(40, 39)] {
            assert!(layout.set_pipe(cell.x, cell.y, &world, &structures));
        }
        for (a, b) in [
            (UVec2::new(40, 40), UVec2::new(41, 40)),
            (UVec2::new(40, 40), UVec2::new(40, 39)),
        ] {
            connect(&mut layout, a, b);
        }
        let mut gas = GasField::from_registry(&registry);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        assert_eq!(
            pipe_gas.add_species_counts_limited(40, 40, &[60, 0, 0]),
            vec![60, 0, 0]
        );
        let mut visuals = PipeFlowVisualState::default();

        assert!(apply_pipe_network_step(
            &layout,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals
        ));
        assert_eq!(pipe_gas.total_amount_particles(40, 40), 30);
        assert_eq!(pipe_gas.total_amount_particles(41, 40), 15);
        assert_eq!(pipe_gas.total_amount_particles(40, 39), 15);
        assert_eq!(visuals.transfers.len(), 2);
    }

    #[test]
    fn three_vent_t_junction_keeps_gas_in_center_segment() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        for cell in [UVec2::new(44, 44), UVec2::new(45, 44), UVec2::new(44, 43)] {
            assert!(layout.set_pipe(cell.x, cell.y, &world, &structures));
        }
        for (a, b) in [
            (UVec2::new(44, 44), UVec2::new(45, 44)),
            (UVec2::new(44, 44), UVec2::new(44, 43)),
        ] {
            connect(&mut layout, a, b);
        }
        assert!(layout.set_vent(44, 44, &world, &structures));
        assert!(layout.set_vent(45, 44, &world, &structures));
        assert!(layout.set_vent(44, 43, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        assert_eq!(
            pipe_gas.add_species_counts_limited(44, 44, &[60, 0, 0]),
            vec![60, 0, 0]
        );
        let mut visuals = PipeFlowVisualState::default();

        assert!(apply_pipe_network_step(
            &layout,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals
        ));
        assert!(pipe_gas.total_amount_particles(44, 44) > 0);
        assert!(pipe_gas.total_amount_particles(45, 44) > 0);
        assert!(pipe_gas.total_amount_particles(44, 43) > 0);
    }

    #[test]
    fn parallel_paths_and_multiple_vents_stay_deterministic() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        for cell in [
            UVec2::new(50, 50),
            UVec2::new(51, 50),
            UVec2::new(52, 50),
            UVec2::new(50, 51),
            UVec2::new(51, 51),
            UVec2::new(52, 51),
        ] {
            assert!(layout.set_pipe(cell.x, cell.y, &world, &structures));
        }
        for (a, b) in [
            (UVec2::new(50, 50), UVec2::new(51, 50)),
            (UVec2::new(51, 50), UVec2::new(52, 50)),
            (UVec2::new(50, 51), UVec2::new(51, 51)),
            (UVec2::new(51, 51), UVec2::new(52, 51)),
            (UVec2::new(50, 50), UVec2::new(50, 51)),
            (UVec2::new(52, 50), UVec2::new(52, 51)),
        ] {
            connect(&mut layout, a, b);
        }
        assert!(layout.set_vent(50, 50, &world, &structures));
        assert!(layout.set_vent(52, 50, &world, &structures));
        assert!(layout.set_vent(52, 51, &world, &structures));

        let mut gas_a = GasField::from_registry(&registry);
        gas_a.set_amount(50, 50, 0, 200.0);
        gas_a.set_amount(52, 51, 1, 50.0);
        gas_a.recompute_total_density_buffer(&world);
        let mut gas_b = gas_a.clone();
        let mut pipe_a = PipeGasField::from_registry(&registry);
        let mut pipe_b = PipeGasField::from_registry(&registry);
        let mut visuals_a = PipeFlowVisualState::default();
        let mut visuals_b = PipeFlowVisualState::default();

        for _ in 0..8 {
            let changed_a =
                apply_pipe_network_step(&layout, &mut pipe_a, &mut gas_a, &world, &mut visuals_a);
            let changed_b =
                apply_pipe_network_step(&layout, &mut pipe_b, &mut gas_b, &world, &mut visuals_b);
            assert_eq!(changed_a, changed_b);
        }

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                for gas_index in 0..gas_a.gas_count() {
                    assert_eq!(
                        gas_a.amount_particles(x, y, gas_index),
                        gas_b.amount_particles(x, y, gas_index)
                    );
                    assert_eq!(
                        pipe_a.amount_particles(x, y, gas_index),
                        pipe_b.amount_particles(x, y, gas_index)
                    );
                }
            }
        }
    }

    #[test]
    fn pipe_capacity_limit_is_preserved() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        build_line(&mut layout, &world, &structures, &[60, 61, 62], 60);
        assert!(layout.set_vent(60, 60, &world, &structures));
        assert!(layout.set_vent(62, 60, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(60, 60, 0, (PIPE_CELL_CAPACITY * 3) as f32);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();
        for _ in 0..8 {
            let _ = apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
        }
        for cell in [UVec2::new(60, 60), UVec2::new(61, 60), UVec2::new(62, 60)] {
            assert!(pipe_gas.total_amount_particles(cell.x, cell.y) <= PIPE_CELL_CAPACITY);
        }
    }

    #[test]
    fn world_vent_threshold_blocks_small_exchange_but_pipe_transfer_has_no_threshold() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        build_line(&mut layout, &world, &structures, &[15, 16], 15);
        assert!(layout.set_vent(15, 15, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(15, 15, 0, (MIN_VENT_PRESSURE_DELTA_PARTICLES - 1) as f32);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        assert_eq!(
            pipe_gas.add_species_counts_limited(15, 15, &[2, 0, 0]),
            vec![2, 0, 0]
        );
        let mut visuals = PipeFlowVisualState::default();

        assert!(apply_pipe_network_step(
            &layout,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals
        ));
        assert_eq!(
            gas.total_amount_rounded(15, 15),
            MIN_VENT_PRESSURE_DELTA_PARTICLES - 1
        );
        assert_eq!(pipe_gas.total_amount_particles(15, 15), 1);
        assert_eq!(pipe_gas.total_amount_particles(16, 15), 1);
        assert!(visuals.transfers.iter().any(
            |transfer| transfer.from == UVec2::new(15, 15) && transfer.to == UVec2::new(16, 15)
        ));
    }

    #[test]
    fn mass_conservation_is_preserved_across_world_and_pipe_buffers() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        build_line(&mut layout, &world, &structures, &[25, 26, 27], 25);
        assert!(layout.set_vent(25, 25, &world, &structures));
        assert!(layout.set_vent(27, 25, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(25, 25, 0, 90.0);
        gas.set_amount(27, 25, 1, 30.0);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let before = total_world_and_pipe_particles(&gas, &pipe_gas);
        let mut visuals = PipeFlowVisualState::default();

        for _ in 0..10 {
            let _ = apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
        }

        let after = total_world_and_pipe_particles(&gas, &pipe_gas);
        assert_eq!(before, after);
    }

    #[test]
    fn residual_gas_in_branching_network_drains_toward_vents() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        for cell in [
            UVec2::new(70, 70),
            UVec2::new(71, 70),
            UVec2::new(72, 70),
            UVec2::new(71, 69),
            UVec2::new(71, 71),
        ] {
            assert!(layout.set_pipe(cell.x, cell.y, &world, &structures));
        }
        for (a, b) in [
            (UVec2::new(71, 70), UVec2::new(70, 70)),
            (UVec2::new(71, 70), UVec2::new(72, 70)),
            (UVec2::new(71, 70), UVec2::new(71, 69)),
            (UVec2::new(71, 70), UVec2::new(71, 71)),
        ] {
            connect(&mut layout, a, b);
        }
        assert!(layout.set_vent(70, 70, &world, &structures));
        assert!(layout.set_vent(72, 70, &world, &structures));
        assert!(layout.set_vent(71, 69, &world, &structures));
        assert!(layout.set_vent(71, 71, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        assert_eq!(
            pipe_gas.add_species_counts_limited(71, 70, &[120, 0, 0]),
            vec![120, 0, 0]
        );
        let before_pipe_total = pipe_gas.total_amount_particles(71, 70);
        let mut visuals = PipeFlowVisualState::default();

        for _ in 0..12 {
            let _ = apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
        }

        let world_received = gas.total_amount_rounded(70, 70)
            + gas.total_amount_rounded(72, 70)
            + gas.total_amount_rounded(71, 69)
            + gas.total_amount_rounded(71, 71);
        let remaining_pipe_total = pipe_gas.total_amount_particles(70, 70)
            + pipe_gas.total_amount_particles(71, 70)
            + pipe_gas.total_amount_particles(72, 70)
            + pipe_gas.total_amount_particles(71, 69)
            + pipe_gas.total_amount_particles(71, 71);
        assert!(world_received > 0);
        assert!(remaining_pipe_total < before_pipe_total);
    }

    #[test]
    fn display_counts_include_transient_flow_when_pipe_storage_is_empty() {
        let registry = registry();
        let pipe_gas = PipeGasField::from_registry(&registry);
        let flow_state = PipeFlowVisualState {
            transfers: vec![super::PipeTransferRecord {
                from: UVec2::new(5, 5),
                to: UVec2::new(6, 5),
                gas_counts: vec![12, 3, 0],
                total_amount: 15,
            }],
        };

        let source_counts = pipe_cell_display_species_counts(&pipe_gas, &flow_state, 5, 5);
        let target_counts = pipe_cell_display_species_counts(&pipe_gas, &flow_state, 6, 5);

        assert_eq!(source_counts[0], 12);
        assert_eq!(target_counts[1], 3);
        assert_eq!(
            pipe_cell_display_total_particles(&pipe_gas, &flow_state, 5, 5),
            15
        );
        assert_eq!(
            pipe_cell_display_total_particles(&pipe_gas, &flow_state, 6, 5),
            15
        );
    }

    #[test]
    fn display_counts_can_ignore_transient_flow_when_requested() {
        let registry = registry();
        let pipe_gas = PipeGasField::from_registry(&registry);
        let flow_state = PipeFlowVisualState {
            transfers: vec![super::PipeTransferRecord {
                from: UVec2::new(5, 5),
                to: UVec2::new(6, 5),
                gas_counts: vec![12, 3, 0],
                total_amount: 15,
            }],
        };

        let source_counts =
            pipe_cell_display_species_counts_with_transfers(&pipe_gas, &flow_state, 5, 5, false);
        let target_total =
            pipe_cell_display_total_particles_with_transfers(&pipe_gas, &flow_state, 6, 5, false);

        assert_eq!(source_counts, vec![0, 0, 0]);
        assert_eq!(target_total, 0);
    }

    #[test]
    fn local_transfer_uses_fixed_half_delta() {
        assert_eq!(desired_local_transfer_amount(0), 0);
        assert_eq!(desired_local_transfer_amount(1), 0);
        assert_eq!(desired_local_transfer_amount(2), 1);
        assert_eq!(desired_local_transfer_amount(9), 4);
    }

    #[test]
    fn branching_uses_shared_outgoing_budget_instead_of_summing_half_delta_per_edge() {
        assert_eq!(combined_outgoing_delta(&[]), 0);
        assert_eq!(combined_outgoing_delta(&[60]), 60);
        assert_eq!(combined_outgoing_delta(&[60, 60]), 60);
        assert_eq!(
            desired_local_transfer_amount(combined_outgoing_delta(&[60, 60])),
            30
        );
    }

    #[test]
    fn gas_does_not_chain_across_two_edges_in_one_tick_when_middle_started_empty() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        build_line(&mut layout, &world, &structures, &[80, 81, 82], 80);
        assert!(layout.set_vent(80, 80, &world, &structures));
        assert!(layout.set_vent(82, 80, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(80, 80, 0, 100.0);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();

        let _ = apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
        assert!(visuals.transfers.is_empty());

        let _ = apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
        assert!(visuals.transfers.iter().any(
            |transfer| transfer.from == UVec2::new(80, 80) && transfer.to == UVec2::new(81, 80)
        ));
        assert!(!visuals.transfers.iter().any(
            |transfer| transfer.from == UVec2::new(81, 80) && transfer.to == UVec2::new(82, 80)
        ));
    }
}
