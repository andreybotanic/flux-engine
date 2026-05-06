use std::collections::{HashMap, HashSet, VecDeque};

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
const PIPE_SOLVER_ITERATIONS: usize = 24;
const PIPE_FLOW_CONDUCTIVITY: f32 = 1.9;
/// Minimum rounded pressure delta between vents required to activate pipe flow.
pub(crate) const MIN_VENT_PRESSURE_DELTA_PARTICLES: u32 = 5;

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

    fn remove_species_counts_exact(&mut self, x: u32, y: u32, removed: &[u32]) {
        let idx = linear_index(x, y);
        for gas_index in 0..self.gas_count {
            let amount = removed.get(gas_index).copied().unwrap_or(0);
            self.cells[idx][gas_index] = self.cells[idx][gas_index].saturating_sub(amount);
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
        let component_lookup = component
            .iter()
            .enumerate()
            .map(|(index, cell)| (*cell, index))
            .collect::<HashMap<_, _>>();

        let vents = component
            .iter()
            .copied()
            .filter(|cell| {
                let pipe_cell = layout.cell(cell.x, cell.y);
                pipe_cell.has_pipe && pipe_cell.has_vent
            })
            .collect::<Vec<_>>();
        if vents.len() < 2 {
            continue;
        }

        let active_vent_pressures = collect_active_vent_pressures(&vents, gas);
        if active_vent_pressures.len() < 2 {
            continue;
        }
        let active_vents = active_vent_pressures
            .keys()
            .copied()
            .collect::<HashSet<_>>();
        let starting_pipe_totals = component
            .iter()
            .map(|cell| pipe_gas.total_amount_particles(cell.x, cell.y))
            .collect::<Vec<_>>();
        let starting_pipe_species = component
            .iter()
            .map(|cell| pipe_gas.species_counts(cell.x, cell.y))
            .collect::<Vec<_>>();
        let potentials = solve_component_potentials(
            layout,
            &component,
            &component_lookup,
            &active_vent_pressures,
        );
        if potentials.is_empty() {
            continue;
        }

        let mut desired_edges = Vec::new();
        let mut desired_out = vec![0.0f32; component.len()];
        let mut desired_in = vec![0.0f32; component.len()];
        for (index, cell) in component.iter().copied().enumerate() {
            for neighbor in layout.connected_neighbors(cell.x, cell.y) {
                let Some(&neighbor_index) = component_lookup.get(&neighbor) else {
                    continue;
                };
                if neighbor_index <= index {
                    continue;
                }
                let delta = desired_edge_flow_from_potential_delta(
                    potentials[index] - potentials[neighbor_index],
                );
                let reverse_delta = desired_edge_flow_from_potential_delta(
                    potentials[neighbor_index] - potentials[index],
                );
                if delta > 0.0 {
                    desired_edges.push((index, neighbor_index, delta));
                    desired_out[index] += delta;
                    desired_in[neighbor_index] += delta;
                } else if reverse_delta > 0.0 {
                    desired_edges.push((neighbor_index, index, reverse_delta));
                    desired_out[neighbor_index] += reverse_delta;
                    desired_in[index] += reverse_delta;
                }
            }
        }
        if desired_edges.is_empty() {
            continue;
        }

        let mut outgoing_by_source = vec![Vec::<(usize, f32)>::new(); component.len()];
        for (source, target, weight) in &desired_edges {
            outgoing_by_source[*source].push((*target, *weight));
        }
        let desired_totals = desired_out
            .iter()
            .map(|desired| desired.round().max(0.0) as u32)
            .collect::<Vec<_>>();

        let mut provisional_edges = HashMap::<(usize, usize), u32>::new();
        for (source_index, edges) in outgoing_by_source.iter().enumerate() {
            if edges.is_empty() {
                continue;
            }
            let desired_total = desired_totals[source_index];
            if desired_total == 0 {
                continue;
            }
            let actual_total = desired_total.min(starting_pipe_totals[source_index]);
            if actual_total == 0 {
                continue;
            }

            let weights = edges.iter().map(|(_, weight)| *weight).collect::<Vec<_>>();
            let split = split_integer_by_weights(actual_total, &weights);
            for ((target_index, _), amount) in edges.iter().zip(split.into_iter()) {
                if amount > 0 {
                    provisional_edges.insert((source_index, *target_index), amount);
                }
            }
        }

        let mut capped_edges = provisional_edges.clone();
        for (target_index, target_cell) in component.iter().copied().enumerate() {
            let incoming = capped_edges
                .iter()
                .filter_map(|((source_index, edge_target), amount)| {
                    if *edge_target == target_index {
                        Some((*source_index, *amount))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if incoming.is_empty() {
                continue;
            }
            let total_incoming: u32 = incoming.iter().map(|(_, amount)| *amount).sum();
            let free = pipe_gas.free_capacity(target_cell.x, target_cell.y);
            if total_incoming <= free {
                continue;
            }
            let weights = incoming
                .iter()
                .map(|(_, amount)| *amount as f32)
                .collect::<Vec<_>>();
            let capped = split_integer_by_weights(free, &weights);
            for ((source_index, _), accepted_amount) in incoming.iter().zip(capped.into_iter()) {
                capped_edges.insert((*source_index, target_index), accepted_amount);
            }
        }

        let mut actual_in = vec![0u32; component.len()];
        let mut actual_out = vec![0u32; component.len()];
        let mut sorted_edges = capped_edges.into_iter().collect::<Vec<_>>();
        sorted_edges.sort_by_key(|((source, target), _)| (*source, *target));
        let mut planned_transfers = Vec::<(usize, usize, Vec<u32>, u32)>::new();
        let mut source_remaining = starting_pipe_species.clone();
        let mut exact_removed_by_source = vec![vec![0u32; pipe_gas.gas_count()]; component.len()];

        for ((source_index, target_index), amount) in sorted_edges {
            if amount == 0 {
                continue;
            }
            let gas_counts = remove_particles_proportional_from_counts(
                &mut source_remaining[source_index],
                amount,
            );
            let total_amount: u32 = gas_counts.iter().copied().sum();
            if total_amount == 0 {
                continue;
            }
            for gas_index in 0..pipe_gas.gas_count() {
                exact_removed_by_source[source_index][gas_index] = exact_removed_by_source
                    [source_index][gas_index]
                    .saturating_add(gas_counts[gas_index]);
            }
            planned_transfers.push((source_index, target_index, gas_counts, total_amount));
        }

        for (source_index, removed) in exact_removed_by_source.iter().enumerate() {
            if removed.iter().copied().sum::<u32>() == 0 {
                continue;
            }
            let source_cell = component[source_index];
            pipe_gas.remove_species_counts_exact(source_cell.x, source_cell.y, removed);
        }

        let mut moved_any = false;
        for (source_index, target_index, gas_counts, _total_amount) in planned_transfers {
            let source_cell = component[source_index];
            let target_cell = component[target_index];
            let accepted_mix =
                pipe_gas.add_species_counts_limited(target_cell.x, target_cell.y, &gas_counts);
            let accepted_total: u32 = accepted_mix.iter().copied().sum();
            if accepted_total == 0 {
                continue;
            }
            actual_out[source_index] = actual_out[source_index].saturating_add(accepted_total);
            actual_in[target_index] = actual_in[target_index].saturating_add(accepted_total);
            pipe_changed = true;
            moved_any = true;
            visual_state.transfers.push(PipeTransferRecord {
                from: source_cell,
                to: target_cell,
                total_amount: accepted_total,
                gas_counts: accepted_mix,
            });
        }

        let mut emitted_any = false;
        for (vent_index, vent_cell) in component.iter().copied().enumerate() {
            if !active_vents.contains(&vent_cell) {
                continue;
            }
            let emit_amount = actual_in[vent_index].saturating_sub(actual_out[vent_index]);
            if emit_amount == 0 {
                continue;
            }
            let removed_mix = pipe_gas.remove_particles_proportional_counts(
                vent_cell.x,
                vent_cell.y,
                emit_amount,
            );
            let added_to_world = add_pipe_mix_to_world(gas, world, vent_cell, &removed_mix);
            if added_to_world > 0 {
                world_changed = true;
                pipe_changed = true;
                emitted_any = true;
            }
        }

        let mut injected_any = false;
        for (source_index, edges) in outgoing_by_source.iter().enumerate() {
            if edges.is_empty() {
                continue;
            }
            let source_cell = component[source_index];
            if !active_vents.contains(&source_cell) {
                continue;
            }
            let target_fill = desired_totals[source_index].min(PIPE_CELL_CAPACITY);
            if target_fill == 0 {
                continue;
            }
            let current_pipe = pipe_gas.total_amount_particles(source_cell.x, source_cell.y);
            let needed_from_world = target_fill.saturating_sub(current_pipe);
            if needed_from_world == 0 {
                continue;
            }
            let removed_mix = gas.remove_particles_proportional_counts(
                source_cell.x,
                source_cell.y,
                needed_from_world,
                world,
            );
            let removed_total: u32 = removed_mix.iter().copied().sum();
            if removed_total == 0 {
                continue;
            }
            let accepted =
                pipe_gas.add_species_counts_limited(source_cell.x, source_cell.y, &removed_mix);
            let mut rejected = vec![0u32; removed_mix.len()];
            for gas_index in 0..removed_mix.len() {
                rejected[gas_index] = removed_mix[gas_index]
                    .saturating_sub(accepted.get(gas_index).copied().unwrap_or(0));
            }
            let rejected_total = add_pipe_mix_to_world(gas, world, source_cell, &rejected);
            let accepted_total = accepted.iter().copied().sum::<u32>();
            if accepted_total > 0 || rejected_total > 0 {
                world_changed = true;
                pipe_changed = true;
                injected_any = true;
            }
        }

        if !moved_any && !emitted_any && !injected_any {
            continue;
        }
    }

    if world_changed {
        gas.recompute_total_density_buffer(world);
    }

    world_changed || pipe_changed
}

fn desired_edge_flow_from_potential_delta(delta: f32) -> f32 {
    delta.max(0.0) * PIPE_FLOW_CONDUCTIVITY
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

fn solve_component_potentials(
    layout: &PipeGrid,
    component: &[UVec2],
    lookup: &HashMap<UVec2, usize>,
    active_vent_pressures: &HashMap<UVec2, u32>,
) -> Vec<f32> {
    let mut potentials = vec![0.0f32; component.len()];
    let boundary_average = if active_vent_pressures.is_empty() {
        0.0
    } else {
        active_vent_pressures.values().copied().sum::<u32>() as f32
            / active_vent_pressures.len() as f32
    };

    for (index, cell) in component.iter().copied().enumerate() {
        potentials[index] = active_vent_pressures
            .get(&cell)
            .copied()
            .map(|value| value as f32)
            .unwrap_or(boundary_average);
    }

    for _ in 0..PIPE_SOLVER_ITERATIONS {
        for (index, cell) in component.iter().copied().enumerate() {
            if let Some(pressure) = active_vent_pressures.get(&cell) {
                potentials[index] = *pressure as f32;
                continue;
            }
            let mut sum = 0.0f32;
            let mut count = 0u32;
            for neighbor in layout.connected_neighbors(cell.x, cell.y) {
                if let Some(&neighbor_index) = lookup.get(&neighbor) {
                    sum += potentials[neighbor_index];
                    count += 1;
                }
            }
            if count > 0 {
                potentials[index] = sum / count as f32;
            }
        }
    }

    potentials
}

fn collect_active_vent_pressures(vents: &[UVec2], gas: &GasField) -> HashMap<UVec2, u32> {
    let vent_pressures = vents
        .iter()
        .copied()
        .map(|cell| (cell, gas.total_amount_rounded(cell.x, cell.y)))
        .collect::<Vec<_>>();
    let mut active = HashMap::<UVec2, u32>::new();

    for (index, (vent_a, pressure_a)) in vent_pressures.iter().copied().enumerate() {
        for (vent_b, pressure_b) in vent_pressures.iter().copied().skip(index + 1) {
            if pressure_a.abs_diff(pressure_b) < MIN_VENT_PRESSURE_DELTA_PARTICLES {
                continue;
            }
            active.insert(vent_a, pressure_a);
            active.insert(vent_b, pressure_b);
        }
    }

    active
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

#[cfg(test)]
mod tests {
    use super::{
        apply_pipe_network_step, desired_edge_flow_from_potential_delta,
        pipe_cell_display_species_counts, pipe_cell_display_species_counts_with_transfers,
        pipe_cell_display_total_particles, pipe_cell_display_total_particles_with_transfers,
        PipeFlowVisualState, PipeGasField, PIPE_CELL_CAPACITY,
    };
    use crate::{
        config::{GasDefinition, GasRegistry},
        simulation::gas::GasField,
        world::{gas_structures::GasStructureGrid, grid::WorldGrid, pipes::PipeGrid},
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

    #[test]
    fn single_vent_component_does_not_move_gas() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        build_line(&mut layout, &world, &structures, &[10, 11], 10);
        assert!(layout.set_vent(10, 10, &world, &structures));
        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(10, 10, 0, 50.0);
        gas.recompute_total_density_buffer(&world);
        let before = gas.total_amount_rounded(10, 10);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();

        assert!(!apply_pipe_network_step(
            &layout,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals
        ));
        assert_eq!(gas.total_amount_rounded(10, 10), before);
        assert!(visuals.transfers.is_empty());
    }

    #[test]
    fn straight_pipe_moves_gas_between_two_vents() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        build_line(&mut layout, &world, &structures, &[20, 21, 22], 20);
        assert!(layout.set_vent(20, 20, &world, &structures));
        assert!(layout.set_vent(22, 20, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(20, 20, 0, 120.0);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();

        let changed_tick_1 =
            apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
        assert!(changed_tick_1);
        assert!(visuals.transfers.is_empty());
        assert!(pipe_gas.total_amount_particles(20, 20) > 0);
        assert_eq!(gas.total_amount_rounded(22, 20), 0);

        let mut changed = changed_tick_1;
        let mut saw_transfers = false;
        for _ in 0..3 {
            changed |=
                apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
            saw_transfers |= !visuals.transfers.is_empty();
        }
        assert!(changed);
        assert!(gas.total_amount_rounded(22, 20) > 0);
        assert!(saw_transfers);
    }

    #[test]
    fn t_junction_splits_flow_across_two_outputs() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        for cell in [
            UVec2::new(40, 40),
            UVec2::new(41, 40),
            UVec2::new(42, 40),
            UVec2::new(41, 39),
        ] {
            assert!(layout.set_pipe(cell.x, cell.y, &world, &structures));
        }
        connect(&mut layout, UVec2::new(40, 40), UVec2::new(41, 40));
        connect(&mut layout, UVec2::new(41, 40), UVec2::new(42, 40));
        connect(&mut layout, UVec2::new(41, 40), UVec2::new(41, 39));
        assert!(layout.set_vent(40, 40, &world, &structures));
        assert!(layout.set_vent(42, 40, &world, &structures));
        assert!(layout.set_vent(41, 39, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(40, 40, 0, 120.0);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let seeded_center_mix = vec![120, 0, 0];
        let accepted_seed = pipe_gas.add_species_counts_limited(41, 40, &seeded_center_mix);
        assert_eq!(accepted_seed, seeded_center_mix);
        let mut visuals = PipeFlowVisualState::default();

        let changed =
            apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
        assert!(changed);
        assert!(gas.total_amount_rounded(42, 40) > 0);
        assert!(gas.total_amount_rounded(41, 39) > 0);
        assert!(visuals.transfers.iter().any(|transfer| {
            transfer.from == UVec2::new(41, 40) && transfer.to == UVec2::new(42, 40)
        }));
        assert!(visuals.transfers.iter().any(|transfer| {
            transfer.from == UVec2::new(41, 40) && transfer.to == UVec2::new(41, 39)
        }));
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

        for _ in 0..5 {
            let changed_a =
                apply_pipe_network_step(&layout, &mut pipe_a, &mut gas_a, &world, &mut visuals_a);
            let changed_b =
                apply_pipe_network_step(&layout, &mut pipe_b, &mut gas_b, &world, &mut visuals_b);
            assert_eq!(changed_a, changed_b);
        }

        for y in 0..crate::world::grid::WORLD_HEIGHT {
            for x in 0..crate::world::grid::WORLD_WIDTH {
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
        for _ in 0..3 {
            let _ = apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
        }
        for cell in [UVec2::new(60, 60), UVec2::new(61, 60), UVec2::new(62, 60)] {
            assert!(
                pipe_gas.total_amount_particles(cell.x, cell.y) <= PIPE_CELL_CAPACITY,
                "pipe cell must respect capacity"
            );
        }
    }

    #[test]
    fn pressure_delta_below_threshold_does_not_move_gas() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        build_line(&mut layout, &world, &structures, &[15, 16, 17], 15);
        assert!(layout.set_vent(15, 15, &world, &structures));
        assert!(layout.set_vent(17, 15, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(15, 15, 0, 4.0);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();

        assert!(!apply_pipe_network_step(
            &layout,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals
        ));
        assert_eq!(gas.total_amount_rounded(15, 15), 4);
        assert_eq!(gas.total_amount_rounded(17, 15), 0);
        assert_eq!(pipe_gas.total_amount_particles(16, 15), 0);
        assert!(visuals.transfers.is_empty());
    }

    #[test]
    fn pressure_delta_at_threshold_starts_flow() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        build_line(&mut layout, &world, &structures, &[25, 26, 27], 25);
        assert!(layout.set_vent(25, 25, &world, &structures));
        assert!(layout.set_vent(27, 25, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(25, 25, 0, 5.0);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();

        let changed_tick_1 =
            apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
        assert!(changed_tick_1);
        assert!(visuals.transfers.is_empty());
        assert!(pipe_gas.total_amount_particles(25, 25) > 0);

        let mut changed = changed_tick_1;
        let mut saw_transfers = false;
        for _ in 0..2 {
            changed |=
                apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
            saw_transfers |= !visuals.transfers.is_empty();
        }
        assert!(changed);
        assert!(pipe_gas.total_amount_particles(25, 25) > 0 || saw_transfers);
    }

    #[test]
    fn inactive_middle_vent_stays_world_passive_when_only_outer_pair_is_active() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        build_line(&mut layout, &world, &structures, &[70, 71, 72], 70);
        assert!(layout.set_vent(70, 70, &world, &structures));
        assert!(layout.set_vent(71, 70, &world, &structures));
        assert!(layout.set_vent(72, 70, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(70, 70, 0, 8.0);
        gas.set_amount(71, 70, 0, 4.0);
        gas.set_amount(72, 70, 0, 2.0);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let seeded_left_mix = vec![3, 0, 0];
        let accepted_seed = pipe_gas.add_species_counts_limited(70, 70, &seeded_left_mix);
        assert_eq!(accepted_seed, seeded_left_mix);
        let mut visuals = PipeFlowVisualState::default();
        let mut saw_middle_transfer = false;

        for _ in 0..4 {
            let _ = apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
            saw_middle_transfer |= visuals.transfers.iter().any(|transfer| {
                transfer.from == UVec2::new(71, 70) || transfer.to == UVec2::new(71, 70)
            });
        }

        assert_eq!(gas.total_amount_rounded(71, 70), 4);
        assert!(gas.total_amount_rounded(70, 70) < 8);
        assert!(saw_middle_transfer);
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
    fn edge_flow_uses_conductivity_multiplier() {
        assert_eq!(desired_edge_flow_from_potential_delta(-10.0), 0.0);
        assert!((desired_edge_flow_from_potential_delta(12.0) - 22.8).abs() < 1e-6);
    }

    #[test]
    fn long_pipe_keeps_higher_throughput_than_plain_delta_division() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut layout = PipeGrid::default();
        build_line(&mut layout, &world, &structures, &[30, 31, 32, 33, 34], 30);
        assert!(layout.set_vent(30, 30, &world, &structures));
        assert!(layout.set_vent(34, 30, &world, &structures));

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(30, 30, 0, 200.0);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();

        let _ = apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
        assert!(visuals.transfers.is_empty());
        let source_fill_after_first_tick = pipe_gas.total_amount_particles(30, 30);

        assert!(
            source_fill_after_first_tick > 50,
            "conductivity should let the source segment fill beyond raw delta/length on a long pipe"
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
