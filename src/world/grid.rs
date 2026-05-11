use bevy::prelude::*;

pub const WORLD_WIDTH: u32 = 102;
pub const WORLD_HEIGHT: u32 = 102;
pub const CELL_SIZE: f32 = 16.0;
pub const CAMERA_MARGIN: f32 = 160.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellKind {
    Solid(CellMaterial),
    Empty,
}

/// Identifies one solid world-cell content item by its stable content id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellMaterial {
    id: &'static str,
}

impl CellMaterial {
    /// Builds a cell material id from a registered static content id.
    pub const fn new(id: &'static str) -> Self {
        Self { id }
    }

    /// Builds a cell material id from runtime-registered content.
    pub fn from_registered_id(id: String) -> Self {
        Self {
            id: Box::leak(id.into_boxed_str()),
        }
    }

    /// Returns the stable content id backing this material.
    pub fn as_str(self) -> &'static str {
        self.id
    }
}

#[derive(Resource, Clone)]
/// Stores `WorldGrid` state.
pub struct WorldGrid {
    cells: Vec<CellKind>,
}

impl Default for WorldGrid {
    fn default() -> Self {
        Self::new(crate::plugins::default_plugin::boundary_cell_material())
    }
}

impl WorldGrid {
    /// Builds a world grid and fills its perimeter with the provided boundary material.
    pub fn new(boundary_material: CellMaterial) -> Self {
        let mut cells = vec![CellKind::Empty; (WORLD_WIDTH * WORLD_HEIGHT) as usize];
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if is_boundary(x, y) {
                    let index = linear_index(x, y);
                    cells[index] = CellKind::Solid(boundary_material);
                }
            }
        }
        Self { cells }
    }
    /// Runs `cell` logic.
    pub fn cell(&self, x: u32, y: u32) -> CellKind {
        self.cells[linear_index(x, y)]
    }

    /// Runs `is_solid` logic.
    pub fn is_solid(&self, x: u32, y: u32) -> bool {
        matches!(self.cell(x, y), CellKind::Solid(_))
    }

    /// Runs `solid_material` logic.
    pub fn solid_material(&self, x: u32, y: u32) -> Option<CellMaterial> {
        match self.cell(x, y) {
            CellKind::Solid(material) => Some(material),
            CellKind::Empty => None,
        }
    }

    /// Runs `set_cell_kind` logic.
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

    /// Runs `set_solid` logic.
    pub fn set_solid(&mut self, x: u32, y: u32) -> bool {
        self.set_solid_with_material(x, y, crate::plugins::default_plugin::brick_cell_material())
    }

    /// Runs `set_solid_with_material` logic.
    pub fn set_solid_with_material(&mut self, x: u32, y: u32, material: CellMaterial) -> bool {
        self.set_cell_kind(x, y, CellKind::Solid(material))
    }

    /// Runs `set_empty` logic.
    pub fn set_empty(&mut self, x: u32, y: u32) -> bool {
        self.set_cell_kind(x, y, CellKind::Empty)
    }

    /// Runs `snapshot_cells` logic.
    pub fn snapshot_cells(&self) -> Vec<CellKind> {
        self.cells.clone()
    }

    /// Runs `restore_cells` logic.
    pub fn restore_cells(&mut self, cells: &[CellKind]) -> Result<(), String> {
        let expected = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        if cells.len() != expected {
            return Err(format!(
                "World cell snapshot length mismatch: got {}, expected {}",
                cells.len(),
                expected
            ));
        }
        validate_boundary_cells(cells)?;
        self.cells.clear();
        self.cells.extend_from_slice(cells);
        Ok(())
    }

    /// Runs `snapshot_cell_codes` logic.
    pub fn snapshot_cell_codes(&self) -> Vec<u8> {
        self.cells
            .iter()
            .map(|cell| encode_cell_kind(*cell))
            .collect()
    }

    /// Runs `restore_from_cell_codes` logic.
    pub fn restore_from_cell_codes(&mut self, codes: &[u8]) -> Result<(), String> {
        let expected = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        if codes.len() != expected {
            return Err(format!(
                "World cell code snapshot length mismatch: got {}, expected {}",
                codes.len(),
                expected
            ));
        }

        let mut decoded = Vec::with_capacity(codes.len());
        for (idx, code) in codes.iter().copied().enumerate() {
            let cell = decode_cell_kind(code).ok_or_else(|| {
                format!(
                    "World cell code snapshot contains invalid code {} at index {}",
                    code, idx
                )
            })?;
            decoded.push(cell);
        }
        self.restore_cells(&decoded)
    }
}

fn encode_cell_kind(cell: CellKind) -> u8 {
    crate::plugins::default_plugin::legacy_cell_kind_code(cell).unwrap_or(0)
}

fn decode_cell_kind(code: u8) -> Option<CellKind> {
    crate::plugins::default_plugin::legacy_cell_kind_from_code(code)
}

fn validate_boundary_cells(cells: &[CellKind]) -> Result<(), String> {
    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            if !is_boundary(x, y) {
                continue;
            }
            let index = linear_index(x, y);
            if !crate::plugins::default_plugin::is_boundary_cell_kind(cells[index]) {
                return Err(format!(
                    "Boundary cell ({}, {}) must be Boundary in snapshot",
                    x, y
                ));
            }
        }
    }
    Ok(())
}

/// Runs `is_boundary` logic.
pub fn is_boundary(x: u32, y: u32) -> bool {
    x == 0 || y == 0 || x == WORLD_WIDTH - 1 || y == WORLD_HEIGHT - 1
}

/// Runs `is_editable_cell` logic.
pub fn is_editable_cell(x: u32, y: u32) -> bool {
    !is_boundary(x, y)
}

/// Runs `linear_index` logic.
pub fn linear_index(x: u32, y: u32) -> usize {
    (y * WORLD_WIDTH + x) as usize
}

/// Runs `world_dimensions` logic.
pub fn world_dimensions() -> Vec2 {
    Vec2::new(
        WORLD_WIDTH as f32 * CELL_SIZE,
        WORLD_HEIGHT as f32 * CELL_SIZE,
    )
}

/// Runs `world_origin` logic.
pub fn world_origin() -> Vec2 {
    -world_dimensions() * 0.5
}

/// Runs `cell_center` logic.
pub fn cell_center(x: u32, y: u32) -> Vec2 {
    world_origin() + Vec2::new((x as f32 + 0.5) * CELL_SIZE, (y as f32 + 0.5) * CELL_SIZE)
}

/// Runs `world_to_cell` logic.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_cells_are_technical_material_and_not_editable() {
        let mut world = WorldGrid::default();
        assert_eq!(
            world.solid_material(0, 0),
            Some(crate::plugins::default_plugin::boundary_cell_material()),
            "Boundary cell should be initialized as technical boundary material"
        );
        assert!(
            !world.set_empty(0, 0),
            "Boundary cell must remain non-editable for tools"
        );
        assert!(
            !world.set_solid_with_material(
                0,
                0,
                crate::plugins::default_plugin::metal_cell_material()
            ),
            "Boundary cell material must not be replaced by editor tools"
        );
        assert_eq!(
            world.solid_material(0, 0),
            Some(crate::plugins::default_plugin::boundary_cell_material()),
            "Boundary material should stay unchanged"
        );
    }

    #[test]
    fn editable_solid_cells_store_selected_material() {
        let mut world = WorldGrid::default();
        assert!(world.set_solid_with_material(
            10,
            10,
            crate::plugins::default_plugin::metal_cell_material()
        ));
        assert_eq!(
            world.solid_material(10, 10),
            Some(crate::plugins::default_plugin::metal_cell_material())
        );
        assert!(world.set_solid_with_material(
            10,
            10,
            crate::plugins::default_plugin::brick_cell_material()
        ));
        assert_eq!(
            world.solid_material(10, 10),
            Some(crate::plugins::default_plugin::brick_cell_material())
        );
    }

    #[test]
    fn snapshot_codes_roundtrip_world_cells() {
        let mut world = WorldGrid::default();
        assert!(world.set_solid_with_material(
            10,
            10,
            crate::plugins::default_plugin::brick_cell_material()
        ));
        assert!(world.set_solid_with_material(
            11,
            10,
            crate::plugins::default_plugin::metal_cell_material()
        ));
        let _ = world.set_empty(12, 10);

        let codes = world.snapshot_cell_codes();
        let mut restored = WorldGrid::default();
        restored
            .restore_from_cell_codes(&codes)
            .expect("restore from valid snapshot");

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                assert_eq!(world.cell(x, y), restored.cell(x, y));
            }
        }
    }

    #[test]
    fn restore_rejects_non_boundary_border_cells() {
        let mut world = WorldGrid::default();
        let mut cells = world.snapshot_cells();
        cells[linear_index(0, 0)] = CellKind::Empty;
        let err = world
            .restore_cells(&cells)
            .expect_err("must reject invalid boundary");
        assert!(err.contains("Boundary cell"));
    }
}
