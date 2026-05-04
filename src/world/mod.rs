pub mod grid;
pub mod gas_structures;

use bevy::prelude::*;

use self::grid::WorldGrid;
use self::gas_structures::GasStructureGrid;

#[derive(Event, Clone, Copy, Debug)]
pub struct WorldCellChanged {
    pub cell: UVec2,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldGrid>()
            .init_resource::<GasStructureGrid>()
            .add_event::<WorldCellChanged>();
    }
}
