use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;

use crate::{
    config::GasRegistry,
    simulation::gas::GasField,
    world::{
        grid::{WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
        structures::{bridge_center_cell, PlacedStructureMap, StructureKind, StructureRotation},
    },
};

mod solver;

pub(crate) const DEFAULT_PIPE_CELL_CAPACITY: u32 = 1_000;
/// Nominal full-segment amount used by UI/tests and as the default config value.
pub const PIPE_CELL_CAPACITY: u32 = DEFAULT_PIPE_CELL_CAPACITY;
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
    capacity_particles: u32,
    keys: Vec<PipeNodeKey>,
    cells: Vec<Vec<u32>>,
}

impl Default for PipeGasField {
    fn default() -> Self {
        Self {
            gas_count: 0,
            capacity_particles: DEFAULT_PIPE_CELL_CAPACITY,
            keys: Vec::new(),
            cells: Vec::new(),
        }
    }
}

impl PipeGasField {
    /// Builds an empty field sized for the current gas registry.
    pub fn from_registry(registry: &GasRegistry) -> Self {
        Self::from_registry_with_capacity(registry, DEFAULT_PIPE_CELL_CAPACITY)
    }

    /// Builds an empty field sized for the current gas registry with a custom segment capacity.
    pub fn from_registry_with_capacity(registry: &GasRegistry, capacity_particles: u32) -> Self {
        Self {
            gas_count: registry.count(),
            capacity_particles: capacity_particles.max(1),
            keys: Vec::new(),
            cells: Vec::new(),
        }
    }

    /// Returns the number of registered gases.
    pub fn gas_count(&self) -> usize {
        self.gas_count
    }

    /// Returns the configured capacity of one pipe segment.
    pub fn capacity_particles(&self) -> u32 {
        self.capacity_particles
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
        self.capacity_particles
            .saturating_sub(self.total_amount_particles(node_id))
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
    pub fn remove_particles_proportional_counts(
        &mut self,
        node_id: usize,
        amount: u32,
    ) -> Vec<u32> {
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
        let accept_total = self
            .capacity_particles
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
            if total > self.capacity_particles {
                return Err(format!(
                    "Pipe gas snapshot exceeds capacity for node {:?}: {} > {}",
                    key, total, self.capacity_particles
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
    config: &crate::simulation::PipeSimulationConfig,
) -> bool {
    solver::apply_pipe_network_step(structures, pipe_gas, gas, world, visual_state, config)
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
                                structure.kind == StructureKind::GasPipeBridge
                                    && structure.origin == key.anchor
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
            .filter_map(|(node_id, key)| {
                (key.kind == PipeContainerKind::Pipe).then_some((key.anchor, node_id))
            })
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
                    let Some(bridge_node_id) = bridge_by_origin.get(&structure.origin).copied()
                    else {
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
struct PipeEdgePlan {
    source_index: usize,
    target_index: usize,
    amount: u32,
}

#[derive(Clone, Debug, Default)]
struct PipeComponentTransferPlan {
    edge_plans: Vec<PipeEdgePlan>,
}

#[derive(Clone, Debug, Default)]
struct PipeNodeDemand {
    distance_to_sink: Option<u32>,
}

#[derive(Clone, Debug)]
struct PipeComponentVentProfile {
    inlet_indices: Vec<usize>,
    outlet_indices: Vec<usize>,
    target_fill: u32,
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
                (bridge_center_cell(bridge.origin, bridge.rotation) == Some(cell))
                    .then_some(node_id)
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
            .find(|structure| {
                structure.kind == StructureKind::GasPipeBridge && structure.origin == key.anchor
            })
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

fn build_sink_oriented_pipe_transfer_plan(
    runtime: &PipeRuntime,
    component: &[usize],
    starting_pipe_totals: &[u32],
    pipe_to_world_requests: &[u32],
) -> Vec<PipeEdgePlan> {
    let mut distances = vec![None; component.len()];
    let mut queue = VecDeque::new();
    for (index, amount) in pipe_to_world_requests.iter().copied().enumerate() {
        if amount == 0 {
            continue;
        }
        distances[index] = Some(0u32);
        queue.push_back(index);
    }
    while let Some(node_index) = queue.pop_front() {
        let next_distance = distances[node_index].unwrap_or(0).saturating_add(1);
        for neighbor in component_neighbor_indices(runtime, component, node_index) {
            if distances[neighbor].is_some() {
                continue;
            }
            distances[neighbor] = Some(next_distance);
            queue.push_back(neighbor);
        }
    }

    let max_distance = distances
        .iter()
        .filter_map(|distance| *distance)
        .max()
        .unwrap_or(0);
    let mut requested_edges = Vec::<PipeEdgePlan>::new();
    let mut closer_requested = vec![0u32; component.len()];
    for distance in 0..=max_distance {
        for node_index in 0..component.len() {
            if distances[node_index] != Some(distance) {
                continue;
            }
            let total_need =
                pipe_to_world_requests[node_index].saturating_add(closer_requested[node_index]);
            if total_need == 0 {
                continue;
            }

            let parents = component_neighbor_indices(runtime, component, node_index)
                .into_iter()
                .filter(|neighbor| distances[*neighbor] == Some(distance.saturating_add(1)))
                .collect::<Vec<_>>();
            if parents.is_empty() {
                continue;
            }

            let split = split_integer_by_weights(
                total_need,
                &parents
                    .iter()
                    .map(|parent| {
                        let total = starting_pipe_totals[*parent];
                        if total == 0 {
                            1.0
                        } else {
                            total as f32
                        }
                    })
                    .collect::<Vec<_>>(),
            );
            for (parent_index, requested_amount) in parents.into_iter().zip(split.into_iter()) {
                if requested_amount == 0 {
                    continue;
                }
                requested_edges.push(PipeEdgePlan {
                    source_index: parent_index,
                    target_index: node_index,
                    amount: requested_amount,
                });
                closer_requested[parent_index] =
                    closer_requested[parent_index].saturating_add(requested_amount);
            }
        }
    }

    let mut accepted = Vec::new();
    for distance in (1..=max_distance).rev() {
        for source_index in 0..component.len() {
            if distances[source_index] != Some(distance) {
                continue;
            }
            let outgoing_requests = requested_edges
                .iter()
                .filter(|request| request.source_index == source_index)
                .cloned()
                .collect::<Vec<_>>();
            if outgoing_requests.is_empty() {
                continue;
            }
            let requested_amounts = outgoing_requests
                .iter()
                .map(|request| request.amount)
                .collect::<Vec<_>>();
            let accepted_total =
                starting_pipe_totals[source_index].min(requested_amounts.iter().copied().sum());
            let accepted_split = split_bounded_integer_requests(accepted_total, &requested_amounts);
            for (request, accepted_amount) in outgoing_requests.into_iter().zip(accepted_split) {
                if accepted_amount == 0 {
                    continue;
                }
                accepted.push(PipeEdgePlan {
                    source_index: request.source_index,
                    target_index: request.target_index,
                    amount: accepted_amount,
                });
            }
        }
    }
    accepted
}

fn build_demand_driven_pipe_transfer_plan(
    runtime: &PipeRuntime,
    component: &[usize],
    starting_pipe_totals: &[u32],
    _starting_world_totals: &[Option<u32>],
    world_to_pipe_requests: &[u32],
    pipe_to_world_requests: &[u32],
    vent_profile: Option<&PipeComponentVentProfile>,
) -> Option<Vec<PipeEdgePlan>> {
    let Some(profile) = vent_profile else {
        return None;
    };
    if profile.inlet_indices.is_empty() {
        return None;
    }

    let mut demands = vec![PipeNodeDemand::default(); component.len()];
    let mut queue = VecDeque::new();
    for &inlet_index in &profile.inlet_indices {
        demands[inlet_index].distance_to_sink = Some(0);
        queue.push_back(inlet_index);
    }
    while let Some(node_index) = queue.pop_front() {
        let next_distance = demands[node_index]
            .distance_to_sink
            .unwrap_or(0)
            .saturating_add(1);
        for neighbor in component_neighbor_indices(runtime, component, node_index) {
            if demands[neighbor].distance_to_sink.is_some() {
                continue;
            }
            demands[neighbor].distance_to_sink = Some(next_distance);
            queue.push_back(neighbor);
        }
    }

    let max_distance = demands
        .iter()
        .filter_map(|demand| demand.distance_to_sink)
        .max()
        .unwrap_or(0);
    if max_distance == 0 && profile.outlet_indices.is_empty() {
        return None;
    }

    let mut requested_edges = Vec::<PipeEdgePlan>::new();
    let mut upstream_requests = vec![0u32; component.len()];
    for distance in (0..=max_distance).rev() {
        for node_index in 0..component.len() {
            if demands[node_index].distance_to_sink != Some(distance) {
                continue;
            }
            let source_total = starting_pipe_totals[node_index];
            let reserved_world_in = world_to_pipe_requests[node_index]
                .min(profile.target_fill.saturating_sub(source_total));
            let local_free_capacity = profile
                .target_fill
                .saturating_sub(source_total)
                .saturating_sub(reserved_world_in);
            let terminal_demand =
                local_free_capacity.saturating_add(pipe_to_world_requests[node_index]);
            let total_need = terminal_demand.saturating_add(upstream_requests[node_index]);
            if total_need == 0 || distance == 0 {
                continue;
            }

            let parents = component_neighbor_indices(runtime, component, node_index)
                .into_iter()
                .filter(|neighbor| {
                    demands[*neighbor].distance_to_sink == Some(distance.saturating_sub(1))
                })
                .collect::<Vec<_>>();
            if parents.is_empty() {
                continue;
            }

            let weights = parents
                .iter()
                .map(|parent| {
                    let total = starting_pipe_totals[*parent];
                    if total == 0 {
                        1.0
                    } else {
                        total as f32
                    }
                })
                .collect::<Vec<_>>();
            let split = split_integer_by_weights(total_need, &weights);
            for (parent_index, requested_amount) in parents.into_iter().zip(split.into_iter()) {
                if requested_amount == 0 {
                    continue;
                }
                requested_edges.push(PipeEdgePlan {
                    source_index: parent_index,
                    target_index: node_index,
                    amount: requested_amount,
                });
                upstream_requests[parent_index] =
                    upstream_requests[parent_index].saturating_add(requested_amount);
            }
        }
    }

    let mut plan = PipeComponentTransferPlan::default();
    for distance in 0..max_distance {
        for source_index in 0..component.len() {
            if demands[source_index].distance_to_sink != Some(distance) {
                continue;
            }
            let outgoing_requests = requested_edges
                .iter()
                .filter(|request| request.source_index == source_index)
                .cloned()
                .collect::<Vec<_>>();
            if outgoing_requests.is_empty() {
                continue;
            }
            let requested_amounts = outgoing_requests
                .iter()
                .map(|request| request.amount)
                .collect::<Vec<_>>();
            let accepted_total =
                starting_pipe_totals[source_index].min(requested_amounts.iter().copied().sum());
            let accepted_split = split_bounded_integer_requests(accepted_total, &requested_amounts);
            for (request, accepted_amount) in outgoing_requests.into_iter().zip(accepted_split) {
                if accepted_amount == 0 {
                    continue;
                }
                plan.edge_plans.push(PipeEdgePlan {
                    source_index: request.source_index,
                    target_index: request.target_index,
                    amount: accepted_amount,
                });
            }
        }
    }

    Some(plan.edge_plans)
}

fn build_local_pipe_transfer_plan(
    runtime: &PipeRuntime,
    component: &[usize],
    starting_pipe_totals: &[u32],
    capacity_particles: u32,
) -> Vec<PipeEdgePlan> {
    let mut requested_edges = Vec::<PipeEdgePlan>::new();
    let mut accepted_outgoing = vec![0u32; component.len()];

    for (source_index, _) in component.iter().copied().enumerate() {
        let source_total = starting_pipe_totals[source_index];
        if source_total == 0 {
            continue;
        }

        let outgoing_requests = component_neighbor_indices(runtime, component, source_index)
            .into_iter()
            .filter_map(|target_index| {
                let target_total = starting_pipe_totals[target_index];
                if source_total > target_total {
                    Some(PipeEdgePlan {
                        source_index,
                        target_index,
                        amount: source_total - target_total,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if outgoing_requests.is_empty() {
            continue;
        }

        let combined_delta = combined_outgoing_delta(
            &outgoing_requests
                .iter()
                .map(|request| request.amount)
                .collect::<Vec<_>>(),
        );
        let desired_total = desired_local_transfer_amount(combined_delta).min(source_total);
        if desired_total == 0 {
            continue;
        }

        let split = split_integer_by_weights(
            desired_total,
            &outgoing_requests
                .iter()
                .map(|request| request.amount as f32)
                .collect::<Vec<_>>(),
        );
        for (request, accepted_amount) in outgoing_requests.into_iter().zip(split.into_iter()) {
            if accepted_amount == 0 {
                continue;
            }
            accepted_outgoing[source_index] =
                accepted_outgoing[source_index].saturating_add(accepted_amount);
            requested_edges.push(PipeEdgePlan {
                source_index: request.source_index,
                target_index: request.target_index,
                amount: accepted_amount,
            });
        }
    }

    let mut accepted_edges = vec![0u32; requested_edges.len()];
    for target_index in 0..component.len() {
        let room = capacity_particles
            .saturating_sub(starting_pipe_totals[target_index])
            .saturating_add(accepted_outgoing[target_index]);
        let incoming = requested_edges
            .iter()
            .enumerate()
            .filter_map(|(edge_index, edge)| {
                (edge.target_index == target_index).then_some((edge_index, edge.amount))
            })
            .collect::<Vec<_>>();
        let total_requested: u32 = incoming.iter().map(|(_, amount)| *amount).sum();
        if total_requested == 0 {
            continue;
        }
        let accepted = split_bounded_integer_requests(
            room.min(total_requested),
            &incoming
                .iter()
                .map(|(_, amount)| *amount)
                .collect::<Vec<_>>(),
        );
        for ((edge_index, _), accepted_amount) in incoming.into_iter().zip(accepted.into_iter()) {
            accepted_edges[edge_index] = accepted_amount;
        }
    }

    requested_edges
        .into_iter()
        .zip(accepted_edges)
        .filter_map(|(request, accepted_amount)| {
            (accepted_amount > 0).then_some(PipeEdgePlan {
                source_index: request.source_index,
                target_index: request.target_index,
                amount: accepted_amount,
            })
        })
        .collect()
}

fn component_neighbor_indices(
    runtime: &PipeRuntime,
    component: &[usize],
    node_index: usize,
) -> Vec<usize> {
    runtime.nodes[component[node_index]]
        .neighbors
        .iter()
        .filter_map(|neighbor| {
            component
                .iter()
                .position(|candidate| *candidate == *neighbor)
        })
        .collect()
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

fn split_bounded_integer_requests(total: u32, requests: &[u32]) -> Vec<u32> {
    if total == 0 || requests.is_empty() {
        return vec![0; requests.len()];
    }
    let mut accepted = split_integer_by_weights(
        total,
        &requests
            .iter()
            .map(|request| *request as f32)
            .collect::<Vec<_>>(),
    );
    for (index, request) in requests.iter().copied().enumerate() {
        accepted[index] = accepted[index].min(request);
    }

    let mut accepted_total: u32 = accepted.iter().copied().sum();
    while accepted_total < total {
        let mut changed = false;
        for (index, request) in requests.iter().copied().enumerate() {
            if accepted[index] >= request {
                continue;
            }
            accepted[index] = accepted[index].saturating_add(1);
            accepted_total = accepted_total.saturating_add(1);
            changed = true;
            if accepted_total == total {
                break;
            }
        }
        if !changed {
            break;
        }
    }

    accepted
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
        PipeTransferVisualPath, PIPE_CELL_CAPACITY,
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

    fn pipe_config() -> crate::simulation::PipeSimulationConfig {
        crate::simulation::PipeSimulationConfig::default()
    }

    fn pipe_pressure_for(total: u32) -> f32 {
        super::solver::pipe_pressure(&pipe_config(), total)
    }

    fn world_pressure_for(total: u32) -> f32 {
        super::solver::world_pressure(&pipe_config(), total)
    }

    fn pressures_match_within(a: f32, b: f32, relative_tolerance: f32) -> bool {
        let denom = a.abs().max(b.abs()).max(1.0);
        ((a - b).abs() / denom) <= relative_tolerance
    }

    fn has_transfer_between(visuals: &PipeFlowVisualState, a: UVec2, b: UVec2) -> bool {
        visuals.transfers.iter().any(|transfer| {
            transfer.total_amount > 0
                && ((transfer.from == a && transfer.to == b)
                    || (transfer.from == b && transfer.to == a))
        })
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

    fn pipe_node_total_at(pipe_gas: &PipeGasField, kind: PipeContainerKind, cell: UVec2) -> u32 {
        let snapshot = pipe_gas.snapshot_state();
        let node_id = snapshot
            .nodes
            .iter()
            .position(|node| node.key.kind == kind && node.key.anchor == cell)
            .expect("pipe node at cell");
        pipe_gas.total_amount_particles(node_id)
    }

    fn place_horizontal_pipe_line(
        structures: &mut PlacedStructureMap,
        world: &WorldGrid,
        start: UVec2,
        length: u32,
    ) {
        for step in 0..length {
            assert!(structures.place_pipe(start.x + step, start.y, world));
        }
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

        let changed = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals,
            &pipe_config(),
        );
        assert!(changed);
        let edge_blocks =
            pipe_cell_display_blocks_with_transfers(&structures, &pipe_gas, &visuals, 20, 20, true);
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
            let _ = apply_pipe_network_step(
                &structures,
                &mut pipe_gas,
                &mut gas,
                &world,
                &mut visuals,
                &pipe_config(),
            );
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
        let changed = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals,
            &pipe_config(),
        );
        assert!(changed);
        assert!(visuals.transfers.iter().any(|transfer| {
            matches!(
                transfer.visual_path,
                PipeTransferVisualPath::BridgeArc { .. }
            )
        }));
        assert!(visuals.transfers.iter().any(|transfer| {
            matches!(transfer.visual_path, PipeTransferVisualPath::Straight)
                && ((transfer.from == UVec2::new(41, 40) && transfer.to == UVec2::new(40, 40))
                    || (transfer.from == UVec2::new(41, 40) && transfer.to == UVec2::new(42, 40))
                    || (transfer.to == UVec2::new(41, 40) && transfer.from == UVec2::new(40, 40))
                    || (transfer.to == UVec2::new(41, 40) && transfer.from == UVec2::new(42, 40)))
        }));
    }

    #[test]
    fn world_to_pipe_input_is_not_relayed_beyond_one_edge_per_tick() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        place_horizontal_pipe_line(&mut structures, &world, UVec2::new(10, 10), 3);
        assert!(structures.place_vent(10, 10, &world));

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(10, 10, 0, 1_000.0);
        let mut visuals = PipeFlowVisualState::default();

        let changed = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals,
            &pipe_config(),
        );
        assert!(changed);
        assert!(pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(10, 10)) > 0);
        assert_eq!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(11, 10)),
            0
        );
        assert_eq!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(12, 10)),
            0
        );
    }

    #[test]
    fn source_refill_keeps_upstream_full_while_front_advances_one_edge_per_tick() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        place_horizontal_pipe_line(&mut structures, &world, UVec2::new(14, 14), 3);
        assert!(structures.place_vent(14, 14, &world));

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        for cell in [UVec2::new(14, 14), UVec2::new(15, 14)] {
            let snapshot = pipe_gas.snapshot_state();
            let node_id = snapshot
                .nodes
                .iter()
                .position(|node| {
                    node.key.kind == PipeContainerKind::Pipe && node.key.anchor == cell
                })
                .expect("pipe node");
            let _ = pipe_gas.add_species_counts_limited(node_id, &[PIPE_CELL_CAPACITY, 0, 0]);
        }

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(14, 14, 0, 100_000.0);
        let mut visuals = PipeFlowVisualState::default();
        let changed = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals,
            &pipe_config(),
        );
        assert!(changed);
        assert_eq!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(14, 14)),
            PIPE_CELL_CAPACITY
        );
        assert_eq!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(15, 14)),
            PIPE_CELL_CAPACITY
        );
        assert_eq!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(16, 14)),
            200
        );
    }

    #[test]
    fn demand_driven_sink_pulls_through_filled_pipe_component() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        place_horizontal_pipe_line(&mut structures, &world, UVec2::new(20, 20), 4);
        assert!(structures.place_vent(23, 20, &world));

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        for x in 20..=23 {
            let snapshot = pipe_gas.snapshot_state();
            let node_id = snapshot
                .nodes
                .iter()
                .position(|node| {
                    node.key.kind == PipeContainerKind::Pipe && node.key.anchor == UVec2::new(x, 20)
                })
                .expect("pipe node");
            let _ = pipe_gas.add_species_counts_limited(node_id, &[PIPE_CELL_CAPACITY, 0, 0]);
        }

        let mut gas = GasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();
        let changed = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals,
            &pipe_config(),
        );
        assert!(changed);
        assert!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(20, 20))
                < PIPE_CELL_CAPACITY
        );
        assert_eq!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(21, 20)),
            PIPE_CELL_CAPACITY
        );
        assert_eq!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(22, 20)),
            PIPE_CELL_CAPACITY
        );
        assert_eq!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(23, 20)),
            PIPE_CELL_CAPACITY
        );
    }

    #[test]
    fn short_pipe_without_sink_demand_does_not_oscillate() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(structures.place_pipe(30, 30, &world));
        assert!(structures.place_pipe(31, 30, &world));

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let snapshot = pipe_gas.snapshot_state();
        let left_node = snapshot
            .nodes
            .iter()
            .position(|node| node.key.anchor == UVec2::new(30, 30))
            .expect("left node");
        let _ = pipe_gas.add_species_counts_limited(left_node, &[PIPE_CELL_CAPACITY, 0, 0]);

        let mut gas = GasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();
        for _ in 0..4 {
            let _ = apply_pipe_network_step(
                &structures,
                &mut pipe_gas,
                &mut gas,
                &world,
                &mut visuals,
                &pipe_config(),
            );
        }

        assert_eq!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(30, 30)),
            PIPE_CELL_CAPACITY / 2
        );
        assert_eq!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(31, 30)),
            PIPE_CELL_CAPACITY / 2
        );
    }

    #[test]
    fn branch_split_favors_larger_downstream_deficit() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        for cell in [
            UVec2::new(40, 40),
            UVec2::new(41, 40),
            UVec2::new(42, 40),
            UVec2::new(41, 39),
        ] {
            assert!(structures.place_pipe(cell.x, cell.y, &world));
        }

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        for (cell, amount) in [
            (UVec2::new(40, 40), 0),
            (UVec2::new(41, 40), PIPE_CELL_CAPACITY / 2),
            (UVec2::new(42, 40), 0),
            (UVec2::new(41, 39), PIPE_CELL_CAPACITY * 4 / 10),
        ] {
            let snapshot = pipe_gas.snapshot_state();
            let node_id = snapshot
                .nodes
                .iter()
                .position(|node| {
                    node.key.kind == PipeContainerKind::Pipe && node.key.anchor == cell
                })
                .expect("pipe node");
            let _ = pipe_gas.add_species_counts_limited(node_id, &[amount, 0, 0]);
        }

        let mut gas = GasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals,
            &pipe_config(),
        );

        let center_to_right = visuals
            .transfers
            .iter()
            .find(|transfer| {
                transfer.from == UVec2::new(41, 40) && transfer.to == UVec2::new(42, 40)
            })
            .map(|transfer| transfer.total_amount)
            .unwrap_or(0);
        let center_to_up = visuals
            .transfers
            .iter()
            .find(|transfer| {
                transfer.from == UVec2::new(41, 40) && transfer.to == UVec2::new(41, 39)
            })
            .map(|transfer| transfer.total_amount)
            .unwrap_or(0);
        assert!(center_to_right > center_to_up);
    }

    #[test]
    fn equal_parallel_routes_share_sink_pull_with_edge_limited_front() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        for cell in [UVec2::new(52, 49), UVec2::new(52, 50), UVec2::new(52, 51)] {
            assert!(structures.place_pipe(cell.x, cell.y, &world));
        }
        assert!(structures.place_vent(52, 50, &world));

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        for cell in [UVec2::new(52, 49), UVec2::new(52, 51)] {
            let snapshot = pipe_gas.snapshot_state();
            let node_id = snapshot
                .nodes
                .iter()
                .position(|node| {
                    node.key.kind == PipeContainerKind::Pipe && node.key.anchor == cell
                })
                .expect("parallel node");
            let _ = pipe_gas.add_species_counts_limited(node_id, &[PIPE_CELL_CAPACITY, 0, 0]);
        }

        let mut gas = GasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals,
            &pipe_config(),
        );

        assert_eq!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(52, 49)),
            PIPE_CELL_CAPACITY - 200
        );
        assert_eq!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(52, 51)),
            PIPE_CELL_CAPACITY - 200
        );
        assert_eq!(
            pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(52, 50)),
            400
        );
    }

    #[test]
    fn long_pipe_high_pressure_to_vacuum_fills_with_dense_front() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        place_horizontal_pipe_line(&mut structures, &world, UVec2::new(5, 60), 50);
        assert!(structures.place_vent(5, 60, &world));
        assert!(structures.place_vent(54, 60, &world));

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let mut gas = GasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();
        let mut first_reach_tick = vec![None; 50];

        for tick in 0..520u32 {
            gas.set_amount(5, 60, 0, 100_000.0);
            gas.set_amount(54, 60, 0, 0.0);
            let _ = apply_pipe_network_step(
                &structures,
                &mut pipe_gas,
                &mut gas,
                &world,
                &mut visuals,
                &pipe_config(),
            );
            for offset in 0..50u32 {
                let total = pipe_node_total_at(
                    &pipe_gas,
                    PipeContainerKind::Pipe,
                    UVec2::new(5 + offset, 60),
                );
                if total >= 900 && first_reach_tick[offset as usize].is_none() {
                    first_reach_tick[offset as usize] = Some(tick);
                }
            }
        }

        let totals = (5..=54)
            .map(|x| pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(x, 60)))
            .collect::<Vec<_>>();
        assert!(
            totals[..49].iter().all(|total| *total >= 990),
            "expected every internal segment to stay at or above nominal full, got {totals:?}"
        );
        assert!(
            totals[49] >= 200,
            "expected outlet vent segment to stay pressurized while exchanging with vacuum, got {totals:?}"
        );
        for window in totals[..49].windows(2) {
            assert!(
                window[0].abs_diff(window[1]) <= 10,
                "neighbor gradient is too steep: {totals:?}"
            );
        }
        let reach_ticks = first_reach_tick[..49]
            .iter()
            .copied()
            .collect::<Option<Vec<_>>>()
            .expect("every internal cell should see the dense front");
        let average_ticks_per_cell = reach_ticks[..49]
            .windows(2)
            .map(|pair| pair[1].saturating_sub(pair[0]) as f32)
            .sum::<f32>()
            / ((49usize.saturating_sub(1)) as f32).max(1.0);
        assert!(
            average_ticks_per_cell <= 10.0,
            "expected dense front to move no slower than 10 ticks/cell, got {average_ticks_per_cell}"
        );
    }

    #[test]
    fn small_two_vent_head_does_not_keep_entire_pipe_fully_saturated() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        place_horizontal_pipe_line(&mut structures, &world, UVec2::new(10, 70), 6);
        assert!(structures.place_vent(10, 70, &world));
        assert!(structures.place_vent(15, 70, &world));

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        for x in 10..=15 {
            let snapshot = pipe_gas.snapshot_state();
            let node_id = snapshot
                .nodes
                .iter()
                .position(|node| {
                    node.key.kind == PipeContainerKind::Pipe && node.key.anchor == UVec2::new(x, 70)
                })
                .expect("pipe node");
            let _ = pipe_gas.add_species_counts_limited(node_id, &[PIPE_CELL_CAPACITY, 0, 0]);
        }

        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(10, 70, 0, 1_200.0);
        gas.set_amount(15, 70, 0, 900.0);
        let mut visuals = PipeFlowVisualState::default();
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals,
            &pipe_config(),
        );

        let totals = (10..=15)
            .map(|x| pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(x, 70)))
            .collect::<Vec<_>>();
        assert!(
            totals.iter().any(|total| *total < PIPE_CELL_CAPACITY),
            "expected at least one segment to drop below full capacity, got {totals:?}"
        );
    }

    #[test]
    fn two_single_cell_rooms_connected_by_one_pipe_equalize_pressure() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        place_horizontal_pipe_line(&mut structures, &world, UVec2::new(30, 74), 8);
        assert!(structures.place_vent(30, 74, &world));
        assert!(structures.place_vent(37, 74, &world));

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(30, 74, 0, 20_000.0);
        gas.set_amount(37, 74, 0, 10_000.0);
        let mut visuals = PipeFlowVisualState::default();

        for _ in 0..10_000 {
            let _ = apply_pipe_network_step(
                &structures,
                &mut pipe_gas,
                &mut gas,
                &world,
                &mut visuals,
                &pipe_config(),
            );
        }

        let left_pressure = world_pressure_for(gas.total_amount_rounded(30, 74));
        let right_pressure = world_pressure_for(gas.total_amount_rounded(37, 74));
        assert!(
            pressures_match_within(left_pressure, right_pressure, 0.03),
            "room pressures did not converge closely enough: left={left_pressure}, right={right_pressure}"
        );
        for x in 30..=37 {
            let pipe_total =
                pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(x, 74));
            let pipe_pressure = pipe_pressure_for(pipe_total);
            assert!(
                pressures_match_within(pipe_pressure, left_pressure, 0.20),
                "pipe segment at x={x} did not match room pressure: pipe_pressure={pipe_pressure}, room_pressure={left_pressure}"
            );
        }
    }

    #[test]
    fn parallel_p_branch_inside_one_network_equalizes_pressure_and_uses_both_routes() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        place_horizontal_pipe_line(&mut structures, &world, UVec2::new(10, 40), 13);
        for cell in [
            UVec2::new(14, 39),
            UVec2::new(14, 38),
            UVec2::new(15, 38),
            UVec2::new(16, 38),
            UVec2::new(17, 38),
            UVec2::new(18, 38),
            UVec2::new(18, 39),
        ] {
            assert!(structures.place_pipe(cell.x, cell.y, &world));
        }
        assert!(structures.place_vent(10, 40, &world));
        assert!(structures.place_vent(22, 40, &world));

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(10, 40, 0, 20_000.0);
        gas.set_amount(22, 40, 0, 5_000.0);
        let mut visuals = PipeFlowVisualState::default();
        let mut direct_route_seen = false;
        let mut branch_route_seen = false;

        for _ in 0..20_000 {
            let _ = apply_pipe_network_step(
                &structures,
                &mut pipe_gas,
                &mut gas,
                &world,
                &mut visuals,
                &pipe_config(),
            );
            direct_route_seen |=
                has_transfer_between(&visuals, UVec2::new(15, 40), UVec2::new(16, 40));
            branch_route_seen |=
                has_transfer_between(&visuals, UVec2::new(15, 38), UVec2::new(16, 38));
        }

        assert!(
            direct_route_seen,
            "expected flow through the direct mainline segment"
        );
        assert!(
            branch_route_seen,
            "expected flow through the P-shaped branch segment"
        );
        let left_pressure = world_pressure_for(gas.total_amount_rounded(10, 40));
        let right_pressure = world_pressure_for(gas.total_amount_rounded(22, 40));
        assert!(
            pressures_match_within(left_pressure, right_pressure, 0.03),
            "room pressures did not converge: left={left_pressure}, right={right_pressure}"
        );
    }

    #[test]
    fn single_high_pressure_vent_fills_dead_end_pipe() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        place_horizontal_pipe_line(&mut structures, &world, UVec2::new(10, 50), 20);
        assert!(structures.place_vent(10, 50, &world));

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let mut gas = GasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();

        for _ in 0..100 {
            gas.set_amount(10, 50, 0, 100_000.0);
            let _ = apply_pipe_network_step(
                &structures,
                &mut pipe_gas,
                &mut gas,
                &world,
                &mut visuals,
                &pipe_config(),
            );
        }

        let totals = (10..30)
            .map(|x| pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(x, 50)))
            .collect::<Vec<_>>();
        assert!(
            totals.iter().all(|total| *total >= 990),
            "expected dead-end pipe to become fully saturated, got {totals:?}"
        );
    }

    #[test]
    fn three_room_star_network_equalizes_pressure() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        for cell in [
            UVec2::new(60, 56),
            UVec2::new(60, 57),
            UVec2::new(60, 58),
            UVec2::new(60, 59),
            UVec2::new(60, 60),
            UVec2::new(59, 60),
            UVec2::new(58, 60),
            UVec2::new(57, 60),
            UVec2::new(56, 60),
            UVec2::new(61, 60),
            UVec2::new(62, 60),
            UVec2::new(63, 60),
            UVec2::new(64, 60),
        ] {
            assert!(structures.place_pipe(cell.x, cell.y, &world));
        }
        assert!(structures.place_vent(60, 56, &world));
        assert!(structures.place_vent(56, 60, &world));
        assert!(structures.place_vent(64, 60, &world));

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(56, 60, 0, 10_000.0);
        gas.set_amount(64, 60, 0, 50_000.0);
        let mut visuals = PipeFlowVisualState::default();

        for _ in 0..10_000 {
            let _ = apply_pipe_network_step(
                &structures,
                &mut pipe_gas,
                &mut gas,
                &world,
                &mut visuals,
                &pipe_config(),
            );
        }

        let target_pressure = world_pressure_for(gas.total_amount_rounded(60, 56));
        for cell in [UVec2::new(60, 56), UVec2::new(56, 60), UVec2::new(64, 60)] {
            let room_pressure = world_pressure_for(gas.total_amount_rounded(cell.x, cell.y));
            assert!(
                pressures_match_within(room_pressure, target_pressure, 0.03),
                "room pressure mismatch at {cell:?}: {room_pressure} vs {target_pressure}"
            );
        }
        let center_pressure = pipe_pressure_for(pipe_node_total_at(
            &pipe_gas,
            PipeContainerKind::Pipe,
            UVec2::new(60, 60),
        ));
        assert!(
            pressures_match_within(center_pressure, target_pressure, 0.06),
            "star center pressure mismatch: {center_pressure} vs {target_pressure}"
        );
    }

    #[test]
    fn pipe_drains_after_source_pressure_drop() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        place_horizontal_pipe_line(&mut structures, &world, UVec2::new(20, 66), 12);
        assert!(structures.place_vent(20, 66, &world));
        assert!(structures.place_vent(31, 66, &world));

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let mut gas = GasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();

        for _ in 0..70 {
            gas.set_amount(20, 66, 0, 100_000.0);
            gas.set_amount(31, 66, 0, 0.0);
            let _ = apply_pipe_network_step(
                &structures,
                &mut pipe_gas,
                &mut gas,
                &world,
                &mut visuals,
                &pipe_config(),
            );
        }
        for _ in 0..220 {
            gas.set_amount(20, 66, 0, 0.0);
            gas.set_amount(31, 66, 0, 0.0);
            let _ = apply_pipe_network_step(
                &structures,
                &mut pipe_gas,
                &mut gas,
                &world,
                &mut visuals,
                &pipe_config(),
            );
        }

        let totals = (20..32)
            .map(|x| pipe_node_total_at(&pipe_gas, PipeContainerKind::Pipe, UVec2::new(x, 66)))
            .collect::<Vec<_>>();
        assert!(
            totals.iter().all(|total| *total <= 50),
            "expected the line to drain after the source pressure drop, got {totals:?}"
        );
    }

    #[test]
    fn whole_component_reacts_beyond_the_first_segment_to_a_single_vacuum_vent() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        place_horizontal_pipe_line(&mut structures, &world, UVec2::new(30, 78), 6);
        assert!(structures.place_vent(35, 78, &world));

        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        for x in 30..=35 {
            let snapshot = pipe_gas.snapshot_state();
            let node_id = snapshot
                .nodes
                .iter()
                .position(|node| {
                    node.key.kind == PipeContainerKind::Pipe && node.key.anchor == UVec2::new(x, 78)
                })
                .expect("pipe node");
            let _ = pipe_gas.add_species_counts_limited(node_id, &[PIPE_CELL_CAPACITY, 0, 0]);
        }

        let mut gas = GasField::from_registry(&registry);
        let mut visuals = PipeFlowVisualState::default();
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut gas,
            &world,
            &mut visuals,
            &pipe_config(),
        );

        assert!(
            has_transfer_between(&visuals, UVec2::new(32, 78), UVec2::new(33, 78)),
            "expected the sink demand to propagate past the first segment: {:?}",
            visuals
                .transfers
                .iter()
                .map(|transfer| (transfer.from, transfer.to, transfer.total_amount))
                .collect::<Vec<_>>()
        );
    }
}
