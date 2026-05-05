use bevy::prelude::*;

use crate::world::grid::{
    is_boundary, is_editable_cell, linear_index, CellKind, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GasStructureCell {
    Source { gas_index: usize, amount: u32 },
    Sink { amount: u32 },
}

#[derive(Clone, Debug)]
/// Stores `GasStructureSnapshot` state.
pub struct GasStructureSnapshot {
    pub kinds: Vec<u8>,
    pub gas_indices: Vec<u32>,
    pub amounts: Vec<u32>,
}

#[derive(Resource, Clone)]
/// Stores `GasStructureGrid` state.
pub struct GasStructureGrid {
    cells: Vec<Option<GasStructureCell>>,
}

impl Default for GasStructureGrid {
    fn default() -> Self {
        let cells = vec![None; (WORLD_WIDTH * WORLD_HEIGHT) as usize];
        Self { cells }
    }
}

impl GasStructureGrid {
    /// Runs `cell` logic.
    pub fn cell(&self, x: u32, y: u32) -> Option<GasStructureCell> {
        self.cells[linear_index(x, y)]
    }

    /// Runs `can_place_at` logic.
    pub fn can_place_at(&self, x: u32, y: u32, world: &WorldGrid) -> bool {
        if !is_editable_cell(x, y) || is_boundary(x, y) {
            return false;
        }
        if self.cell(x, y).is_some() {
            return false;
        }
        matches!(world.cell(x, y), CellKind::Empty)
    }

    /// Runs `blocks_solid_placement` logic.
    pub fn blocks_solid_placement(&self, x: u32, y: u32) -> bool {
        self.cell(x, y).is_some()
    }

    /// Runs `set_source` logic.
    pub fn set_source(
        &mut self,
        x: u32,
        y: u32,
        gas_index: usize,
        amount: u32,
        world: &WorldGrid,
    ) -> bool {
        if amount == 0 || !self.can_place_at(x, y, world) {
            return false;
        }
        let idx = linear_index(x, y);
        self.cells[idx] = Some(GasStructureCell::Source { gas_index, amount });
        true
    }

    /// Runs `set_sink` logic.
    pub fn set_sink(&mut self, x: u32, y: u32, amount: u32, world: &WorldGrid) -> bool {
        if amount == 0 || !self.can_place_at(x, y, world) {
            return false;
        }
        let idx = linear_index(x, y);
        self.cells[idx] = Some(GasStructureCell::Sink { amount });
        true
    }

    /// Runs `clear` logic.
    pub fn clear(&mut self, x: u32, y: u32) -> bool {
        let idx = linear_index(x, y);
        if self.cells[idx].is_none() {
            return false;
        }
        self.cells[idx] = None;
        true
    }

    /// Runs `update_source` logic.
    pub fn update_source(
        &mut self,
        x: u32,
        y: u32,
        gas_index: usize,
        amount: u32,
        world: &WorldGrid,
    ) -> bool {
        if amount == 0 || is_boundary(x, y) || world.is_solid(x, y) {
            return false;
        }
        let idx = linear_index(x, y);
        match self.cells[idx] {
            Some(GasStructureCell::Source { .. }) => {
                self.cells[idx] = Some(GasStructureCell::Source { gas_index, amount });
                true
            }
            _ => false,
        }
    }

    /// Runs `update_sink` logic.
    pub fn update_sink(&mut self, x: u32, y: u32, amount: u32, world: &WorldGrid) -> bool {
        if amount == 0 || is_boundary(x, y) || world.is_solid(x, y) {
            return false;
        }
        let idx = linear_index(x, y);
        match self.cells[idx] {
            Some(GasStructureCell::Sink { .. }) => {
                self.cells[idx] = Some(GasStructureCell::Sink { amount });
                true
            }
            _ => false,
        }
    }

    /// Runs `iter_cells` logic.
    pub fn iter_cells(&self) -> impl Iterator<Item = (u32, u32, GasStructureCell)> + '_ {
        self.cells.iter().enumerate().filter_map(|(idx, value)| {
            value.map(|cell| {
                let x = (idx as u32) % WORLD_WIDTH;
                let y = (idx as u32) / WORLD_WIDTH;
                (x, y, cell)
            })
        })
    }

    /// Runs `snapshot_state` logic.
    pub fn snapshot_state(&self) -> GasStructureSnapshot {
        let mut kinds = Vec::with_capacity(self.cells.len());
        let mut gas_indices = Vec::with_capacity(self.cells.len());
        let mut amounts = Vec::with_capacity(self.cells.len());

        for cell in &self.cells {
            match cell {
                None => {
                    kinds.push(0);
                    gas_indices.push(0);
                    amounts.push(0);
                }
                Some(GasStructureCell::Source { gas_index, amount }) => {
                    kinds.push(1);
                    gas_indices.push((*gas_index).min(u32::MAX as usize) as u32);
                    amounts.push(*amount);
                }
                Some(GasStructureCell::Sink { amount }) => {
                    kinds.push(2);
                    gas_indices.push(0);
                    amounts.push(*amount);
                }
            }
        }

        GasStructureSnapshot {
            kinds,
            gas_indices,
            amounts,
        }
    }

    /// Runs `restore_state` logic.
    pub fn restore_state(
        &mut self,
        snapshot: &GasStructureSnapshot,
        world: &WorldGrid,
    ) -> Result<(), String> {
        let expected = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        if snapshot.kinds.len() != expected
            || snapshot.gas_indices.len() != expected
            || snapshot.amounts.len() != expected
        {
            return Err(format!(
                "GasStructure snapshot length mismatch: kinds={}, gas_indices={}, amounts={}, expected={}",
                snapshot.kinds.len(),
                snapshot.gas_indices.len(),
                snapshot.amounts.len(),
                expected
            ));
        }

        let mut next = vec![None; expected];
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let idx = linear_index(x, y);
                let kind = snapshot.kinds[idx];
                let gas_index = snapshot.gas_indices[idx] as usize;
                let amount = snapshot.amounts[idx];
                if kind != 0 {
                    if is_boundary(x, y) || world.is_solid(x, y) {
                        return Err(format!(
                            "GasStructure snapshot has structure in non-empty/non-editable cell ({}, {})",
                            x, y
                        ));
                    }
                    if amount == 0 {
                        return Err(format!(
                            "GasStructure snapshot contains zero amount at ({}, {})",
                            x, y
                        ));
                    }
                }
                next[idx] = match kind {
                    0 => None,
                    1 => Some(GasStructureCell::Source { gas_index, amount }),
                    2 => Some(GasStructureCell::Sink { amount }),
                    _ => {
                        return Err(format!(
                            "GasStructure snapshot has unknown kind {} at ({}, {})",
                            kind, x, y
                        ));
                    }
                };
            }
        }
        self.cells = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{GasStructureCell, GasStructureGrid};
    use crate::world::grid::{CellMaterial, WorldGrid};

    #[test]
    fn placement_requires_empty_non_boundary_cell() {
        let mut grid = GasStructureGrid::default();
        let mut world = WorldGrid::default();
        assert!(grid.set_source(10, 10, 0, 7, &world));
        assert!(
            !grid.set_sink(10, 10, 5, &world),
            "must not replace existing structure"
        );

        assert!(world.set_solid_with_material(11, 11, CellMaterial::Brick));
        assert!(
            !grid.set_sink(11, 11, 5, &world),
            "must not place structure into solid cell"
        );
        assert!(
            !grid.set_sink(0, 0, 5, &world),
            "must not place structure into boundary cell"
        );
    }

    #[test]
    fn snapshot_roundtrip() {
        let mut grid = GasStructureGrid::default();
        let world = WorldGrid::default();
        assert!(grid.set_source(20, 21, 2, 99, &world));
        assert!(grid.set_sink(22, 21, 50, &world));
        let snapshot = grid.snapshot_state();

        let mut restored = GasStructureGrid::default();
        restored
            .restore_state(&snapshot, &world)
            .expect("valid snapshot must restore");

        assert_eq!(
            restored.cell(20, 21),
            Some(GasStructureCell::Source {
                gas_index: 2,
                amount: 99
            })
        );
        assert_eq!(
            restored.cell(22, 21),
            Some(GasStructureCell::Sink { amount: 50 })
        );
    }
}
