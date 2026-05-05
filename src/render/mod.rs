mod pipe_highlight_material;
pub mod world_view;

use bevy::prelude::*;
use pipe_highlight_material::PipeHighlightMaterialPlugin;

use self::world_view::{
    apply_overlay_mode, apply_overlay_visibility_mode, draw_cursor_grid_overlay,
    setup_simulation_images, setup_world_view, sync_gas_display_texture,
    sync_gas_structure_visuals, sync_pipe_flow_packets, sync_pipe_overlay_visuals,
    sync_pipe_world_visuals, sync_structure_edit_highlight, sync_wall_visuals, update_overlay_mode,
    GasStructureEntities, PipeEntities, WallEntities,
};
use crate::input::camera::spawn_main_camera;

/// Stores `RenderPlugin` state.
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PipeHighlightMaterialPlugin)
            .init_resource::<OverlayMode>()
            .init_resource::<GasVisualSettings>()
            .init_resource::<WallEntities>()
            .init_resource::<GasStructureEntities>()
            .init_resource::<PipeEntities>()
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
                    apply_overlay_visibility_mode,
                    sync_wall_visuals,
                    sync_gas_structure_visuals,
                    sync_pipe_world_visuals,
                    sync_structure_edit_highlight,
                    sync_gas_display_texture,
                    sync_pipe_overlay_visuals,
                ),
            )
            .add_systems(Update, (sync_pipe_flow_packets, draw_cursor_grid_overlay));
    }
}

pub use world_view::{GasVisualSettings, OverlayMode};
