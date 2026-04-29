pub mod grid;

use bevy::prelude::*;

use self::grid::WorldGrid;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldGrid>();
    }
}
