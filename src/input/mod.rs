pub mod camera;

use bevy::prelude::*;

use self::camera::{camera_pan_zoom, spawn_main_camera};

/// Stores `InputPlugin` state.
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_main_camera)
            .add_systems(Update, camera_pan_zoom);
    }
}
