use bevy::prelude::*;

pub const WORLD_WIDTH: u32 = 102;
pub const WORLD_HEIGHT: u32 = 102;
pub const CELL_SIZE: f32 = 16.0;
pub const CAMERA_MARGIN: f32 = 160.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellKind {
    Solid,
    Empty,
}

#[derive(Resource, Clone)]
pub struct WorldGrid {
    cells: Vec<CellKind>,
}

impl Default for WorldGrid {
    fn default() -> Self {
        let mut cells = vec![CellKind::Empty; (WORLD_WIDTH * WORLD_HEIGHT) as usize];
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if is_boundary(x, y) {
                    let index = linear_index(x, y);
                    cells[index] = CellKind::Solid;
                }
            }
        }
        Self { cells }
    }
}

impl WorldGrid {
    pub fn cell(&self, x: u32, y: u32) -> CellKind {
        self.cells[linear_index(x, y)]
    }

    pub fn is_solid(&self, x: u32, y: u32) -> bool {
        self.cell(x, y) == CellKind::Solid
    }

    pub fn set_cell_kind(&mut self, x: u32, y: u32, kind: CellKind) -> bool {
        if !is_editable_cell(x, y) {
            return false;
        }

        let index = linear_index(x, y);
        if self.cells[index] == kind {
            return false;
        }

        self.cells[index] = kind;
        true
    }

    pub fn set_solid(&mut self, x: u32, y: u32) -> bool {
        self.set_cell_kind(x, y, CellKind::Solid)
    }

    pub fn set_empty(&mut self, x: u32, y: u32) -> bool {
        self.set_cell_kind(x, y, CellKind::Empty)
    }
}

pub fn is_boundary(x: u32, y: u32) -> bool {
    x == 0 || y == 0 || x == WORLD_WIDTH - 1 || y == WORLD_HEIGHT - 1
}

pub fn is_editable_cell(x: u32, y: u32) -> bool {
    !is_boundary(x, y)
}

pub fn linear_index(x: u32, y: u32) -> usize {
    (y * WORLD_WIDTH + x) as usize
}

pub fn world_dimensions() -> Vec2 {
    Vec2::new(
        WORLD_WIDTH as f32 * CELL_SIZE,
        WORLD_HEIGHT as f32 * CELL_SIZE,
    )
}

pub fn world_origin() -> Vec2 {
    -world_dimensions() * 0.5
}

pub fn cell_center(x: u32, y: u32) -> Vec2 {
    world_origin() + Vec2::new((x as f32 + 0.5) * CELL_SIZE, (y as f32 + 0.5) * CELL_SIZE)
}

pub fn world_to_cell(world_position: Vec2) -> Option<UVec2> {
    let local = world_position - world_origin();
    if local.x < 0.0 || local.y < 0.0 {
        return None;
    }

    let x = (local.x / CELL_SIZE).floor() as i32;
    let y = (local.y / CELL_SIZE).floor() as i32;

    if x < 0 || y < 0 || x >= WORLD_WIDTH as i32 || y >= WORLD_HEIGHT as i32 {
        return None;
    }

    Some(UVec2::new(x as u32, y as u32))
}
