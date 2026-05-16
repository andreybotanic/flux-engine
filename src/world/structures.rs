use std::collections::HashSet;

use bevy::prelude::*;

use crate::{
    config::{CellVisualPlacementConfigMap, StructureVisualConfigMap},
    world::grid::{
        is_boundary, is_editable_cell, linear_index, CellMaterial, WorldGrid, WORLD_HEIGHT,
        WORLD_WIDTH,
    },
};

/// Identifies the visual/runtime layer used by a structure or world cell.
///
/// # Fields
/// - `id`: Stable registered layer id backing this handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayerKind {
    id: &'static str,
}

impl LayerKind {
    /// Builds a layer id from a registered static content id.
    pub const fn new(id: &'static str) -> Self {
        Self { id }
    }

    /// Builds a layer id from runtime-registered content.
    pub fn from_registered_id(id: String) -> Self {
        Self {
            id: Box::leak(id.into_boxed_str()),
        }
    }

    /// Returns the stable id backing this layer.
    pub fn as_str(self) -> &'static str {
        self.id
    }
}

/// Core appearance layer shared by world cells and placeable structures.
pub const APPEARANCE_LAYER: LayerKind = LayerKind::new("flux.core.layer.appearance");

/// Identifies the marker rendered inside a layer cell.
///
/// # Fields
/// - `id`: Stable registered marker id backing this handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerMarkerKind {
    id: &'static str,
}

impl LayerMarkerKind {
    /// Builds a marker id from a registered static content id.
    pub const fn new(id: &'static str) -> Self {
        Self { id }
    }

    /// Builds a marker id from runtime-registered content.
    pub fn from_registered_id(id: String) -> Self {
        Self {
            id: Box::leak(id.into_boxed_str()),
        }
    }

    /// Returns the stable id backing this marker.
    pub fn as_str(self) -> &'static str {
        self.id
    }
}

/// Defines whether a layer cell participates in collision checks.
///
/// # Variants
/// - `RenderOnly`: The cell is visible but does not block placement in its layer.
/// - `Special`: The cell participates in layer-specific collision checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerCollisionKind {
    RenderOnly,
    Special,
}

/// Describes one occupied local cell inside a structure layer.
///
/// # Fields
/// - `local_cell`: Cell coordinates relative to the structure origin.
/// - `marker_kind`: Marker id rendered for this local cell.
/// - `collision`: Collision behavior of this cell inside its layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerCellSpec {
    pub local_cell: IVec2,
    pub marker_kind: LayerMarkerKind,
    pub collision: LayerCollisionKind,
}

/// Describes all cells that belong to one structure layer.
///
/// # Fields
/// - `kind`: Layer id that owns the listed cells.
/// - `cells`: Local-cell specifications that belong to this layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureLayer {
    pub kind: LayerKind,
    pub cells: Vec<LayerCellSpec>,
}

/// Stores static display/runtime information for a structure or world material.
///
/// # Fields
/// - `layers`: Layer definitions that describe footprint, markers and collisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureDescriptor {
    pub layers: Vec<StructureLayer>,
}

impl StructureDescriptor {
    /// Returns every occupied local cell, de-duplicated across layers.
    pub fn occupied_local_cells(&self) -> Vec<IVec2> {
        let mut cells = Vec::new();
        for layer in &self.layers {
            for cell in &layer.cells {
                if !cells.contains(&cell.local_cell) {
                    cells.push(cell.local_cell);
                }
            }
        }
        cells
    }

    /// Returns the inclusive local-cell bounds of the structure footprint.
    pub fn local_bounds(&self) -> Option<(IVec2, IVec2)> {
        let cells = self.occupied_local_cells();
        let mut iter = cells.into_iter();
        let first = iter.next()?;
        let mut min = first;
        let mut max = first;
        for cell in iter {
            min.x = min.x.min(cell.x);
            min.y = min.y.min(cell.y);
            max.x = max.x.max(cell.x);
            max.y = max.y.max(cell.y);
        }
        Some((min, max))
    }

    /// Returns the footprint size in world cells.
    pub fn size_in_cells(&self) -> UVec2 {
        let Some((min, max)) = self.local_bounds() else {
            return UVec2::ZERO;
        };
        UVec2::new(
            (max.x - min.x + 1).max(0) as u32,
            (max.y - min.y + 1).max(0) as u32,
        )
    }
}

/// Enumerates placeable structure kinds stored in the runtime world.
///
/// # Fields
/// - `id`: Stable registered content id backing this structure kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StructureKind {
    id: &'static str,
}

impl StructureKind {
    /// Builds a structure content id from a registered static content id.
    pub const fn new(id: &'static str) -> Self {
        Self { id }
    }

    /// Builds a structure kind from runtime-registered content.
    pub fn from_registered_id(id: String) -> Self {
        Self {
            id: Box::leak(id.into_boxed_str()),
        }
    }

    /// Returns the stable content id backing this structure kind.
    pub fn as_str(self) -> &'static str {
        self.id
    }
}

/// Stores a canonical rotation for a placed structure.
///
/// # Variants
/// - `Deg0`: Default unrotated orientation.
/// - `Deg90`: Clockwise quarter-turn orientation.
/// - `Deg180`: Half-turn orientation.
/// - `Deg270`: Clockwise three-quarter-turn orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StructureRotation {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

impl StructureRotation {
    /// Rotates the value clockwise by 90 degrees.
    pub fn rotated_right(self) -> Self {
        match self {
            Self::Deg0 => Self::Deg90,
            Self::Deg90 => Self::Deg180,
            Self::Deg180 => Self::Deg270,
            Self::Deg270 => Self::Deg0,
        }
    }

    /// Returns the bridge-friendly next rotation (`0° <-> 90°`).
    pub fn next_bridge_rotation(self) -> Self {
        match self {
            Self::Deg0 | Self::Deg180 => Self::Deg90,
            Self::Deg90 | Self::Deg270 => Self::Deg0,
        }
    }
}

/// Stores runtime parameters for structures that need editable state.
///
/// # Variants
/// - `None`: Structure has no editable runtime parameters.
/// - `GasSource`: Structure emits a configured gas substance and amount.
/// - `GasSink`: Structure consumes a configured amount of gas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureParams {
    None,
    GasSource { gas_index: usize, amount: u32 },
    GasSink { amount: u32 },
}

/// Stores one compact entity visual/topology state payload.
///
/// # Fields
/// - `0`: Opaque plugin-defined state bits serialized as raw `u16`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackedState(pub u16);

impl PackedState {
    /// Returns the raw packed state value.
    pub fn value(self) -> u16 {
        self.0
    }
}

/// Stores the stable id of one placed structure instance.
///
/// # Fields
/// - `0`: Raw runtime id used to reference one placed structure instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlacedStructureId(pub u32);

/// Stores one placed structure instance in the world.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedStructure {
    pub id: PlacedStructureId,
    pub kind: StructureKind,
    pub origin: UVec2,
    pub rotation: StructureRotation,
    pub state: PackedState,
    pub params: StructureParams,
    occupied_cells: Vec<UVec2>,
}

impl PlacedStructure {
    /// Returns the descriptor that defines this structure's layers.
    pub fn descriptor(&self) -> StructureDescriptor {
        structure_descriptor(self.kind, self.rotation)
    }

    /// Returns the footprint size in world cells.
    pub fn size_in_cells(&self) -> UVec2 {
        self.descriptor().size_in_cells()
    }

    /// Returns every occupied world cell for this structure.
    pub fn occupied_cells(&self) -> Vec<UVec2> {
        self.occupied_cells.clone()
    }
}

/// Stores one saved structure entry for schema v4 serialization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedStructureSnapshotEntry {
    pub kind: StructureKind,
    pub origin: UVec2,
    pub rotation: StructureRotation,
    pub state: PackedState,
    pub params: StructureParams,
}

/// Stores the snapshot state of the unified placed-structure map.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PlacedStructureSnapshot {
    pub entries: Vec<PlacedStructureSnapshotEntry>,
    pub pipe_cuts: Vec<[UVec2; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PipeCut {
    a: UVec2,
    b: UVec2,
}

impl PipeCut {
    fn new(a: UVec2, b: UVec2) -> Option<Self> {
        let dx = a.x as i32 - b.x as i32;
        let dy = a.y as i32 - b.y as i32;
        if dx.abs() + dy.abs() != 1 {
            return None;
        }
        Some(if (a.y, a.x) <= (b.y, b.x) {
            Self { a, b }
        } else {
            Self { a: b, b: a }
        })
    }

    fn touches(self, cell: UVec2) -> bool {
        self.a == cell || self.b == cell
    }
}

/// Stores all placeable structures plus pipe cut metadata.
#[derive(Resource, Clone)]
pub struct PlacedStructureMap {
    next_id: u32,
    structures: Vec<PlacedStructure>,
    cell_index: Vec<Vec<PlacedStructureId>>,
    pipe_cuts: HashSet<PipeCut>,
}

impl Default for PlacedStructureMap {
    fn default() -> Self {
        Self {
            next_id: 1,
            structures: Vec::new(),
            cell_index: vec![Vec::new(); (WORLD_WIDTH * WORLD_HEIGHT) as usize],
            pipe_cuts: HashSet::new(),
        }
    }
}

impl PlacedStructureMap {
    /// Returns all placed structures in stable id order.
    pub fn iter(&self) -> impl Iterator<Item = &PlacedStructure> {
        self.structures.iter()
    }

    /// Returns a structure by id.
    pub fn structure(&self, id: PlacedStructureId) -> Option<&PlacedStructure> {
        self.structures.iter().find(|structure| structure.id == id)
    }

    /// Returns every structure id that occupies the cell.
    pub fn structure_ids_at(&self, x: u32, y: u32) -> &[PlacedStructureId] {
        &self.cell_index[linear_index(x, y)]
    }

    /// Returns all structures that occupy the cell.
    pub fn structures_at(&self, x: u32, y: u32) -> Vec<&PlacedStructure> {
        self.structure_ids_at(x, y)
            .iter()
            .filter_map(|id| self.structure(*id))
            .collect()
    }

    /// Returns the first editable structure found in the cell.
    pub fn editable_structure_at(&self, x: u32, y: u32) -> Option<&PlacedStructure> {
        self.structures_at(x, y).into_iter().find(|structure| {
            crate::plugins::default_plugin::is_editable_gas_structure(structure.kind)
        })
    }

    /// Returns true when a plain pipe occupies the cell.
    pub fn has_pipe_at(&self, x: u32, y: u32) -> bool {
        self.structures_at(x, y)
            .into_iter()
            .any(|structure| crate::plugins::default_plugin::is_pipe_structure(structure.kind))
    }

    /// Returns true when a vent occupies the cell.
    pub fn has_vent_at(&self, x: u32, y: u32) -> bool {
        self.structures_at(x, y)
            .into_iter()
            .any(|structure| crate::plugins::default_plugin::is_vent_structure(structure.kind))
    }

    /// Returns true when any bridge occupies the cell.
    pub fn has_bridge_at(&self, x: u32, y: u32) -> bool {
        self.structures_at(x, y).into_iter().any(|structure| {
            crate::plugins::default_plugin::is_gas_pipe_bridge_structure(structure.kind)
        })
    }

    /// Returns true when any structure in the cell blocks solid placement.
    pub fn blocks_solid_placement(&self, x: u32, y: u32) -> bool {
        self.structures_at(x, y).into_iter().any(|structure| {
            crate::plugins::default_plugin::blocks_default_solid_placement(structure.kind)
        })
    }

    /// Places a plain pipe if the cell is valid and not already occupied by one.
    pub fn place_pipe(&mut self, x: u32, y: u32, world: &WorldGrid) -> bool {
        let placed = self.place_structure(
            crate::plugins::default_plugin::pipe_structure_kind(),
            UVec2::new(x, y),
            StructureRotation::Deg0,
            PackedState(0),
            StructureParams::None,
            world,
        );
        if placed.is_some() {
            self.refresh_pipe_states_around_cells(&[UVec2::new(x, y)]);
            true
        } else {
            false
        }
    }

    /// Applies one continuous pipe stroke and only connects cells along that stroke.
    ///
    /// New cells created by the stroke stay cut off from side-adjacent pipes unless that
    /// adjacency is explicitly part of the stroke path. Re-drawing along an existing line
    /// removes cuts between consecutive path cells and therefore restores the connection.
    pub fn apply_pipe_path(&mut self, path: &[UVec2], world: &WorldGrid) -> bool {
        if path.is_empty() {
            return false;
        }

        let mut changed = false;
        let mut stroke_pairs = HashSet::new();
        let mut newly_placed = Vec::new();
        for &cell in path {
            if self.place_pipe(cell.x, cell.y, world) {
                newly_placed.push(cell);
                changed = true;
            }
        }

        for pair in path.windows(2) {
            let Some(normalized) = PipeCut::new(pair[0], pair[1]) else {
                continue;
            };
            stroke_pairs.insert(normalized);
            if self.remove_pipe_cut(pair[0], pair[1]) {
                changed = true;
                self.refresh_pipe_states_around_cells(&[pair[0], pair[1]]);
            }
        }

        for cell in newly_placed {
            for neighbor in orthogonal_neighbors(cell) {
                if !self.has_pipe_at(neighbor.x, neighbor.y) {
                    continue;
                }
                let Some(pair) = PipeCut::new(cell, neighbor) else {
                    continue;
                };
                if stroke_pairs.contains(&pair) {
                    continue;
                }
                if self.add_pipe_cut(cell, neighbor) {
                    changed = true;
                    self.refresh_pipe_states_around_cells(&[cell, neighbor]);
                }
            }
        }

        if changed {
            self.refresh_pipe_states_around_cells(path);
        }
        changed
    }

    /// Places a vent if the cell is valid and not already occupied by one.
    pub fn place_vent(&mut self, x: u32, y: u32, world: &WorldGrid) -> bool {
        self.place_structure(
            crate::plugins::default_plugin::vent_structure_kind(),
            UVec2::new(x, y),
            StructureRotation::Deg0,
            PackedState(0),
            StructureParams::None,
            world,
        )
        .is_some()
    }

    /// Places a gas source if the cell is valid and the amount is positive.
    pub fn place_gas_source(
        &mut self,
        x: u32,
        y: u32,
        gas_index: usize,
        amount: u32,
        world: &WorldGrid,
    ) -> Option<PlacedStructureId> {
        if amount == 0 {
            return None;
        }
        self.place_structure(
            crate::plugins::default_plugin::gas_source_structure_kind(),
            UVec2::new(x, y),
            StructureRotation::Deg0,
            PackedState(0),
            StructureParams::GasSource { gas_index, amount },
            world,
        )
    }

    /// Places a gas sink if the cell is valid and the amount is positive.
    pub fn place_gas_sink(
        &mut self,
        x: u32,
        y: u32,
        amount: u32,
        world: &WorldGrid,
    ) -> Option<PlacedStructureId> {
        if amount == 0 {
            return None;
        }
        self.place_structure(
            crate::plugins::default_plugin::gas_sink_structure_kind(),
            UVec2::new(x, y),
            StructureRotation::Deg0,
            PackedState(0),
            StructureParams::GasSink { amount },
            world,
        )
    }

    /// Places a gas bridge if its footprint is valid.
    pub fn place_bridge(
        &mut self,
        origin: UVec2,
        rotation: StructureRotation,
        world: &WorldGrid,
    ) -> Option<PlacedStructureId> {
        self.place_structure(
            crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
            origin,
            rotation,
            bridge_packed_state_for_rotation(rotation),
            StructureParams::None,
            world,
        )
    }

    /// Updates an existing gas source in place.
    pub fn update_gas_source(
        &mut self,
        id: PlacedStructureId,
        gas_index: usize,
        amount: u32,
    ) -> bool {
        if amount == 0 {
            return false;
        }
        let Some(structure) = self
            .structures
            .iter_mut()
            .find(|structure| structure.id == id)
        else {
            return false;
        };
        if !crate::plugins::default_plugin::is_gas_source_structure(structure.kind) {
            return false;
        }
        let next = StructureParams::GasSource { gas_index, amount };
        if structure.params == next {
            return false;
        }
        structure.params = next;
        true
    }

    /// Updates an existing gas sink in place.
    pub fn update_gas_sink(&mut self, id: PlacedStructureId, amount: u32) -> bool {
        if amount == 0 {
            return false;
        }
        let Some(structure) = self
            .structures
            .iter_mut()
            .find(|structure| structure.id == id)
        else {
            return false;
        };
        if !crate::plugins::default_plugin::is_gas_sink_structure(structure.kind) {
            return false;
        }
        let next = StructureParams::GasSink { amount };
        if structure.params == next {
            return false;
        }
        structure.params = next;
        true
    }

    /// Removes one structure by id.
    pub fn remove_structure(&mut self, id: PlacedStructureId) -> bool {
        let Some(index) = self
            .structures
            .iter()
            .position(|structure| structure.id == id)
        else {
            return false;
        };
        let removed = self.structures.remove(index);
        let removed_is_pipe = crate::plugins::default_plugin::is_pipe_structure(removed.kind);
        for cell in removed.occupied_cells() {
            let ids = &mut self.cell_index[linear_index(cell.x, cell.y)];
            ids.retain(|candidate| *candidate != id);
            if removed_is_pipe {
                self.pipe_cuts.retain(|cut| !cut.touches(cell));
            }
        }
        if removed_is_pipe {
            self.refresh_pipe_states_around_cells(&removed.occupied_cells);
        } else if crate::plugins::default_plugin::is_gas_pipe_bridge_structure(removed.kind) {
            let mut affected = removed.occupied_cells.clone();
            if let Some(center) = bridge_center_cell(removed.origin, removed.rotation) {
                affected.push(center);
            }
            self.refresh_pipe_states_around_cells(&affected);
        }
        true
    }

    /// Removes every structure that occupies the cell and returns removed ids.
    pub fn clear_cell(&mut self, x: u32, y: u32) -> Vec<PlacedStructureId> {
        let ids = self.structure_ids_at(x, y).to_vec();
        let mut removed = Vec::new();
        for id in ids {
            if self.remove_structure(id) {
                removed.push(id);
            }
        }
        removed
    }

    /// Records a scissors cut between two adjacent world cells.
    pub fn add_pipe_cut(&mut self, a: UVec2, b: UVec2) -> bool {
        let Some(cut) = PipeCut::new(a, b) else {
            return false;
        };
        let changed = self.pipe_cuts.insert(cut);
        if changed {
            self.refresh_pipe_states_around_cells(&[a, b]);
        }
        changed
    }

    /// Removes a previously recorded scissors cut between two adjacent cells.
    pub fn remove_pipe_cut(&mut self, a: UVec2, b: UVec2) -> bool {
        let Some(cut) = PipeCut::new(a, b) else {
            return false;
        };
        let changed = self.pipe_cuts.remove(&cut);
        if changed {
            self.refresh_pipe_states_around_cells(&[a, b]);
        }
        changed
    }

    /// Returns true when a scissors cut blocks the cell adjacency.
    pub fn is_pipe_cut(&self, a: UVec2, b: UVec2) -> bool {
        PipeCut::new(a, b)
            .map(|cut| self.pipe_cuts.contains(&cut))
            .unwrap_or(false)
    }

    fn refresh_pipe_states_around_cells(&mut self, cells: &[UVec2]) {
        let mut affected_ids = std::collections::BTreeSet::new();
        for cell in cells {
            for neighbor in orthogonal_neighbors_including_self(*cell) {
                for id in self.structure_ids_at(neighbor.x, neighbor.y) {
                    affected_ids.insert(*id);
                }
            }
        }

        for id in affected_ids {
            let Some(index) = self
                .structures
                .iter()
                .position(|structure| structure.id == id)
            else {
                continue;
            };
            if !crate::plugins::default_plugin::is_pipe_structure(self.structures[index].kind) {
                continue;
            }
            let origin = self.structures[index].origin;
            let mask = self.pipe_connection_mask_for_cell(origin);
            self.structures[index].state = PackedState(mask as u16);
        }
    }

    fn pipe_connection_mask_for_cell(&self, cell: UVec2) -> u8 {
        let mut mask = 0u8;
        if cell.y > 0 {
            let neighbor = UVec2::new(cell.x, cell.y - 1);
            if self.has_pipe_at(neighbor.x, neighbor.y) && !self.is_pipe_cut(cell, neighbor) {
                mask |= 0b0001;
            }
        }
        if cell.x + 1 < WORLD_WIDTH {
            let neighbor = UVec2::new(cell.x + 1, cell.y);
            if self.has_pipe_at(neighbor.x, neighbor.y) && !self.is_pipe_cut(cell, neighbor) {
                mask |= 0b0010;
            }
        }
        if cell.y + 1 < WORLD_HEIGHT {
            let neighbor = UVec2::new(cell.x, cell.y + 1);
            if self.has_pipe_at(neighbor.x, neighbor.y) && !self.is_pipe_cut(cell, neighbor) {
                mask |= 0b0100;
            }
        }
        if cell.x > 0 {
            let neighbor = UVec2::new(cell.x - 1, cell.y);
            if self.has_pipe_at(neighbor.x, neighbor.y) && !self.is_pipe_cut(cell, neighbor) {
                mask |= 0b1000;
            }
        }
        mask
    }

    /// Saves the placed-structure state.
    pub fn snapshot_state(&self) -> PlacedStructureSnapshot {
        let mut entries = self
            .structures
            .iter()
            .map(|structure| PlacedStructureSnapshotEntry {
                kind: structure.kind,
                origin: structure.origin,
                rotation: structure.rotation,
                state: structure.state,
                params: structure.params,
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| {
            (
                entry.origin.y,
                entry.origin.x,
                kind_sort_key(entry.kind),
                rotation_sort_key(entry.rotation),
                entry.state.value(),
                params_sort_key(entry.params),
            )
        });
        let mut pipe_cuts = self
            .pipe_cuts
            .iter()
            .map(|cut| [cut.a, cut.b])
            .collect::<Vec<_>>();
        pipe_cuts.sort_by_key(|pair| (pair[0].y, pair[0].x, pair[1].y, pair[1].x));
        PlacedStructureSnapshot { entries, pipe_cuts }
    }

    /// Restores the placed-structure state from a validated snapshot.
    pub fn restore_state(
        &mut self,
        snapshot: &PlacedStructureSnapshot,
        world: &WorldGrid,
    ) -> Result<(), String> {
        let mut next = Self::default();
        for entry in &snapshot.entries {
            next.place_structure(
                entry.kind,
                entry.origin,
                entry.rotation,
                entry.state,
                entry.params,
                world,
            )
            .ok_or_else(|| {
                format!(
                    "PlacedStructure snapshot contains invalid entry {:?} at ({}, {})",
                    entry.kind, entry.origin.x, entry.origin.y
                )
            })?;
        }
        for [a, b] in &snapshot.pipe_cuts {
            if !next.add_pipe_cut(*a, *b) {
                return Err(format!(
                    "PlacedStructure snapshot contains invalid pipe cut between ({}, {}) and ({}, {})",
                    a.x, a.y, b.x, b.y
                ));
            }
        }
        *self = next;
        Ok(())
    }

    /// Returns every same-cell pipe-bearing structure in the cell.
    pub fn pipe_bearing_structures_at(&self, x: u32, y: u32) -> Vec<&PlacedStructure> {
        self.structures_at(x, y)
            .into_iter()
            .filter(|structure| {
                if crate::plugins::default_plugin::is_pipe_structure(structure.kind) {
                    structure.origin == UVec2::new(x, y)
                } else if crate::plugins::default_plugin::is_gas_pipe_bridge_structure(
                    structure.kind,
                ) {
                    bridge_center_cell(structure.origin, structure.rotation)
                        == Some(UVec2::new(x, y))
                } else {
                    false
                }
            })
            .collect()
    }

    /// Returns bridges whose connection markers occupy the cell.
    pub fn bridge_connection_structures_at(&self, x: u32, y: u32) -> Vec<&PlacedStructure> {
        self.structures_at(x, y)
            .into_iter()
            .filter(|structure| {
                crate::plugins::default_plugin::is_gas_pipe_bridge_structure(structure.kind)
                    && structure
                        .descriptor()
                        .layers
                        .iter()
                        .find(|layer| {
                            layer.kind
                                == crate::plugins::default_plugin::gas_pipe_connections_layer()
                        })
                        .map(|layer| {
                            layer.cells.iter().any(|cell| {
                                offset_world_cell(structure.origin, cell.local_cell)
                                    == Some(UVec2::new(x, y))
                            })
                        })
                        .unwrap_or(false)
            })
            .collect()
    }

    /// Places a generic structure using a descriptor supplied by the content registry.
    pub fn place_structure_with_descriptor(
        &mut self,
        kind: StructureKind,
        origin: UVec2,
        rotation: StructureRotation,
        state: PackedState,
        params: StructureParams,
        descriptor: &StructureDescriptor,
        world: &WorldGrid,
    ) -> Option<PlacedStructureId> {
        if !self.can_place_structure_with_descriptor(kind, origin, descriptor, world) {
            return None;
        }
        self.insert_structure(kind, origin, rotation, state, params, descriptor)
    }

    fn place_structure(
        &mut self,
        kind: StructureKind,
        origin: UVec2,
        rotation: StructureRotation,
        state: PackedState,
        params: StructureParams,
        world: &WorldGrid,
    ) -> Option<PlacedStructureId> {
        if !self.can_place_structure(kind, origin, rotation, world) {
            return None;
        }
        let descriptor = structure_descriptor(kind, rotation);
        self.insert_structure(kind, origin, rotation, state, params, &descriptor)
    }

    fn insert_structure(
        &mut self,
        kind: StructureKind,
        origin: UVec2,
        rotation: StructureRotation,
        state: PackedState,
        params: StructureParams,
        descriptor: &StructureDescriptor,
    ) -> Option<PlacedStructureId> {
        let occupied_cells = descriptor
            .occupied_local_cells()
            .into_iter()
            .filter_map(|local| offset_world_cell(origin, local))
            .collect::<Vec<_>>();
        if occupied_cells.is_empty() {
            return None;
        }
        let id = PlacedStructureId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        let structure = PlacedStructure {
            id,
            kind,
            origin,
            rotation,
            state,
            params,
            occupied_cells,
        };
        for cell in structure.occupied_cells() {
            self.cell_index[linear_index(cell.x, cell.y)].push(id);
        }
        self.structures.push(structure);
        self.structures.sort_by_key(|structure| structure.id.0);
        Some(id)
    }

    fn can_place_structure(
        &self,
        kind: StructureKind,
        origin: UVec2,
        rotation: StructureRotation,
        world: &WorldGrid,
    ) -> bool {
        let descriptor = structure_descriptor(kind, rotation);
        let occupied_cells = descriptor.occupied_local_cells();
        if occupied_cells.is_empty() {
            return false;
        }

        for local in &occupied_cells {
            let Some(cell) = offset_world_cell(origin, *local) else {
                return false;
            };
            if !is_editable_cell(cell.x, cell.y) {
                return false;
            }
        }

        if crate::plugins::default_plugin::is_pipe_structure(kind) {
            if is_boundary(origin.x, origin.y) || self.has_pipe_at(origin.x, origin.y) {
                return false;
            }
            if self
                .structures_at(origin.x, origin.y)
                .into_iter()
                .any(|structure| {
                    crate::plugins::default_plugin::is_gas_source_structure(structure.kind)
                        || crate::plugins::default_plugin::is_gas_sink_structure(structure.kind)
                })
            {
                return false;
            }
        } else if crate::plugins::default_plugin::is_vent_structure(kind) {
            if is_boundary(origin.x, origin.y)
                || world.is_solid(origin.x, origin.y)
                || self.has_vent_at(origin.x, origin.y)
            {
                return false;
            }
            if self
                .structures_at(origin.x, origin.y)
                .into_iter()
                .any(|structure| {
                    crate::plugins::default_plugin::is_gas_source_structure(structure.kind)
                        || crate::plugins::default_plugin::is_gas_sink_structure(structure.kind)
                        || crate::plugins::default_plugin::is_gas_pipe_bridge_structure(
                            structure.kind,
                        )
                })
            {
                return false;
            }
        } else if crate::plugins::default_plugin::is_gas_source_structure(kind)
            || crate::plugins::default_plugin::is_gas_sink_structure(kind)
        {
            if is_boundary(origin.x, origin.y) || world.is_solid(origin.x, origin.y) {
                return false;
            }
            if !self.structures_at(origin.x, origin.y).is_empty() {
                return false;
            }
        } else if crate::plugins::default_plugin::is_gas_pipe_bridge_structure(kind) {
            if !matches!(rotation, StructureRotation::Deg0 | StructureRotation::Deg90) {
                return false;
            }
            for local in &occupied_cells {
                let Some(cell) = offset_world_cell(origin, *local) else {
                    return false;
                };
                if is_boundary(cell.x, cell.y) {
                    return false;
                }
                if self.has_vent_at(cell.x, cell.y) {
                    return false;
                }
            }
            let Some(center) = bridge_center_cell(origin, rotation) else {
                return false;
            };
            if self
                .pipe_bearing_structures_at(center.x, center.y)
                .into_iter()
                .any(|structure| {
                    crate::plugins::default_plugin::is_gas_pipe_bridge_structure(structure.kind)
                })
            {
                return false;
            }
        }

        for layer in &descriptor.layers {
            for cell in &layer.cells {
                if cell.collision != LayerCollisionKind::Special {
                    continue;
                }
                let Some(world_cell) = offset_world_cell(origin, cell.local_cell) else {
                    return false;
                };
                if layer_collision_blocked(self, world, layer.kind, world_cell, kind) {
                    return false;
                }
            }
        }

        true
    }

    fn can_place_structure_with_descriptor(
        &self,
        kind: StructureKind,
        origin: UVec2,
        descriptor: &StructureDescriptor,
        world: &WorldGrid,
    ) -> bool {
        let occupied_cells = descriptor.occupied_local_cells();
        if occupied_cells.is_empty() {
            return false;
        }

        for local in &occupied_cells {
            let Some(cell) = offset_world_cell(origin, *local) else {
                return false;
            };
            if !is_editable_cell(cell.x, cell.y) {
                return false;
            }
        }

        for layer in &descriptor.layers {
            for cell in &layer.cells {
                if cell.collision != LayerCollisionKind::Special {
                    continue;
                }
                let Some(world_cell) = offset_world_cell(origin, cell.local_cell) else {
                    return false;
                };
                if layer_collision_blocked(self, world, layer.kind, world_cell, kind) {
                    return false;
                }
            }
        }

        true
    }
}

/// Builds a descriptor for one world cell material.
pub fn cell_material_descriptor(material: CellMaterial) -> StructureDescriptor {
    crate::plugins::default_plugin::cell_layer_descriptor(material)
}

/// Returns the base sprite size for a world material in world cells.
pub fn cell_material_sprite_size_in_cells(
    material: CellMaterial,
    cell_visual_layouts: &CellVisualPlacementConfigMap,
) -> UVec2 {
    let _ = cell_visual_layouts;
    crate::plugins::default_plugin::cell_content_descriptor(material)
        .visual
        .size_in_cells
}

/// Builds a descriptor for one placeable structure kind/rotation pair.
pub fn structure_descriptor(
    kind: StructureKind,
    rotation: StructureRotation,
) -> StructureDescriptor {
    crate::plugins::default_plugin::structure_layer_descriptor(kind, rotation)
}

/// Returns the base sprite size for a structure in world cells.
pub fn structure_sprite_size_in_cells(
    kind: StructureKind,
    rotation: StructureRotation,
    structure_visuals: &StructureVisualConfigMap,
) -> UVec2 {
    let _ = structure_visuals;
    let base_size = crate::plugins::default_plugin::structure_content_descriptor(kind)
        .visual
        .size_in_cells;
    if crate::plugins::default_plugin::is_gas_pipe_bridge_structure(kind) {
        let _ = rotation;
        base_size
    } else {
        rotated_size_in_cells(base_size, rotation)
    }
}

/// Returns the rotated footprint size defined by the visual config of the structure.
pub fn structure_footprint_size_in_cells(
    kind: StructureKind,
    rotation: StructureRotation,
    structure_visuals: &StructureVisualConfigMap,
) -> UVec2 {
    let _ = structure_visuals;
    let base_size = crate::plugins::default_plugin::structure_content_descriptor(kind)
        .visual
        .size_in_cells;
    rotated_size_in_cells(base_size, rotation)
}

fn rotated_size_in_cells(size_in_cells: UVec2, rotation: StructureRotation) -> UVec2 {
    match rotation {
        StructureRotation::Deg0 | StructureRotation::Deg180 => size_in_cells,
        StructureRotation::Deg90 | StructureRotation::Deg270 => {
            UVec2::new(size_in_cells.y, size_in_cells.x)
        }
    }
}

/// Returns the bridge center cell for the given origin/rotation.
pub fn bridge_center_cell(origin: UVec2, rotation: StructureRotation) -> Option<UVec2> {
    match rotation {
        StructureRotation::Deg0 | StructureRotation::Deg180 => {
            offset_world_cell(origin, IVec2::new(1, 0))
        }
        StructureRotation::Deg90 | StructureRotation::Deg270 => {
            offset_world_cell(origin, IVec2::new(0, 1))
        }
    }
}

/// Returns the bridge footprint cells for the given rotation.
pub fn bridge_local_cells(rotation: StructureRotation) -> Vec<IVec2> {
    match rotation {
        StructureRotation::Deg0 | StructureRotation::Deg180 => {
            vec![IVec2::new(0, 0), IVec2::new(1, 0), IVec2::new(2, 0)]
        }
        StructureRotation::Deg90 | StructureRotation::Deg270 => {
            vec![IVec2::new(0, 0), IVec2::new(0, 1), IVec2::new(0, 2)]
        }
    }
}

/// Returns the bridge connection marker cells for the given rotation.
pub fn bridge_connection_local_cells(rotation: StructureRotation) -> Vec<IVec2> {
    match rotation {
        StructureRotation::Deg0 | StructureRotation::Deg180 => {
            vec![IVec2::new(0, 0), IVec2::new(2, 0)]
        }
        StructureRotation::Deg90 | StructureRotation::Deg270 => {
            vec![IVec2::new(0, 0), IVec2::new(0, 2)]
        }
    }
}

fn bridge_packed_state_for_rotation(rotation: StructureRotation) -> PackedState {
    match rotation {
        StructureRotation::Deg0 | StructureRotation::Deg180 => PackedState(0),
        StructureRotation::Deg90 | StructureRotation::Deg270 => PackedState(1),
    }
}

fn layer_collision_blocked(
    structures: &PlacedStructureMap,
    world: &WorldGrid,
    layer_kind: LayerKind,
    cell: UVec2,
    incoming_kind: StructureKind,
) -> bool {
    if layer_kind == APPEARANCE_LAYER && world.is_solid(cell.x, cell.y) {
        let world_descriptor = cell_material_descriptor(
            world
                .solid_material(cell.x, cell.y)
                .expect("solid cell has material"),
        );
        if world_descriptor.layers.iter().any(|layer| {
            layer.kind == layer_kind
                && layer
                    .cells
                    .iter()
                    .any(|spec| spec.collision == LayerCollisionKind::Special)
        }) && !crate::plugins::default_plugin::is_gas_pipe_bridge_structure(incoming_kind)
        {
            return true;
        }
    }

    structures
        .structures_at(cell.x, cell.y)
        .into_iter()
        .any(|structure| structure_collision_on_layer(structure, layer_kind, cell))
}

fn structure_collision_on_layer(
    structure: &PlacedStructure,
    layer_kind: LayerKind,
    world_cell: UVec2,
) -> bool {
    structure
        .descriptor()
        .layers
        .into_iter()
        .filter(|layer| layer.kind == layer_kind)
        .flat_map(|layer| layer.cells)
        .any(|cell| {
            cell.collision == LayerCollisionKind::Special
                && offset_world_cell(structure.origin, cell.local_cell) == Some(world_cell)
        })
}

fn offset_world_cell(origin: UVec2, local: IVec2) -> Option<UVec2> {
    let x = origin.x as i32 + local.x;
    let y = origin.y as i32 + local.y;
    if x < 0 || y < 0 || x >= WORLD_WIDTH as i32 || y >= WORLD_HEIGHT as i32 {
        return None;
    }
    Some(UVec2::new(x as u32, y as u32))
}

fn orthogonal_neighbors(cell: UVec2) -> Vec<UVec2> {
    let mut neighbors = Vec::with_capacity(4);
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

fn orthogonal_neighbors_including_self(cell: UVec2) -> Vec<UVec2> {
    let mut cells = Vec::with_capacity(5);
    cells.push(cell);
    cells.extend(orthogonal_neighbors(cell));
    cells
}

fn kind_sort_key(kind: StructureKind) -> u8 {
    crate::plugins::default_plugin::default_structure_sort_key(kind)
}

fn rotation_sort_key(rotation: StructureRotation) -> u8 {
    match rotation {
        StructureRotation::Deg0 => 0,
        StructureRotation::Deg90 => 1,
        StructureRotation::Deg180 => 2,
        StructureRotation::Deg270 => 3,
    }
}

fn params_sort_key(params: StructureParams) -> (u8, u32, u32) {
    match params {
        StructureParams::None => (0, 0, 0),
        StructureParams::GasSource { gas_index, amount } => (1, gas_index as u32, amount),
        StructureParams::GasSink { amount } => (2, 0, amount),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bridge_center_cell, bridge_connection_local_cells, bridge_local_cells,
        cell_material_descriptor, cell_material_sprite_size_in_cells, structure_descriptor,
        structure_footprint_size_in_cells, structure_sprite_size_in_cells, LayerCollisionKind,
        PlacedStructureMap, StructureRotation, APPEARANCE_LAYER,
    };
    use crate::{
        config::{CellVisualPlacementConfigMap, StructureVisualConfigMap, VisualPlacementConfig},
        world::grid::WorldGrid,
    };

    fn structure_visuals() -> StructureVisualConfigMap {
        StructureVisualConfigMap::from_entries(vec![
            (
                crate::plugins::default_plugin::pipe_structure_kind(),
                VisualPlacementConfig {
                    label: "Pipe".to_string(),
                    draw_priority: 100,
                    size_in_cells: bevy::prelude::UVec2::ONE,
                },
            ),
            (
                crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
                VisualPlacementConfig {
                    label: "Bridge".to_string(),
                    draw_priority: 110,
                    size_in_cells: bevy::prelude::UVec2::new(3, 1),
                },
            ),
            (
                crate::plugins::default_plugin::vent_structure_kind(),
                VisualPlacementConfig {
                    label: "Vent".to_string(),
                    draw_priority: 120,
                    size_in_cells: bevy::prelude::UVec2::ONE,
                },
            ),
            (
                crate::plugins::default_plugin::gas_source_structure_kind(),
                VisualPlacementConfig {
                    label: "Gas Source".to_string(),
                    draw_priority: 130,
                    size_in_cells: bevy::prelude::UVec2::ONE,
                },
            ),
            (
                crate::plugins::default_plugin::gas_sink_structure_kind(),
                VisualPlacementConfig {
                    label: "Gas Sink".to_string(),
                    draw_priority: 130,
                    size_in_cells: bevy::prelude::UVec2::ONE,
                },
            ),
        ])
    }

    fn cell_visual_layouts() -> CellVisualPlacementConfigMap {
        CellVisualPlacementConfigMap::from_entries(vec![
            (
                crate::plugins::default_plugin::boundary_cell_material(),
                VisualPlacementConfig {
                    label: "Boundary".to_string(),
                    draw_priority: 1000,
                    size_in_cells: bevy::prelude::UVec2::ONE,
                },
            ),
            (
                crate::plugins::default_plugin::brick_cell_material(),
                VisualPlacementConfig {
                    label: "Brick".to_string(),
                    draw_priority: 1000,
                    size_in_cells: bevy::prelude::UVec2::ONE,
                },
            ),
            (
                crate::plugins::default_plugin::metal_cell_material(),
                VisualPlacementConfig {
                    label: "Metal".to_string(),
                    draw_priority: 1000,
                    size_in_cells: bevy::prelude::UVec2::ONE,
                },
            ),
        ])
    }

    #[test]
    fn bridge_descriptor_uses_expected_cells_for_both_orientations() {
        assert_eq!(
            bridge_local_cells(StructureRotation::Deg0),
            vec![
                bevy::prelude::IVec2::new(0, 0),
                bevy::prelude::IVec2::new(1, 0),
                bevy::prelude::IVec2::new(2, 0)
            ]
        );
        assert_eq!(
            bridge_local_cells(StructureRotation::Deg90),
            vec![
                bevy::prelude::IVec2::new(0, 0),
                bevy::prelude::IVec2::new(0, 1),
                bevy::prelude::IVec2::new(0, 2)
            ]
        );
        assert_eq!(
            bridge_connection_local_cells(StructureRotation::Deg0),
            vec![
                bevy::prelude::IVec2::new(0, 0),
                bevy::prelude::IVec2::new(2, 0)
            ]
        );
        assert_eq!(
            bridge_connection_local_cells(StructureRotation::Deg90),
            vec![
                bevy::prelude::IVec2::new(0, 0),
                bevy::prelude::IVec2::new(0, 2)
            ]
        );
    }

    #[test]
    fn gas_pipe_connection_markers_exist_only_on_bridge_edges() {
        let descriptor = structure_descriptor(
            crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
            StructureRotation::Deg0,
        );
        let connection_layer = descriptor
            .layers
            .iter()
            .find(|layer| {
                layer.kind == crate::plugins::default_plugin::gas_pipe_connections_layer()
            })
            .expect("bridge has gas connection layer");
        assert_eq!(connection_layer.cells.len(), 2);
        assert!(connection_layer.cells.iter().all(|cell| {
            cell.marker_kind
                == crate::plugins::default_plugin::gas_pipe_connection_bidirectional_marker()
                && cell.collision == LayerCollisionKind::Special
        }));
    }

    #[test]
    fn ordinary_pipe_has_only_appearance_layer() {
        let descriptor = structure_descriptor(
            crate::plugins::default_plugin::pipe_structure_kind(),
            StructureRotation::Deg0,
        );
        assert_eq!(descriptor.layers.len(), 1);
        assert_eq!(descriptor.layers[0].kind, APPEARANCE_LAYER);
    }

    #[test]
    fn structure_descriptors_report_size_in_cells() {
        let structure_visuals = structure_visuals();
        let cell_visual_layouts = cell_visual_layouts();
        assert_eq!(
            structure_descriptor(
                crate::plugins::default_plugin::pipe_structure_kind(),
                StructureRotation::Deg0
            )
            .size_in_cells(),
            bevy::prelude::UVec2::ONE
        );
        assert_eq!(
            structure_descriptor(
                crate::plugins::default_plugin::vent_structure_kind(),
                StructureRotation::Deg0
            )
            .size_in_cells(),
            bevy::prelude::UVec2::ONE
        );
        assert_eq!(
            structure_descriptor(
                crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
                StructureRotation::Deg0
            )
            .size_in_cells(),
            bevy::prelude::UVec2::new(3, 1)
        );
        assert_eq!(
            structure_descriptor(
                crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
                StructureRotation::Deg90
            )
            .size_in_cells(),
            bevy::prelude::UVec2::new(1, 3)
        );
        assert_eq!(
            cell_material_sprite_size_in_cells(
                crate::plugins::default_plugin::brick_cell_material(),
                &cell_visual_layouts
            ),
            bevy::prelude::UVec2::ONE
        );
        assert_eq!(
            structure_footprint_size_in_cells(
                crate::plugins::default_plugin::pipe_structure_kind(),
                StructureRotation::Deg0,
                &structure_visuals
            ),
            bevy::prelude::UVec2::ONE
        );
        assert_eq!(
            structure_footprint_size_in_cells(
                crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
                StructureRotation::Deg0,
                &structure_visuals
            ),
            bevy::prelude::UVec2::new(3, 1)
        );
        assert_eq!(
            structure_footprint_size_in_cells(
                crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
                StructureRotation::Deg90,
                &structure_visuals
            ),
            bevy::prelude::UVec2::new(1, 3)
        );
        assert_eq!(
            structure_sprite_size_in_cells(
                crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
                StructureRotation::Deg90,
                &structure_visuals
            ),
            bevy::prelude::UVec2::new(3, 1)
        );
    }

    #[test]
    fn walls_use_special_appearance_cell() {
        let descriptor =
            cell_material_descriptor(crate::plugins::default_plugin::brick_cell_material());
        assert_eq!(descriptor.layers.len(), 1);
        assert_eq!(descriptor.layers[0].kind, APPEARANCE_LAYER);
        assert_eq!(descriptor.layers[0].cells.len(), 1);
        assert_eq!(
            descriptor.layers[0].cells[0].collision,
            LayerCollisionKind::Special
        );
    }

    #[test]
    fn bridge_can_pass_through_wall_but_vent_cannot_be_placed_on_it() {
        let mut world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(world.set_solid_with_material(
            20,
            20,
            crate::plugins::default_plugin::brick_cell_material()
        ));
        assert!(structures
            .place_bridge(
                bevy::prelude::UVec2::new(19, 20),
                StructureRotation::Deg0,
                &world
            )
            .is_some());
        assert!(!structures.place_vent(19, 20, &world));
    }

    #[test]
    fn placing_bridge_keeps_single_structure_entry_without_plain_pipe() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let origin = bevy::prelude::UVec2::new(30, 30);
        let bridge_id = structures
            .place_bridge(origin, StructureRotation::Deg0, &world)
            .expect("bridge placement");
        let center = bridge_center_cell(origin, StructureRotation::Deg0).expect("bridge center");

        assert_eq!(structures.iter().count(), 1);
        let bridge = structures.structure(bridge_id).expect("placed bridge");
        assert_eq!(
            bridge.kind,
            crate::plugins::default_plugin::gas_pipe_bridge_structure_kind()
        );
        assert!(!structures.has_pipe_at(center.x, center.y));
    }

    #[test]
    fn bridge_clears_as_whole_structure_from_any_occupied_cell() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let bridge = structures
            .place_bridge(
                bevy::prelude::UVec2::new(40, 40),
                StructureRotation::Deg0,
                &world,
            )
            .expect("bridge placement");
        assert_eq!(structures.clear_cell(41, 40), vec![bridge]);
        assert!(structures.structures_at(40, 40).is_empty());
        assert!(structures.structures_at(41, 40).is_empty());
        assert!(structures.structures_at(42, 40).is_empty());
    }

    #[test]
    fn removing_bridge_does_not_clear_pipe_cut_on_underlying_pipes() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let left = bevy::prelude::UVec2::new(60, 60);
        let right = bevy::prelude::UVec2::new(61, 60);

        assert!(structures.place_pipe(left.x, left.y, &world));
        assert!(structures.place_pipe(right.x, right.y, &world));
        assert!(structures.add_pipe_cut(left, right));
        let left_before = structures
            .structures_at(left.x, left.y)
            .into_iter()
            .find(|structure| crate::plugins::default_plugin::is_pipe_structure(structure.kind))
            .expect("left pipe exists")
            .state
            .value();

        let bridge = structures
            .place_bridge(left, StructureRotation::Deg0, &world)
            .expect("bridge placement");
        assert!(structures.remove_structure(bridge));

        assert!(
            structures.is_pipe_cut(left, right),
            "bridge removal must not delete existing scissors cuts for plain pipes"
        );
        let left_after = structures
            .structures_at(left.x, left.y)
            .into_iter()
            .find(|structure| crate::plugins::default_plugin::is_pipe_structure(structure.kind))
            .expect("left pipe exists after bridge removal")
            .state
            .value();
        assert_eq!(
            left_after, left_before,
            "underlying pipe state must remain stable after bridge removal"
        );
    }

    #[test]
    fn pipe_stroke_only_connects_consecutive_cells_on_the_drawn_line() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();

        assert!(structures.apply_pipe_path(
            &[
                bevy::prelude::UVec2::new(20, 20),
                bevy::prelude::UVec2::new(21, 20),
                bevy::prelude::UVec2::new(22, 20),
            ],
            &world,
        ));
        assert!(structures.apply_pipe_path(
            &[
                bevy::prelude::UVec2::new(24, 20),
                bevy::prelude::UVec2::new(25, 20),
                bevy::prelude::UVec2::new(26, 20),
            ],
            &world,
        ));
        assert!(structures.apply_pipe_path(
            &[
                bevy::prelude::UVec2::new(23, 18),
                bevy::prelude::UVec2::new(23, 19),
                bevy::prelude::UVec2::new(23, 20),
                bevy::prelude::UVec2::new(23, 21),
                bevy::prelude::UVec2::new(23, 22),
            ],
            &world,
        ));

        assert!(structures.has_pipe_at(23, 20));
        assert!(structures.is_pipe_cut(
            bevy::prelude::UVec2::new(22, 20),
            bevy::prelude::UVec2::new(23, 20),
        ));
        assert!(structures.is_pipe_cut(
            bevy::prelude::UVec2::new(23, 20),
            bevy::prelude::UVec2::new(24, 20),
        ));
        assert!(!structures.is_pipe_cut(
            bevy::prelude::UVec2::new(23, 19),
            bevy::prelude::UVec2::new(23, 20),
        ));
        assert!(!structures.is_pipe_cut(
            bevy::prelude::UVec2::new(23, 20),
            bevy::prelude::UVec2::new(23, 21),
        ));
    }

    #[test]
    fn pipe_stroke_can_restore_connection_when_redrawn_along_existing_pair() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();

        assert!(structures.place_pipe(30, 30, &world));
        assert!(structures.place_pipe(31, 30, &world));
        assert!(structures.add_pipe_cut(
            bevy::prelude::UVec2::new(30, 30),
            bevy::prelude::UVec2::new(31, 30),
        ));

        assert!(structures.apply_pipe_path(
            &[
                bevy::prelude::UVec2::new(30, 30),
                bevy::prelude::UVec2::new(31, 30),
            ],
            &world,
        ));
        assert!(!structures.is_pipe_cut(
            bevy::prelude::UVec2::new(30, 30),
            bevy::prelude::UVec2::new(31, 30),
        ));
    }

    #[test]
    fn pipe_stroke_keeps_preexisting_cross_connection_intact() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();

        assert!(structures.apply_pipe_path(
            &[
                bevy::prelude::UVec2::new(40, 18),
                bevy::prelude::UVec2::new(40, 19),
                bevy::prelude::UVec2::new(40, 20),
                bevy::prelude::UVec2::new(40, 21),
                bevy::prelude::UVec2::new(40, 22),
            ],
            &world,
        ));
        assert!(structures.apply_pipe_path(
            &[
                bevy::prelude::UVec2::new(39, 20),
                bevy::prelude::UVec2::new(40, 20),
                bevy::prelude::UVec2::new(41, 20),
            ],
            &world,
        ));

        assert!(!structures.is_pipe_cut(
            bevy::prelude::UVec2::new(40, 19),
            bevy::prelude::UVec2::new(40, 20),
        ));
        assert!(!structures.is_pipe_cut(
            bevy::prelude::UVec2::new(40, 20),
            bevy::prelude::UVec2::new(40, 21),
        ));
        assert!(!structures.is_pipe_cut(
            bevy::prelude::UVec2::new(39, 20),
            bevy::prelude::UVec2::new(40, 20),
        ));
        assert!(!structures.is_pipe_cut(
            bevy::prelude::UVec2::new(40, 20),
            bevy::prelude::UVec2::new(41, 20),
        ));
    }

    #[test]
    fn updating_source_with_same_params_is_a_no_op() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let id = structures
            .place_gas_source(50, 50, 1, 120, &world)
            .expect("source placement");

        assert!(!structures.update_gas_source(id, 1, 120));
        assert!(structures.update_gas_source(id, 2, 120));
    }

    #[test]
    fn updating_sink_with_same_params_is_a_no_op() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let id = structures
            .place_gas_sink(52, 52, 80, &world)
            .expect("sink placement");

        assert!(!structures.update_gas_sink(id, 80));
        assert!(structures.update_gas_sink(id, 81));
    }
}
