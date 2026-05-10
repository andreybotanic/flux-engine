use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    config::GasRegistry,
    save::WorldLoadState,
    simulation::{
        backend::{SimulationBackend, SimulationBackendConfig},
        gas::GasField,
        GpuRuntimeState, SimulationControl, SimulationPerfStats, SimulationSet,
    },
    world::{
        grid::{WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
        structures::{bridge_center_cell, PlacedStructureMap, StructureParams, StructureRotation},
    },
};

pub mod pressure;
pub mod scenarios;
mod solver;

/// Registers the built-in default plugin runtime systems for pipe-owned behavior.
pub struct DefaultPluginRuntimePlugin;

impl Plugin for DefaultPluginRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PipeSimulationConfig>()
            .init_resource::<PipeFlowVisualState>()
            .add_systems(Startup, initialize_pipe_state_from_registry)
            .add_systems(Update, clear_stale_pipe_flow_on_pause_transition)
            .add_systems(
                FixedUpdate,
                run_default_plugin_pre_gas_step.in_set(SimulationSet::PluginPreStep),
            );
    }
}

fn initialize_pipe_state_from_registry(mut commands: Commands, registry: Res<GasRegistry>) {
    commands.insert_resource(PipeGasField::from_registry(&registry));
    commands.insert_resource(PipeFluxField::default());
}

pub(crate) fn pipe_flow_reset_needed(previous_paused: Option<bool>, current_paused: bool) -> bool {
    previous_paused
        .map(|previous| previous != current_paused)
        .unwrap_or(false)
}

fn clear_stale_pipe_flow_on_pause_transition(
    control: Res<SimulationControl>,
    mut pipe_flow_visuals: ResMut<PipeFlowVisualState>,
    mut previous_paused: Local<Option<bool>>,
) {
    if pipe_flow_reset_needed(*previous_paused, control.paused) {
        pipe_flow_visuals.transfers.clear();
    }
    *previous_paused = Some(control.paused);
}

fn run_default_plugin_pre_gas_step(
    control: Res<SimulationControl>,
    backend: Res<SimulationBackendConfig>,
    world_load_state: Res<WorldLoadState>,
    config: Res<PipeSimulationConfig>,
    structures: Res<PlacedStructureMap>,
    mut pipe_gas: ResMut<PipeGasField>,
    mut pipe_flux: ResMut<PipeFluxField>,
    mut pipe_flow_visuals: ResMut<PipeFlowVisualState>,
    mut gas: ResMut<GasField>,
    world: Res<WorldGrid>,
    mut perf: ResMut<SimulationPerfStats>,
    mut gpu_state: ResMut<GpuRuntimeState>,
) {
    if !world_load_state.has_world || control.paused {
        return;
    }

    let pipe_started_at = std::time::Instant::now();
    let changed_by_pipes = apply_pipe_network_step(
        &structures,
        &mut pipe_gas,
        &mut pipe_flux,
        &mut gas,
        &world,
        &mut pipe_flow_visuals,
        &config,
    );
    let pipe_elapsed_ms = pipe_started_at.elapsed().as_secs_f32() * 1000.0;
    perf.last_pipe_step_ms = pipe_elapsed_ms;
    perf.avg_pipe_step_ms = if perf.avg_pipe_step_ms <= f32::EPSILON {
        pipe_elapsed_ms
    } else {
        perf.avg_pipe_step_ms * 0.9 + pipe_elapsed_ms * 0.1
    };

    let changed_by_structures = apply_gas_structures_pre_step(&structures, &mut gas, &world);
    if (changed_by_pipes || changed_by_structures) && backend.backend == SimulationBackend::Gpu {
        gpu_state.mark_needs_full_upload();
    }
}

#[derive(Resource, Clone, Copy)]
/// Stores configuration for the pressure-driven pipe simulation owned by `flux.default`.
pub struct PipeSimulationConfig {
    pub cell_volume_ratio: f32,
    pub cell_particle_pressure_pa: f32,
    pub pipe_flux_gain: f32,
    pub pipe_flux_damping: f32,
    pub max_pipe_flux_particles_per_tick: f32,
    pub vent_discharge_coefficient: f32,
    pub max_vent_flux_particles_per_tick: f32,
    pub vent_choked_pressure_ratio: f32,
    pub pressure_epsilon_pa: f32,
}

impl Default for PipeSimulationConfig {
    fn default() -> Self {
        Self {
            cell_volume_ratio: 25.0,
            cell_particle_pressure_pa: 1.0,
            pipe_flux_gain: 8_000.0,
            pipe_flux_damping: 0.993,
            max_pipe_flux_particles_per_tick: 50_000.0,
            vent_discharge_coefficient: 7.8,
            max_vent_flux_particles_per_tick: 200_000.0,
            vent_choked_pressure_ratio: 0.53,
            pressure_epsilon_pa: 0.01,
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

    fn sign_for(self, source: PipeNodeKey, target: PipeNodeKey) -> f32 {
        if self.a == source && self.b == target {
            1.0
        } else {
            -1.0
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct PipeFluxTopologySignature {
    node_keys: Vec<PipeNodeKey>,
    edges: Vec<PipeEdgeKey>,
    vent_nodes: Vec<PipeNodeKey>,
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

    fn signed_flux(&self, source: PipeNodeKey, target: PipeNodeKey) -> f32 {
        let key = PipeEdgeKey::new(source, target);
        self.flux_by_edge.get(&key).copied().unwrap_or(0.0) * key.sign_for(source, target)
    }

    fn set_signed_flux(&mut self, source: PipeNodeKey, target: PipeNodeKey, flux: f32) {
        let key = PipeEdgeKey::new(source, target);
        self.flux_by_edge
            .insert(key, flux * key.sign_for(source, target));
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
            if crate::plugins::default_plugin::is_vent_structure(structure.kind) {
                if let Some(node_id) = pipe_by_cell.get(&structure.origin).copied() {
                    nodes[node_id].vent_cell = Some(structure.origin);
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

        Self { nodes }
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
    let mut vent_nodes = Vec::new();
    for (node_id, node) in runtime.nodes.iter().enumerate() {
        let source_key = pipe_gas.keys[node_id];
        if node.vent_cell.is_some() {
            vent_nodes.push(source_key);
        }
        for neighbor in node.neighbors.iter().copied() {
            let target_key = pipe_gas.keys[neighbor];
            if source_key < target_key {
                edges.push(PipeEdgeKey::new(source_key, target_key));
            }
        }
    }
    edges.sort();
    vent_nodes.sort();
    PipeFluxTopologySignature {
        node_keys: pipe_gas.keys.clone(),
        edges,
        vent_nodes,
    }
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

#[cfg(test)]
mod tests;
