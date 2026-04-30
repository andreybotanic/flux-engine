pub mod grid;

use bevy::prelude::*;

use self::grid::WorldGrid;

#[derive(Event, Clone, Copy, Debug)]
pub struct WorldCellChanged {
    pub cell: UVec2,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldGrid>()
            .add_event::<WorldCellChanged>();
    }
}
