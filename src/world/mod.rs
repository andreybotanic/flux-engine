pub mod gas_structures;
pub mod grid;
pub mod pipes;

use bevy::prelude::*;

use self::gas_structures::GasStructureGrid;
use self::grid::WorldGrid;
use self::pipes::PipeGrid;

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
            .init_resource::<GasStructureGrid>()
            .init_resource::<PipeGrid>()
            .add_event::<WorldCellChanged>();
    }
}
