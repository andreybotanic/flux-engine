pub mod world_view;

use bevy::prelude::*;

use self::world_view::{
    apply_overlay_mode, draw_cursor_grid_overlay, setup_simulation_images, setup_world_view,
    sync_gas_display_texture, sync_gas_structure_visuals, sync_structure_edit_highlight,
    sync_wall_visuals, update_overlay_mode, GasStructureEntities, WallEntities,
};
use crate::input::camera::spawn_main_camera;

/// Stores `RenderPlugin` state.
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OverlayMode>()
            .init_resource::<GasVisualSettings>()
            .init_resource::<WallEntities>()
            .init_resource::<GasStructureEntities>()
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
                    sync_gas_structure_visuals,
                    sync_structure_edit_highlight,
                    sync_gas_display_texture,
                    draw_cursor_grid_overlay,
                ),
            );
    }
}

pub use world_view::{GasVisualSettings, OverlayMode};
