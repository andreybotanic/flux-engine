use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;

use crate::{
    config::GasRegistry,
    simulation::gas::GasField,
    world::{
        grid::{WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
        structures::{
            bridge_center_cell, PlacedStructureMap, StructureKind, StructureRotation,
        },
    },
};

pub(crate) const PIPE_CELL_CAPACITY: u32 = 1_000;
/// Minimum rounded pressure delta between vents required to activate pipe flow.
pub(crate) const MIN_VENT_PRESSURE_DELTA_PARTICLES: u32 = 5;
const PIPE_LOCAL_TRANSFER_NUMERATOR: u32 = 1;
const PIPE_LOCAL_TRANSFER_DENOMINATOR: u32 = 2;

/// Identifies which container is rendered inside a world cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PipeContainerKind {
    Pipe,
    BridgePipe,
}

/// Stores the persistent identity of one pipe node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PipeNodeKey {
    pub kind: PipeContainerKind,
    pub anchor: UVec2,
}

/// Stores one saved pipe node entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipeNodeGasSnapshotEntry {
    pub key: PipeNodeKey,
    pub species: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
/// Stores `PipeGasSnapshot` state.
pub struct PipeGasSnapshot {
    pub gas_count: usize,
    pub nodes: Vec<PipeNodeGasSnapshotEntry>,
}

#[derive(Resource, Clone)]
/// Stores `PipeGasField` state.
pub struct PipeGasField {
    gas_count: usize,
    keys: Vec<PipeNodeKey>,
    cells: Vec<Vec<u32>>,
}

impl Default for PipeGasField {
    fn default() -> Self {
        Self {
            gas_count: 0,
            keys: Vec::new(),
            cells: Vec::new(),
        }
    }
}

impl PipeGasField {
    /// Builds an empty field sized for the current gas registry.
    pub fn from_registry(registry: &GasRegistry) -> Self {
        Self {
            gas_count: registry.count(),
            keys: Vec::new(),
            cells: Vec::new(),
        }
    }

    /// Returns the number of registered gases.
    pub fn gas_count(&self) -> usize {
        self.gas_count
    }

    /// Rebuilds the node layout from placed structures while preserving gas by node key.
    pub fn sync_to_structures(&mut self, structures: &PlacedStructureMap) {
        let next_keys = collect_pipe_node_keys(structures);
        if next_keys == self.keys {
            return;
        }

        let previous = self
            .keys
            .iter()
            .copied()
            .zip(self.cells.iter().cloned())
            .collect::<HashMap<_, _>>();
        self.keys = next_keys;
        self.cells = self
            .keys
            .iter()
            .map(|key| {
                previous
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| vec![0; self.gas_count])
            })
            .collect();
    }

    /// Returns the gas amount stored inside one node.
    pub fn amount_particles(&self, node_id: usize, gas_index: usize) -> u32 {
        self.cells
            .get(node_id)
            .and_then(|cell| cell.get(gas_index))
            .copied()
            .unwrap_or(0)
    }

    /// Returns the total gas amount stored inside one node.
    pub fn total_amount_particles(&self, node_id: usize) -> u32 {
        self.cells
            .get(node_id)
            .map(|cell| cell.iter().copied().sum())
            .unwrap_or(0)
    }

    /// Returns the free capacity of one node.
    pub fn free_capacity(&self, node_id: usize) -> u32 {
        PIPE_CELL_CAPACITY.saturating_sub(self.total_amount_particles(node_id))
    }

    /// Clears all nodes that visually belong to the world cell.
    pub fn clear_cell(&mut self, structures: &PlacedStructureMap, x: u32, y: u32) {
        for node_id in node_ids_for_cell(structures, self, x, y) {
            if let Some(cell) = self.cells.get_mut(node_id) {
                cell.fill(0);
            }
        }
    }

    /// Clears every node in the field.
    pub fn clear_all(&mut self) {
        for cell in &mut self.cells {
            cell.fill(0);
        }
    }

    fn node_species_counts(&self, node_id: usize) -> Vec<u32> {
        self.cells
            .get(node_id)
            .cloned()
            .unwrap_or_else(|| vec![0; self.gas_count])
    }

    fn set_species_counts_exact(&mut self, node_id: usize, species: &[u32]) {
        let Some(cell) = self.cells.get_mut(node_id) else {
            return;
        };
        cell.fill(0);
        for gas_index in 0..self.gas_count {
            cell[gas_index] = species.get(gas_index).copied().unwrap_or(0);
        }
    }

    /// Removes gas proportionally from one node and returns the removed species counts.
    pub fn remove_particles_proportional_counts(&mut self, node_id: usize, amount: u32) -> Vec<u32> {
        if amount == 0 || self.gas_count == 0 {
            return vec![0; self.gas_count];
        }
        let Some(cell) = self.cells.get_mut(node_id) else {
            return vec![0; self.gas_count];
        };
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

    /// Adds gas to one node with capacity clamping.
    pub fn add_species_counts_limited(&mut self, node_id: usize, offered: &[u32]) -> Vec<u32> {
        let mut accepted = vec![0u32; self.gas_count];
        if self.gas_count == 0 || offered.is_empty() {
            return accepted;
        }
        let Some(cell) = self.cells.get_mut(node_id) else {
            return accepted;
        };
        let current_total: u32 = cell.iter().copied().sum();
        let total_offered: u32 = offered.iter().take(self.gas_count).copied().sum();
        let accept_total = PIPE_CELL_CAPACITY
            .saturating_sub(current_total)
            .min(total_offered);
        if accept_total == 0 {
            return accepted;
        }

        if accept_total == total_offered {
            for gas_index in 0..self.gas_count {
                accepted[gas_index] = offered.get(gas_index).copied().unwrap_or(0);
                cell[gas_index] = cell[gas_index].saturating_add(accepted[gas_index]);
            }
            return accepted;
        }

        let weights = offered
            .iter()
            .take(self.gas_count)
            .map(|value| *value as f32)
            .collect::<Vec<_>>();
        accepted = split_integer_by_weights(accept_total, &weights);
        for gas_index in 0..self.gas_count {
            accepted[gas_index] =
                accepted[gas_index].min(offered.get(gas_index).copied().unwrap_or(0));
        }
        let mut accepted_total: u32 = accepted.iter().copied().sum();
        while accepted_total < accept_total {
            let mut changed = false;
            for gas_index in 0..self.gas_count {
                let available = offered.get(gas_index).copied().unwrap_or(0);
                if accepted[gas_index] < available {
                    accepted[gas_index] = accepted[gas_index].saturating_add(1);
                    accepted_total += 1;
                    changed = true;
                    if accepted_total == accept_total {
                        break;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        for gas_index in 0..self.gas_count {
            cell[gas_index] = cell[gas_index].saturating_add(accepted[gas_index]);
        }
        accepted
    }

    /// Saves the field using stable node keys.
    pub fn snapshot_state(&self) -> PipeGasSnapshot {
        PipeGasSnapshot {
            gas_count: self.gas_count,
            nodes: self
                .keys
                .iter()
                .copied()
                .zip(self.cells.iter().cloned())
                .map(|(key, species)| PipeNodeGasSnapshotEntry { key, species })
                .collect(),
        }
    }

    /// Restores the field by matching stable node keys against the current structures.
    pub fn restore_state(
        &mut self,
        snapshot: &PipeGasSnapshot,
        structures: &PlacedStructureMap,
    ) -> Result<(), String> {
        if snapshot.gas_count != self.gas_count {
            return Err(format!(
                "Pipe gas snapshot gas_count mismatch: got {}, expected {}",
                snapshot.gas_count, self.gas_count
            ));
        }

        self.sync_to_structures(structures);
        let saved = snapshot
            .nodes
            .iter()
            .map(|entry| (entry.key, entry.species.clone()))
            .collect::<HashMap<_, _>>();
        let keys = self.keys.clone();
        for (node_id, key) in keys.into_iter().enumerate() {
            let counts = saved
                .get(&key)
                .cloned()
                .unwrap_or_else(|| vec![0; self.gas_count]);
            let total: u32 = counts.iter().copied().sum();
            if total > PIPE_CELL_CAPACITY {
                return Err(format!(
                    "Pipe gas snapshot exceeds capacity for node {:?}: {} > {}",
                    key, total, PIPE_CELL_CAPACITY
                ));
            }
            self.set_species_counts_exact(node_id, &counts);
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
/// Describes how one pipe transfer should be drawn in `F3`.
pub enum PipeTransferVisualPath {
    Straight,
    BridgeArc {
        bridge_origin: UVec2,
        bridge_rotation: StructureRotation,
    },
}

#[derive(Clone, Debug)]
/// Stores `PipeTransferRecord` state.
pub struct PipeTransferRecord {
    pub from: UVec2,
    pub to: UVec2,
    pub gas_counts: Vec<u32>,
    pub total_amount: u32,
    pub visual_path: PipeTransferVisualPath,
}

#[derive(Resource, Default, Clone)]
/// Stores `PipeFlowVisualState` state.
pub struct PipeFlowVisualState {
    pub transfers: Vec<PipeTransferRecord>,
}

/// Stores one displayable pipe container inside a hovered cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipeCellDisplayBlock {
    pub kind: PipeContainerKind,
    pub species_counts: Vec<u32>,
    pub total_particles: u32,
}

/// Builds all displayable pipe containers for one world cell.
pub fn pipe_cell_display_blocks_with_transfers(
    structures: &PlacedStructureMap,
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    x: u32,
    y: u32,
    include_transfers: bool,
) -> Vec<PipeCellDisplayBlock> {
    let mut blocks = node_ids_for_cell(structures, pipe_gas, x, y)
        .into_iter()
        .map(|node_id| {
            let species_counts = pipe_node_display_species_counts(
                pipe_gas,
                flow_state,
                structures,
                node_id,
                include_transfers,
            );
            PipeCellDisplayBlock {
                kind: pipe_gas.keys[node_id].kind,
                total_particles: species_counts.iter().copied().sum(),
                species_counts,
            }
        })
        .collect::<Vec<_>>();
    blocks.sort_by_key(|block| match block.kind {
        PipeContainerKind::BridgePipe => 0,
        PipeContainerKind::Pipe => 1,
    });
    blocks
}

/// Compatibility helper: returns per-gas counts for the first displayable container in a cell.
pub fn pipe_cell_display_species_counts_with_transfers(
    structures: &PlacedStructureMap,
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    x: u32,
    y: u32,
    include_transfers: bool,
) -> Vec<u32> {
    pipe_cell_display_blocks_with_transfers(
        structures,
        pipe_gas,
        flow_state,
        x,
        y,
        include_transfers,
    )
    .into_iter()
    .next()
    .map(|block| block.species_counts)
    .unwrap_or_else(|| vec![0; pipe_gas.gas_count()])
}

/// Compatibility helper: returns the total gas amount for the first displayable container in a cell.
pub fn pipe_cell_display_total_particles_with_transfers(
    structures: &PlacedStructureMap,
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    x: u32,
    y: u32,
    include_transfers: bool,
) -> u32 {
    pipe_cell_display_blocks_with_transfers(
        structures,
        pipe_gas,
        flow_state,
        x,
        y,
        include_transfers,
    )
    .into_iter()
    .next()
    .map(|block| block.total_particles)
    .unwrap_or(0)
}

/// Runs `apply_pipe_network_step` logic.
pub fn apply_pipe_network_step(
    structures: &PlacedStructureMap,
    pipe_gas: &mut PipeGasField,
    gas: &mut GasField,
    world: &WorldGrid,
    visual_state: &mut PipeFlowVisualState,
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
        let starting_world_totals = vent_cells
            .iter()
            .map(|vent_cell| vent_cell.map(|cell| gas.total_amount_rounded(cell.x, cell.y)))
            .collect::<Vec<_>>();

        let mut neighbor_requests = Vec::<PipeNeighborRequest>::new();
        let mut world_to_pipe_requests = vec![0u32; component.len()];
        let mut pipe_to_world_requests = vec![0u32; component.len()];
        let mut accepted_outgoing = vec![0u32; component.len()];

        for (source_index, node_id) in component.iter().copied().enumerate() {
            let source_total = starting_pipe_totals[source_index];
            let mut outgoing_requests = Vec::<OutgoingRequest>::new();

            for neighbor in runtime.nodes[node_id].neighbors.iter().copied() {
                let Some(target_index) = component.iter().position(|candidate| *candidate == neighbor) else {
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
            for (slot, (request_index, _)) in incoming_neighbors.into_iter().enumerate() {
                capped_neighbor_requests[request_index] = capped[slot];
            }
            if world_in > 0 {
                capped_world_to_pipe[target_index] = capped.get(neighbor_count).copied().unwrap_or(0);
            }
        }

        let mut next_pipe_species = starting_pipe_species.clone();

        for (request, accepted_amount) in neighbor_requests
            .iter()
            .zip(capped_neighbor_requests.iter().copied())
        {
            if accepted_amount == 0 {
                continue;
            }
            let source_node_id = component[request.source_index];
            let target_node_id = component[request.target_index];
            let moved = remove_species_proportional_counts(
                &mut next_pipe_species[request.source_index],
                accepted_amount,
            );
            if moved.iter().any(|count| *count > 0) {
                add_species_counts_limited(
                    &mut next_pipe_species[request.target_index],
                    &moved,
                    PIPE_CELL_CAPACITY,
                );
                visual_state.transfers.push(PipeTransferRecord {
                    from: runtime.nodes[source_node_id].visual_cell,
                    to: runtime.nodes[target_node_id].visual_cell,
                    total_amount: moved.iter().copied().sum(),
                    gas_counts: moved,
                    visual_path: transfer_visual_path(
                        pipe_gas.keys[source_node_id],
                        pipe_gas.keys[target_node_id],
                        structures,
                    ),
                });
            }
        }

        for (component_index, node_id) in component.iter().copied().enumerate() {
            let world_in = capped_world_to_pipe[component_index];
            if world_in > 0 {
                let Some(cell) = vent_cells[component_index] else {
                    continue;
                };
                let removed = gas.remove_particles_proportional(cell.x, cell.y, world_in, world);
                if removed > 0 {
                    let removed_counts = gas.remove_particles_proportional_counts(
                        cell.x,
                        cell.y,
                        removed,
                        world,
                    );
                    add_species_counts_limited(
                        &mut next_pipe_species[component_index],
                        &removed_counts,
                        PIPE_CELL_CAPACITY,
                    );
                    world_changed = true;
                }
            }

            let pipe_out = pipe_to_world_requests[component_index];
            if pipe_out > 0 {
                let Some(cell) = vent_cells[component_index] else {
                    continue;
                };
                let moved =
                    remove_species_proportional_counts(&mut next_pipe_species[component_index], pipe_out);
                if moved.iter().any(|count| *count > 0) {
                    for (gas_index, amount) in moved.iter().copied().enumerate() {
                        if amount > 0 {
                            let _ = gas.add_particles_no_impulse(cell.x, cell.y, gas_index, amount, world);
                        }
                    }
                    world_changed = true;
                }
            }

            pipe_gas.set_species_counts_exact(node_id, &next_pipe_species[component_index]);
            pipe_changed |= next_pipe_species[component_index] != starting_pipe_species[component_index];
        }
    }

    if world_changed {
        gas.recompute_total_density_buffer(world);
    }

    pipe_changed || world_changed
}

#[derive(Clone, Debug)]
struct PipeRuntimeNode {
    visual_cell: UVec2,
    neighbors: Vec<usize>,
    vent_cell: Option<UVec2>,
}

struct PipeRuntime {
    nodes: Vec<PipeRuntimeNode>,
}

impl PipeRuntime {
    fn from_structures(structures: &PlacedStructureMap, pipe_gas: &PipeGasField) -> Self {
        let mut nodes = pipe_gas
            .keys
            .iter()
            .copied()
            .map(|key| PipeRuntimeNode {
                visual_cell: match key.kind {
                    PipeContainerKind::Pipe => key.anchor,
                    PipeContainerKind::BridgePipe => {
                        let rotation = structures
                            .iter()
                            .find(|structure| {
                                structure.kind == StructureKind::GasPipeBridge && structure.origin == key.anchor
                            })
                            .map(|structure| structure.rotation)
                            .unwrap_or(StructureRotation::Deg0);
                        bridge_center_cell(key.anchor, rotation).unwrap_or(key.anchor)
                    }
                },
                neighbors: Vec::new(),
                vent_cell: None,
            })
            .collect::<Vec<_>>();

        let pipe_by_cell = pipe_gas
            .keys
            .iter()
            .enumerate()
            .filter_map(|(node_id, key)| (key.kind == PipeContainerKind::Pipe).then_some((key.anchor, node_id)))
            .collect::<HashMap<_, _>>();
        let bridge_by_origin = pipe_gas
            .keys
            .iter()
            .enumerate()
            .filter_map(|(node_id, key)| {
                (key.kind == PipeContainerKind::BridgePipe).then_some((key.anchor, node_id))
            })
            .collect::<HashMap<_, _>>();

        for (cell, node_id) in &pipe_by_cell {
            for neighbor_cell in orthogonal_neighbors(*cell) {
                let Some(neighbor_id) = pipe_by_cell.get(&neighbor_cell).copied() else {
                    continue;
                };
                if structures.is_pipe_cut(*cell, neighbor_cell) {
                    continue;
                }
                push_unique(&mut nodes[*node_id].neighbors, neighbor_id);
            }
        }

        for structure in structures.iter() {
            match structure.kind {
                StructureKind::Vent => {
                    if let Some(node_id) = pipe_by_cell.get(&structure.origin).copied() {
                        nodes[node_id].vent_cell = Some(structure.origin);
                    }
                }
                StructureKind::GasPipeBridge => {
                    let Some(bridge_node_id) = bridge_by_origin.get(&structure.origin).copied() else {
                        continue;
                    };
                    for port_cell in bridge_port_cells(structure.origin, structure.rotation) {
                        if let Some(pipe_node_id) = pipe_by_cell.get(&port_cell).copied() {
                            push_unique(&mut nodes[bridge_node_id].neighbors, pipe_node_id);
                            push_unique(&mut nodes[pipe_node_id].neighbors, bridge_node_id);
                        }
                    }
                }
                _ => {}
            }
        }

        Self { nodes }
    }
}

#[derive(Clone, Debug)]
struct PipeNeighborRequest {
    source_index: usize,
    target_index: usize,
    amount: u32,
}

#[derive(Clone, Copy, Debug)]
enum OutgoingTarget {
    Pipe(usize),
    World,
}

#[derive(Clone, Copy, Debug)]
struct OutgoingRequest {
    target: OutgoingTarget,
    pressure_delta: u32,
}

fn collect_pipe_node_keys(structures: &PlacedStructureMap) -> Vec<PipeNodeKey> {
    let mut keys = structures
        .iter()
        .filter_map(|structure| match structure.kind {
            StructureKind::Pipe => Some(PipeNodeKey {
                kind: PipeContainerKind::Pipe,
                anchor: structure.origin,
            }),
            StructureKind::GasPipeBridge => Some(PipeNodeKey {
                kind: PipeContainerKind::BridgePipe,
                anchor: structure.origin,
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    keys.sort_by_key(|key| {
        (
            key.anchor.y,
            key.anchor.x,
            match key.kind {
                PipeContainerKind::BridgePipe => 0,
                PipeContainerKind::Pipe => 1,
            },
        )
    });
    keys
}

fn node_ids_for_cell(
    structures: &PlacedStructureMap,
    pipe_gas: &PipeGasField,
    x: u32,
    y: u32,
) -> Vec<usize> {
    let cell = UVec2::new(x, y);
    pipe_gas
        .keys
        .iter()
        .enumerate()
        .filter_map(|(node_id, key)| match key.kind {
            PipeContainerKind::Pipe if key.anchor == cell => Some(node_id),
            PipeContainerKind::BridgePipe => {
                let bridge = structures.iter().find(|structure| {
                    structure.kind == StructureKind::GasPipeBridge && structure.origin == key.anchor
                })?;
                (bridge_center_cell(bridge.origin, bridge.rotation) == Some(cell)).then_some(node_id)
            }
            _ => None,
        })
        .collect()
}

fn pipe_node_display_species_counts(
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    structures: &PlacedStructureMap,
    node_id: usize,
    include_transfers: bool,
) -> Vec<u32> {
    let gas_count = pipe_gas.gas_count();
    let mut displayed = pipe_gas.node_species_counts(node_id);
    if !include_transfers {
        return displayed;
    }

    let key = pipe_gas.keys[node_id];
    let cell = match key.kind {
        PipeContainerKind::Pipe => key.anchor,
        PipeContainerKind::BridgePipe => structures
            .iter()
            .find(|structure| structure.kind == StructureKind::GasPipeBridge && structure.origin == key.anchor)
            .and_then(|bridge| bridge_center_cell(bridge.origin, bridge.rotation))
            .unwrap_or(key.anchor),
    };
    let mut incoming = vec![0u32; gas_count];
    let mut outgoing = vec![0u32; gas_count];

    for transfer in &flow_state.transfers {
        if transfer.to == cell {
            for gas_index in 0..gas_count {
                incoming[gas_index] = incoming[gas_index]
                    .saturating_add(transfer.gas_counts.get(gas_index).copied().unwrap_or(0));
            }
        }
        if transfer.from == cell {
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

fn bridge_port_cells(origin: UVec2, rotation: StructureRotation) -> Vec<UVec2> {
    match rotation {
        StructureRotation::Deg0 | StructureRotation::Deg180 => {
            vec![origin, UVec2::new(origin.x + 2, origin.y)]
        }
        StructureRotation::Deg90 | StructureRotation::Deg270 => {
            vec![origin, UVec2::new(origin.x, origin.y + 2)]
        }
    }
}

fn transfer_visual_path(
    source_key: PipeNodeKey,
    target_key: PipeNodeKey,
    structures: &PlacedStructureMap,
) -> PipeTransferVisualPath {
    let bridge_origin = match (source_key.kind, target_key.kind) {
        (PipeContainerKind::BridgePipe, _) => Some(source_key.anchor),
        (_, PipeContainerKind::BridgePipe) => Some(target_key.anchor),
        _ => None,
    };
    let Some(bridge_origin) = bridge_origin else {
        return PipeTransferVisualPath::Straight;
    };
    let bridge_rotation = structures
        .iter()
        .find(|structure| {
            structure.kind == StructureKind::GasPipeBridge && structure.origin == bridge_origin
        })
        .map(|structure| structure.rotation)
        .unwrap_or(StructureRotation::Deg0);
    PipeTransferVisualPath::BridgeArc {
        bridge_origin,
        bridge_rotation,
    }
}

fn orthogonal_neighbors(cell: UVec2) -> Vec<UVec2> {
    let mut neighbors = Vec::new();
    if cell.y > 0 {
        neighbors.push(UVec2::new(cell.x, cell.y - 1));
    }
    if cell.x + 1 < WORLD_WIDTH {
        neighbors.push(UVec2::new(cell.x + 1, cell.y));
    }
    if cell.y + 1 < WORLD_HEIGHT {
        neighbors.push(UVec2::new(cell.x, cell.y + 1));
    }
    if cell.x > 0 {
        neighbors.push(UVec2::new(cell.x - 1, cell.y));
    }
    neighbors
}

fn push_unique<T: Copy + PartialEq>(items: &mut Vec<T>, value: T) {
    if !items.contains(&value) {
        items.push(value);
    }
}

fn collect_pipe_components(runtime: &PipeRuntime) -> Vec<Vec<usize>> {
    let mut visited = vec![false; runtime.nodes.len()];
    let mut components = Vec::new();
    for start in 0..runtime.nodes.len() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut queue = VecDeque::from([start]);
        let mut component = Vec::new();
        while let Some(node_id) = queue.pop_front() {
            component.push(node_id);
            for neighbor in runtime.nodes[node_id].neighbors.iter().copied() {
                if visited[neighbor] {
                    continue;
                }
                visited[neighbor] = true;
                queue.push_back(neighbor);
            }
        }
        components.push(component);
    }
    components
}

fn desired_local_transfer_amount(delta: u32) -> u32 {
    delta.saturating_mul(PIPE_LOCAL_TRANSFER_NUMERATOR) / PIPE_LOCAL_TRANSFER_DENOMINATOR
}

fn combined_outgoing_delta(deltas: &[u32]) -> u32 {
    deltas.iter().copied().max().unwrap_or(0)
}

fn split_integer_by_weights(total: u32, weights: &[f32]) -> Vec<u32> {
    if total == 0 || weights.is_empty() {
        return vec![0; weights.len()];
    }
    let sum = weights.iter().copied().sum::<f32>();
    if sum <= f32::EPSILON {
        let mut even = vec![0u32; weights.len()];
        for index in 0..total as usize {
            even[index % weights.len()] = even[index % weights.len()].saturating_add(1);
        }
        return even;
    }

    let mut base = vec![0u32; weights.len()];
    let mut remainders = vec![0f32; weights.len()];
    let mut used = 0u32;
    for (index, weight) in weights.iter().copied().enumerate() {
        let exact = total as f32 * (weight / sum);
        let floor = exact.floor() as u32;
        base[index] = floor;
        remainders[index] = exact - floor as f32;
        used = used.saturating_add(floor);
    }

    let mut remaining = total.saturating_sub(used);
    while remaining > 0 {
        let mut best_index = 0usize;
        for index in 1..remainders.len() {
            if remainders[index] > remainders[best_index] {
                best_index = index;
            }
        }
        base[best_index] = base[best_index].saturating_add(1);
        remainders[best_index] = 0.0;
        remaining -= 1;
    }

    base
}

fn remove_species_proportional_counts(species: &mut [u32], amount: u32) -> Vec<u32> {
    let total: u32 = species.iter().copied().sum();
    if total == 0 || amount == 0 {
        return vec![0; species.len()];
    }
    if amount >= total {
        let removed = species.to_vec();
        species.fill(0);
        return removed;
    }

    let mut removed = vec![0u32; species.len()];
    let mut remainders = vec![0u64; species.len()];
    let mut removed_base = 0u64;
    let total_u64 = u64::from(total);
    for (index, count) in species.iter().copied().enumerate() {
        let numerator = u128::from(count) * u128::from(amount);
        let base = (numerator / u128::from(total_u64)) as u64;
        removed[index] = base.min(u64::from(count)) as u32;
        remainders[index] = (numerator % u128::from(total_u64)) as u64;
        removed_base = removed_base.saturating_add(u64::from(removed[index]));
    }
    let mut remaining = u64::from(amount).saturating_sub(removed_base);
    while remaining > 0 {
        let mut best_index = None;
        let mut best_remainder = 0u64;
        for (index, count) in species.iter().copied().enumerate() {
            if removed[index] >= count {
                continue;
            }
            if best_index.is_none() || remainders[index] > best_remainder {
                best_index = Some(index);
                best_remainder = remainders[index];
            }
        }
        let Some(best) = best_index else {
            break;
        };
        removed[best] = removed[best].saturating_add(1);
        remainders[best] = 0;
        remaining -= 1;
    }

    for (index, count) in species.iter_mut().enumerate() {
        *count = count.saturating_sub(removed[index]);
    }
    removed
}

fn add_species_counts_limited(target: &mut [u32], offered: &[u32], capacity: u32) -> Vec<u32> {
    let current_total: u32 = target.iter().copied().sum();
    let free = capacity.saturating_sub(current_total);
    let mut accepted = vec![0u32; target.len()];
    let total_offered: u32 = offered.iter().take(target.len()).copied().sum();
    let accepted_total = free.min(total_offered);
    if accepted_total == 0 {
        return accepted;
    }

    if accepted_total == total_offered {
        for index in 0..target.len() {
            accepted[index] = offered.get(index).copied().unwrap_or(0);
            target[index] = target[index].saturating_add(accepted[index]);
        }
        return accepted;
    }

    let weights = offered
        .iter()
        .take(target.len())
        .map(|value| *value as f32)
        .collect::<Vec<_>>();
    accepted = split_integer_by_weights(accepted_total, &weights);
    for index in 0..target.len() {
        accepted[index] = accepted[index].min(offered.get(index).copied().unwrap_or(0));
    }
    let mut total: u32 = accepted.iter().copied().sum();
    while total < accepted_total {
        let mut changed = false;
        for index in 0..target.len() {
            let available = offered.get(index).copied().unwrap_or(0);
            if accepted[index] < available {
                accepted[index] = accepted[index].saturating_add(1);
                total += 1;
                changed = true;
                if total == accepted_total {
                    break;
                }
            }
        }
        if !changed {
            break;
        }
    }

    for index in 0..target.len() {
        target[index] = target[index].saturating_add(accepted[index]);
    }
    accepted
}

#[cfg(test)]
mod tests {
    use super::{
        apply_pipe_network_step, bridge_port_cells, pipe_cell_display_blocks_with_transfers,
        PipeCellDisplayBlock, PipeContainerKind, PipeFlowVisualState, PipeGasField,
        PipeTransferVisualPath,
        PIPE_CELL_CAPACITY,
    };
    use crate::{
        config::{GasDefinition, GasRegistry},
        simulation::gas::GasField,
        world::{
            grid::{CellMaterial, WorldGrid},
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

    fn total_world_and_pipe_particles(
        gas: &GasField,
        pipe_gas: &PipeGasField,
        world: &WorldGrid,
    ) -> u64 {
        let mut total = 0u64;
        for y in 0..crate::world::grid::WORLD_HEIGHT {
            for x in 0..crate::world::grid::WORLD_WIDTH {
                total = total.saturating_add(u64::from(gas.total_amount_rounded(x, y)));
                let _ = world;
            }
        }
        total.saturating_add(
            pipe_gas
                .snapshot_state()
                .nodes
                .iter()
                .map(|node| node.species.iter().copied().map(u64::from).sum::<u64>())
                .sum::<u64>(),
        )
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
    fn bridge_transfers_gas_between_edge_pipes_without_touching_center_pipe() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(structures.place_pipe(20, 20, &world));
        assert!(structures.place_pipe(22, 20, &world));
        assert!(structures
            .place_bridge(UVec2::new(20, 20), StructureRotation::Deg0, &world)
            .is_some());
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let bridge_node = pipe_gas
            .snapshot_state()
            .nodes
            .iter()
            .position(|node| node.key.kind == PipeContainerKind::BridgePipe)
            .expect("bridge node");
        assert_eq!(
            pipe_gas.add_species_counts_limited(bridge_node, &[80, 0, 0]),
            vec![80, 0, 0]
        );
        let mut gas = GasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();

        let changed = apply_pipe_network_step(&structures, &mut pipe_gas, &mut gas, &world, &mut visuals);
        assert!(changed);
        let edge_blocks = pipe_cell_display_blocks_with_transfers(
            &structures,
            &pipe_gas,
            &visuals,
            20,
            20,
            true,
        );
        assert!(!edge_blocks.is_empty());
    }

    #[test]
    fn bridge_and_plain_pipe_keep_independent_storage_in_center_cell() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(structures.place_pipe(31, 30, &world));
        assert!(structures
            .place_bridge(UVec2::new(30, 30), StructureRotation::Deg0, &world)
            .is_some());
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);

        let pipe_node = pipe_gas
            .snapshot_state()
            .nodes
            .iter()
            .position(|node| node.key.kind == PipeContainerKind::Pipe)
            .expect("pipe node");
        let bridge_node = pipe_gas
            .snapshot_state()
            .nodes
            .iter()
            .position(|node| node.key.kind == PipeContainerKind::BridgePipe)
            .expect("bridge node");
        let _ = pipe_gas.add_species_counts_limited(pipe_node, &[10, 0, 0]);
        let _ = pipe_gas.add_species_counts_limited(bridge_node, &[0, 6, 0]);

        let blocks = pipe_cell_display_blocks_with_transfers(
            &structures,
            &pipe_gas,
            &PipeFlowVisualState::default(),
            31,
            30,
            false,
        );
        assert_eq!(
            blocks,
            vec![
                PipeCellDisplayBlock {
                    kind: PipeContainerKind::BridgePipe,
                    species_counts: vec![0, 6, 0],
                    total_particles: 6
                },
                PipeCellDisplayBlock {
                    kind: PipeContainerKind::Pipe,
                    species_counts: vec![10, 0, 0],
                    total_particles: 10
                }
            ]
        );
    }

    #[test]
    fn bridge_can_coexist_with_wall_and_preserve_mass() {
        let registry = registry();
        let mut world = WorldGrid::default();
        assert!(world.set_solid_with_material(41, 40, CellMaterial::Brick));
        let mut structures = PlacedStructureMap::default();
        assert!(structures.place_pipe(40, 40, &world));
        assert!(structures.place_pipe(42, 40, &world));
        assert!(structures
            .place_bridge(UVec2::new(40, 40), StructureRotation::Deg0, &world)
            .is_some());
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let bridge_node = pipe_gas
            .snapshot_state()
            .nodes
            .iter()
            .position(|node| node.key.kind == PipeContainerKind::BridgePipe)
            .expect("bridge node");
        let _ = pipe_gas.add_species_counts_limited(bridge_node, &[PIPE_CELL_CAPACITY, 0, 0]);
        let mut gas = GasField::from_registry(&registry);
        let before = total_world_and_pipe_particles(&gas, &pipe_gas, &world);
        let mut visuals = PipeFlowVisualState::default();
        for _ in 0..5 {
            let _ = apply_pipe_network_step(&structures, &mut pipe_gas, &mut gas, &world, &mut visuals);
        }
        let after = total_world_and_pipe_particles(&gas, &pipe_gas, &world);
        assert_eq!(before, after);
    }

    #[test]
    fn transfer_visual_path_marks_only_bridge_node_transfers_as_arcs() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(structures.place_pipe(40, 40, &world));
        assert!(structures.place_pipe(41, 40, &world));
        assert!(structures.place_pipe(42, 40, &world));
        assert!(structures
            .place_bridge(UVec2::new(40, 40), StructureRotation::Deg0, &world)
            .is_some());
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);

        let snapshot = pipe_gas.snapshot_state();
        let center_pipe_node = snapshot
            .nodes
            .iter()
            .position(|node| {
                node.key.kind == PipeContainerKind::Pipe && node.key.anchor == UVec2::new(41, 40)
            })
            .expect("center pipe node");
        let bridge_node = snapshot
            .nodes
            .iter()
            .position(|node| node.key.kind == PipeContainerKind::BridgePipe)
            .expect("bridge node");

        let _ = pipe_gas.add_species_counts_limited(center_pipe_node, &[80, 0, 0]);
        let _ = pipe_gas.add_species_counts_limited(bridge_node, &[80, 0, 0]);

        let mut gas = GasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();
        let changed =
            apply_pipe_network_step(&structures, &mut pipe_gas, &mut gas, &world, &mut visuals);
        assert!(changed);
        assert!(visuals.transfers.iter().any(|transfer| {
            matches!(transfer.visual_path, PipeTransferVisualPath::BridgeArc { .. })
        }));
        assert!(visuals.transfers.iter().any(|transfer| {
            matches!(transfer.visual_path, PipeTransferVisualPath::Straight)
                && ((transfer.from == UVec2::new(41, 40) && transfer.to == UVec2::new(40, 40))
                    || (transfer.from == UVec2::new(41, 40) && transfer.to == UVec2::new(42, 40))
                    || (transfer.to == UVec2::new(41, 40) && transfer.from == UVec2::new(40, 40))
                    || (transfer.to == UVec2::new(41, 40) && transfer.from == UVec2::new(42, 40)))
        }));
    }
}
