pub mod world_view;

use bevy::prelude::*;

use self::world_view::{
    apply_overlay_mode, setup_simulation_images, setup_world_view, sync_gas_display_texture,
    sync_wall_visuals, update_overlay_mode, WallEntities,
};
use crate::input::camera::spawn_main_camera;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OverlayMode>()
            .init_resource::<GasVisualSettings>()
            .init_resource::<WallEntities>()
            .add_systems(
                Startup,
                (
                    setup_simulation_images,
                    setup_world_view.after(setup_simulation_images),
                )
                    .after(spawn_main_camera),
            )
            .add_systems(
                Update,
                (
                    update_overlay_mode,
                    apply_overlay_mode,
                    sync_wall_visuals,
                    sync_gas_display_texture,
                ),
            );
    }
}

pub use world_view::{GasVisualSettings, OverlayMode};
