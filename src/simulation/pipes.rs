use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;

use crate::{
    config::GasRegistry,
    simulation::gas::GasField,
    world::{
        grid::{linear_index, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
        pipes::PipeGrid,
    },
};

const PIPE_CELL_CAPACITY: u32 = 1_000;
const PIPE_SOLVER_ITERATIONS: usize = 24;

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

        let potentials =
            solve_component_potentials(layout, gas, &component, &component_lookup, &vents);
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
                let delta = (potentials[index] - potentials[neighbor_index]).max(0.0);
                let reverse_delta = (potentials[neighbor_index] - potentials[index]).max(0.0);
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

        let mut pre_injected = vec![0u32; component.len()];
        let mut provisional_edges = HashMap::<(usize, usize), u32>::new();
        for (source_index, edges) in outgoing_by_source.iter().enumerate() {
            if edges.is_empty() {
                continue;
            }
            let source_cell = component[source_index];
            let desired_total = desired_out[source_index].round().max(0.0) as u32;
            if desired_total == 0 {
                continue;
            }
            let pipe_available = pipe_gas.total_amount_particles(source_cell.x, source_cell.y);
            let world_available = if vents.contains(&source_cell) {
                gas.total_amount_rounded(source_cell.x, source_cell.y)
            } else {
                0
            };
            let available_total = pipe_available.saturating_add(world_available);
            let actual_total = desired_total.min(available_total);
            if actual_total == 0 {
                continue;
            }

            if actual_total > pipe_available {
                let needed_from_world = actual_total - pipe_available;
                let removed_mix = gas.remove_particles_proportional_counts(
                    source_cell.x,
                    source_cell.y,
                    needed_from_world,
                    world,
                );
                let removed: u32 = removed_mix.iter().copied().sum();
                if removed > 0 {
                    let accepted = pipe_gas.add_species_counts_limited(
                        source_cell.x,
                        source_cell.y,
                        &removed_mix,
                    );
                    let mut rejected = vec![0u32; removed_mix.len()];
                    for gas_index in 0..removed_mix.len() {
                        rejected[gas_index] = removed_mix[gas_index]
                            .saturating_sub(accepted.get(gas_index).copied().unwrap_or(0));
                    }
                    let rejected_total = add_pipe_mix_to_world(gas, world, source_cell, &rejected);
                    pre_injected[source_index] = accepted.iter().copied().sum();
                    if pre_injected[source_index] > 0 || rejected_total > 0 {
                        world_changed = true;
                        pipe_changed = true;
                    }
                }
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
        for ((source_index, target_index), amount) in sorted_edges {
            if amount == 0 {
                continue;
            }
            let source_cell = component[source_index];
            let target_cell = component[target_index];
            let removed_mix =
                pipe_gas.remove_particles_proportional_counts(source_cell.x, source_cell.y, amount);
            let accepted_mix =
                pipe_gas.add_species_counts_limited(target_cell.x, target_cell.y, &removed_mix);
            let accepted_total: u32 = accepted_mix.iter().copied().sum();
            if accepted_total == 0 {
                continue;
            }
            actual_out[source_index] = actual_out[source_index].saturating_add(accepted_total);
            actual_in[target_index] = actual_in[target_index].saturating_add(accepted_total);
            pipe_changed = true;
            visual_state.transfers.push(PipeTransferRecord {
                from: source_cell,
                to: target_cell,
                total_amount: accepted_total,
                gas_counts: accepted_mix,
            });
        }

        for (vent_index, vent_cell) in component.iter().copied().enumerate() {
            if !vents.contains(&vent_cell) {
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
            }
        }
    }

    if world_changed {
        gas.recompute_total_density_buffer(world);
    }

    world_changed || pipe_changed
}

fn solve_component_potentials(
    layout: &PipeGrid,
    gas: &GasField,
    component: &[UVec2],
    lookup: &HashMap<UVec2, usize>,
    vents: &[UVec2],
) -> Vec<f32> {
    let mut potentials = vec![0.0f32; component.len()];
    let vent_pressures = vents
        .iter()
        .copied()
        .map(|cell| (cell, gas.total_amount(cell.x, cell.y).max(0.0)))
        .collect::<HashMap<_, _>>();
    let boundary_average = if vent_pressures.is_empty() {
        0.0
    } else {
        vent_pressures.values().copied().sum::<f32>() / vent_pressures.len() as f32
    };

    for (index, cell) in component.iter().copied().enumerate() {
        potentials[index] = vent_pressures
            .get(&cell)
            .copied()
            .unwrap_or(boundary_average);
    }

    for _ in 0..PIPE_SOLVER_ITERATIONS {
        for (index, cell) in component.iter().copied().enumerate() {
            if let Some(pressure) = vent_pressures.get(&cell) {
                potentials[index] = *pressure;
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
    use super::{apply_pipe_network_step, PipeFlowVisualState, PipeGasField, PIPE_CELL_CAPACITY};
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
        gas.set_amount(20, 20, 0, 90.0);
        gas.recompute_total_density_buffer(&world);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();

        let mut changed = false;
        for _ in 0..4 {
            changed |=
                apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
        }
        assert!(changed);
        assert!(gas.total_amount_rounded(22, 20) > 0);
        assert!(!visuals.transfers.is_empty());
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
        let mut visuals = PipeFlowVisualState::default();

        let mut changed = false;
        for _ in 0..4 {
            changed |=
                apply_pipe_network_step(&layout, &mut pipe_gas, &mut gas, &world, &mut visuals);
        }
        assert!(changed);
        assert!(gas.total_amount_rounded(42, 40) > 0);
        assert!(gas.total_amount_rounded(41, 39) > 0);
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
}
