pub mod gas_structures;
pub mod grid;
pub mod pipes;
pub mod structures;

use bevy::prelude::*;

use self::grid::WorldGrid;
use self::structures::PlacedStructureMap;

#[derive(Event, Clone, Copy, Debug)]
/// Stores `WorldCellChanged` state.
pub struct WorldCellChanged {
    pub cell: UVec2,
}

/// Stores `WorldPlugin` state.
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldGrid>()
            .init_resource::<PlacedStructureMap>()
            .add_event::<WorldCellChanged>();
    }
}
