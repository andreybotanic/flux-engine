use bevy::prelude::*;

use crate::world::{
    gas_structures::GasStructureGrid,
    grid::{is_boundary, is_editable_cell, linear_index, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
};

const PIPE_CONNECTION_MASK: u8 = 0b0000_1111;
const PIPE_PRESENT_BIT: u8 = 0b0001_0000;
const PIPE_VENT_BIT: u8 = 0b0010_0000;

const DIR_UP: u8 = 0b0001;
const DIR_RIGHT: u8 = 0b0010;
const DIR_DOWN: u8 = 0b0100;
const DIR_LEFT: u8 = 0b1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
/// Stores `PipeCell` state.
pub struct PipeCell {
    pub has_pipe: bool,
    pub connections: u8,
    pub has_vent: bool,
}

impl PipeCell {
    fn encoded(self) -> u8 {
        (self.connections & PIPE_CONNECTION_MASK)
            | if self.has_pipe { PIPE_PRESENT_BIT } else { 0 }
            | if self.has_vent { PIPE_VENT_BIT } else { 0 }
    }

    fn from_encoded(encoded: u8) -> Self {
        Self {
            has_pipe: (encoded & PIPE_PRESENT_BIT) != 0,
            connections: encoded & PIPE_CONNECTION_MASK,
            has_vent: (encoded & PIPE_VENT_BIT) != 0,
        }
    }
}

#[derive(Clone, Debug)]
/// Stores `PipeLayoutSnapshot` state.
pub struct PipeLayoutSnapshot {
    pub cells: Vec<u8>,
}

#[derive(Resource, Clone)]
/// Stores `PipeGrid` state.
pub struct PipeGrid {
    cells: Vec<PipeCell>,
}

impl Default for PipeGrid {
    fn default() -> Self {
        Self {
            cells: vec![PipeCell::default(); (WORLD_WIDTH * WORLD_HEIGHT) as usize],
        }
    }
}

impl PipeGrid {
    /// Runs `cell` logic.
    pub fn cell(&self, x: u32, y: u32) -> PipeCell {
        self.cells[linear_index(x, y)]
    }

    /// Runs `has_pipe` logic.
    pub fn has_pipe(&self, x: u32, y: u32) -> bool {
        self.cell(x, y).has_pipe
    }

    /// Runs `has_vent` logic.
    pub fn has_vent(&self, x: u32, y: u32) -> bool {
        self.cell(x, y).has_vent
    }

    /// Runs `blocks_solid_placement` logic.
    pub fn blocks_solid_placement(&self, x: u32, y: u32) -> bool {
        let cell = self.cell(x, y);
        cell.has_vent
    }

    /// Runs `can_place_pipe_at` logic.
    pub fn can_place_pipe_at(
        &self,
        x: u32,
        y: u32,
        _world: &WorldGrid,
        structures: &GasStructureGrid,
    ) -> bool {
        if !is_editable_cell(x, y) || is_boundary(x, y) {
            return false;
        }
        if structures.cell(x, y).is_some() {
            return false;
        }
        true
    }

    /// Runs `can_place_vent_at` logic.
    pub fn can_place_vent_at(
        &self,
        x: u32,
        y: u32,
        world: &WorldGrid,
        structures: &GasStructureGrid,
    ) -> bool {
        if !is_editable_cell(x, y) || is_boundary(x, y) || world.is_solid(x, y) {
            return false;
        }
        if structures.cell(x, y).is_some() {
            return false;
        }
        true
    }

    /// Runs `set_pipe` logic.
    pub fn set_pipe(
        &mut self,
        x: u32,
        y: u32,
        world: &WorldGrid,
        structures: &GasStructureGrid,
    ) -> bool {
        if !self.can_place_pipe_at(x, y, world, structures) {
            return false;
        }
        let idx = linear_index(x, y);
        if self.cells[idx].has_pipe {
            return false;
        }
        self.cells[idx].has_pipe = true;
        true
    }

    /// Runs `set_vent` logic.
    pub fn set_vent(
        &mut self,
        x: u32,
        y: u32,
        world: &WorldGrid,
        structures: &GasStructureGrid,
    ) -> bool {
        if !self.can_place_vent_at(x, y, world, structures) {
            return false;
        }
        let idx = linear_index(x, y);
        if self.cells[idx].has_vent {
            return false;
        }
        self.cells[idx].has_vent = true;
        true
    }

    /// Runs `clear_cell` logic.
    pub fn clear_cell(&mut self, x: u32, y: u32) -> bool {
        let idx = linear_index(x, y);
        let previous = self.cells[idx];
        if previous == PipeCell::default() {
            return false;
        }
        self.clear_connections_for_cell(x, y);
        self.cells[idx] = PipeCell::default();
        true
    }

    /// Runs `remove_pipe_keep_vent` logic.
    pub fn remove_pipe_keep_vent(&mut self, x: u32, y: u32) -> bool {
        let idx = linear_index(x, y);
        let previous = self.cells[idx];
        if !previous.has_pipe {
            return false;
        }
        self.clear_connections_for_cell(x, y);
        self.cells[idx].has_pipe = false;
        self.cells[idx].connections = 0;
        true
    }

    /// Runs `add_connection` logic.
    pub fn add_connection(&mut self, from: UVec2, to: UVec2) -> bool {
        let Some((from_bit, to_bit)) = connection_bits(from, to) else {
            return false;
        };
        if !self.has_pipe(from.x, from.y) || !self.has_pipe(to.x, to.y) {
            return false;
        }

        let from_idx = linear_index(from.x, from.y);
        let to_idx = linear_index(to.x, to.y);
        let before_from = self.cells[from_idx].connections;
        let before_to = self.cells[to_idx].connections;
        self.cells[from_idx].connections |= from_bit;
        self.cells[to_idx].connections |= to_bit;
        before_from != self.cells[from_idx].connections
            || before_to != self.cells[to_idx].connections
    }

    /// Runs `remove_connection` logic.
    pub fn remove_connection(&mut self, from: UVec2, to: UVec2) -> bool {
        let Some((from_bit, to_bit)) = connection_bits(from, to) else {
            return false;
        };
        let from_idx = linear_index(from.x, from.y);
        let to_idx = linear_index(to.x, to.y);
        let before_from = self.cells[from_idx].connections;
        let before_to = self.cells[to_idx].connections;
        self.cells[from_idx].connections &= !from_bit;
        self.cells[to_idx].connections &= !to_bit;
        before_from != self.cells[from_idx].connections
            || before_to != self.cells[to_idx].connections
    }

    /// Runs `iter_cells` logic.
    pub fn iter_cells(&self) -> impl Iterator<Item = (u32, u32, PipeCell)> + '_ {
        self.cells.iter().enumerate().filter_map(|(idx, value)| {
            if *value == PipeCell::default() {
                return None;
            }
            let x = (idx as u32) % WORLD_WIDTH;
            let y = (idx as u32) / WORLD_WIDTH;
            Some((x, y, *value))
        })
    }

    /// Runs `connected_neighbors` logic.
    pub fn connected_neighbors(&self, x: u32, y: u32) -> impl Iterator<Item = UVec2> + '_ {
        let cell = self.cell(x, y);
        Direction::all().into_iter().filter_map(move |direction| {
            if !cell.has_pipe || (cell.connections & direction.bit()) == 0 {
                return None;
            }
            direction.step(UVec2::new(x, y))
        })
    }

    /// Runs `snapshot_state` logic.
    pub fn snapshot_state(&self) -> PipeLayoutSnapshot {
        PipeLayoutSnapshot {
            cells: self.cells.iter().copied().map(PipeCell::encoded).collect(),
        }
    }

    /// Runs `restore_state` logic.
    pub fn restore_state(
        &mut self,
        snapshot: &PipeLayoutSnapshot,
        world: &WorldGrid,
        structures: &GasStructureGrid,
    ) -> Result<(), String> {
        let expected = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        if snapshot.cells.len() != expected {
            return Err(format!(
                "Pipe layout snapshot length mismatch: got {}, expected {}",
                snapshot.cells.len(),
                expected
            ));
        }

        let mut next = vec![PipeCell::default(); expected];
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let idx = linear_index(x, y);
                let cell = PipeCell::from_encoded(snapshot.cells[idx]);
                if (cell.has_pipe || cell.has_vent)
                    && (!is_editable_cell(x, y) || structures.cell(x, y).is_some())
                {
                    return Err(format!(
                        "Pipe layout snapshot has pipe content in blocked cell ({}, {})",
                        x, y
                    ));
                }
                if cell.has_vent && world.is_solid(x, y) {
                    return Err(format!(
                        "Pipe layout snapshot has vent in solid cell ({}, {})",
                        x, y
                    ));
                }
                if !cell.has_pipe && cell.connections != 0 {
                    return Err(format!(
                        "Pipe layout snapshot contains connections without pipe at ({}, {})",
                        x, y
                    ));
                }
                next[idx] = cell;
            }
        }

        self.cells = next;
        self.validate_connections()
    }

    fn clear_connections_for_cell(&mut self, x: u32, y: u32) {
        let cell = self.cell(x, y);
        for direction in Direction::all() {
            if (cell.connections & direction.bit()) == 0 {
                continue;
            }
            if let Some(neighbor) = direction.step(UVec2::new(x, y)) {
                let neighbor_idx = linear_index(neighbor.x, neighbor.y);
                self.cells[neighbor_idx].connections &= !direction.opposite().bit();
            }
        }
        let idx = linear_index(x, y);
        self.cells[idx].connections = 0;
    }

    fn validate_connections(&self) -> Result<(), String> {
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let cell = self.cell(x, y);
                for direction in Direction::all() {
                    if (cell.connections & direction.bit()) == 0 {
                        continue;
                    }
                    if !cell.has_pipe {
                        return Err(format!(
                            "Pipe layout has connection without pipe at ({}, {})",
                            x, y
                        ));
                    }
                    let Some(neighbor) = direction.step(UVec2::new(x, y)) else {
                        return Err(format!(
                            "Pipe layout has out-of-bounds connection at ({}, {})",
                            x, y
                        ));
                    };
                    let neighbor_cell = self.cell(neighbor.x, neighbor.y);
                    if !neighbor_cell.has_pipe
                        || (neighbor_cell.connections & direction.opposite().bit()) == 0
                    {
                        return Err(format!(
                            "Pipe layout has non-symmetric connection between ({}, {}) and ({}, {})",
                            x, y, neighbor.x, neighbor.y
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    fn all() -> [Self; 4] {
        [Self::Up, Self::Right, Self::Down, Self::Left]
    }

    fn bit(self) -> u8 {
        match self {
            Self::Up => DIR_UP,
            Self::Right => DIR_RIGHT,
            Self::Down => DIR_DOWN,
            Self::Left => DIR_LEFT,
        }
    }

    fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Right => Self::Left,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
        }
    }

    fn step(self, cell: UVec2) -> Option<UVec2> {
        match self {
            Self::Up if cell.y > 0 => Some(UVec2::new(cell.x, cell.y - 1)),
            Self::Right if cell.x + 1 < WORLD_WIDTH => Some(UVec2::new(cell.x + 1, cell.y)),
            Self::Down if cell.y + 1 < WORLD_HEIGHT => Some(UVec2::new(cell.x, cell.y + 1)),
            Self::Left if cell.x > 0 => Some(UVec2::new(cell.x - 1, cell.y)),
            _ => None,
        }
    }
}

fn connection_bits(from: UVec2, to: UVec2) -> Option<(u8, u8)> {
    match (to.x as i32 - from.x as i32, to.y as i32 - from.y as i32) {
        (0, -1) => Some((DIR_UP, DIR_DOWN)),
        (1, 0) => Some((DIR_RIGHT, DIR_LEFT)),
        (0, 1) => Some((DIR_DOWN, DIR_UP)),
        (-1, 0) => Some((DIR_LEFT, DIR_RIGHT)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{PipeCell, PipeGrid};
    use crate::world::{
        gas_structures::GasStructureGrid,
        grid::{CellMaterial, WorldGrid},
    };
    use bevy::prelude::UVec2;

    #[test]
    fn pipe_and_vent_follow_placement_rules() {
        let mut grid = PipeGrid::default();
        let mut world = WorldGrid::default();
        let mut structures = GasStructureGrid::default();

        assert!(grid.set_pipe(10, 10, &world, &structures));
        assert!(grid.set_vent(10, 10, &world, &structures));
        assert_eq!(
            grid.cell(10, 10),
            PipeCell {
                has_pipe: true,
                connections: 0,
                has_vent: true,
            }
        );

        assert!(world.set_solid_with_material(11, 10, CellMaterial::Brick));
        assert!(
            grid.set_pipe(11, 10, &world, &structures),
            "solid cell must allow pipe"
        );
        assert!(
            !grid.set_vent(11, 10, &world, &structures),
            "solid cell must still reject vent"
        );

        assert!(structures.set_sink(12, 10, 5, &world));
        assert!(
            !grid.set_vent(12, 10, &world, &structures),
            "legacy gas structure cell must reject vent"
        );
    }

    #[test]
    fn solid_placement_is_blocked_only_by_vent() {
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut grid = PipeGrid::default();

        assert!(!grid.blocks_solid_placement(14, 14));
        assert!(grid.set_pipe(14, 14, &world, &structures));
        assert!(
            !grid.blocks_solid_placement(14, 14),
            "pipe alone must not block solid placement"
        );
        assert!(grid.set_pipe(15, 14, &world, &structures));
        assert!(grid.set_vent(15, 14, &world, &structures));
        assert!(
            grid.blocks_solid_placement(15, 14),
            "vent must still block solid placement"
        );
    }

    #[test]
    fn connections_are_symmetric_and_clear_with_pipe_removal() {
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut grid = PipeGrid::default();
        assert!(grid.set_pipe(20, 20, &world, &structures));
        assert!(grid.set_pipe(21, 20, &world, &structures));
        assert!(grid.add_connection(UVec2::new(20, 20), UVec2::new(21, 20)));
        assert_eq!(grid.cell(20, 20).connections, 0b0010);
        assert_eq!(grid.cell(21, 20).connections, 0b1000);

        assert!(grid.remove_pipe_keep_vent(20, 20));
        assert_eq!(grid.cell(20, 20).connections, 0);
        assert_eq!(grid.cell(21, 20).connections, 0);
    }

    #[test]
    fn snapshot_roundtrip_preserves_layout() {
        let world = WorldGrid::default();
        let structures = GasStructureGrid::default();
        let mut grid = PipeGrid::default();
        assert!(grid.set_pipe(30, 30, &world, &structures));
        assert!(grid.set_pipe(30, 31, &world, &structures));
        assert!(grid.set_vent(30, 30, &world, &structures));
        assert!(grid.add_connection(UVec2::new(30, 30), UVec2::new(30, 31)));

        let snapshot = grid.snapshot_state();
        let mut restored = PipeGrid::default();
        restored
            .restore_state(&snapshot, &world, &structures)
            .expect("valid pipe snapshot");
        assert_eq!(restored.cell(30, 30), grid.cell(30, 30));
        assert_eq!(restored.cell(30, 31), grid.cell(30, 31));
    }
}
