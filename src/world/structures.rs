use std::collections::HashSet;

use bevy::prelude::*;

use crate::world::grid::{
    is_boundary, is_editable_cell, linear_index, CellMaterial, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH,
};

/// Identifies the visual/runtime layer used by a structure or world cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayerKind {
    Appearance,
    GasPipeConnections,
}

/// Identifies the marker rendered inside a layer cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerMarkerKind {
    Solid(CellMaterial),
    Pipe,
    Vent,
    GasSource,
    GasSink,
    GasPipeBridge,
    GasPipeConnectionBidirectional,
}

/// Defines whether a layer cell participates in collision checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerCollisionKind {
    RenderOnly,
    Special,
}

/// Describes one occupied local cell inside a structure layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerCellSpec {
    pub local_cell: IVec2,
    pub marker_kind: LayerMarkerKind,
    pub collision: LayerCollisionKind,
}

/// Describes all cells that belong to one structure layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureLayer {
    pub kind: LayerKind,
    pub cells: Vec<LayerCellSpec>,
}

/// Stores static display/runtime information for a structure or world material.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StructureKind {
    Pipe,
    Vent,
    GasSource,
    GasSink,
    GasPipeBridge,
}

/// Stores a canonical rotation for a placed structure.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureParams {
    None,
    GasSource { gas_index: usize, amount: u32 },
    GasSink { amount: u32 },
}

/// Stores the stable id of one placed structure instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlacedStructureId(pub u32);

/// Stores one placed structure instance in the world.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedStructure {
    pub id: PlacedStructureId,
    pub kind: StructureKind,
    pub origin: UVec2,
    pub rotation: StructureRotation,
    pub params: StructureParams,
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
        self.descriptor()
            .occupied_local_cells()
            .into_iter()
            .filter_map(|local| offset_world_cell(self.origin, local))
            .collect()
    }
}

/// Stores one saved structure entry for schema v4 serialization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedStructureSnapshotEntry {
    pub kind: StructureKind,
    pub origin: UVec2,
    pub rotation: StructureRotation,
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
            matches!(
                structure.kind,
                StructureKind::GasSource | StructureKind::GasSink
            )
        })
    }

    /// Returns true when a plain pipe occupies the cell.
    pub fn has_pipe_at(&self, x: u32, y: u32) -> bool {
        self.structures_at(x, y)
            .into_iter()
            .any(|structure| structure.kind == StructureKind::Pipe)
    }

    /// Returns true when a vent occupies the cell.
    pub fn has_vent_at(&self, x: u32, y: u32) -> bool {
        self.structures_at(x, y)
            .into_iter()
            .any(|structure| structure.kind == StructureKind::Vent)
    }

    /// Returns true when any bridge occupies the cell.
    pub fn has_bridge_at(&self, x: u32, y: u32) -> bool {
        self.structures_at(x, y)
            .into_iter()
            .any(|structure| structure.kind == StructureKind::GasPipeBridge)
    }

    /// Returns true when any structure in the cell blocks solid placement.
    pub fn blocks_solid_placement(&self, x: u32, y: u32) -> bool {
        self.structures_at(x, y).into_iter().any(|structure| {
            matches!(
                structure.kind,
                StructureKind::Vent | StructureKind::GasSource | StructureKind::GasSink
            )
        })
    }

    /// Places a plain pipe if the cell is valid and not already occupied by one.
    pub fn place_pipe(&mut self, x: u32, y: u32, world: &WorldGrid) -> bool {
        self.place_structure(
            StructureKind::Pipe,
            UVec2::new(x, y),
            StructureRotation::Deg0,
            StructureParams::None,
            world,
        )
        .is_some()
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
            changed |= self.remove_pipe_cut(pair[0], pair[1]);
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
                changed |= self.add_pipe_cut(cell, neighbor);
            }
        }

        changed
    }

    /// Places a vent if the cell is valid and not already occupied by one.
    pub fn place_vent(&mut self, x: u32, y: u32, world: &WorldGrid) -> bool {
        self.place_structure(
            StructureKind::Vent,
            UVec2::new(x, y),
            StructureRotation::Deg0,
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
            StructureKind::GasSource,
            UVec2::new(x, y),
            StructureRotation::Deg0,
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
            StructureKind::GasSink,
            UVec2::new(x, y),
            StructureRotation::Deg0,
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
            StructureKind::GasPipeBridge,
            origin,
            rotation,
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
        let Some(structure) = self.structures.iter_mut().find(|structure| structure.id == id) else {
            return false;
        };
        if structure.kind != StructureKind::GasSource {
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
        let Some(structure) = self.structures.iter_mut().find(|structure| structure.id == id) else {
            return false;
        };
        if structure.kind != StructureKind::GasSink {
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
        let Some(index) = self.structures.iter().position(|structure| structure.id == id) else {
            return false;
        };
        let removed = self.structures.remove(index);
        for cell in removed.occupied_cells() {
            let ids = &mut self.cell_index[linear_index(cell.x, cell.y)];
            ids.retain(|candidate| *candidate != id);
            self.pipe_cuts.retain(|cut| !cut.touches(cell));
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
        self.pipe_cuts.insert(cut)
    }

    /// Removes a previously recorded scissors cut between two adjacent cells.
    pub fn remove_pipe_cut(&mut self, a: UVec2, b: UVec2) -> bool {
        let Some(cut) = PipeCut::new(a, b) else {
            return false;
        };
        self.pipe_cuts.remove(&cut)
    }

    /// Returns true when a scissors cut blocks the cell adjacency.
    pub fn is_pipe_cut(&self, a: UVec2, b: UVec2) -> bool {
        PipeCut::new(a, b)
            .map(|cut| self.pipe_cuts.contains(&cut))
            .unwrap_or(false)
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
                params: structure.params,
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| {
            (
                entry.origin.y,
                entry.origin.x,
                kind_sort_key(entry.kind),
                rotation_sort_key(entry.rotation),
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
            next.place_structure(entry.kind, entry.origin, entry.rotation, entry.params, world)
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
            .filter(|structure| match structure.kind {
                StructureKind::Pipe => structure.origin == UVec2::new(x, y),
                StructureKind::GasPipeBridge => bridge_center_cell(structure.origin, structure.rotation)
                    == Some(UVec2::new(x, y)),
                _ => false,
            })
            .collect()
    }

    /// Returns bridges whose connection markers occupy the cell.
    pub fn bridge_connection_structures_at(&self, x: u32, y: u32) -> Vec<&PlacedStructure> {
        self.structures_at(x, y)
            .into_iter()
            .filter(|structure| {
                structure.kind == StructureKind::GasPipeBridge
                    && structure
                        .descriptor()
                        .layers
                        .iter()
                        .find(|layer| layer.kind == LayerKind::GasPipeConnections)
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

    fn place_structure(
        &mut self,
        kind: StructureKind,
        origin: UVec2,
        rotation: StructureRotation,
        params: StructureParams,
        world: &WorldGrid,
    ) -> Option<PlacedStructureId> {
        if !self.can_place_structure(kind, origin, rotation, world) {
            return None;
        }
        let id = PlacedStructureId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        let structure = PlacedStructure {
            id,
            kind,
            origin,
            rotation,
            params,
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

        match kind {
            StructureKind::Pipe => {
                if is_boundary(origin.x, origin.y) || self.has_pipe_at(origin.x, origin.y) {
                    return false;
                }
                if self
                    .structures_at(origin.x, origin.y)
                    .into_iter()
                    .any(|structure| matches!(structure.kind, StructureKind::GasSource | StructureKind::GasSink))
                {
                    return false;
                }
            }
            StructureKind::Vent => {
                if is_boundary(origin.x, origin.y)
                    || world.is_solid(origin.x, origin.y)
                    || self.has_vent_at(origin.x, origin.y)
                {
                    return false;
                }
                if self.structures_at(origin.x, origin.y).into_iter().any(|structure| {
                    matches!(
                        structure.kind,
                        StructureKind::GasSource | StructureKind::GasSink | StructureKind::GasPipeBridge
                    )
                }) {
                    return false;
                }
            }
            StructureKind::GasSource | StructureKind::GasSink => {
                if is_boundary(origin.x, origin.y) || world.is_solid(origin.x, origin.y) {
                    return false;
                }
                if !self.structures_at(origin.x, origin.y).is_empty() {
                    return false;
                }
            }
            StructureKind::GasPipeBridge => {
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
                    .any(|structure| structure.kind == StructureKind::GasPipeBridge)
                {
                    return false;
                }
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
    StructureDescriptor {
        layers: vec![StructureLayer {
            kind: LayerKind::Appearance,
            cells: vec![LayerCellSpec {
                local_cell: IVec2::ZERO,
                marker_kind: LayerMarkerKind::Solid(material),
                collision: LayerCollisionKind::Special,
            }],
        }],
    }
}

/// Returns the base sprite size for a world material in world cells.
pub fn cell_material_sprite_size_in_cells(_material: CellMaterial) -> UVec2 {
    UVec2::ONE
}

/// Builds a descriptor for one placeable structure kind/rotation pair.
pub fn structure_descriptor(kind: StructureKind, rotation: StructureRotation) -> StructureDescriptor {
    match kind {
        StructureKind::Pipe => StructureDescriptor {
            layers: vec![StructureLayer {
                kind: LayerKind::Appearance,
                cells: vec![LayerCellSpec {
                    local_cell: IVec2::ZERO,
                    marker_kind: LayerMarkerKind::Pipe,
                    collision: LayerCollisionKind::RenderOnly,
                }],
            }],
        },
        StructureKind::Vent => StructureDescriptor {
            layers: vec![
                StructureLayer {
                    kind: LayerKind::Appearance,
                    cells: vec![LayerCellSpec {
                        local_cell: IVec2::ZERO,
                        marker_kind: LayerMarkerKind::Vent,
                        collision: LayerCollisionKind::Special,
                    }],
                },
                StructureLayer {
                    kind: LayerKind::GasPipeConnections,
                    cells: vec![LayerCellSpec {
                        local_cell: IVec2::ZERO,
                        marker_kind: LayerMarkerKind::GasPipeConnectionBidirectional,
                        collision: LayerCollisionKind::Special,
                    }],
                },
            ],
        },
        StructureKind::GasSource => StructureDescriptor {
            layers: vec![StructureLayer {
                kind: LayerKind::Appearance,
                cells: vec![LayerCellSpec {
                    local_cell: IVec2::ZERO,
                    marker_kind: LayerMarkerKind::GasSource,
                    collision: LayerCollisionKind::Special,
                }],
            }],
        },
        StructureKind::GasSink => StructureDescriptor {
            layers: vec![StructureLayer {
                kind: LayerKind::Appearance,
                cells: vec![LayerCellSpec {
                    local_cell: IVec2::ZERO,
                    marker_kind: LayerMarkerKind::GasSink,
                    collision: LayerCollisionKind::Special,
                }],
            }],
        },
        StructureKind::GasPipeBridge => {
            let bridge_cells = bridge_local_cells(rotation);
            let connection_cells = bridge_connection_local_cells(rotation);
            StructureDescriptor {
                layers: vec![
                    StructureLayer {
                        kind: LayerKind::Appearance,
                        cells: bridge_cells
                            .into_iter()
                            .map(|local_cell| LayerCellSpec {
                                local_cell,
                                marker_kind: LayerMarkerKind::GasPipeBridge,
                                collision: LayerCollisionKind::RenderOnly,
                            })
                            .collect(),
                    },
                    StructureLayer {
                        kind: LayerKind::GasPipeConnections,
                        cells: connection_cells
                            .into_iter()
                            .map(|local_cell| LayerCellSpec {
                                local_cell,
                                marker_kind: LayerMarkerKind::GasPipeConnectionBidirectional,
                                collision: LayerCollisionKind::Special,
                            })
                            .collect(),
                    },
                ],
            }
        }
    }
}

/// Returns the base sprite size for a structure in world cells.
pub fn structure_sprite_size_in_cells(
    kind: StructureKind,
    rotation: StructureRotation,
) -> UVec2 {
    match kind {
        StructureKind::GasPipeBridge => {
            let _ = rotation;
            UVec2::new(3, 1)
        }
        _ => structure_descriptor(kind, rotation).size_in_cells(),
    }
}

/// Returns the bridge center cell for the given origin/rotation.
pub fn bridge_center_cell(origin: UVec2, rotation: StructureRotation) -> Option<UVec2> {
    match rotation {
        StructureRotation::Deg0 | StructureRotation::Deg180 => offset_world_cell(origin, IVec2::new(1, 0)),
        StructureRotation::Deg90 | StructureRotation::Deg270 => offset_world_cell(origin, IVec2::new(0, 1)),
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

fn layer_collision_blocked(
    structures: &PlacedStructureMap,
    world: &WorldGrid,
    layer_kind: LayerKind,
    cell: UVec2,
    incoming_kind: StructureKind,
) -> bool {
    if layer_kind == LayerKind::Appearance && world.is_solid(cell.x, cell.y) {
        let world_descriptor = cell_material_descriptor(
            world.solid_material(cell.x, cell.y)
                .expect("solid cell has material"),
        );
        if world_descriptor.layers.iter().any(|layer| {
            layer.kind == layer_kind
                && layer
                    .cells
                    .iter()
                    .any(|spec| spec.collision == LayerCollisionKind::Special)
        }) && incoming_kind != StructureKind::GasPipeBridge
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

fn kind_sort_key(kind: StructureKind) -> u8 {
    match kind {
        StructureKind::Pipe => 0,
        StructureKind::Vent => 1,
        StructureKind::GasSource => 2,
        StructureKind::GasSink => 3,
        StructureKind::GasPipeBridge => 4,
    }
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
        bridge_connection_local_cells, bridge_local_cells, cell_material_descriptor,
        cell_material_sprite_size_in_cells, structure_descriptor, structure_sprite_size_in_cells,
        LayerCollisionKind, LayerKind, LayerMarkerKind, PlacedStructureMap, StructureKind,
        StructureRotation,
    };
    use crate::world::grid::{CellMaterial, WorldGrid};

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
            vec![bevy::prelude::IVec2::new(0, 0), bevy::prelude::IVec2::new(2, 0)]
        );
        assert_eq!(
            bridge_connection_local_cells(StructureRotation::Deg90),
            vec![bevy::prelude::IVec2::new(0, 0), bevy::prelude::IVec2::new(0, 2)]
        );
    }

    #[test]
    fn gas_pipe_connection_markers_exist_only_on_bridge_edges() {
        let descriptor = structure_descriptor(StructureKind::GasPipeBridge, StructureRotation::Deg0);
        let connection_layer = descriptor
            .layers
            .iter()
            .find(|layer| layer.kind == LayerKind::GasPipeConnections)
            .expect("bridge has gas connection layer");
        assert_eq!(connection_layer.cells.len(), 2);
        assert!(connection_layer.cells.iter().all(|cell| {
            cell.marker_kind == LayerMarkerKind::GasPipeConnectionBidirectional
                && cell.collision == LayerCollisionKind::Special
        }));
    }

    #[test]
    fn ordinary_pipe_has_only_appearance_layer() {
        let descriptor = structure_descriptor(StructureKind::Pipe, StructureRotation::Deg0);
        assert_eq!(descriptor.layers.len(), 1);
        assert_eq!(descriptor.layers[0].kind, LayerKind::Appearance);
    }

    #[test]
    fn structure_descriptors_report_size_in_cells() {
        assert_eq!(
            structure_descriptor(StructureKind::Pipe, StructureRotation::Deg0).size_in_cells(),
            bevy::prelude::UVec2::ONE
        );
        assert_eq!(
            structure_descriptor(StructureKind::Vent, StructureRotation::Deg0).size_in_cells(),
            bevy::prelude::UVec2::ONE
        );
        assert_eq!(
            structure_descriptor(StructureKind::GasPipeBridge, StructureRotation::Deg0)
                .size_in_cells(),
            bevy::prelude::UVec2::new(3, 1)
        );
        assert_eq!(
            structure_descriptor(StructureKind::GasPipeBridge, StructureRotation::Deg90)
                .size_in_cells(),
            bevy::prelude::UVec2::new(1, 3)
        );
        assert_eq!(cell_material_sprite_size_in_cells(CellMaterial::Brick), bevy::prelude::UVec2::ONE);
        assert_eq!(
            structure_sprite_size_in_cells(StructureKind::Pipe, StructureRotation::Deg0),
            bevy::prelude::UVec2::ONE
        );
        assert_eq!(
            structure_sprite_size_in_cells(StructureKind::GasPipeBridge, StructureRotation::Deg0),
            bevy::prelude::UVec2::new(3, 1)
        );
        assert_eq!(
            structure_sprite_size_in_cells(StructureKind::GasPipeBridge, StructureRotation::Deg90),
            bevy::prelude::UVec2::new(3, 1)
        );
    }

    #[test]
    fn walls_use_special_appearance_cell() {
        let descriptor = cell_material_descriptor(CellMaterial::Brick);
        assert_eq!(descriptor.layers.len(), 1);
        assert_eq!(descriptor.layers[0].kind, LayerKind::Appearance);
        assert_eq!(descriptor.layers[0].cells.len(), 1);
        assert_eq!(descriptor.layers[0].cells[0].collision, LayerCollisionKind::Special);
    }

    #[test]
    fn bridge_can_pass_through_wall_but_vent_cannot_be_placed_on_it() {
        let mut world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(world.set_solid_with_material(20, 20, CellMaterial::Brick));
        assert!(structures.place_bridge(
            bevy::prelude::UVec2::new(19, 20),
            StructureRotation::Deg0,
            &world
        ).is_some());
        assert!(!structures.place_vent(19, 20, &world));
    }

    #[test]
    fn bridge_clears_as_whole_structure_from_any_occupied_cell() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let bridge = structures
            .place_bridge(bevy::prelude::UVec2::new(40, 40), StructureRotation::Deg0, &world)
            .expect("bridge placement");
        assert_eq!(structures.clear_cell(41, 40), vec![bridge]);
        assert!(structures.structures_at(40, 40).is_empty());
        assert!(structures.structures_at(41, 40).is_empty());
        assert!(structures.structures_at(42, 40).is_empty());
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
