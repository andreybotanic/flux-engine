use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::{
    config::GasRegistry,
    simulation::gas::GasField,
    world::{
        grid::{WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
        structures::{bridge_center_cell, PlacedStructureMap, StructureParams, StructureRotation},
    },
};

pub mod pressure;
pub mod scenarios;
mod solver;

/// Registers supporting resources required by the built-in default runtime plugin.
pub struct DefaultPluginSupportPlugin;

impl Plugin for DefaultPluginSupportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PipeSimulationConfig>()
            .init_resource::<PipeFlowVisualState>()
            .add_systems(Startup, initialize_pipe_state_from_registry);
    }
}

fn initialize_pipe_state_from_registry(mut commands: Commands, registry: Res<GasRegistry>) {
    commands.insert_resource(PipeGasField::from_registry(&registry));
    commands.insert_resource(PipeFluxField::default());
}

#[cfg(test)]
pub(crate) fn pipe_flow_reset_needed(previous_paused: Option<bool>, current_paused: bool) -> bool {
    previous_paused
        .map(|previous| previous != current_paused)
        .unwrap_or(false)
}

#[derive(Resource, Clone, Copy)]
/// Stores configuration for the conveyor-style pipe simulation owned by `flux.default`.
pub struct PipeSimulationConfig {
    pub cell_volume_ratio: f32,
    pub cell_particle_pressure_pa: f32,
    pub pipe_step_interval_ticks: u32,
    pub max_pipe_hop_particles_per_step: u32,
    pub min_pipe_branch_residual_particles: u32,
    pub vent_discharge_coefficient: f32,
    pub max_vent_flux_particles_per_tick: f32,
    pub vent_choked_pressure_ratio: f32,
    pub pressure_epsilon_pa: f32,
    pub pump_input_pressure_pa: f32,
}

impl Default for PipeSimulationConfig {
    fn default() -> Self {
        Self {
            cell_volume_ratio: 25.0,
            cell_particle_pressure_pa: 0.2,
            pipe_step_interval_ticks: 10,
            max_pipe_hop_particles_per_step: 50_000,
            min_pipe_branch_residual_particles: 1,
            vent_discharge_coefficient: 7.8,
            max_vent_flux_particles_per_tick: 200_000.0,
            vent_choked_pressure_ratio: 0.53,
            pressure_epsilon_pa: 0.01,
            pump_input_pressure_pa: -100.0,
        }
    }
}

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

impl PartialOrd for PipeNodeKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PipeNodeKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.anchor.y, self.anchor.x, self.kind).cmp(&(other.anchor.y, other.anchor.x, other.kind))
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PipeEdgeKey {
    a: PipeNodeKey,
    b: PipeNodeKey,
}

impl PipeEdgeKey {
    fn new(first: PipeNodeKey, second: PipeNodeKey) -> Self {
        if first <= second {
            Self {
                a: first,
                b: second,
            }
        } else {
            Self {
                a: second,
                b: first,
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct PipeFluxTopologySignature {
    node_keys: Vec<PipeNodeKey>,
    edges: Vec<PipeEdgeKey>,
    port_nodes: Vec<(PipeNodeKey, PipeRuntimePortKind)>,
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

    /// Returns the number of known pipe nodes.
    pub fn node_count(&self) -> usize {
        self.keys.len()
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

    /// Returns the stable identity of one node.
    pub fn node_key(&self, node_id: usize) -> Option<PipeNodeKey> {
        self.keys.get(node_id).copied()
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

    /// Adds gas counts to one node without any pipe-capacity limit.
    pub fn add_species_counts(&mut self, node_id: usize, offered: &[u32]) {
        let Some(cell) = self.cells.get_mut(node_id) else {
            return;
        };
        for gas_index in 0..self.gas_count {
            cell[gas_index] =
                cell[gas_index].saturating_add(offered.get(gas_index).copied().unwrap_or(0));
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
            self.set_species_counts_exact(node_id, &counts);
        }
        Ok(())
    }
}

#[derive(Resource, Clone, Default)]
/// Stores persistent pipe-edge flux between ticks for the pressure solver.
pub struct PipeFluxField {
    signature: PipeFluxTopologySignature,
    flux_by_edge: HashMap<PipeEdgeKey, f32>,
}

impl PipeFluxField {
    /// Clears every remembered edge flux.
    pub fn clear_all(&mut self) {
        self.signature = PipeFluxTopologySignature::default();
        self.flux_by_edge.clear();
    }

    #[cfg(test)]
    /// Returns the number of remembered pipe edges.
    pub(crate) fn edge_count(&self) -> usize {
        self.flux_by_edge.len()
    }

    fn sync_to_runtime(&mut self, runtime: &PipeRuntime, pipe_gas: &PipeGasField) {
        let signature = pipe_flux_topology_signature(runtime, pipe_gas);
        if self.signature == signature {
            return;
        }
        self.signature = signature.clone();
        self.flux_by_edge = signature
            .edges
            .iter()
            .copied()
            .map(|edge| (edge, 0.0))
            .collect();
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
    pub from_kind: PipeContainerKind,
    pub to: UVec2,
    pub to_kind: PipeContainerKind,
    pub gas_counts: Vec<u32>,
    pub total_amount: u32,
    pub visual_path: PipeTransferVisualPath,
}

#[derive(Resource, Default, Clone)]
/// Stores `PipeFlowVisualState` state.
pub struct PipeFlowVisualState {
    pub transfers: Vec<PipeTransferRecord>,
    pub previous_transfers: Vec<PipeTransferRecord>,
    pub interval_ticks: u32,
    pub tick_in_interval: u32,
    pub flow_progress: f32,
    pub hop_started_this_tick: bool,
}

impl PipeFlowVisualState {
    /// Advances one simulation tick and returns whether this tick should execute a pipe hop.
    pub fn begin_tick(&mut self, configured_interval_ticks: u32) -> bool {
        let interval_ticks = configured_interval_ticks.max(1);
        if self.interval_ticks != interval_ticks {
            self.interval_ticks = interval_ticks;
            self.tick_in_interval = 0;
            self.flow_progress = 0.0;
            self.hop_started_this_tick = false;
        }

        let is_hop_tick = self.tick_in_interval == 0;
        self.hop_started_this_tick = is_hop_tick;
        self.flow_progress = if interval_ticks <= 1 {
            1.0
        } else {
            self.tick_in_interval as f32 / interval_ticks as f32
        };
        self.tick_in_interval = (self.tick_in_interval + 1) % interval_ticks;
        is_hop_tick
    }

    /// Returns the current interpolation phase of the active conveyor interval.
    pub fn flow_progress(&self) -> f32 {
        self.flow_progress
    }

    /// Rotates transfer buffers when a new hop starts so rendering can keep the previous hop endpoint for one boundary tick.
    pub fn begin_hop_recording(&mut self) {
        self.previous_transfers = std::mem::take(&mut self.transfers);
    }

    /// Returns transfers/progress to render for this tick.
    ///
    /// Renderer always uses current hop transfers so the first boundary frame
    /// (`progress = 0`) is visible and no animation step is skipped.
    pub fn render_view(&self) -> (&[PipeTransferRecord], f32) {
        (&self.transfers, self.flow_progress)
    }

    /// Clears transfer history and resets conveyor animation phase.
    pub fn reset_flow(&mut self) {
        self.transfers.clear();
        self.previous_transfers.clear();
        self.tick_in_interval = 0;
        self.flow_progress = 0.0;
        self.hop_started_this_tick = false;
    }
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
    pipe_flux: &mut PipeFluxField,
    gas: &mut GasField,
    world: &WorldGrid,
    visual_state: &mut PipeFlowVisualState,
    config: &PipeSimulationConfig,
) -> bool {
    solver::apply_pipe_network_step(
        structures,
        pipe_gas,
        pipe_flux,
        gas,
        world,
        visual_state,
        config,
    )
}

/// Builds `[DEBUG]` HUD lines for pipe/vent diagnostics in one hovered world cell.
///
/// The output mirrors intake gating used by the conveyor solver:
/// direction, outlet, vent buffer block, pressure request and node capacity.
pub fn build_pipe_debug_hud_lines_for_cell(
    cell: UVec2,
    structures: &PlacedStructureMap,
    pipe_gas: &PipeGasField,
    gas: &GasField,
    config: &PipeSimulationConfig,
    flow_state: &PipeFlowVisualState,
) -> Vec<String> {
    solver::build_pipe_debug_hud_lines_for_cell(cell, structures, pipe_gas, gas, config, flow_state)
}

#[cfg(test)]
pub(crate) fn offer_based_intake_particles_for_test(
    config: &PipeSimulationConfig,
    offer_pa: f32,
    path_count: u32,
) -> u32 {
    solver::offer_based_intake_particles_for_test(config, offer_pa, path_count)
}

/// Applies default plugin gas source/sink structures before the core cell-gas step.
pub fn apply_gas_structures_pre_step(
    structures: &PlacedStructureMap,
    gas: &mut GasField,
    world: &WorldGrid,
) -> bool {
    let mut changed = false;
    for structure in structures.iter() {
        match structure.params {
            StructureParams::GasSource { gas_index, amount }
                if structure.kind
                    == crate::plugins::default_plugin::gas_source_structure_kind() =>
            {
                if gas.add_particles_no_impulse(
                    structure.origin.x,
                    structure.origin.y,
                    gas_index,
                    amount,
                    world,
                ) > 0
                {
                    changed = true;
                }
            }
            StructureParams::GasSink { amount }
                if structure.kind == crate::plugins::default_plugin::gas_sink_structure_kind() =>
            {
                if gas.remove_particles_proportional(
                    structure.origin.x,
                    structure.origin.y,
                    amount,
                    world,
                ) > 0
                {
                    changed = true;
                }
            }
            StructureParams::None
            | StructureParams::GasSource { .. }
            | StructureParams::GasSink { .. } => {}
        }
    }

    if changed {
        gas.recompute_total_density_buffer(world);
    }

    changed
}

#[derive(Clone, Debug)]
struct PipeRuntimeNode {
    visual_cell: UVec2,
    neighbors: Vec<usize>,
    port: Option<PipeRuntimePort>,
}

struct PipeRuntime {
    nodes: Vec<PipeRuntimeNode>,
    pumps: Vec<PipeRuntimePump>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum PipeRuntimePortKind {
    Vent,
    PumpIn,
    PumpOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PipeRuntimePort {
    kind: PipeRuntimePortKind,
    world_cell: UVec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PipeRuntimePump {
    input_node: Option<usize>,
    output_node: Option<usize>,
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
                                crate::plugins::default_plugin::is_gas_pipe_bridge_structure(
                                    structure.kind,
                                ) && structure.origin == key.anchor
                            })
                            .map(|structure| structure.rotation)
                            .unwrap_or(StructureRotation::Deg0);
                        bridge_center_cell(key.anchor, rotation).unwrap_or(key.anchor)
                    }
                },
                neighbors: Vec::new(),
                port: None,
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
        let mut pump_ports = Vec::new();
        let mut blocked_internal_edges = HashSet::new();
        for structure in structures.iter() {
            if !crate::plugins::default_plugin::is_gas_pump_structure(structure.kind) {
                continue;
            }
            let (input_cell, output_cell) = pump_port_cells(structure.origin, structure.rotation);
            pump_ports.push((structure.id, input_cell, output_cell));
            if let Some(edge) = normalized_cell_edge(input_cell, output_cell) {
                blocked_internal_edges.insert(edge);
            }
        }

        for (cell, node_id) in &pipe_by_cell {
            for neighbor_cell in orthogonal_neighbors(*cell) {
                let Some(neighbor_id) = pipe_by_cell.get(&neighbor_cell).copied() else {
                    continue;
                };
                if structures.is_pipe_cut(*cell, neighbor_cell) {
                    continue;
                }
                if let Some(edge) = normalized_cell_edge(*cell, neighbor_cell) {
                    if blocked_internal_edges.contains(&edge) {
                        continue;
                    }
                }
                push_unique(&mut nodes[*node_id].neighbors, neighbor_id);
            }
        }

        for structure in structures.iter() {
            if crate::plugins::default_plugin::is_vent_structure(structure.kind) {
                if let Some(node_id) = pipe_by_cell.get(&structure.origin).copied() {
                    nodes[node_id].port = Some(PipeRuntimePort {
                        kind: PipeRuntimePortKind::Vent,
                        world_cell: structure.origin,
                    });
                }
            } else if crate::plugins::default_plugin::is_gas_pipe_bridge_structure(structure.kind) {
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
        }
        let pumps = pump_ports
            .into_iter()
            .map(|(_pump_id, input_cell, output_cell)| {
                let input_node = pipe_by_cell.get(&input_cell).copied();
                let output_node = pipe_by_cell.get(&output_cell).copied();
                if let Some(node_id) = input_node {
                    nodes[node_id].port = Some(PipeRuntimePort {
                        kind: PipeRuntimePortKind::PumpIn,
                        world_cell: input_cell,
                    });
                }
                if let Some(node_id) = output_node {
                    nodes[node_id].port = Some(PipeRuntimePort {
                        kind: PipeRuntimePortKind::PumpOut,
                        world_cell: output_cell,
                    });
                }
                PipeRuntimePump {
                    input_node,
                    output_node,
                }
            })
            .collect::<Vec<_>>();

        Self { nodes, pumps }
    }
}

fn collect_pipe_node_keys(structures: &PlacedStructureMap) -> Vec<PipeNodeKey> {
    let mut keys = structures
        .iter()
        .filter_map(|structure| {
            if crate::plugins::default_plugin::is_pipe_structure(structure.kind) {
                Some(PipeNodeKey {
                    kind: PipeContainerKind::Pipe,
                    anchor: structure.origin,
                })
            } else if crate::plugins::default_plugin::is_gas_pipe_bridge_structure(structure.kind) {
                Some(PipeNodeKey {
                    kind: PipeContainerKind::BridgePipe,
                    anchor: structure.origin,
                })
            } else {
                None
            }
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
                    crate::plugins::default_plugin::is_gas_pipe_bridge_structure(structure.kind)
                        && structure.origin == key.anchor
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
                crate::plugins::default_plugin::is_gas_pipe_bridge_structure(structure.kind)
                    && structure.origin == key.anchor
            })
            .and_then(|bridge| bridge_center_cell(bridge.origin, bridge.rotation))
            .unwrap_or(key.anchor),
    };
    let mut incoming = vec![0u32; gas_count];
    let mut outgoing = vec![0u32; gas_count];

    for transfer in &flow_state.transfers {
        if transfer.to == cell && transfer.to_kind == key.kind {
            for gas_index in 0..gas_count {
                incoming[gas_index] = incoming[gas_index]
                    .saturating_add(transfer.gas_counts.get(gas_index).copied().unwrap_or(0));
            }
        }
        if transfer.from == cell && transfer.from_kind == key.kind {
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

fn pump_port_cells(origin: UVec2, rotation: StructureRotation) -> (UVec2, UVec2) {
    match rotation {
        StructureRotation::Deg0 => (origin, UVec2::new(origin.x + 1, origin.y)),
        StructureRotation::Deg90 => (origin, UVec2::new(origin.x, origin.y + 1)),
        StructureRotation::Deg180 => (UVec2::new(origin.x + 1, origin.y), origin),
        StructureRotation::Deg270 => (UVec2::new(origin.x, origin.y + 1), origin),
    }
}

fn normalized_cell_edge(a: UVec2, b: UVec2) -> Option<[UVec2; 2]> {
    let dx = a.x as i32 - b.x as i32;
    let dy = a.y as i32 - b.y as i32;
    if dx.abs() + dy.abs() != 1 {
        return None;
    }
    Some(if (a.y, a.x) <= (b.y, b.x) {
        [a, b]
    } else {
        [b, a]
    })
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
            crate::plugins::default_plugin::is_gas_pipe_bridge_structure(structure.kind)
                && structure.origin == bridge_origin
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

fn pipe_flux_topology_signature(
    runtime: &PipeRuntime,
    pipe_gas: &PipeGasField,
) -> PipeFluxTopologySignature {
    let mut edges = Vec::new();
    let mut port_nodes = Vec::new();
    for (node_id, node) in runtime.nodes.iter().enumerate() {
        let source_key = pipe_gas.keys[node_id];
        if let Some(port) = node.port {
            port_nodes.push((source_key, port.kind));
        }
        for neighbor in node.neighbors.iter().copied() {
            let target_key = pipe_gas.keys[neighbor];
            if source_key < target_key {
                edges.push(PipeEdgeKey::new(source_key, target_key));
            }
        }
    }
    edges.sort();
    port_nodes.sort();
    PipeFluxTopologySignature {
        node_keys: pipe_gas.keys.clone(),
        edges,
        port_nodes,
    }
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

#[cfg(test)]
mod tests;
